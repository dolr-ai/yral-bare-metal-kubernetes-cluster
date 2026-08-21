// Card components for blog posts and project entries.
//
// All functions are plain Rust functions returning `impl IntoView` — no
// `#[component]` macro — and build their HTML with Leptos builder syntax
// (no `view!` macro). They are pure presentation functions: callers pass
// already-loaded content fields, and the functions return a view.

use leptos::html;
use leptos::prelude::*;

use crate::content::render::{
    blog_post_icon_url, format_date_display, format_month_year, format_tags,
    format_technologies, project_cover_photo_url,
};

/// Renders a single blog post as a card-shaped link.
///
/// The outer `<a>` carries the hover-border styling; inside it a `<li>` lays
/// out the topic icon, title, publication date, description, and tags in a
/// responsive grid (single column on mobile, two columns on md+).
pub fn blog_post_card(
    title: String,
    description: String,
    tags: Vec<String>,
    created: String,
    icon: String,
    slug: String,
) -> impl IntoView {
    let icon_url = blog_post_icon_url(&icon);
    let alt_text = format!("{} icon", icon.replace(".svg", ""));
    let formatted_date = format_date_display(&created);
    let formatted_tags = format_tags(&tags);

    html::a()
        .attr("href", slug)
        .attr(
            "class",
            "border-4 rounded-lg border-transparent block my-2 p-2 md:py-6",
        )
        .child(
            html::li()
                .attr("class", "grid grid-cols-1")
                .child(
                    html::img()
                        .attr("src", icon_url)
                        .attr("alt", alt_text)
                        .attr(
                            "class",
                            "justify-self-center h-32 my-8 md:row-span-4 md:w-24 md:h-auto md:mr-8 md:ml-4 md:my-0 md:self-center",
                        )
                        .attr("loading", "lazy"),
                )
                .child(html::h2().attr("class", "font-bold text-2xl").child(title))
                .child(
                    html::span()
                        .attr("class", "text-gray-400")
                        .child("published on ")
                        .child(
                            html::span()
                                .attr("class", "text-emerald-600 font-medium")
                                .child(formatted_date),
                        ),
                )
                .child(html::p().attr("class", "my-2").child(description))
                .child(
                    html::span()
                        .attr("class", "text-fuchsia-600")
                        .child(formatted_tags),
                ),
        )
}

/// Renders a single project entry as a card with a cover photo and metadata.
///
/// The outer `<li>` carries the hover-border styling; inside it an `<a>` wraps
/// the cover photo, title, date range, description, and technologies list.
pub fn project_entry_card(
    title: String,
    description: String,
    technologies: Vec<String>,
    start_date: String,
    end_date: String,
    cover_photo: String,
    slug: String,
) -> impl IntoView {
    let cover_photo_url = project_cover_photo_url(&cover_photo);
    let formatted_start_date = format_month_year(&start_date);
    let formatted_end_date = format_month_year(&end_date);
    let formatted_technologies = format_technologies(&technologies);

    html::li()
        .attr(
            "class",
            "border-4 rounded-lg border-transparent block my-2 p-2 md:py-6",
        )
        .child(
            html::a()
                .attr("href", slug)
                .child(
                    html::img()
                        .attr("src", cover_photo_url)
                        .attr("alt", "Large screenshot of landing page")
                        .attr("width", "1280")
                        .attr("height", "720")
                        .attr("loading", "lazy"),
                )
                .child(
                    html::h2()
                        .attr("class", "text-2xl font-medium my-2")
                        .child(title),
                )
                .child(
                    html::p()
                        .child("worked on this between ")
                        .child(
                            html::span()
                                .attr("class", "font-semibold")
                                .child(formatted_start_date),
                        )
                        .child(" and ")
                        .child(
                            html::span()
                                .attr("class", "font-semibold")
                                .child(formatted_end_date),
                        ),
                )
                .child(html::p().attr("class", "my-4").child(description))
                .child(
                    html::p()
                        .attr("class", "text-fuchsia-600 text-sm")
                        .child(formatted_technologies),
                ),
        )
}