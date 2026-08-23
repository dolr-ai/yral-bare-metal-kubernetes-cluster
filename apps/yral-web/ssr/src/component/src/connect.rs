use super::login_modal::login_modal;
use crate::buttons::highlighted_button;
use leptos::prelude::*;

pub fn connect_login(
    login_text: &'static str,
    cta_location: &'static str,
    show_login: RwSignal<bool>,
    redirect_to: Option<String>,
) -> impl IntoView {
    let _ = cta_location;
    (
        highlighted_button(
            move || {
                if show_login.get() {
                    "Connecting..."
                } else {
                    login_text
                }
            },
            move || {
                show_login.set(true);
            },
            "w-full".to_string(),
            false,
            false,
        ),
        login_modal(show_login, redirect_to, false, String::new()),
    )
}
