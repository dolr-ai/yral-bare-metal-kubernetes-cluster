use serde::{Deserialize, Serialize};

/// Frontmatter for a blog post markdown file.
///
/// Fields map directly to the YAML frontmatter in content/blog/<slug>.md:
/// ```yaml
/// title: "Post Title"
/// description: "Short description"
/// author: "Saikat Das"
/// tags: [tag1, tag2]
/// icon: "react.png"
/// date: "2020-04-05"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogPostFrontmatter {
    pub title: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub icon: String,
    pub date: String,
}

/// A fully-loaded blog post with its frontmatter, slug, and rendered HTML body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlogPost {
    pub title: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub icon: String,
    pub created: String,
    pub slug: String,
    /// The rendered HTML body (from markdown → HTML via comrak).
    pub body_html: String,
}

/// Frontmatter for a project entry markdown file.
///
/// ```yaml
/// title: "Project Title"
/// description: "Short description"
/// technologiesUsed: [Tech1, Tech2]
/// author: "Saikat Das"
/// coverPhoto: "slug/cover.jpg"
/// startDate: "2021-04-25"
/// endDate: "2021-07-19"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEntryFrontmatter {
    pub title: String,
    pub description: String,
    pub technologies_used: Vec<String>,
    pub author: String,
    pub cover_photo: String,
    pub start_date: String,
    pub end_date: String,
}

/// A fully-loaded project entry with its frontmatter, slug, and rendered HTML body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEntry {
    pub title: String,
    pub description: String,
    pub technologies_used: Vec<String>,
    pub author: String,
    pub cover_photo: String,
    pub start_date: String,
    pub end_date: String,
    pub slug: String,
    pub body_html: String,
}