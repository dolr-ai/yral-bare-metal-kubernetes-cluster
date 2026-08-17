use codee::string::FromToStringCodec;
use consts::NOTIFICATIONS_ENABLED_STORE;
use leptos::prelude::*;
use leptos::web_sys::Notification;
use leptos_icons::Icon;
use leptos_use::storage::use_local_storage;
use state::canisters::auth_state;

use crate::{
    buttons::HighlightedButton, icons::notification_nudge::NotificationNudgeIcon,
    overlay::ShadowOverlay,
};

#[component]
pub fn NotificationNudge(pop_up: RwSignal<bool>) -> impl IntoView {
    let auth = auth_state();

    let (notifs_enabled, set_notifs_enabled, _) =
        use_local_storage::<bool, FromToStringCodec>(NOTIFICATIONS_ENABLED_STORE);

    let popup_signal = Signal::derive(move || {
        !notifs_enabled.get() && pop_up.get()
    });

    let notification_action: Action<(), ()> = Action::new_unsync(move |()| async move {
        // Push notifications decommissioned — just toggle the local state.
        let _ = auth.auth_cans_if_available();
        set_notifs_enabled.set(true);
    });

    view! {
        <ShadowOverlay show=popup_signal>
            <div class="fixed top-1/2 left-1/2 p-8 w-full text-white rounded-lg shadow-xl transform -translate-x-1/2 -translate-y-1/2 bg-neutral-900 min-w-[343px] max-w-[550px]">
                <button
                    on:click=move |_| {
                        pop_up.set(false);
                    }
                    aria-label="Close notification"
                    class="absolute top-3 right-3 p-1 rounded-full transition-colors hover:text-white bg-neutral-800 text-neutral-300"
                >
                    <Icon icon=icondata::IoClose attr:class="w-6 h-6" />
                </button>

                <div class="flex flex-col gap-4 items-center pt-4 text-center">
                    <Icon icon=NotificationNudgeIcon attr:class="w-32 h-32 mb-2 text-orange-500" />
                    <h1 class="mb-2 text-2xl font-bold">"Stay in the Loop!"</h1>
                    <p class="mb-6 max-w-xs text-lg font-light text-neutral-400">
                        "Your video is processing in the background. Enable notifications so you don\'t miss a beat — feel free to explore the app while we handle the upload!"
                    </p>
                    <HighlightedButton
                        alt_style=false
                        on_click=move || {
                            notification_action.dispatch(());
                        }
                        classes="w-full py-3 bg-linear-to-r from-fuchsia-600 to-pink-500 hover:from-fuchsia-500 hover:to-pink-400 text-white font-semibold rounded-lg shadow-md transition-all"
                            .to_string()
                    >
                        <span>"Turn on alerts"</span>
                    </HighlightedButton>
                </div>
            </div>
        </ShadowOverlay>
    }
}
