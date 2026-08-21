// ContentProvider — loads all blog posts and project entries at startup and
// stores them for the lifetime of the server. Provided to pages via Leptos context.
//
// In islands mode, #[component] functions run server-side by default, so they
// can directly access this provider via expect_context() without #[server]
// functions or Suspense.

use std::sync::Arc;

#[cfg(feature = "ssr")]
use std::path::PathBuf;

use super::types::{BlogPost, ProjectEntry};

/// Holds all parsed content loaded at startup.
#[derive(Clone)]
pub struct ContentProvider {
    blog_posts: Arc<Vec<BlogPost>>,
    project_entries: Arc<Vec<ProjectEntry>>,
}

impl ContentProvider {
    /// Creates a new ContentProvider by loading all markdown files from the
    /// content/ directory relative to the crate root.
    ///
    /// Only available on the server (SSR). On the client (hydrate), this struct
    /// is constructed as an empty stub — islands mode means non-island components
    /// only run on the server, so the client never actually calls this.
    #[cfg(feature = "ssr")]
    pub fn new() -> Self {
        let content_dir = Self::resolve_content_dir();

        let blog_posts = super::loader::load_all_blog_posts(&content_dir);
        let project_entries = super::loader::load_all_project_entries(&content_dir);

        tracing::info!(
            "Loaded {} blog posts and {} project entries from {:?}",
            blog_posts.len(),
            project_entries.len(),
            content_dir
        );

        Self {
            blog_posts: Arc::new(blog_posts),
            project_entries: Arc::new(project_entries),
        }
    }

    /// Client-side stub — returns an empty ContentProvider.
    /// In islands mode, non-island components only run on the server,
    /// so this is never actually called on the client.
    #[cfg(not(feature = "ssr"))]
    pub fn new() -> Self {
        Self {
            blog_posts: Arc::new(Vec::new()),
            project_entries: Arc::new(Vec::new()),
        }
    }

    /// Returns all blog posts, sorted newest-first by date.
    pub fn blog_posts(&self) -> &[BlogPost] {
        &self.blog_posts
    }

    /// Returns all project entries, sorted newest-first by start date.
    pub fn project_entries(&self) -> &[ProjectEntry] {
        &self.project_entries
    }

    /// Finds a blog post by its slug (URL path component, e.g. "react-hooks-usestate").
    pub fn find_blog_post(&self, slug: &str) -> Option<&BlogPost> {
        let full_slug = format!("/blog/posts/{slug}");
        self.blog_posts.iter().find(|post| post.slug == full_slug)
    }

    /// Finds a project entry by its slug (URL path component, e.g. "go-bazzinga").
    pub fn find_project_entry(&self, slug: &str) -> Option<&ProjectEntry> {
        let full_slug = format!("/projects/entries/{slug}");
        self.project_entries.iter().find(|entry| entry.slug == full_slug)
    }

    /// Returns the top N most recent blog posts.
    pub fn recent_blog_posts(&self, count: usize) -> Vec<&BlogPost> {
        self.blog_posts.iter().take(count).collect()
    }

    /// Resolves the content/ directory path relative to the crate root.
    ///
    /// The content directory lives at apps/my-website/content/ relative to the
    /// workspace root. We resolve it using CARGO_MANIFEST_DIR (set at compile time).
    #[cfg(feature = "ssr")]
    fn resolve_content_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("content")
    }
}

impl Default for ContentProvider {
    fn default() -> Self {
        Self::new()
    }
}