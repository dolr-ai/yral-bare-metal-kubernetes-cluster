// Rendering helpers for content. These are pure functions that convert
// content data into Leptos views using builder syntax.

use leptos::prelude::*;

use super::types::{BlogPost, ProjectEntry};

/// Renders raw HTML (from markdown) safely into a Leptos view.
///
/// The markdown → HTML conversion happens server-side via comrak. The resulting
/// HTML is inserted into the DOM using leptos's trusted HTML injection, which
/// bypasses Leptos's HTML escaping. This is safe because the content is our own
/// markdown, not user input.
pub fn render_raw_html(html: &str) -> impl IntoView {
    leptos::html::div().inner_html(html.to_string())
}

/// Formats an ISO 8601 date string (e.g. "2020-04-05") as a human-readable
/// date string (e.g. "Sun Apr 5 2020").
pub fn format_date_display(iso_date: &str) -> String {
    // Parse the ISO date and format it like JavaScript's Date.toDateString()
    // which produces "Wed Apr 05 2020" style output.
    match chrono::NaiveDate::parse_from_str(iso_date, "%Y-%m-%d") {
        Ok(date) => date.format("%a %b %d %Y").to_string(),
        Err(_) => iso_date.to_string(),
    }
}

/// Formats an ISO 8601 date string as "MMM yyyy" (e.g. "Apr 2021").
pub fn format_month_year(iso_date: &str) -> String {
    match chrono::NaiveDate::parse_from_str(iso_date, "%Y-%m-%d") {
        Ok(date) => date.format("%b %Y").to_string(),
        Err(_) => iso_date.to_string(),
    }
}

/// Formats blog post tags as "#tag1 #tag2" string.
pub fn format_tags(tags: &[String]) -> String {
    tags.iter()
        .map(|tag| format!("#{tag}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Formats project technologies as "#tech1 #tech2" string.
pub fn format_technologies(technologies: &[String]) -> String {
    technologies
        .iter()
        .map(|tech| format!("#{tech}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Returns the URL for a blog post topic icon image.
pub fn blog_post_icon_url(icon: &str) -> String {
    format!("/assets/icons/post-topics/{icon}")
}

/// Returns the URL for a project entry cover photo image.
pub fn project_cover_photo_url(cover_photo: &str) -> String {
    format!("/assets/images/routes/projects/entries/{cover_photo}")
}

/// Extracts the slug path component from a full blog post slug URL.
/// E.g. "/blog/posts/react-hooks-usestate" → "react-hooks-usestate"
pub fn extract_blog_post_slug(post: &BlogPost) -> &str {
    post.slug
        .strip_prefix("/blog/posts/")
        .unwrap_or(&post.slug)
}

/// Extracts the slug path component from a full project entry slug URL.
/// E.g. "/projects/entries/go-bazzinga" → "go-bazzinga"
pub fn extract_project_entry_slug(entry: &ProjectEntry) -> &str {
    entry
        .slug
        .strip_prefix("/projects/entries/")
        .unwrap_or(&entry.slug)
}