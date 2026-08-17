use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::{overlay::VideoDetailsOverlay, video_loader::VideoView};
use crate::scrolling_post_view::MuteUnmuteOverlay;
use component::{back_btn::go_back_or_fallback, spinner::FullScreenSpinner};
use leptos_router::{components::Redirect, hooks::use_params, params::Params};
use state::audio_state::AudioState;
#[cfg(feature = "ssr")]
use utils::user_identity::propic_from_principal;
use utils::{bg_url, send_wrap};
use utils::posts::PostDetails;
#[derive(Params, PartialEq, Clone)]
struct PostParams {
    canister_id: Option<String>,
    post_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum PostFetchError {
    Invalid,
    Unavailable,
    GetUid(String),
}

#[component]
fn SinglePostViewInner(post: PostDetails) -> impl IntoView {
    let AudioState { muted, volume } = expect_context();
    let bg_url = bg_url(&post.uid);
    let to_load = Memo::new(|_| true);

    view! {
        <div class="w-dvw h-dvh">
            <div class="overflow-hidden relative w-full h-full bg-transparent">
                <div
                    class="absolute top-0 left-0 w-full h-full bg-center bg-cover z-1 blur-lg"
                    style:background-color="rgb(0, 0, 0)"
                    style:background-image=format!("url({bg_url})")
                ></div>
                <VideoDetailsOverlay post=post.clone() />
                <VideoView post=Some(post.into()) muted volume autoplay_at_render=true to_load />
            </div>
            <MuteUnmuteOverlay muted />
        </div>
    }
    .into_any()
}

#[component]
fn UnavailablePost() -> impl IntoView {
    view! {
        <div class="flex flex-col gap-2 justify-center items-center bg-black h-dvh w-dvw">
            <span class="text-lg text-white md:text-xl lg:text-2xl">Post is unavailable</span>
            <button
                on:click=|_| go_back_or_fallback("/")
                class="py-2 px-4 text-center text-white rounded-full bg-primary-600"
            >
                Go back
            </button>
        </div>
    }
}

#[component]
pub fn SinglePost() -> impl IntoView {
    let params = use_params::<PostParams>();

    let fetch_post = Resource::new(move || params.get(), move |params| {
        send_wrap(async move {
            let params = params.map_err(|_| PostFetchError::Invalid)?;
            let canister_id = params.canister_id.ok_or(PostFetchError::Invalid)?;
            let post_id = params.post_id.ok_or(PostFetchError::Invalid)?;

            // Fetch post from SpacetimeDB (SSR) or IC (hydrate fallback).
            #[cfg(feature = "ssr")]
            {
                use tokio::sync::oneshot;
                use yral_database_spacetime_bindings::get_individual_post_details_by_id;

                let conn = state::spacetime::spacetime_conn();
                let (tx, rx) = oneshot::channel();
                conn.procedures.get_individual_post_details_by_id_then(
                    post_id.clone(),
                    move |_ctx, result| { let _ = tx.send(result.ok().flatten()); },
                );
                let post = match rx.await.unwrap_or(None) {
                    Some(p) => p,
                    None => return Err(PostFetchError::Unavailable),
                };
                // Map SpacetimeDB PostDetailsForFrontend to the PostDetails struct
                // expected by the rest of the page.
                let poster_principal = post.creator_principal_text.clone();
                let poster_principal_text = &poster_principal;
                Ok(PostDetails {
                    canister_id: canister_id.clone(),
                    post_id: post.id,
                    uid: post.video_uid,
                    description: post.description,
                    views: post.total_view_count,
                    likes: post.like_count,
                    display_name: None,
                    username: None,
                    propic_url: propic_from_principal(poster_principal_text),
                    liked_by_user: Some(post.liked_by_me),
                    poster_principal,
                    creator_follows_user: None,
                    user_follows_creator: None,
                    creator_bio: None,
                    hastags: post.hashtags,
                    is_nsfw: false,
                    created_at: {
                        let micros = post.created_at.to_micros_since_unix_epoch();
                        web_time::Duration::new(
                            (micros / 1_000_000) as u64,
                            ((micros % 1_000_000) * 1000) as u32,
                        )
                    },
                    nsfw_probability: 0.0,
                })
            }

            #[cfg(not(feature = "ssr"))]
            {
                // Hydrate: post data is serialized from SSR pass.
                // If SSR didn't populate it, the post is unavailable.
                Err(PostFetchError::Unavailable)
            }
        })
    });

    view! {
        <Suspense fallback=FullScreenSpinner>
            {move || {
                fetch_post
                    .get()
                    .map(|post| match post {
                        Ok(post) => view! { <SinglePostViewInner post /> }.into_any(),
                        Err(PostFetchError::Invalid) => view! { <Redirect path="/" /> }.into_any(),
                        Err(PostFetchError::Unavailable) => view! { <UnavailablePost /> }.into_any(),
                        Err(PostFetchError::GetUid(e)) => {
                            view! { <Redirect path=format!("/error?err={e}") /> }.into_any()
                        }
                    })
            }}

        </Suspense>
    }
}
