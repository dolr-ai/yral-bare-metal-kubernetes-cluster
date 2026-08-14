use auth::{
    extract_identity, generate_anonymous_identity_if_required, set_anonymous_identity_cookie,
    AnonymousIdentity,
};
use candid::Principal;
use codee::string::FromToStringCodec;
use consts::{
    auth::REFRESH_MAX_AGE, ACCOUNT_CONNECTED_STORE, AUTH_UTIL_COOKIES_MAX_AGE_MS,
    USER_CANISTER_ID_STORE, USER_PRINCIPAL_STORE,
};
use futures::FutureExt;
use ic_agent::identity::DelegatedIdentity;
use ic_agent::Identity;
use leptos::prelude::*;
use leptos_use::{use_cookie_with_options, UseCookieOptions};
use serde::{Deserialize, Serialize};
use std::future::{Future, IntoFuture};
use std::sync::Arc;
use types::delegated_identity::DelegatedIdentityWire;
use utils::user_identity::ProfileDetails;
use utils::UserAuthInfo;
use utils::{types::NewIdentity, MockPartialEq};

/// The authenticated user session. Replaces `Canisters<true>` from
/// yral-canisters-common. Holds the IC delegated identity (for principal
/// derivation and signing) and the user's profile (from SpacetimeDB).
#[derive(Clone)]
pub struct AuthSession {
    identity: Arc<DelegatedIdentity>,
    id_wire: Arc<DelegatedIdentityWire>,
    user_principal: Principal,
    user_canister: Principal,
    profile: ProfileDetails,
}

impl AuthSession {
    pub fn user_principal(&self) -> Principal {
        self.user_principal
    }

    pub fn user_canister(&self) -> Principal {
        self.user_canister
    }

    pub fn profile_details(&self) -> ProfileDetails {
        self.profile.clone()
    }

    pub fn user_identity(&self) -> utils::user_identity::UserIdentity {
        utils::user_identity::UserIdentity::from(self.profile.clone())
    }

    pub fn identity(&self) -> &DelegatedIdentity {
        &self.identity
    }

    pub fn id_wire(&self) -> &DelegatedIdentityWire {
        &self.id_wire
    }

    /// Construct a new AuthSession from its components.
    pub fn new(
        identity: Arc<DelegatedIdentity>,
        id_wire: Arc<DelegatedIdentityWire>,
        user_principal: Principal,
        user_canister: Principal,
        profile: ProfileDetails,
    ) -> Self {
        Self {
            identity,
            id_wire,
            user_principal,
            user_canister,
            profile,
        }
    }
}

impl UserAuthInfo for AuthSession {
    fn user_principal(&self) -> Principal {
        self.user_principal
    }

    fn user_canister(&self) -> Principal {
        self.user_canister
    }

    fn user_identity(&self) -> utils::user_identity::UserIdentity {
        utils::user_identity::UserIdentity::from(self.profile.clone())
    }
}

