use leptos::{ev, html, prelude::*};
use leptos_icons::{Icon, IconProps};

/// Go back or navigate to a fallback route.
/// Does nothing in SSR mode — only navigates on the client (hydrate).
/// Ideal for calling from a button click handler.
pub fn go_back_or_fallback(fallback: &str) {
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = fallback;
        return;
    }
    #[cfg(feature = "hydrate")]
    {
        let win = window();
        let referrer = win
            .document()
            .map(|d| d.referrer())
            .and_then(|r| url::Url::parse(&r).ok());
        let cur_url = url::Url::parse(&win.location().href().unwrap_or_default()).ok();

        if cur_url.as_ref().and_then(|u| u.host_str())
            == referrer.as_ref().and_then(|r| r.host_str())
        {
            let history = leptos::web_sys::window().history();
            if let Ok(history) = history {
                _ = history.back();
            }
        } else {
            use_navigate()(fallback, Default::default());
        }
    }
}

pub fn back_button(fallback: Signal<String>) -> impl IntoView {
    html::button()
        .on(ev::click, move |_| go_back_or_fallback(&fallback.get_untracked()))
        .attr("class", "items-center")
        .child(Icon(IconProps::builder().icon(icondata::AiLeftOutlined).build()))
}
