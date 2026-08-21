// Content loading and parsing for the my-website Leptos app.
//
// In islands mode, #[component] functions (non-island) run server-side only.
// This module reads markdown files with YAML frontmatter at startup, parses
// them into typed structs, and provides them via Leptos context to pages.
//
// Blog posts and project entries are stored as markdown files in content/
// and are embedded into the binary at compile time using include_str! macros
// via the ContentProvider, which is constructed once and shared via context.

pub mod types;
#[cfg(feature = "ssr")]
pub mod loader;
pub mod provider;
pub mod render;

pub use provider::ContentProvider;
pub use types::{BlogPost, BlogPostFrontmatter, ProjectEntry, ProjectEntryFrontmatter};