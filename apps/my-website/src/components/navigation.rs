// Navigation menu placeholder.
//
// The interactive navigation menu (hamburger toggle + navbar links) is
// implemented as an island so it hydrates in the browser — see
// `crate::islands::navigation_menu`. This module provides a placeholder used
// in non-island, server-rendered-only contexts where no interactivity is
// needed.

use leptos::html;
use leptos::prelude::*;

/// Returns an empty view. The real navigation menu lives in the
/// `navigation_menu` island; this placeholder exists so server-rendered-only
/// layouts have a drop-in that renders nothing.
pub fn navigation_menu_placeholder() -> impl IntoView {
    html::div()
}