async fn do_canister_auth(
    auth: DelegatedIdentityWire,
    _fallback_username: Option<String>,
) -> Result<AuthSession, ServerFnError> {
    // Reconstruct the IC delegated identity from the wire.
    // This is needed for user_principal() derivation and cryptographic
    // signing (referrals, notifications). No IC canister calls are made.
    let id: DelegatedIdentity = auth
        .clone()
        .try_into()
        .map_err(|e| ServerFnError::new(format!("Failed to reconstruct identity: {e}")))?;
    let id = Arc::new(id);
    let id_wire = Arc::new(auth);

    let user_principal = id.sender().expect("expect principal to be present");
    let principal_text = user_principal.to_text();

    leptos::logging::log!("Authenticating user with principal {principal_text}");

    // Fetch profile from SpacetimeDB.
    #[cfg(feature = "ssr")]
    {
        use tokio::sync::oneshot;
        use yral_database_spacetime_bindings::get_user_profile_details_v_7;

        let conn = crate::spacetime::spacetime_conn();
        let (tx, rx) = oneshot::channel();
        conn.procedures.get_user_profile_details_v_7_then(
            principal_text.clone(),
            move |_ctx, result| {
                let _ = tx.send(result.ok().flatten());
            },
        );

        let profile = if let Some(p) = rx.await.unwrap_or(None) {
            ProfileDetails {
                username: None, // username comes from metadata service
                lifetime_earnings: 0,
                followers_cnt: p.followers_count,
                following_cnt: p.following_count,
                profile_pic: p.profile_picture.as_ref().map(|pic| pic.url.clone()),
                display_name: None,
                user_identifier: user_principal.to_text(),
                hots: 0,
                nots: 0,
                bio: if p.bio.is_empty() {
                    None
                } else {
                    Some(p.bio.clone())
                },
                website_url: if p.website_url.is_empty() {
                    None
                } else {
                    Some(p.website_url.clone())
                },
                caller_follows_user: p.caller_follows_user,
                user_follows_caller: p.user_follows_caller,
            }
        } else {
            // New user — default profile.
            ProfileDetails {
                username: None,
                lifetime_earnings: 0,
                followers_cnt: 0,
                following_cnt: 0,
                profile_pic: None,
                display_name: None,
                user_identifier: user_principal.to_text(),
                hots: 0,
                nots: 0,
                bio: None,
                website_url: None,
                caller_follows_user: None,
                user_follows_caller: None,
            }
        };

        let session = AuthSession {
            identity: id,
            id_wire,
            user_principal,
            user_canister: user_principal,
            profile,
        };

        Ok(session)
    }

    #[cfg(not(feature = "ssr"))]
    {
        // Hydrate: profile was serialized from SSR pass.
        // Return a default — the actual profile data is available via SSR state.
        let profile = ProfileDetails {
            username: None,
            lifetime_earnings: 0,
            followers_cnt: 0,
            following_cnt: 0,
            profile_pic: None,
            display_name: None,
            user_identifier: user_principal.to_text(),
            hots: 0,
            nots: 0,
            bio: None,
            website_url: None,
            caller_follows_user: None,
            user_follows_caller: None,
        };
        Ok(AuthSession {
            identity: id,
            id_wire,
            user_principal,
            user_canister: user_principal,
            profile,
        })
    }
}
type AuthCansResource = LocalResource<Result<AuthSession, ServerFnError>>;

/// The Authenticated Canisters helper resource
/// prefer using helpers from [crate::component::canisters_prov]
/// instead
pub fn auth_state() -> AuthState {
    expect_context()
}

#[derive(Copy, Clone)]
pub struct AuthState {
    _temp_identity_resource: OnceResource<Option<AnonymousIdentity>>,
    _temp_id_cookie_resource: LocalResource<()>,
    is_logged_in_with_oauth: (Signal<Option<bool>>, WriteSignal<Option<bool>>),
    new_identity_setter: RwSignal<Option<NewIdentity>>,
    pub canisters_resource: AuthCansResource,
    user_canister_id_cookie: (Signal<Option<Principal>>, WriteSignal<Option<Principal>>),
    pub user_principal: Resource<Result<Principal, ServerFnError>>,
    user_principal_cookie: (Signal<Option<Principal>>, WriteSignal<Option<Principal>>),
    pub user_identity: Resource<Result<NewIdentity, ServerFnError>>,
    new_cans_setter: RwSignal<Option<AuthSession>>,
}

