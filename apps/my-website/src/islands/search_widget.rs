use leptos::ev;
use leptos::html;
use leptos::prelude::*;

use crate::components::cards::blog_post_card;
use crate::content::BlogPost;

/// Interactive search widget island.
///
/// A text input that filters the list of blog posts client-side. The blog
/// posts are passed as a prop and serialized to the island via the
/// `data-props` attribute (this requires `BlogPost` to derive
/// `Serialize + Deserialize`, which it does in `content::types`).
///
/// Filtering is simple case-insensitive substring matching against the post
/// title, description, and tags — no lunr dependency for now. When the search
/// text is empty, all posts are shown.
///
/// This is an island because it requires client-side interactivity (input
/// handler + reactive search-text state + derived filtered list). The
/// `#[island]` macro is the one necessary exception to the no-macros rule —
/// it generates the `wasm_bindgen` exports and `Island::new` wiring needed for
/// hydration. The function body uses builder syntax (no `view!` macro).
#[island]
pub fn SearchWidget(blog_posts: Vec<BlogPost>) -> impl IntoView {
    let (search_text, set_search_text) = signal(String::new());

    // Memoized filtered list. Recomputes only when search_text changes.
    let filtered_posts = Memo::new(move |_| {
        let query = search_text.get().to_lowercase();
        if query.is_empty() {
            return blog_posts.clone();
        }
        blog_posts
            .iter()
            .filter(|post| {
                post.title.to_lowercase().contains(&query)
                    || post.description.to_lowercase().contains(&query)
                    || post
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query))
            })
            .cloned()
            .collect::<Vec<_>>()
    });

    html::div()
        .child(
            html::input()
                .attr("type", "text")
                .attr(
                    "placeholder",
                    "Keep typing to see search results...",
                )
                .attr(
                    "class",
                    "block w-11/12 mx-auto my-6 border-b-2 border-gray-300 text-xl focus:outline-none focus:border-emerald-400 active:border-emerald-400",
                )
                .on(ev::input, move |event| {
                    let value = event_target_value(&event);
                    set_search_text.set(value);
                }),
        )
        .child(move || {
            let posts = filtered_posts.get();
            html::ul().child(
                posts
                    .iter()
                    .map(|post| {
                        blog_post_card(
                            post.title.clone(),
                            post.description.clone(),
                            post.tags.clone(),
                            post.created.clone(),
                            post.icon.clone(),
                            post.slug.clone(),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        })
}