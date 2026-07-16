/// All possible steps in our processing pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Deduplication,
    ExtractFrames,
    GcsUpload,
    NsfwDetection,
    NsfwDetectionV2,
    NsfwApiHandoff,
    NsfwApiStatusPoll,
    StorjIngest,
}

impl std::fmt::Display for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Step::Deduplication => "deduplication",
            Step::ExtractFrames => "extract_frames",
            Step::GcsUpload => "gcs_upload",
            Step::NsfwDetection => "nsfw_detection",
            Step::NsfwDetectionV2 => "nsfw_detection_v2",
            Step::NsfwApiHandoff => "nsfw_api_handoff",
            Step::NsfwApiStatusPoll => "nsfw_api_status_poll",
            Step::StorjIngest => "storj_ingest",
        };

        f.write_str(text)
    }
}

#[macro_export]
macro_rules! setup_context {
    ($video_id:expr, $step:expr, {
        $($key:literal: $value:expr),+ $(,)?
    }) => {
        log::info!(
            "pipeline context: video_id={}, step={}, {}",
            $video_id, $step,
            [$((format!("{}={:?}", $key, $value))),+].join(", ")
        )
    };
    ($video_id:expr, $step:expr) => {
        log::info!("pipeline context: video_id={}, step={}", $video_id, $step)
    };
}
