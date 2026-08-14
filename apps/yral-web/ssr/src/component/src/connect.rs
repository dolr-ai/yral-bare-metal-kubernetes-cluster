use leptos::prelude::*;

use crate::buttons::HighlightedButton;

use super::login_modal::LoginModal;

#[component]
pub fn ConnectLogin(
    #[prop(optional, default = "Login")] login_text: &'static str,
    #[prop(optional, default = "menu")] cta_location: &'static str,
    #[prop(optional, default = RwSignal::new(false))] show_login: RwSignal<bool>,
    #[prop(optional, into)] redirect_to: Option<String>,
) -> impl IntoView {
    let _ = cta_location;
    view! {
        <HighlightedButton
            classes="w-full".to_string()
            alt_style=false
            disabled=false
            on_click=move || {
                show_login.set(true);
            }
        >
            {move || if show_login.get() { "Connecting..." } else { login_text }}
        </HighlightedButton>
        <LoginModal show=show_login redirect_to />
    }
}
