use leptos::prelude::*;

pub mod events;
pub mod video_analytics;

#[derive(Clone, Default)]
pub struct EventHistory {
    pub event_name: RwSignal<String>,
}
