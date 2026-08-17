use indexmap::IndexSet;
use leptos::logging;
use leptos::{html::Video, prelude::*};

use component::video_player::VideoPlayer;
use futures::FutureExt;
use gloo::timers::future::TimeoutFuture;
use utils::ml_feed::QuickPostDetails;
use utils::{bg_url, mp4_url, send_wrap};

/// Maximum PostDetails, time in milliseconds to waitay promise to resolve
const VIDEO_PLAY_TIMEOUT_MS: u64 = 5000;

use crate::post_view::PostDetailsCacheCtx;
use crate::scrolling_post_view::PostDetailResolver;

use super::overlay::VideoDetailsOverlay;

#[component]
pub fn BgView<DetailResolver>(
    video_queue: RwSignal<IndexSet<DetailResolver>>,
    idx: usize,
    children: Children,
) -> impl IntoView
where
    DetailResolver: PostDetailResolver + Clone + PartialEq + Sync + Send + 'static,
{
    let post_with_prev = Memo::new(move |_| {
        video_queue.with(|q| {
            let cur_post = q.get_index(idx).cloned();
            cur_post
        })
    });

    let PostDetailsCacheCtx {
        post_details: post_details_cache,
    } = expect_context();

    let post_details_with_prev_post = LocalResource::new(move || async move {
        let current_post_resolver = post_with_prev.get();
        let Some(resolver) = current_post_resolver else {
            leptos::logging::debug_warn!("returning None for post?");
            return Ok(None);
        };

        let QuickPostDetails {
            canister_id,
            post_id,
            ..
        } = resolver.get_quick_post_details();

        let post_id_c = post_id.clone();
        let canister_id_c = canister_id.clone();
        let cached_post_details = post_details_cache
            .try_with_value(|m| m.get(&(canister_id_c, post_id_c)).cloned())
            .flatten();

        // SAFETY: this send wrap is safe as we are guaranteed to run in a
        // single threaded env due to LocalResource
        let post_details = match cached_post_details {
            Some(details) => details,
            None => {
                let details = send_wrap(resolver.get_post_details()).await?;
                post_details_cache
                    .try_update_value(|m| m.insert((canister_id.clone(), post_id.clone()), details.clone()));
                details
            }
        };
        Ok::<_, ServerFnError>(Some(post_details))
    });

    let uid = move || {
        post_with_prev.get()
            .as_ref()
            .map(|q| q.get_quick_post_details().video_uid)
            .unwrap_or_default()
    };

    let high_priority = idx < 3;

    view! {
        <div class="overflow-hidden relative w-full h-full bg-transparent">
            <div
                class="absolute top-0 left-0 w-full h-full bg-center bg-cover z-1 blur-lg bg-black"
                style:background-image=move || format!("url({})", bg_url(uid()))
            ></div>
            <Suspense>
            {move || Suspend::new(async move {
                // let (post, prev_post) = try_or_redirect_opt!(post_details_with_prev_post.await);
                let post = post_details_with_prev_post.await.inspect_err(|err| leptos::logging::error!("Failed to load post details: {err:#?}")).ok()?;
                Some(view! { <VideoDetailsOverlay post=post? high_priority /> }.into_view())
            })}
            </Suspense>
            {children()}
        </div>
    }
    .into_any()
}

#[component]
pub fn VideoView(
    #[prop(into)] post: Signal<Option<QuickPostDetails>>,
    #[prop(optional)] _ref: NodeRef<Video>,
    #[prop(optional)] autoplay_at_render: bool,
    to_load: Memo<bool>,
    muted: RwSignal<bool>,
    volume: RwSignal<f64>,
    #[prop(optional, into)] is_current: Option<Signal<bool>>,
    #[prop(optional, into)] high_priority: bool,
) -> impl IntoView {
    let post_for_uid = post;
    let uid = Memo::new(move |_| {
        if !to_load.get() {
            return None;
        }
        post_for_uid.with(|p| p.as_ref().map(|p| p.video_uid.clone()))
    });
    let view_bg_url = move || uid.get().map(bg_url);
    let view_video_url = move || uid.get().map(mp4_url);

    // Preload the background image
    // This is a workaround to ensure the image is loaded before the video starts
    Effect::new(move |_| {
        use leptos::web_sys::HtmlImageElement;

        if let Some(bg_url) = view_bg_url() {
            if let Ok(img) = HtmlImageElement::new() {
                img.set_src(&bg_url);
            }
        }
    });

    // Handles mute/unmute
    Effect::new(move |_| {
        let vid = _ref.get()?;
        vid.set_muted(muted.get());
        Some(())
    });

    // Handles volume change
    Effect::new(move |_| {
        let vid = _ref.get()?;
        vid.set_volume(volume.get());
        Some(())
    });

    Effect::new(move |_| {
        let vid = _ref.get()?;
        // the attributes in DOM don't seem to be working
        vid.set_muted(muted.get_untracked());
        // vid.set_loop(true);
        if autoplay_at_render {
            vid.set_autoplay(true);
            _ = vid.play();
        }
        Some(())
    });

    view! {
        <VideoPlayer
            node_ref=_ref
            muted
            autoplay=is_current.unwrap_or(false.into())
            view_bg_url=Signal::derive(view_bg_url)
            view_video_url=Signal::derive(view_video_url)
            high_priority
        />
    }
    .into_any()
}

#[component]
pub fn VideoViewForQueue<DetailResolver>(
    post: RwSignal<Option<DetailResolver>>,
    current_idx: RwSignal<usize>,
    idx: usize,
    muted: RwSignal<bool>,
    volume: RwSignal<f64>,
    to_load: Memo<bool>,
) -> impl IntoView
where
    DetailResolver: PostDetailResolver + Clone + Sync + Send + 'static,
{
    let container_ref = NodeRef::<Video>::new();

    let quick_post_details =
        Signal::derive(move || post.get().map(|post| post.get_quick_post_details()));

    // Track if video is already playing to prevent multiple play attempts
    let is_playing = RwSignal::new(false);

    // Handles autoplay
    Effect::new(move |_| {
        let Some(vid) = container_ref.get() else {
            return;
        };

        let is_current = idx == current_idx.get();
        if !is_current {
            if is_playing.get_untracked() {
                is_playing.set(false);
                _ = vid.pause();
            }
            return;
        }

        // Only attempt to play if not already playing
        if is_current && !is_playing.get_untracked() {
            is_playing.set(true);
            vid.set_autoplay(true);

            let promise = vid.play();
            if let Ok(promise) = promise {
                wasm_bindgen_futures::spawn_local(async move {
                    // Create futures
                    let mut play_future = wasm_bindgen_futures::JsFuture::from(promise).fuse();
                    let mut timeout_future =
                        TimeoutFuture::new(VIDEO_PLAY_TIMEOUT_MS as u32).fuse();

                    // Race between play and timeout
                    futures::select! {
                        play_result = play_future => {
                            if let Err(e) = play_result {
                                logging::error!("video_log: Video play() promise failed: {e:?}");
                            }
                        }
                        _ = timeout_future => {
                            logging::error!("video_log: Video play() did not resolve within 5 seconds");
                        }
                    }
                });
            }
        }
    });

    // Create a signal that tracks whether this video is current
    let is_current_signal = Signal::derive(move || idx == current_idx.get());

    let high_priority = idx < 3;

    view! {
        <VideoView
            post=quick_post_details
            _ref=container_ref
            to_load
            muted
            volume
            is_current=is_current_signal
            high_priority
        />
    }
    .into_any()
}
