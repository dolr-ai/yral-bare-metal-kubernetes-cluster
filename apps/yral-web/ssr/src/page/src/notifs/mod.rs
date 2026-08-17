use leptos::{either::Either, prelude::*};
use leptos_router::components::Redirect;
use state::canisters::auth_state;

use utils::user_identity::UserIdentity;

#[component]
fn NotifInnerComponent(details: UserIdentity) -> impl IntoView {
    let auth_state = auth_state();

    let on_token_click: Action<(), ()> = Action::new_unsync(move |()| async move {
        // Push notifications decommissioned — no-op.
        let _ = auth_state.auth_cans_if_available();
    });

    view! {
        <h1>"YRAL Notifs for"</h1>
        <h2>{details.username_or_fallback()}</h2>
        <br />
        <div class="flex flex-row gap-2 text-black">
            <button
                class="p-2 bg-gray-200 rounded-md"
                on:click=move |_| {
                    on_token_click.dispatch(());
                }
            >
                "Get Token"
            </button>
        </div>
    }
}

#[component]
pub fn Notif() -> impl IntoView {
    let auth = auth_state();
    view! {
        <div class="grid grid-cols-1 justify-items-center place-content-center w-screen h-screen">
            <Suspense>
                {move || Suspend::new(async move {
                    let res = auth.auth_cans().await;
                    match res {
                        Ok(cans) => {
                            Either::Left(
                                view! { <NotifInnerComponent details=cans.user_identity() /> },
                            )
                        }
                        Err(e) => {
                            Either::Right(view! { <Redirect path=format!("/error?err={e}") /> })
                        }
                    }
                })}
            </Suspense>
        </div>
    }
}
