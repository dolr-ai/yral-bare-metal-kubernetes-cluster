use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::use_params;
use leptos_router::params::Params;

use crate::components::seo::seo_meta_header;
use crate::content::render::{format_month_year, render_raw_html};

/// Route params for `/projects/entries/:slug`.
#[derive(Params, PartialEq, Clone)]
struct ProjectEntryParams {
    slug: String,
}

pub fn project_entry_page() -> impl IntoView {
    let content = expect_context::<crate::content::ContentProvider>();
    let params = use_params::<ProjectEntryParams>();
    let slug = params
        .get()
        .map(|p| p.slug)
        .unwrap_or_default();

    match content.find_project_entry(&slug) {
        Some(entry) => (
            seo_meta_header(&entry.title, &entry.description),
            html::article()
                .attr("class", "prose prose-emerald max-w-screen-md mx-auto my-8 px-4")
                .child(html::h1().attr("class", "!mb-2").child(entry.title.clone()))
                .child(
                    html::span()
                        .attr("class", "text-gray-400")
                        .child("worked on between ")
                        .child(
                            html::span()
                                .attr("class", "text-emerald-600 font-medium")
                                .child(format_month_year(&entry.start_date)),
                        )
                        .child(" to ")
                        .child(
                            html::span()
                                .attr("class", "text-emerald-600 font-medium")
                                .child(format_month_year(&entry.end_date)),
                        ),
                )
                .child(render_raw_html(&entry.body_html))
                .child(html::h2().child("This Project Uses"))
                .child(html::ul().child(
                    entry
                        .technologies_used
                        .iter()
                        .map(|technology| html::li().child(technology.clone()))
                        .collect::<Vec<_>>(),
                )),
        )
            .into_any(),
        None => crate::page::not_found::not_found_page().into_any(),
    }
}