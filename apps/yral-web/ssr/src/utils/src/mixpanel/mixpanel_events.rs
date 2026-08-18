use chrono::Utc;
use codee::string::{FromToStringCodec, JsonSerdeCodec};
use consts::AUTH_JOURNEY_PAGE;
use consts::{AUTH_JOURNET, CUSTOM_DEVICE_ID, DEVICE_ID, NSFW_ENABLED_COOKIE};
use leptos::logging;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_use::storage::use_local_storage;
use leptos_use::use_timeout_fn;
use leptos_use::{use_cookie, use_cookie_with_options, UseCookieOptions, UseTimeoutFnReturn};
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use crate::UserAuthInfo;

use crate::mixpanel::state::MixpanelState;

#[server]
async fn track_event_server_fn(props: Value) -> Result<(), ServerFnError> {
    use axum::http::HeaderMap;
    use axum_extra::headers::UserAgent;
    use axum_extra::TypedHeader;
    use leptos_axum::extract;

    let mut props = props;

    // Attempt to extract headers and User-Agent
    let result: Result<(HeaderMap, TypedHeader<UserAgent>), _> = extract().await;

    let (ip, ua) = match result {
        Ok((headers, TypedHeader(user_agent))) => {
            let ip = headers
                .get("x-forwarded-for")
                .and_then(|val| val.to_str().ok())
                .and_then(|s| s.split(',').next())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let ua = user_agent.as_str().to_string();
            (Some(ip), Some(ua))
        }
        Err(_) => (None, None),
    };

    // Inject metadata into props
    props["ip"] = ip.clone().into();
    props["ip_addr"] = ip.clone().into();
    props["user_agent"] = ua.clone().into();

    // user_type enrichment via MetadataClient removed (IC decommissioned).
    // SpacetimeDB-based enrichment will be added in a follow-up PR.
    if props.get("user_type").is_none() {
        let is_logged_in = props
            .get("is_logged_in")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if is_logged_in {
            // TODO: fetch signup_at from SpacetimeDB to classify new vs repeat
            props["user_type"] = "repeat".into();
        }
    }

    Ok(())
}

pub fn parse_query_params_utm() -> Result<Vec<(String, String)>, String> {
    if let Some(storage) = window()
        .local_storage()
        .map_err(|e| format!("Failed to access localstorage: {e:?}"))?
    {
        if let Some(url_str) = storage
            .get_item("initial_url")
            .map_err(|e| format!("Failed to get utm from localstorage: {e:?}"))?
        {
            let url =
                reqwest::Url::parse(&url_str).map_err(|e| format!("Failed to parse url: {e:?}"))?;
            storage
                .remove_item("initial_url")
                .map_err(|e| format!("Failed to remove initial_url from localstorage: {e:?}"))?;
            return Ok(url
                .query_pairs()
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect());
        }
    }
    Ok(Vec::new())
}

pub(super) fn send_event_to_server<T>(event_name: &str, props: T)
where
    T: Serialize,
{
    let payload = get_event_payload(event_name, props);
    spawn_local(async {
        let res = track_event_server_fn(payload).await;
        match res {
            Ok(_) => {}
            Err(e) => logging::error!("Error tracking Mixpanel event: {}", e),
        }
    });
}

pub(super) async fn send_event_to_server_async<T>(event_name: &str, props: T)
where
    T: Serialize,
{
    let payload = get_event_payload(event_name, props);
    let res = track_event_server_fn(payload).await;
    match res {
        Ok(_) => {}
        Err(e) => logging::error!("Error tracking Mixpanel event: {}", e),
    }
}

fn get_event_payload<T>(event_name: &str, props: T) -> Value
where
    T: Serialize,
{
    let mut props = serde_json::to_value(&props).unwrap();
    props["event"] = event_name.into();
    props["time"] = chrono::Utc::now().timestamp().into();
    props["$device_id"] = MixpanelGlobalProps::get_device_id().into();
    props["custom_device_id"] = MixpanelGlobalProps::get_custom_device_id().into();
    let user_id = props.get("user_id").and_then(Value::as_str);
    props["principal"] = if user_id.is_some() {
        user_id.into()
    } else {
        props.get("visitor_id").and_then(Value::as_str).into()
    };
    let current_url = window().location().href().ok();
    if let Some(url) = current_url {
        props["current_url"] = url.clone().into();
        props["$current_url"] = url.into();
    }
    // HistoryCtx-based UTM enrichment removed (event_streaming module decommissioned).
    if let Ok(utms) = parse_query_params_utm() {
        for (key, value) in utms {
            props[key] = value.into();
        }
    }
    props
}

