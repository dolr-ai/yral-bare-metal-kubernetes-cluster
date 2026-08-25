use leptos::ev;
use leptos::html;
use leptos::prelude::*;

use crate::icons::flat_color::deployment::deployment_icon;
use crate::icons::flat_color::home::home_icon;
use crate::icons::flat_color::news::news_icon;

/// On wide screens the navigation menu auto-opens. Below this width the user
/// toggles it manually via the hamburger button. Matches the original SvelteKit
/// constant in `$utils/constants`.
#[allow(dead_code)] // used in #[cfg(target_arch = "wasm32")] blocks only
const WIDTH_TO_AUTO_OPEN_NAVIGATION_MENU_AT: i32 = 1536;

/// Type-erases a flat-color icon builder into a `fn(&str) -> AnyView` so all nav
/// links share a uniform icon type in the `NAV_LINKS` const array. Each icon
/// function returns a distinct concrete `impl IntoView` type, so we wrap them.
fn home_icon_view(class_name: &str) -> AnyView {
    home_icon(class_name).into_any()
}

/// See [`home_icon_view`].
fn news_icon_view(class_name: &str) -> AnyView {
    news_icon(class_name).into_any()
}

/// See [`home_icon_view`].
fn deployment_icon_view(class_name: &str) -> AnyView {
    deployment_icon(class_name).into_any()
}

/// A single entry in the navigation menu. The `icon` field holds a function
/// pointer to one of the `*_icon_view` wrappers above, returning `AnyView` so
/// all entries share a uniform type.
struct NavLink {
    text: &'static str,
    url: &'static str,
    icon: fn(&str) -> AnyView,
}

/// The three top-level navigation links, each with a flat-color icon.
const NAV_LINKS: &[NavLink] = &[
    NavLink {
        text: "About Me",
        url: "/",
        icon: home_icon_view,
    },
    NavLink {
        text: "Blog",
        url: "/blog",
        icon: news_icon_view,
    },
    NavLink {
        text: "Things I've built",
        url: "/projects",
        icon: deployment_icon_view,
    },
];

/// Interactive navigation menu island.
///
/// A floating hamburger button (bottom-right) toggles a navigation panel. On
/// wide screens (>= 1536px) the panel auto-opens on mount; on narrower screens
/// the user toggles it manually by clicking the hamburger button. Clicking a
/// nav link on a narrow screen closes the panel again.
///
/// This is an island because it requires client-side interactivity (click
/// handler + reactive open/close state + window-width check on mount). The
/// `#[island]` macro is the one necessary exception to the no-macros rule —
/// it generates the `wasm_bindgen` exports and `Island::new` wiring needed for
/// hydration. The function body uses builder syntax (no `view!` macro).
#[island]
pub fn NavigationMenu() -> impl IntoView {
    let (menu_open, set_menu_open) = signal(false);

    // Auto-open the menu on wide screens. This only runs in the browser (the
    // `#[island]` body compiles for both SSR and CSR, but `web_sys` is only
    // available on wasm32, so the whole effect is cfg-gated to the client).
    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            if !menu_open.get_untracked() {
                let width_is_wide = leptos::web_sys::window()
                    .expect("window should be available in wasm")
                    .inner_width()
                    .ok()
                    .and_then(|value| value.as_f64())
                    .map(|pixels| pixels as i32 >= WIDTH_TO_AUTO_OPEN_NAVIGATION_MENU_AT)
                    .unwrap_or(false);
                if width_is_wide {
                    set_menu_open.set(true);
                }
            }
        });
    }

    html::div()
        .child(
            // Hamburger button (floating, bottom-right)
            html::button()
                .attr("type", "button")
                .attr("aria-label", "Toggle navigation menu")
                .attr(
                    "class",
                    "w-12 h-12 fixed bottom-8 right-8 bg-gradient-to-tr from-emerald-500 via-blue-500 to-fuchsia-500 rounded-lg focus:outline-none focus:ring focus:ring-emerald-300 shadow-lg z-10",
                )
                .on(ev::click, move |_| {
                    set_menu_open.update(|open| *open = !*open)
                })
                .child(
                    html::div()
                        .class(move || if menu_open.get() { Some("open") } else { None })
                        .child(html::span().attr("class", "block h-0.5 w-6 bg-white my-1.5 rounded-full transition-all"))
                        .child(html::span().attr("class", "block h-0.5 w-6 bg-white my-1.5 rounded-full transition-all"))
                        .child(html::span().attr("class", "block h-0.5 w-6 bg-white my-1.5 rounded-full transition-all")),
                ),
        )
        .child(move || {
            if menu_open.get() {
                Some(
                    html::aside()
                        .attr(
                            "class",
                            "fixed bottom-8 right-8 w-64 h-96 shadow rounded-lg bg-gray-50",
                        )
                        .child(
                            html::nav().child(
                                html::ul().child(
                                    NAV_LINKS
                                        .iter()
                                        .map(|link| {
                                            let set_menu_open = set_menu_open;
                                            html::a()
                                                .attr("href", link.url)
                                                .attr(
                                                    "class",
                                                    "block py-2 px-6 text-lg text-emerald-800 hover:text-emerald-600 active:text-emerald-600 hover:bg-gradient-to-tr hover:from-emerald-100 hover:via-blue-100 hover:to-fuchsia-100 active:bg-gradient-to-tr active:from-emerald-100 active:via-blue-100 active:to-fuchsia-100",
                                                )
                                                .on(ev::click, move |_| {
                                                    #[cfg(target_arch = "wasm32")]
                                                    {
                                                        let width_is_wide =
                                                            leptos::web_sys::window()
                                                                .expect("window should be available in wasm")
                                                                .inner_width()
                                                                .ok()
                                                                .and_then(|value| {
                                                                    value.as_f64()
                                                                })
                                                                .map(|pixels| {
                                                                    pixels as i32
                                                                        >= WIDTH_TO_AUTO_OPEN_NAVIGATION_MENU_AT
                                                                })
                                                                .unwrap_or(false);
                                                        if !width_is_wide {
                                                            set_menu_open.set(false);
                                                        }
                                                    }
                                                    // On non-wasm (SSR) there is nothing to
                                                    // close — navigation is a full page load.
                                                    #[cfg(not(target_arch = "wasm32"))]
                                                    {
                                                        let _ = set_menu_open;
                                                    }
                                                })
                                                .child(
                                                    html::li()
                                                        .attr(
                                                            "class",
                                                            "h-10 flex flex-row items-center",
                                                        )
                                                        .child((link.icon)("h-5 mr-3"))

                                                        .child(
                                                            html::span().child(link.text),
                                                        ),
                                                )
                                        })
                                        .collect::<Vec<_>>(),
                                ),
                            ),
                        ),
                )
            } else {
                None
            }
        })
}