use candid::Principal;
use leptos::prelude::Signal;
use leptos::prelude::*;
use serde_json::json;
use sns_validation::pbs::sns_pb::SnsInitPayload;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderKind {
    #[cfg(any(feature = "oauth-ssr", feature = "oauth-hydrate"))]
    YralAuth,
}

use circular_buffer::CircularBuffer;

#[derive(Clone)]
pub struct HistoryCtx {
    pub history: RwSignal<CircularBuffer<3, String>>,
    pub utm: RwSignal<Vec<(String, String)>>,
}

impl Default for HistoryCtx {
    fn default() -> Self {
        Self {
            history: RwSignal::new(CircularBuffer::<3, String>::new()),
            utm: RwSignal::new(Vec::new()),
        }
    }
}

impl HistoryCtx {
    pub fn new() -> Self {
        Self {
            history: RwSignal::new(CircularBuffer::<3, String>::new()),
            utm: RwSignal::new(Vec::new()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.history.get_untracked().len() == 0
    }

    pub fn len(&self) -> usize {
        self.history.get_untracked().len()
    }

    pub fn push(&self, url: &str) {
        self.history.update(move |h| h.push_back(url.to_string()));
    }

    pub fn push_utm(&self, utm: Vec<(String, String)>) {
        let utm: Vec<(String, String)> = utm
            .iter()
            .filter(|(k, _)| k.contains("utm"))
            .cloned()
            .collect();
        if utm.is_empty() {
            return;
        }
        self.utm.set(utm);
    }

    pub fn back(&self, fallback: &str) -> String {
        self.history.update(move |h| {
            h.pop_back();
        });

        let url = self.history.with(|h| h.back().cloned());
        if let Some(url) = url {
            self.history.update(move |h| {
                h.pop_back();
            });
            url
        } else {
            fallback.to_string()
        }
    }

    pub fn prev_url(&self) -> Option<String> {
        self.history.with(|h| h.back().cloned())
    }

    pub fn prev_url_untracked(&self) -> Option<String> {
        self.history.with_untracked(|h| h.back().cloned())
    }

    pub fn log_history(&self) -> String {
        let history = self.history.get();
        let history_str = history
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" -> ");
        history_str
    }
}

use crate::ml_feed::QuickPostDetails;
use crate::user_identity::UserIdentity;
use leptos::html::Video;
use yral_canisters_common::utils::posts::PostDetails;
use crate::UserAuthInfo;

pub enum AnalyticsEvent {
    VideoWatched(VideoWatched),
    LikeVideo(LikeVideo),
    ShareVideo(ShareVideo),
    Refer(Refer),
    ReferShareLink(ReferShareLink),
    LoginSuccessful(LoginSuccessful),
    LoginMethodSelected(LoginMethodSelected),
    LoginJoinOverlayViewed(LoginJoinOverlayViewed),
    LoginCta(LoginCta),
    LogoutClicked(LogoutClicked),
    LogoutConfirmation(LogoutConfirmation),
    ErrorEvent(ErrorEvent),
    TokenCreationStarted(TokenCreationStarted),
    TokensTransferred(TokensTransferred),
    PageVisit(PageVisit),
}

#[derive(Clone)]
pub struct EventUserDetails {
    pub details: UserIdentity,
    pub canister_id: Principal,
}

#[derive(Clone, Copy)]
pub struct EventCtx {
    pub is_connected: StoredValue<Box<dyn Fn() -> bool + Send + Sync>>,
    pub user_details: StoredValue<Box<dyn Fn() -> Option<EventUserDetails> + Send + Sync>>,
}

impl EventCtx {
    /// DO NOT USE THIS TO RENDER DOM
    pub fn user_details(&self) -> Option<EventUserDetails> {
        self.user_details.with_value(|c| c())
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.with_value(|c| c())
    }
}

// VideoEventData is now exported from video_analytics module
pub use crate::event_streaming::video_analytics::VideoEventData;

#[derive(Default)]
pub struct VideoWatched;

impl VideoWatched {
    pub fn send_event(
        &self,
        ctx: EventCtx,
        vid_details: Signal<Option<QuickPostDetails>>,
        container_ref: NodeRef<Video>,
        muted: RwSignal<bool>,
    ) {
        // Delegate to the refactored implementation
        use crate::event_streaming::video_analytics::VideoWatchedHandler;
        let handler = VideoWatchedHandler::new();
        handler.setup_event_tracking(ctx, vid_details, container_ref, muted);
    }

