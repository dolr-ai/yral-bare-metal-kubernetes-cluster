// Server-side content loader: reads markdown files from content/ directory,
// parses YAML frontmatter, and converts markdown body to HTML via comrak.

use std::path::Path;

use comrak::{markdown_to_html, Options};
use noyalib::compat::serde_yaml;

use super::types::{BlogPost, BlogPostFrontmatter, ProjectEntry, ProjectEntryFrontmatter};

/// Parsed markdown file: frontmatter + body markdown.
struct ParsedMarkdown<F> {
    frontmatter: F,
    body_markdown: String,
}

/// Splits a markdown file into YAML frontmatter (between `---` delimiters)
/// and the remaining markdown body. Returns an error if the frontmatter is
/// missing or malformed.
fn split_frontmatter<F: serde::de::DeserializeOwned + 'static>(
    content: &str,
) -> Result<ParsedMarkdown<F>, String> {
    let content = content.trim_start_matches('\u{feff}');

    if !content.starts_with("---") {
        return Err("Missing frontmatter delimiter (---)".to_string());
    }

    // Find the closing --- after the opening one
    let after_first_delimiter = &content[3..];
    let end_of_frontmatter = after_first_delimiter
        .find("\n---")
        .ok_or("Missing closing frontmatter delimiter")?;

    let yaml_content = &after_first_delimiter[..end_of_frontmatter].trim();
    let body_markdown = after_first_delimiter[end_of_frontmatter + 4..].trim().to_string();

    let frontmatter: F =
        serde_yaml::from_str(yaml_content).map_err(|err| format!("YAML parse error: {err}"))?;

    Ok(ParsedMarkdown {
        frontmatter,
        body_markdown,
    })
}

/// Converts markdown to HTML using comrak with GitHub-flavored markdown options.
fn render_markdown_to_html(markdown: &str) -> String {
    let mut options = Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.parse.smart = true;
    options.render.r#unsafe = true; // Allow raw HTML (e.g., <img>, <iframe> in posts)

    markdown_to_html(markdown, &options)
}

/// Loads and parses all blog post markdown files from content/blog/.
///
/// Each file is named <slug>.md. The slug is derived from the filename
/// (without the .md extension) and prefixed with /blog/posts/ for the URL.
///
/// Returns posts sorted newest-first by date.
pub fn load_all_blog_posts(content_dir: &Path) -> Vec<BlogPost> {
    let blog_dir = content_dir.join("blog");

    let mut posts: Vec<BlogPost> = Vec::new();

    let entries = match std::fs::read_dir(&blog_dir) {
        Ok(entries) => entries,
        Err(err) => {
            tracing::error!("Failed to read blog directory {:?}: {err}", blog_dir);
            return posts;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }

        let slug = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => {
                tracing::error!("Failed to read {:?}: {err}", path);
                continue;
            }
        };

        let parsed: ParsedMarkdown<BlogPostFrontmatter> = match split_frontmatter(&content) {
            Ok(parsed) => parsed,
            Err(err) => {
                tracing::error!("Failed to parse frontmatter for {}: {err}", slug);
                continue;
            }
        };

        let body_html = render_markdown_to_html(&parsed.body_markdown);

        posts.push(BlogPost {
            title: parsed.frontmatter.title,
            description: parsed.frontmatter.description,
            author: parsed.frontmatter.author,
            tags: parsed.frontmatter.tags,
            icon: parsed.frontmatter.icon,
            created: parsed.frontmatter.date,
            slug: format!("/blog/posts/{slug}"),
            body_html,
        });
    }

    // Sort newest-first by date (string comparison works for ISO 8601 dates)
    posts.sort_by(|a, b| b.created.cmp(&a.created));
    posts
}

/// Loads and parses all project entry markdown files from content/projects/.
///
/// Each file is named <slug>.md. The slug is derived from the filename
/// (without the .md extension) and prefixed with /projects/entries/ for the URL.
///
/// Returns entries sorted newest-first by start date.
pub fn load_all_project_entries(content_dir: &Path) -> Vec<ProjectEntry> {
    let projects_dir = content_dir.join("projects");

    let mut entries: Vec<ProjectEntry> = Vec::new();

    let dir_entries = match std::fs::read_dir(&projects_dir) {
        Ok(entries) => entries,
        Err(err) => {
            tracing::error!("Failed to read projects directory {:?}: {err}", projects_dir);
            return entries;
        }
    };

    for entry in dir_entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }

        let slug = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => {
                tracing::error!("Failed to read {:?}: {err}", path);
                continue;
            }
        };

        let parsed: ParsedMarkdown<ProjectEntryFrontmatter> = match split_frontmatter(&content) {
            Ok(parsed) => parsed,
            Err(err) => {
                tracing::error!("Failed to parse frontmatter for {}: {err}", slug);
                continue;
            }
        };

        let body_html = render_markdown_to_html(&parsed.body_markdown);

        entries.push(ProjectEntry {
            title: parsed.frontmatter.title,
            description: parsed.frontmatter.description,
            technologies_used: parsed.frontmatter.technologies_used,
            author: parsed.frontmatter.author,
            cover_photo: parsed.frontmatter.cover_photo,
            start_date: parsed.frontmatter.start_date,
            end_date: parsed.frontmatter.end_date,
            slug: format!("/projects/entries/{slug}"),
            body_html,
        });
    }

    // Sort newest-first by start date
    entries.sort_by(|a, b| b.start_date.cmp(&a.start_date));
    entries
}

/// Finds a single blog post by its slug (the URL path component, e.g. "react-hooks-usestate").
pub fn find_blog_post_by_slug<'a>(posts: &'a [BlogPost], slug: &str) -> Option<&'a BlogPost> {
    let full_slug = format!("/blog/posts/{slug}");
    posts.iter().find(|post| post.slug == full_slug)
}

/// Finds a single project entry by its slug (the URL path component, e.g. "go-bazzinga").
pub fn find_project_entry_by_slug<'a>(entries: &'a [ProjectEntry], slug: &str) -> Option<&'a ProjectEntry> {
    let full_slug = format!("/projects/entries/{slug}");
    entries.iter().find(|entry| entry.slug == full_slug)
}