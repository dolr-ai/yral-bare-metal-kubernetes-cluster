use crate::overlay::{PopupOverlay, PopupOverlayProps};
use leptos::{children::ToChildren, ev, html, prelude::*, reactive::wrappers::write::SignalSetter};
use leptos_icons::{Icon, IconProps};
use state::app_state::AppState;
use utils::{
    host::get_host,
    web::{copy_to_clipboard, share_url},
};

use crate::icons::share_icon::ShareIcon;

pub fn share_content(
    share_link: String,
    message: String,
    show_popup: SignalSetter<bool>,
) -> impl IntoView {
    let app_state = use_context::<AppState>();
    let share_link_social = share_link.clone();
    let copy = share_link.clone();
    let copy_clipboard = move |_| {
        copy_to_clipboard(&copy);
    };

    html::div()
        .attr("class", "flex flex-col gap-6 items-center p-6 w-full h-full bg-white rounded-lg shadow-lg")
        .child(
            html::div()
                .attr("class", "flex flex-col gap-2 items-center")
                .child(
                    html::img()
                        .attr("class", "w-16 h-16 md:w-20 md:h-20")
                        .attr(
                            "src",
                            format!(
                                "/{}/favicon.svg",
                                app_state.clone().unwrap().asset_path()
                            ),
                        )
                        .attr(
                            "alt",
                            format!("{} Logo", app_state.clone().unwrap().name),
                        ),
                )
                .child(
                    html::span()
                        .attr("class", "text-xl font-semibold text-center md:text-2xl")
                        .child("Share this app"),
                ),
        )
        .child(social_share(message, share_link_social))
        .child(
            html::div()
                .attr("class", "flex overflow-x-auto justify-center items-center px-10 mx-1 space-x-2 w-full h-10 rounded-xl border-2 md:h-20 border-neutral-700")
                .child(
                    html::span()
                        .attr("class", "text-lg text-black md:text-xl truncate")
                        .child(share_link.clone()),
                )
                .child(
                    html::button()
                        .on(ev::click, copy_clipboard)
                        .child(
                            Icon(IconProps::builder().icon(icondata::BiCopyRegular).build())
                                .attr("class", "w-6 h-6 text-black cursor-pointer"),
                        ),
                ),
        )
        .child(
            html::button()
                .on(ev::click, move |_| show_popup.set(false))
                .attr("class", "py-4 w-3/4 text-lg text-center text-white rounded-full bg-primary-600")
                .child("Back"),
        )
        .into_any()
}

fn social_share(share_link: String, message: String) -> impl IntoView {
    let encoded_message = urlencoding::encode(&message);

    let facebook_url = format!("http://www.facebook.com/share.php?u={share_link}&quote={encoded_message}");
    let whatsapp_url = format!("https://wa.me/?text={encoded_message}");
    let twitter_url = format!("https://twitter.com/intent/tweet?text={encoded_message}");
    let telegram_url = format!("https://telegram.me/share/url?url={}", &share_link);
    let linkedin_url = format!(
        "https://linkedin.com/sharing/share-offsite/?url={}&title={}",
        &share_link, encoded_message
    );

    let social_icon = |icon: &'static icondata_core::IconData, class: &'static str, href: String| {
        html::a()
            .attr("href", href)
            .attr("target", "_blank")
            .child(Icon(IconProps::builder().icon(icon).build()).attr("class", class))
    };

    html::div()
        .attr("class", "flex gap-4")
        .child(social_icon(icondata::BsFacebook, "text-3xl md:text-4xl text-primary-600", facebook_url))
        .child(social_icon(icondata::BsTwitterX, "text-3xl md:text-4xl text-primary-600", twitter_url))
        .child(social_icon(icondata::FaSquareWhatsappBrands, "text-3xl md:text-4xl text-primary-600", whatsapp_url))
        .child(social_icon(icondata::TbBrandLinkedinFilled, "text-3xl md:text-4xl text-primary-600", linkedin_url))
        .child(social_icon(icondata::TbBrandTelegramOutline, "text-3xl md:text-4xl text-primary-600", telegram_url))
}

pub fn share_button_with_fallback_popup(
    share_link: String,
    message: String,
    style: String,
) -> impl IntoView {
    let base_url = get_host();
    let show_fallback = RwSignal::new(false);
    let share_link_c = share_link.clone();
    let on_share_click = move |event: ev::MouseEvent| {
        event.stop_propagation();
        if share_url(&share_link_c).is_none() {
            show_fallback.set(true);
        }
    };

    let class = format!(
        "text-white text-center text-lg md:text-xl flex items-center justify-center {style}",
    );

    let full_share_link = format!("{base_url}{share_link}");
    let message_clone = message.clone();

    (
        html::button()
            .on(ev::click, on_share_click)
            .attr("class", class)
            .child(
                Icon(IconProps::builder().icon(ShareIcon).build())
                    .attr("class", "h-6 w-6 text-neutral-300"),
            ),
        PopupOverlay(
            PopupOverlayProps::builder()
                .show(show_fallback)
                .children(ToChildren::to_children(move || {
                    share_content(
                        full_share_link.clone(),
                        message_clone.clone(),
                        show_fallback.into(),
                    )
                }))
                .build(),
        ),
    )
        .into_any()
}
