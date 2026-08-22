pub fn get_host() -> String {
    #[cfg(feature = "hydrate")]
    {
        use leptos_use::use_window;

        use_window()
            .as_ref()
            .unwrap()
            .location()
            .host()
            .unwrap()
            .to_string()
    }

    #[cfg(feature = "ssr")]
    {
        use leptos::prelude::*;

        use axum::http::request::Parts;
        let parts: Option<Parts> = use_context();
        if parts.is_none() {
            return "".to_string();
        }
        let headers = parts.unwrap().headers;
        headers
            .get("Host")
            .map(|h| h.to_str().unwrap_or_default().to_string())
            .unwrap_or_default()
    }

    #[cfg(not(any(feature = "hydrate", feature = "ssr")))]
    {
        "".to_string()
    }
}

// TODO: migrate to AppType
pub fn show_nsfw_content() -> bool {
    false
}