/// Global properties for Mixpanel events
#[derive(Clone, Serialize)]
pub struct MixpanelGlobalProps {
    pub user_id: Option<String>,
    pub visitor_id: Option<String>,
    pub username: Option<String>,
    pub is_logged_in: bool,
    pub canister_id: String,
    pub is_nsfw_enabled: bool,
}

impl MixpanelGlobalProps {
    pub fn new(
        user_id: String,
        canister_id: String,
        is_logged_in: bool,
        is_nsfw_enabled: bool,
        username: Option<String>,
    ) -> Self {
        Self {
            user_id: if is_logged_in {
                Some(user_id.clone())
            } else {
                None
            },
            visitor_id: if !is_logged_in {
                Some(user_id)
            } else {
                None
            },
            is_logged_in,
            canister_id,
            is_nsfw_enabled,
            username,
        }
    }

    /// Load global state (login, principal, NSFW toggle)
    pub fn try_get(cans: &impl UserAuthInfo, is_logged_in: bool) -> Self {
        let (is_nsfw_enabled, _) = use_cookie_with_options::<bool, FromToStringCodec>(
            NSFW_ENABLED_COOKIE,
            UseCookieOptions::default()
                .path("/")
                .max_age(consts::auth::REFRESH_MAX_AGE.as_secs() as i64)
                .same_site(leptos_use::SameSite::Lax),
        );
        let is_nsfw_enabled = is_nsfw_enabled.get_untracked().unwrap_or(false);

        Self {
            user_id: if is_logged_in {
                Some(cans.user_id().clone())
            } else {
                None
            },
            visitor_id: if !is_logged_in {
                Some(cans.user_id().clone())
            } else {
                None
            },
            is_logged_in,
            canister_id: cans.user_canister().clone(),
            is_nsfw_enabled,
            username: cans.user_identity().username,
        }
    }

    pub fn get_device_id() -> String {
        let device_id = MixpanelState::get_device_id();
        if let Some(device_id) = device_id.get_untracked() {
            device_id
        } else {
            let device_id_val = crate::local_storage::LocalStorage::uuid_get_or_init(DEVICE_ID);
            device_id.set(Some(device_id_val.clone()));
            device_id_val
        }
    }

    pub fn get_custom_device_id() -> String {
        let custom_device_id = MixpanelState::get_custom_device_id();
        if let Some(custom_device_id) = custom_device_id.get_untracked() {
            custom_device_id
        } else {
            let custom_device_id_val =
                crate::local_storage::LocalStorage::uuid_get_or_init(CUSTOM_DEVICE_ID);
            custom_device_id.set(Some(custom_device_id_val.clone()));
            custom_device_id_val
        }
    }

    pub fn get_auth_journey() -> String {
        let (auth_journey, _, _) = use_local_storage::<String, FromToStringCodec>(AUTH_JOURNET);
        // Extracting the device ID value
        let auth_journey_value = auth_journey.get_untracked();
        if auth_journey_value.is_empty() {
            "unknown".to_string()
        } else {
            auth_journey_value
        }
    }
    pub fn set_auth_journey(auth_journey: String) {
        let (_, set_auth_journey, _) = use_local_storage::<String, FromToStringCodec>(AUTH_JOURNET);
        set_auth_journey.set(auth_journey);
    }

    pub fn try_get_with_nsfw_info(
        cans: &impl UserAuthInfo,
        is_logged_in: bool,
        is_nsfw_enabled: bool,
    ) -> Self {
        Self {
            user_id: if is_logged_in {
                Some(cans.user_id().clone())
            } else {
                None
            },
            visitor_id: if !is_logged_in {
                Some(cans.user_id().clone())
            } else {
                None
            },
            is_logged_in,
            canister_id: cans.user_canister().clone(),
            is_nsfw_enabled,
            username: cans.user_identity().username,
        }
    }

    pub fn page_name(&self) -> BottomNavigationCategory {
        #[cfg(feature = "hydrate")]
        {
            let path = window().location().pathname().unwrap_or_default();
            path.try_into().unwrap_or(BottomNavigationCategory::Menu)
        }
        #[cfg(not(feature = "hydrate"))]
        {
            log::error!("calling MixpanelGlobalProps::page_name from SSR is not sane");
            BottomNavigationCategory::Menu
        }
    }
}

use std::convert::TryFrom;

impl TryFrom<String> for BottomNavigationCategory {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.contains("/wallet/") {
            return Ok(BottomNavigationCategory::Wallet);
        }

