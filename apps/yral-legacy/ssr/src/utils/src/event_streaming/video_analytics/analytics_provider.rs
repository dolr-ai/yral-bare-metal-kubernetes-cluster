use crate::event_streaming::events::EventCtx;
use crate::ml_feed::QuickPostDetails;

#[derive(Clone)]
pub enum VideoAnalyticsEvent {
    VideoStarted {
        post: QuickPostDetails,
        is_logged_in: bool,
    },
    VideoViewed {
        post: QuickPostDetails,
        is_logged_in: bool,
    },
    VideoMuted {
        post: QuickPostDetails,
        muted: bool,
    },
}

pub trait VideoAnalyticsProvider: Send + Sync {
    fn track_event(&self, event: VideoAnalyticsEvent, ctx: EventCtx);
}
