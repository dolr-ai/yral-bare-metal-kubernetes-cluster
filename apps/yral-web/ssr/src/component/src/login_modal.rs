use super::{
    auth_providers::login_providers,
    overlay::{ShadowOverlay, ShadowOverlayProps, ShowOverlay},
};
use leptos::{children::ToChildren, prelude::*};

pub fn login_modal(
    show: RwSignal<bool>,
    redirect_to: Option<String>,
    reload_window: bool,
    text: String,
) -> impl IntoView {
    let lock_closing = RwSignal::new(false);
    ShadowOverlay(
        ShadowOverlayProps::builder()
            .show(ShowOverlay::MaybeClosable {
                show,
                closable: lock_closing,
            })
            .children(ToChildren::to_children(move || {
                login_providers(
                    show,
                    lock_closing,
                    redirect_to.clone(),
                    reload_window,
                    text.clone(),
                )
            }))
            .build(),
    )
}
