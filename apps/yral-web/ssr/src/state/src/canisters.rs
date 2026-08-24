use codee::string::FromToStringCodec;
use consts::{
    auth::REFRESH_MAX_AGE, ACCOUNT_CONNECTED_STORE, AUTH_UTIL_COOKIES_MAX_AGE_MS,
    USER_CANISTER_ID_STORE, USER_PRINCIPAL_STORE,
};
use leptos::prelude::*;
use leptos_use::{use_cookie_with_options, UseCookieOptions};
use serde::{Deserialize, Serialize};
use std::future::Future;
use utils::user_identity::ProfileDetails;
use utils::UserAuthInfo;
use utils::{types::NewIdentity, MockPartialEq};

/// The authenticated user session. Replaces the old IC identity-based
/// `Canisters<true>` with a simple JWT-based session. The `user_id` is
/// the JWT `sub` claim (OAuth sub or UUID for AI accounts). No IC identity,
/// no delegation chain, no Secp256k1 keys.
#[derive(Clone)]
pub struct AuthSession {
    user_id: String,
    id_token: String,
    refresh_token: String,
    profile: ProfileDetails,
}

impl AuthSession {
    pub fn user_id(&self) -> String {
        self.user_id.clone()
    }

    pub fn id_token(&self) -> &str {
        &self.id_token
    }

    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }

    pub fn profile_details(&self) -> ProfileDetails {
        self.profile.clone()
    }

    pub fn user_identity(&self) -> utils::user_identity::UserIdentity {
        utils::user_identity::UserIdentity::from(self.profile.clone())
    }

    pub fn new(
        user_id: String,
        id_token: String,
        refresh_token: String,
        profile: ProfileDetails,
    ) -> Self {
        Self {
            user_id,
            id_token,
            refresh_token,
            profile,
        }
    }
}

impl UserAuthInfo for AuthSession {
    fn user_id(&self) -> String {
        self.user_id.clone()
    }

    fn user_canister(&self) -> String {
        self.user_id.clone()
    }

    fn user_identity(&self) -> utils::user_identity::UserIdentity {
        utils::user_identity::UserIdentity::from(self.profile.clone())
    }
}

