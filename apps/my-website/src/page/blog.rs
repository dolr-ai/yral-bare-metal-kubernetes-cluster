use leptos::html;
use leptos::prelude::*;

use crate::components::cards::blog_post_card;
use crate::components::seo::seo_meta_header;
use crate::islands::search_widget::{SearchWidget, SearchWidgetProps};

pub fn blog_page() -> impl IntoView {
    let content = expect_context::<crate::content::ContentProvider>();
    let blog_posts = content.blog_posts().to_vec();

    (
        seo_meta_header(
            "Saikat's Blog",
            "This is my blog where I write about what I learn including things like SvelteJS, TailwindCSS, NodeJS, Linux, PostgreSQL among a few",
        ),
        html::main()
            .attr("class", "max-w-screen-md mx-auto p-2")
            .child(SearchWidget(SearchWidgetProps {
                blog_posts: blog_posts.to_vec(),
            }))
            .child(
                html::h1()
                    .attr("class", "text-3xl text-center mt-16 mb-8")
                    .child("Blog Posts"),
            )
            .child(
                html::ul().child(
                    blog_posts
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
                ),
            ),
    )
}