impl Default for AuthState {
    fn default() -> Self {
        // Super complex, don't mess with this.

        let temp_identity_resource = OnceResource::new(async move {
            generate_anonymous_identity_if_required()
                .await
                .expect("Failed to generate anonymous identity?!")
        });
        let temp_id_cookie_resource = LocalResource::new(move || async move {
            let Some(temp_identity) = temp_identity_resource.await else {
                return;
            };
            if let Err(e) = set_anonymous_identity_cookie(temp_identity.refresh_token).await {
                log::error!("Failed to set anonymous identity as cookie?! err {e}");
            }
        });

        let is_logged_in_with_oauth = use_cookie_with_options::<bool, FromToStringCodec>(
            ACCOUNT_CONNECTED_STORE,
            UseCookieOptions::default()
                .path("/")
                .max_age(REFRESH_MAX_AGE.as_millis() as i64),
        );

        let new_identity_setter = RwSignal::new(None::<NewIdentity>);

        let user_identity_resource = Resource::new(
            move || MockPartialEq(new_identity_setter.get()),
            move |auth_id| async move {
                let temp_identity = temp_identity_resource.await;

                if let Some(id_wire) = auth_id.0 {
                    return Ok::<_, ServerFnError>(id_wire);
                }

                let Some(id) = temp_identity else {
                    let id_wire = match extract_identity().await {
                        Ok(Some(identity)) => identity,
                        Ok(None) => return Err(ServerFnError::new("No refresh cookie set?!")),
                        Err(e) => {
                            return Err(ServerFnError::new(e.to_string()));
                        }
                    };
                    return Ok(NewIdentity {
                        id_wire,
                        fallback_username: None,
                        email: None,
                    });
                };

                Ok(NewIdentity {
                    id_wire: id.identity,
                    fallback_username: None,
                    email: None,
                })
            },
        );

        let new_cans_setter = RwSignal::new(None::<AuthSession>);

        let canisters_resource: AuthCansResource = LocalResource::new(move || {
            user_identity_resource.track();
            let new_cans = new_cans_setter.get();
            async move {
                let new_id = user_identity_resource.await?;
                match new_cans {
                    Some(cans)
                        if cans.user_principal()
                            == Principal::self_authenticating(&new_id.id_wire.from_key) =>
                    {
                        return Ok::<_, ServerFnError>(cans);
                    }
                    // this means that the user did the following:
                    // 1. Changed their username, then
                    // 2. Logged in with oauth (or logged out)
                    _ => {}
                };

                let res = do_canister_auth(new_id.id_wire, new_id.fallback_username).await?;

                Ok::<_, ServerFnError>(res)
            }
        });

        let user_principal_cookie = use_cookie_with_options::<Principal, FromToStringCodec>(
            USER_PRINCIPAL_STORE,
            UseCookieOptions::default()
                .path("/")
                .max_age(AUTH_UTIL_COOKIES_MAX_AGE_MS),
        );
        let user_principal = Resource::new(
            move || {
                user_identity_resource.track();
                MockPartialEq(())
            },
            move |_| async move {
                if let Some(princ) = user_principal_cookie.0.try_get_untracked().flatten() {
                    return Ok(princ);
                }

                let id_wire = user_identity_resource.await?;
                let princ = Principal::self_authenticating(&id_wire.id_wire.from_key);
                user_principal_cookie.1.set(Some(princ));

                Ok(princ)
            },
        );

        let user_canister_id_cookie = use_cookie_with_options::<Principal, FromToStringCodec>(
            USER_CANISTER_ID_STORE,
            UseCookieOptions::default()
                .path("/")
                .max_age(AUTH_UTIL_COOKIES_MAX_AGE_MS),
        );

        Self {
            _temp_identity_resource: temp_identity_resource,
            _temp_id_cookie_resource: temp_id_cookie_resource,
            is_logged_in_with_oauth,
            new_identity_setter,
            canisters_resource,
            user_principal,
            user_principal_cookie,
            user_canister_id_cookie,
            user_identity: user_identity_resource,
            new_cans_setter,
        }
    }
}

impl AuthState {
    pub fn is_logged_in_with_oauth(&self) -> Signal<bool> {
        let logged_in = self.is_logged_in_with_oauth.0;
        Signal::derive(move || logged_in.get().unwrap_or_default())
    }

