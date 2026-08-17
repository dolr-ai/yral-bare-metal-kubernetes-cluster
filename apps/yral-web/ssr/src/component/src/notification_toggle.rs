use codee::string::FromToStringCodec;
use consts::NOTIFICATIONS_ENABLED_STORE;
use leptos::html::Input;
use leptos::{ev, prelude::*};
use leptos_icons::*;
use leptos_use::storage::use_local_storage;
use leptos_use::use_event_listener;
use state::canisters::auth_state;

use crate::toggle::Toggle;

#[component]
pub fn NotificationToggle(
    #[prop(optional)] show_icon: bool,
    #[prop(optional)] show_label: bool,
    #[prop(optional)] icon: Option<icondata::Icon>,
    #[prop(optional)] label_text: Option<String>,
    #[prop(optional)] custom_class: Option<String>,
) -> impl IntoView {
    // Default values
    let icon = icon.unwrap_or(icondata::BiCommentDotsRegular);
    let label_text = label_text.unwrap_or_else(|| "Enable Notifications".to_string());
    let custom_class =
        custom_class.unwrap_or_else(|| "flex items-center justify-between w-full".to_string());

    // Notifications state management
    let (notifs_enabled, set_notifs_enabled, _) =
        use_local_storage::<bool, FromToStringCodec>(NOTIFICATIONS_ENABLED_STORE);

    let notifs_enabled_signal = Signal::derive(move || {
        notifs_enabled.get()
    });

    let toggle_ref = NodeRef::<Input>::new();
    let auth = auth_state();

    // Main notification toggle action
    let on_toggle_action: Action<(), ()> = Action::new_unsync(move |()| async move {
        // Push notifications decommissioned — just toggle local state.
        let _ = auth.auth_cans_if_available();
        let notifs_enabled_val = notifs_enabled.get_untracked();
        set_notifs_enabled.set(!notifs_enabled_val);
    });

    // Listen for toggle changes
    _ = use_event_listener(toggle_ref, ev::change, move |_| {
        on_toggle_action.dispatch(());
    });

    if show_icon || show_label {
        view! {
            <div class=custom_class>
                <div class="flex flex-row gap-4 items-center flex-1">
                    {show_icon.then(|| view! { <Icon attr:class="text-2xl flex-shrink-0" icon=icon /> })}
                    {show_label.then(|| view! { <span class="text-wrap">{label_text}</span> })}
                </div>
                <div class="flex-shrink-0">
                    <Toggle checked=notifs_enabled_signal node_ref=toggle_ref />
                </div>
            </div>
        }.into_any()
    } else {
        view! {
            <div class=custom_class>
                <div class="flex-shrink-0">
                    <Toggle checked=notifs_enabled_signal node_ref=toggle_ref />
                </div>
            </div>
        }
        .into_any()
    }
}