    pub fn send_event_with_current(
        &self,
        ctx: EventCtx,
        vid_details: Signal<Option<QuickPostDetails>>,
        container_ref: NodeRef<Video>,
        muted: RwSignal<bool>,
        is_current: Signal<bool>,
    ) {
        // Delegate to the refactored implementation
        use crate::event_streaming::video_analytics::VideoWatchedHandler;
        let handler = VideoWatchedHandler::new();
        handler.setup_event_tracking_with_current(
            ctx,
            vid_details,
            container_ref,
            muted,
            Some(is_current),
        );
    }
}

#[derive(Default)]
pub struct LikeVideo;

impl LikeVideo {
    pub fn send_event(&self, ctx: EventCtx, post_details: PostDetails, likes: RwSignal<u64>) {
        let _ = (ctx, post_details, likes);
    }
}

#[derive(Default)]
pub struct ShareVideo;

impl ShareVideo {
    pub fn send_event(&self, ctx: EventCtx, post_details: PostDetails) {
        let _ = (ctx, post_details);
    }
}

#[derive(Default)]
pub struct Refer;

impl Refer {
    pub fn send_event(&self, ctx: EventCtx) {
        let _ = ctx;
    }
}

#[derive(Default)]
pub struct ReferShareLink;

impl ReferShareLink {
    pub fn send_event(&self, ctx: EventCtx) {
        let _ = ctx;
    }
}

#[derive(Default)]
pub struct LoginSuccessful;

impl LoginSuccessful {
    pub fn send_event(&self, canisters: &impl UserAuthInfo) -> Result<(), anyhow::Error> {
        let _ = canisters;
        Ok(())
    }
}

#[derive(Default)]
pub struct LoginMethodSelected;

impl LoginMethodSelected {
    pub fn send_event(&self, prov: ProviderKind) {
        let _ = prov;
    }
}

#[derive(Default)]
pub struct LoginJoinOverlayViewed;

impl LoginJoinOverlayViewed {
    pub fn send_event(&self, ctx: EventCtx) {
        let _ = ctx;
    }
}

#[derive(Default)]
pub struct LoginCta;

impl LoginCta {
    pub fn send_event(&self, cta_location: String) {
        let _ = cta_location;
    }
}

#[derive(Default)]
pub struct LogoutClicked;

impl LogoutClicked {
    pub fn send_event(&self, ctx: EventCtx) {
        let _ = ctx;
    }
}

#[derive(Default)]
pub struct LogoutConfirmation;

impl LogoutConfirmation {
    pub fn send_event(&self, ctx: EventCtx) {
        let _ = ctx;
    }
}

#[derive(Default)]
pub struct ErrorEvent;

impl ErrorEvent {
    pub fn send_event(&self, ctx: EventCtx, error_str: String) {
        let _ = (ctx, error_str);
    }
}

#[derive(Default)]
pub struct TokenCreationStarted;

impl TokenCreationStarted {
    pub fn send_event(&self, ctx: EventCtx, sns_init_payload: SnsInitPayload) {
        let _ = (ctx, sns_init_payload);
    }
}

#[derive(Default)]
pub struct TokensTransferred;

impl TokensTransferred {
    pub fn send_event(&self, amount: String, to: Principal, cans_store: &impl UserAuthInfo) {
        let _ = (amount, to, cans_store);
    }
}

#[derive(Default)]
pub struct PageVisit;

impl PageVisit {
    pub fn send_event(&self, user_id: Principal, is_connected: bool, pathname: String) {
        let _ = (user_id, is_connected, pathname);
    }
}
