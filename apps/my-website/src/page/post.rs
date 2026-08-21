use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::use_params;
use leptos_router::params::Params;

use crate::components::seo::seo_meta_header;
use crate::content::render::{format_date_display, render_raw_html};

/// Route params for `/blog/posts/:slug`.
#[derive(Params, PartialEq, Clone)]
struct BlogPostParams {
    slug: String,
}

pub fn blog_post_page() -> impl IntoView {
    let content = expect_context::<crate::content::ContentProvider>();
    let params = use_params::<BlogPostParams>();
    let slug = params
        .get()
        .map(|p| p.slug)
        .unwrap_or_default();

    match content.find_blog_post(&slug) {
        Some(post) => (
            seo_meta_header(&post.title, &post.description),
            html::article()
                .attr("class", "prose prose-emerald max-w-screen-md mx-auto my-8 px-4")
                .child(html::h1().attr("class", "!mb-2").child(post.title.clone()))
                .child(
                    html::span()
                        .attr("class", "text-gray-400")
                        .child("published on ")
                        .child(
                            html::span()
                                .attr("class", "text-emerald-600 font-medium")
                                .child(format_date_display(&post.created)),
                        ),
                )
                .child(render_raw_html(&post.body_html)),
        )
            .into_any(),
        None => crate::page::not_found::not_found_page().into_any(),
    }
}