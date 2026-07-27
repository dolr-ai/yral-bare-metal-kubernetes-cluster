use leptos::html::Video;
use leptos::prelude::*;

use crate::event_streaming::events::EventCtx;
use crate::ml_feed::QuickPostDetails;

pub struct VideoWatchedHandler {
    progress_tracker: super::VideoProgressTracker,
}

impl Default for VideoWatchedHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoWatchedHandler {
    pub fn new() -> Self {
        Self {
            progress_tracker: super::VideoProgressTracker::new(),
        }
    }

    pub fn setup_event_tracking(
        &self,
        _ctx: EventCtx,
        _vid_details: Signal<Option<QuickPostDetails>>,
        _container_ref: NodeRef<Video>,
        _muted: RwSignal<bool>,
    ) {
    }

    pub fn setup_event_tracking_with_current(
        &self,
        _ctx: EventCtx,
        _vid_details: Signal<Option<QuickPostDetails>>,
        _container_ref: NodeRef<Video>,
        _muted: RwSignal<bool>,
        _is_current: Option<Signal<bool>>,
    ) {
    }
}
