use leptos::html;
use leptos::prelude::*;

use crate::components::cards::project_entry_card;
use crate::components::seo::seo_meta_header;

pub fn projects_page() -> impl IntoView {
    let content = expect_context::<crate::content::ContentProvider>();
    let project_entries = content.project_entries().to_vec();

    (
        seo_meta_header(
            "Things I've Built",
            "This is my portfolio of sorts where I document and showcase things I've built, going over their highlights or salient points.",
        ),
        html::main()
            .attr("class", "max-w-screen-md mx-auto p-2")
            .child(html::h1()
                .attr("class", "text-3xl text-center my-8")
                .child("Things I've Built"))
            .child(html::ul().child(
                project_entries
                    .iter()
                    .map(|entry| {
                        project_entry_card(
                            entry.title.clone(),
                            entry.description.clone(),
                            entry.technologies_used.clone(),
                            entry.start_date.clone(),
                            entry.end_date.clone(),
                            entry.cover_photo.clone(),
                            entry.slug.clone(),
                        )
                    })
                    .collect::<Vec<_>>(),
            )),
    )
}