async fn do_session_auth(
    user_id: String,
    id_token: String,
    refresh_token: String,
    _fallback_username: Option<String>,
) -> Result<AuthSession, ServerFnError> {
    leptos::logging::log!("Authenticating user with user_id {user_id}");

    // Fetch profile from SpacetimeDB.
    #[cfg(feature = "ssr")]
    {
        use tokio::sync::oneshot;
        use yral_database_spacetime_bindings::get_user_profile_details;

        let conn = crate::spacetime::spacetime_conn();
        let (tx, rx) = oneshot::channel();
        conn.procedures.get_user_profile_details_then(
            user_id.clone(),
            move |_ctx, result| {
                let _ = tx.send(result.ok().flatten());
            },
        );

        let profile = if let Some(p) = rx.await.unwrap_or(None) {
            ProfileDetails {
                username: None,
                lifetime_earnings: 0,
                followers_cnt: p.followers_count,
                following_cnt: p.following_count,
                profile_pic: p.profile_picture.as_ref().map(|pic| pic.url.clone()),
                display_name: None,
                user_identifier: user_id.clone(),
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
            ProfileDetails {
                username: None,
                lifetime_earnings: 0,
                followers_cnt: 0,
                following_cnt: 0,
                profile_pic: None,
                display_name: None,
                user_identifier: user_id.clone(),
                hots: 0,
                nots: 0,
                bio: None,
                website_url: None,
                caller_follows_user: None,
                user_follows_caller: None,
            }
        };

        Ok(AuthSession {
            user_id,
            id_token,
            refresh_token,
            profile,
        })
    }

    #[cfg(not(feature = "ssr"))]
    {
        let profile = ProfileDetails {
            username: None,
            lifetime_earnings: 0,
            followers_cnt: 0,
            following_cnt: 0,
            profile_pic: None,
            display_name: None,
            user_identifier: user_id.clone(),
            hots: 0,
            nots: 0,
            bio: None,
            website_url: None,
            caller_follows_user: None,
            user_follows_caller: None,
        };
        Ok(AuthSession {
            user_id,
            id_token,
            refresh_token,
            profile,
        })
    }
}

type AuthCansResource = LocalResource<Result<AuthSession, ServerFnError>>;

/// The authenticated session helper resource.
/// Prefer using helpers from [crate::component::canisters_prov] instead.
pub fn auth_state() -> AuthState {
    expect_context()
}

#[derive(Copy, Clone)]
pub struct AuthState {
    is_logged_in_with_oauth: (Signal<Option<bool>>, WriteSignal<Option<bool>>),
    new_identity_setter: RwSignal<Option<NewIdentity>>,
    pub canisters_resource: AuthCansResource,
    user_canister_id_cookie: (Signal<Option<String>>, WriteSignal<Option<String>>),
    pub user_id: Resource<Result<String, ServerFnError>>,
    user_id_cookie: (Signal<Option<String>>, WriteSignal<Option<String>>),
    pub user_identity: Resource<Result<NewIdentity, ServerFnError>>,
    new_cans_setter: RwSignal<Option<AuthSession>>,
}

impl Default for AuthState {
    fn default() -> Self {
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
                if let Some(id) = auth_id.0 {
                    return Ok::<_, ServerFnError>(id);
                }
                Err(ServerFnError::new("No identity set"))
            },
        );

        let new_cans_setter = RwSignal::new(None::<AuthSession>);

        let canisters_resource: AuthCansResource = LocalResource::new(move || {
            user_identity_resource.track();
            let new_cans = new_cans_setter.get();
            async move {
                let new_id = user_identity_resource.await?;
                match new_cans {
                    Some(cans) if cans.user_id() == new_id.user_id => {
                        return Ok::<_, ServerFnError>(cans);
                    }
                    _ => {}
                };

                let res = do_session_auth(
                    new_id.user_id,
                    new_id.id_token,
                    new_id.refresh_token,
                    new_id.fallback_username,
                )
                .await?;

                Ok::<_, ServerFnError>(res)
            }
        });

        let user_id_cookie = use_cookie_with_options::<String, FromToStringCodec>(
            USER_PRINCIPAL_STORE,
            UseCookieOptions::default()
                .path("/")
                .max_age(AUTH_UTIL_COOKIES_MAX_AGE_MS),
        );
        let user_id = Resource::new(
            move || {
                user_identity_resource.track();
                MockPartialEq(())
            },
            move |_| async move {
                if let Some(user_id) = user_id_cookie.0.try_get_untracked().flatten() {
                    return Ok(user_id);
                }

                let new_id = user_identity_resource.await?;
                user_id_cookie.1.set(Some(new_id.user_id.clone()));

                Ok(new_id.user_id)
            },
        );

        let user_canister_id_cookie = use_cookie_with_options::<String, FromToStringCodec>(
            USER_CANISTER_ID_STORE,
            UseCookieOptions::default()
                .path("/")
                .max_age(AUTH_UTIL_COOKIES_MAX_AGE_MS),
        );

        Self {
            is_logged_in_with_oauth,
            new_identity_setter,
            canisters_resource,
            user_id,
            user_id_cookie,
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
    pub fn set_new_identity(&self, new_identity: NewIdentity, is_logged_in_with_oauth: bool) {
        self.is_logged_in_with_oauth
            .1
            .set(Some(is_logged_in_with_oauth));

        self.user_canister_id_cookie.1.set(None);
        self.user_id_cookie.1.set(Some(new_identity.user_id.clone()));
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
    pub async fn auth_cans(&self) -> Result<AuthSession, ServerFnError> {
        self.canisters_resource.await
    }

    /// Get the user_id if loaded (no tracking)
    pub fn user_id_if_available(&self) -> Option<String> {
        self.user_id_cookie.0.get_untracked()
    }

    /// WARN: This function MUST be used with `<Suspense>`, if used inside view! {}
    pub async fn user_canister(&self) -> Result<String, ServerFnError> {
        if let Some(canister_id) = self.user_canister_id_cookie.0.get_untracked() {
            return Ok(canister_id);
        }

        let session = self.canisters_resource.await?;
        let canister_id = session.user_id();
        self.user_canister_id_cookie.1.set(Some(canister_id.clone()));

        Ok(canister_id)
    }

    /// Get the user canister if loaded (no tracking)
    pub fn user_canister_if_available(&self) -> Option<String> {
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

    #[cfg(feature = "hydrate")]
    pub fn auth_cans_if_available(&self) -> Option<AuthSession> {
        use std::future::IntoFuture;
        use futures::FutureExt;
        self.canisters_resource
            .into_future()
            .now_or_never()
            .and_then(|c| c.ok())
    }

    #[cfg(not(feature = "hydrate"))]
    pub fn auth_cans_if_available(&self) -> Option<AuthSession> {
        None
    }

    /// Update the username of the user
    pub async fn update_username(
        &self,
        cans: AuthSession,
        _new_username: String,
    ) -> Result<(), ServerFnError> {
        self.new_cans_setter.set(Some(cans));
        Ok(())
    }

    /// Update the cached canisters state
    pub fn update_canisters(&self, cans: AuthSession) {
        self.new_cans_setter.set(Some(cans));
    }
}