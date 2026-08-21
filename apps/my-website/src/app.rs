use leptos::html;
use leptos::hydration::{AutoReload, HydrationScripts};
use leptos::prelude::*;
use leptos_meta::{MetaTags, Title, TitleProps, provide_meta_context};
use leptos_router::{
    components::{Route, RouteChildren, RouteProps, Router, RouterProps, Routes, RoutesProps},
    path,
};

use crate::{
    content::ContentProvider,
    page::{blog::blog_page, home::home_page, not_found::not_found_page, projects::projects_page},
};

/// The HTML document shell. In islands mode this runs on the server only.
/// The `islands=true` flag on HydrationScripts tells cargo-leptos to emit
/// per-island hydration scripts instead of full-app hydration.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    // Special leptos-only components that have no builder equivalent are
    // invoked via their props builder structs.
    use leptos::hydration::{AutoReloadProps, HydrationScriptsProps};

    html::html()
        .attr("lang", "en")
        .child(
            html::head()
                .child(html::meta().attr("charset", "utf-8"))
                .child(
                    html::meta()
                        .attr("name", "viewport")
                        .attr("content", "width=device-width, initial-scale=1"),
                )
                .child(
                    html::link()
                        .attr("rel", "icon")
                        .attr("href", "/favicon.png"),
                )
                .child(
                    html::link()
                        .attr("rel", "manifest")
                        .attr("href", "/manifest.webmanifest"),
                )
                .child(AutoReload(
                    AutoReloadProps::builder().options(options.clone()).build(),
                ))
                .child(HydrationScripts(
                    HydrationScriptsProps::builder()
                        .options(options.clone())
                        .islands(true)
                        .build(),
                ))
                .child(
                    html::link()
                        .attr("rel", "stylesheet")
                        .attr("id", "leptos")
                        .attr("href", format!("/pkg/{}.css", options.output_name)),
                )
                .child(MetaTags()),
        )
        .child(html::body().child(app()))
}

pub fn app() -> impl IntoView {
    provide_meta_context();

    // ContentProvider loads all blog posts and project entries at startup
    // (server-side only in islands mode). Pages access it via context.
    let content = ContentProvider::new();

    provide_context(content);

    (
        Title(TitleProps::builder().text("Saikat's Website").build()),
        html::main().child(Router(
            RouterProps::builder()
                .children(ToChildren::to_children(|| {
                    Routes(
                        RoutesProps::builder()
                            .fallback(|| "Page not found.".into_view())
                            .children(RouteChildren::to_children(|| {
                                (
                                    Route(
                                        RouteProps::builder()
                                            .path(path!("/"))
                                            .view(home_page)
                                            .build(),
                                    ),
                                    Route(
                                        RouteProps::builder()
                                            .path(path!("/blog"))
                                            .view(blog_page)
                                            .build(),
                                    ),
                                    Route(
                                        RouteProps::builder()
                                            .path(path!("/blog/posts/:slug"))
                                            .view(crate::page::post::blog_post_page)
                                            .build(),
                                    ),
                                    Route(
                                        RouteProps::builder()
                                            .path(path!("/projects"))
                                            .view(projects_page)
                                            .build(),
                                    ),
                                    Route(
                                        RouteProps::builder()
                                            .path(path!("/projects/entries/:slug"))
                                            .view(crate::page::entry::project_entry_page)
                                            .build(),
                                    ),
                                    Route(
                                        RouteProps::builder()
                                            .path(path!("/404"))
                                            .view(not_found_page)
                                            .build(),
                                    ),
                                )
                            }))
                            .build(),
                    )
                }))
                .build(),
        )),
        crate::islands::navigation_menu::NavigationMenu(),
    )
}
