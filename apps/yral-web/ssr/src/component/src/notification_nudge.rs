use codee::string::FromToStringCodec;
use consts::NOTIFICATIONS_ENABLED_STORE;
use leptos::{children::ToChildren, ev, html, prelude::*};
use leptos_icons::{Icon, IconProps};
use leptos_use::storage::use_local_storage;
use state::canisters::auth_state;

use crate::{
    buttons::highlighted_button, icons::notification_nudge::NotificationNudgeIcon,
    overlay::{ShadowOverlay, ShadowOverlayProps},
};

pub fn notification_nudge(pop_up: RwSignal<bool>) -> impl IntoView {
    let auth = auth_state();

    let (notifs_enabled, set_notifs_enabled, _) =
        use_local_storage::<bool, FromToStringCodec>(NOTIFICATIONS_ENABLED_STORE);

    let popup_signal = Signal::derive(move || !notifs_enabled.get() && pop_up.get());

    let notification_action: Action<(), ()> = Action::new_unsync(move |()| async move {
        let _ = auth.auth_cans_if_available();
        set_notifs_enabled.set(true);
    });

    ShadowOverlay(
        ShadowOverlayProps::builder()
            .show(popup_signal)
            .children(ToChildren::to_children(move || {
                html::div()
                    .attr("class", "fixed top-1/2 left-1/2 p-8 w-full text-white rounded-lg shadow-xl transform -translate-x-1/2 -translate-y-1/2 bg-neutral-900 min-w-[343px] max-w-[550px]")
                    .child(
                        html::button()
                            .on(ev::click, move |_| {
                                pop_up.set(false);
                            })
                            .attr("aria-label", "Close notification")
                            .attr("class", "absolute top-3 right-3 p-1 rounded-full transition-colors hover:text-white bg-neutral-800 text-neutral-300")
                            .child(Icon(IconProps::builder().icon(icondata::IoClose).build()).attr("class", "w-6 h-6")),
                    )
                    .child(
                        html::div()
                            .attr("class", "flex flex-col gap-4 items-center pt-4 text-center")
                            .child(Icon(IconProps::builder().icon(NotificationNudgeIcon).build()).attr("class", "w-32 h-32 mb-2 text-orange-500"))
                            .child(html::h1().attr("class", "mb-2 text-2xl font-bold").child("Stay in the Loop!"))
                            .child(
                                html::p()
                                    .attr("class", "mb-6 max-w-xs text-lg font-light text-neutral-400")
                                    .child("Your video is processing in the background. Enable notifications so you don't miss a beat — feel free to explore the app while we handle the upload!"),
                            )
                            .child(
                                highlighted_button(
                                    html::span().child("Turn on alerts"),
                                    move || {
                                        notification_action.dispatch(());
                                    },
                                    "w-full py-3 bg-linear-to-r from-fuchsia-600 to-pink-500 hover:from-fuchsia-500 hover:to-pink-400 text-white font-semibold rounded-lg shadow-md transition-all".to_string(),
                                    false,
                                    false,
                                ),
                            ),
                    )
            }))
            .build(),
    )
}