        match value.as_str() {
            "/wallet" => Ok(BottomNavigationCategory::Wallet),
            "/menu" => Ok(BottomNavigationCategory::Menu),
            _ => Err(()),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MixpanelVideoClickedCTAType {
    Like,
    Share,
    ReferAndEarn,
    Report,
    NsfwToggle,
    Mute,
    Unmute,
    VideoPlay,
    Leaderboard,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MixpanelMenuClickedCTAType {
    TalkToTheTeam,
    TermsOfService,
    PrivacyPolicy,
    LogOut,
    FollowOn,
    ReferAndEarn,
    Leaderboard,
    Settings,
    AboutUs,
    Follow,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]

pub enum StakeType {
    DolrAi,
    Btc,
    Usdc,
    Yral,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BottomNavigationCategory {
    #[default]
    Menu,
    Wallet,
}
pub struct MixPanelEvent;

macro_rules! derive_event {
    ($name:ident = $ev:expr => { $($prop:ident: $typ:ty),* }) => {
        #[allow(non_camel_case_types)]
        #[derive(serde::Serialize)]
        struct $name {
            user_id: Option<String>,
            visitor_id: Option<String>,
            username: Option<String>,
            is_logged_in: bool,
            canister_id: String,
            is_nsfw_enabled: bool,
            $($prop: $typ),*
        }

        impl $name {
            #[allow(clippy::too_many_arguments)]
            pub fn new(
                global: MixpanelGlobalProps,
                $($prop: $typ),*
            ) -> Self {
                let MixpanelGlobalProps {
                    user_id,
                    visitor_id,
                    username,
                    is_logged_in,
                    canister_id,
                    is_nsfw_enabled,
                } = global;
                Self {
                    user_id,
                    visitor_id,
                    username,
                    is_logged_in,
                    canister_id,
                    is_nsfw_enabled,
                    $($prop),*
                }
            }
        }
        // static assert to ensure $name begins with track_
        const _: () = {
            assert!(matches!(stringify!($name).as_bytes().split_at(6), (b"track_", _)));
        };

        impl MixPanelEvent {
            #[allow(clippy::too_many_arguments)]
            pub fn $name(
                global: MixpanelGlobalProps,
                $($prop: $typ),*
            ) {
                let event = $name::new(global, $($prop),*);
                send_event_to_server($ev, event);
            }
        }
    };
    ($name:ident { $($prop:ident: $typ:ty),* }) => {
        derive_event!(
            $name = &stringify!($name)[6..] => { $($prop: $typ),* }
        );
    }
}

derive_event!(track_menu_page_viewed {});

derive_event!(track_unlock_higher_bets_popup_shown {
    page_name: String,
    stake_amount: u64,
    stake_type: StakeType
});

derive_event!(track_edit_username_clicked {});

derive_event!(track_wallet_page_viewed {});

derive_event!(track_menu_clicked {
    cta_type: MixpanelMenuClickedCTAType
});

derive_event!(track_account_deleted { page_name: String });

derive_event!(track_withdraw_tokens_clicked {
    token_clicked: StakeType
});

derive_event!(track_referral_link_copied {
    referral_bonus: u64
});

derive_event!(track_refer_friend_clicked {
    cta_type: String,
    page_name: String
});

derive_event!(track_share_invites_clicked {
    referral_bonus: u64
});

derive_event!(track_bottom_navigation_clicked {
    category_name: BottomNavigationCategory
});

derive_event!(track_enable_notifications { toggle: bool });

derive_event!(track_signup_clicked {
    page_name: BottomNavigationCategory
});

derive_event!(track_auth_screen_viewed {
    page_name: BottomNavigationCategory
});

derive_event!(track_auth_initiated = "signup_journey_selected" => {
    auth_journey: String,
    page_name: BottomNavigationCategory
});

derive_event!(track_signup_success {
    is_referral: bool,
    referrer_user_id: Option<String>,
    auth_journey: String,
    page_name: BottomNavigationCategory
});

derive_event!(track_login_success {
    auth_journey: String,
    page_name: BottomNavigationCategory
});

derive_event!(track_enable_nsfw_popup_shown { page_name: String });

derive_event!(track_nsfw_enabled {
    publisher_user_id: String,
    video_id: String,
    is_nsfw: bool,
    page_name: String,
    cta_type: Option<String>
});

derive_event!(track_nsfw_false = "NSFW_false" => {
    publisher_user_id: String,
    video_id: String,
    is_nsfw: bool,
    page_name: String,
    cta_type: Option<String>
});

derive_event!(track_video_clicked {
    publisher_user_id: String,
    video_id: String,
    cta_type: MixpanelVideoClickedCTAType
});

derive_event!(track_video_reported {
    publisher_user_id: String,
    video_id: String,
    is_nsfw: bool,
    report_reason: String
});

derive_event!(track_video_clicked_profile = "video_clicked" => {
    publisher_user_id: String,
    like_count: u64,
    view_count: u64,
    video_id: String,
    cta_type: MixpanelVideoClickedCTAType,
    position: Option<u64>,
    is_own_profile: bool,
    is_nsfw: bool,
    page_name: String
});

derive_event!(track_video_clicked_leaderboard = "video_clicked" => {
    video_id: String,
    publisher_user_id: String,
    like_count: u64,
    view_count: u64,
    is_leaderboard_active: bool,
    is_nsfw: bool,
    cta_type: MixpanelVideoClickedCTAType
});

derive_event!(track_leaderboard_page_viewed {
    is_tournament_active: bool
});

derive_event!(track_refer_and_earn { refer_link: String });

derive_event!(track_video_viewed {
    video_id: String,
    publiser_user_id: String
});

derive_event!(track_video_impression {
    video_id: String,
    publisher_user_id: String,
    like_count: u64,
    view_count: u64,
    is_nsfw: bool
});

derive_event!(track_video_started {
    video_id: String,
    publisher_user_id: String
});

derive_event!(track_third_party_wallet_transferred {
    token_transferred: f64,
    transferred_to: String,
    token_name: String,
    gas_fee: f64
});

derive_event!(track_username_saved {});

impl MixPanelEvent {
    fn clear_auth_journey_page() {
        let (_, set_auth_journey_page) =
            use_cookie::<BottomNavigationCategory, JsonSerdeCodec>(AUTH_JOURNEY_PAGE);
        logging::log!("Clearing auth journey page");
        set_auth_journey_page.set(None);
    }
    pub async fn track_login_success_async(
        global: MixpanelGlobalProps,
        auth_journey: String,
        page_name: BottomNavigationCategory,
    ) {
        let props = track_login_success::new(global, auth_journey, page_name);
        send_event_to_server_async("login_success", props).await;
    }

    pub async fn track_signup_success_async(
        global: MixpanelGlobalProps,
        is_referral: bool,
        referrer_user_id: Option<String>,
        auth_journey: String,
        page_name: BottomNavigationCategory,
    ) {
        let props = track_signup_success::new(
            global,
            is_referral,
            referrer_user_id,
            auth_journey,
            page_name,
        );
        send_event_to_server_async("signup_success", props).await;
        Self::clear_auth_journey_page();
    }
    pub fn track_login_success_sync(
        global: MixpanelGlobalProps,
        auth_journey: String,
        page_name: BottomNavigationCategory,
    ) {
        let props = track_login_success::new(global, auth_journey, page_name);
        send_event_to_server("login_success", props);
        Self::clear_auth_journey_page();
    }

    pub fn track_signup_success_sync(
        global: MixpanelGlobalProps,
        is_referral: bool,
        referrer_user_id: Option<String>,
        auth_journey: String,
        page_name: BottomNavigationCategory,
    ) {
        let props = track_signup_success::new(
            global,
            is_referral,
            referrer_user_id,
            auth_journey,
            page_name,
        );
        send_event_to_server("signup_success", props);
        Self::clear_auth_journey_page();
    }

    pub fn track_page_viewed(page: String, p: MixpanelGlobalProps) {
        let UseTimeoutFnReturn { start, .. } = use_timeout_fn(
            move |_| {
                let props = p.clone();
                match page.as_str() {
                    "/menu" => {
                        Self::track_menu_page_viewed(props);
                    }
                    "/profile/edit/username" => {
                        Self::track_edit_username_clicked(props);
                    }
                    page if page.contains("wallet") => {
                        Self::track_wallet_page_viewed(props);
                    }
                    _ => (),
                };
                send_event_to_server("page_viewed", p.clone());

                // TODO: Will be used later
                // if props.page.contains("/profile/") {
                //     let home_props: MixpanelPageViewedProps = props.clone();
                //     let publisher_user_id = home_props
                //         .page
                //         .split("/profile/")
                //         .nth(1)
                //         .and_then(|s| s.split('/').next())
                //         .unwrap_or_default()
                //         .to_string();

                //     if Principal::from_text(publisher_user_id.clone())
                //         .ok()
                //         .is_some()
                //     {
                //         let principal = if home_props.user_id.is_some() {
                //             home_props.user_id.clone().unwrap()
                //         } else {
                //             home_props.visitor_id.clone().unwrap()
                //         };

                //         let is_own_profile = publisher_user_id == principal;

                //         Self::track_profile_page_viewed(MixpanelProfilePageViewedProps {
                //             user_id: home_props.user_id,
                //             visitor_id: home_props.visitor_id,
                //             is_logged_in: home_props.is_logged_in,
                //             canister_id: home_props.canister_id,
                //             is_nsfw_enabled: home_props.is_nsfw_enabled,
                //             is_own_profile,
                //             publisher_user_id,
                //         });
                //     }
                // }
            },
            10.0,
        );
        start(());
    }
}
