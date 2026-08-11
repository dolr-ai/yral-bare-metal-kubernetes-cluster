use auth::logout_identity;
use codee::string::FromToStringCodec;
use component::loading::Loading;
use consts::{DEVICE_ID, NOTIFICATIONS_ENABLED_STORE};
use leptos::prelude::*;
use leptos_router::components::Redirect;
use leptos_use::storage::use_local_storage;
use state::canisters::auth_state;
use utils::types::NewIdentity;

#[component]
pub fn Logout() -> impl IntoView {
    let auth = auth_state();
    let auth_res = OnceResource::new_blocking(logout_identity());

    let (_, set_notifs_enabled, _) =
        use_local_storage::<bool, FromToStringCodec>(NOTIFICATIONS_ENABLED_STORE);

    let (_, _set_device_id, _) = use_local_storage::<String, FromToStringCodec>(DEVICE_ID);

    view! {
        <Loading text="Logging out...".to_string()>
            <Suspense>
                {move || Suspend::new(async move {
                    let res = auth_res.await;
                    match res {
                        Ok(id) => {
                            auth.set_new_identity(NewIdentity::new_without_username(id), false);
                            set_notifs_enabled.set(false);
                            #[cfg(feature = "hydrate")]
                            {
                                let device_id = uuid::Uuid::new_v4().to_string();
                                set_device_id.set(device_id);
                            }
                            view! { <Redirect path="/menu" /> }
                        }
                        Err(e) => {
                            view! { <Redirect path=format!("/error?err={e}") /> }
                        }
                    }
                })}
            </Suspense>
        </Loading>
    }
}