    /// Updates the identity
    /// fallback_username will be the username of this id
    /// if not already set
    pub fn set_new_identity(&self, new_identity: NewIdentity, is_logged_in_with_oauth: bool) {
        self.is_logged_in_with_oauth
            .1
            .set(Some(is_logged_in_with_oauth));

        self.user_canister_id_cookie.1.set(None);
        self.user_principal_cookie
            .1
            .set(Some(Principal::self_authenticating(
                &new_identity.id_wire.from_key,
            )));
        self.new_identity_setter.set(Some(new_identity));
    }

    pub async fn set_new_identity_and_wait_for_authentication(
        &self,
        new_identity: NewIdentity,
        is_logged_in_with_oauth: bool,
    ) -> Result<AuthSession, ServerFnError> {
        self.set_new_identity(new_identity, is_logged_in_with_oauth);
        self.canisters_resource.ready().await;

        self.auth_cans().await
    }

    /// WARN: This function MUST be used with `<Suspense>`, if used inside view! {}
    /// this also tracks any changes made to user's identity, if used with <Suspend>
    pub async fn auth_cans(&self) -> Result<AuthSession, ServerFnError> {
        self.canisters_resource.await
    }

    /// Get the user principal if loaded
    /// does not have any tracking
    /// NOT RECOMMENDED TO BE USED IN DOM
    pub fn user_principal_if_available(&self) -> Option<Principal> {
        self.user_principal_cookie.0.get_untracked()
    }

    /// WARN: This function MUST be used with `<Suspense>`, if used inside view! {}
    /// this also tracks any changes made to user's identity, if used with <Suspend>
    pub async fn user_canister(&self) -> Result<Principal, ServerFnError> {
        if let Some(canister_id) = self.user_canister_id_cookie.0.get_untracked() {
            return Ok(canister_id);
        }

        let cans_wire = self.canisters_resource.await?;

        let canister_id = cans_wire.user_canister();
        self.user_canister_id_cookie.1.set(Some(canister_id));

        Ok(canister_id)
    }

    /// Get the user canister if loaded
    /// does not have any tracking
    /// NOT RECOMMENDED TO BE USED IN DOM
    pub fn user_canister_if_available(&self) -> Option<Principal> {
        self.user_canister_id_cookie.0.get_untracked()
    }

    pub fn derive_resource<
        S: Clone + 'static,
        D: Serialize + for<'x> Deserialize<'x> + 'static,
        DFut: Future<Output = Result<D, ServerFnError>> + 'static,
    >(
        &self,
        tracker: impl Fn() -> S + 'static,
        fetcher: impl Fn(AuthSession, S) -> DFut + 'static + Clone,
    ) -> LocalResource<Result<D, ServerFnError>> {
        let cans = self.canisters_resource;
        LocalResource::new(move || {
            cans.track();
            let state = tracker();
            let fetcher = fetcher.clone();
            async move {
                let cans = cans.await?;
                fetcher(cans, state).await
            }
        })
    }

    /// WARN: Use this very carefully, this function only exists for very fine-tuned optimizations
    /// for critical pages
    /// this definitely must not be used in DOM
    /// this always be `None` for ssr
    pub fn auth_cans_if_available(&self) -> Option<AuthSession> {
        #[cfg(not(feature = "hydrate"))]
        {
            None
        }

        #[cfg(feature = "hydrate")]
        self.canisters_resource
            .into_future()
            .now_or_never()
            .and_then(|c| c.ok())
    }

    /// Update the username of the user
    /// WARN: all subscribers to the canisters resource will be notified
    pub async fn update_username(
        &self,
        mut cans: AuthSession,
        new_username: String,
    ) -> Result<(), ServerFnError> {
        self.new_cans_setter.set(Some(cans));

        Ok(())
    }

    /// Update the cached canisters state
    /// WARN: all subscribers to the canisters resource will be notified
    pub fn update_canisters(&self, cans: AuthSession) {
        self.new_cans_setter.set(Some(cans));
    }
}
