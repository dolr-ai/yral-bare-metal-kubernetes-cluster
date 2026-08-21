// SEO meta header component.
//
// Renders the document `<title>` and social-sharing meta tags (Twitter Card
// and Open Graph) via leptos_meta's `Title` and `Meta` components. These are
// leptos_meta component macros, so we invoke them through their generated
// props builder structs (`TitleProps::builder()`, `MetaProps::builder()`)
// rather than the `view!` macro — the same pattern used in `app.rs`.
//
// `Title` and `Meta` register their content into the `MetaContext` during
// server-side rendering; `MetaTags()` in the document shell (`app.rs`) then
// injects the registered tags into the `<head>`. This is why `seo_meta_header`
// can be called from anywhere in the view tree (e.g. inside a page `<main>`)
// and still land in the `<head>` of the rendered HTML.

use leptos::prelude::*;
use leptos_meta::{Meta, MetaProps, Title, TitleProps};

/// The default social preview image used across all pages.
const SOCIAL_PREVIEW_IMAGE_URL: &str =
    "https://saikat.dev/assets/logos/logo-square-192.png";

/// The Twitter `@handle` for the site author.
const TWITTER_HANDLE: &str = "@saikatdas0790";

/// Renders the SEO meta header for a page given its title and description.
///
/// Returns a tuple of `Title` + several `Meta` components. leptos_meta collects
/// these into the document `<head>` during SSR.
pub fn seo_meta_header(site_title: &str, site_description: &str) -> impl IntoView {
    let site_title = site_title.to_string();
    let site_description = site_description.to_string();

    (
        Title(TitleProps::builder().text(site_title.clone()).build()),
        // Standard meta description.
        Meta(
            MetaProps::builder()
                .name("description")
                .content(site_description.clone())
                .build(),
        ),
        // Twitter Card metadata.
        Meta(
            MetaProps::builder()
                .name("twitter:card")
                .content("summary".to_string())
                .build(),
        ),
        Meta(
            MetaProps::builder()
                .name("twitter:site")
                .content(TWITTER_HANDLE.to_string())
                .build(),
        ),
        Meta(
            MetaProps::builder()
                .name("twitter:title")
                .content(site_title.clone())
                .build(),
        ),
        Meta(
            MetaProps::builder()
                .name("twitter:description")
                .content(site_description.clone())
                .build(),
        ),
        Meta(
            MetaProps::builder()
                .name("twitter:image")
                .content(SOCIAL_PREVIEW_IMAGE_URL.to_string())
                .build(),
        ),
        // Open Graph metadata.
        Meta(
            MetaProps::builder()
                .property("og:type")
                .content("website".to_string())
                .build(),
        ),
        Meta(
            MetaProps::builder()
                .property("og:title")
                .content(site_title.clone())
                .build(),
        ),
        Meta(
            MetaProps::builder()
                .property("og:description")
                .content(site_description.clone())
                .build(),
        ),
        Meta(
            MetaProps::builder()
                .property("og:image")
                .content(SOCIAL_PREVIEW_IMAGE_URL.to_string())
                .build(),
        ),
    )
}