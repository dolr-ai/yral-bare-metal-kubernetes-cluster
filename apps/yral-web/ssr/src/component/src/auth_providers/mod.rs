#[cfg(feature = "ssr")]
mod server_impl;
#[cfg(any(feature = "oauth-ssr", feature = "oauth-hydrate"))]
pub mod yral;

use leptos::logging;
use leptos::prelude::ServerFnError;
use leptos::{ev, prelude::*, reactive::wrappers::write::SignalSetter};
use leptos_icons::Icon;
use leptos_router::hooks::use_navigate;
use state::canisters::auth_state;
use state::canisters::AuthSession;
use utils::ProviderKind;
use utils::send_wrap;
use utils::types::NewIdentity;

#[server]
async fn mark_user_registered(user_id: String) -> Result<bool, ServerFnError> {
    server_impl::mark_user_registered(user_id).await
}

pub async fn handle_user_login(
    canisters: AuthSession,
    email: Option<String>,
) -> Result<(), ServerFnError> {
    let user_id = canisters.user_id();
    let first_time_login = mark_user_registered(user_id.clone()).await?;

    // Email is stored via SpacetimeDB (set_email reducer) if provided.
    // The old MetadataClient call is removed — SpacetimeDB handles this.
    if let Some(email) = email {
        leptos::logging::log!(
            "User {user_id} email: {email} (first_time_login={first_time_login})"
        );
    }

    Ok(())
}

#[derive(Clone, Copy)]
pub struct LoginProvCtx {
    /// Setting processing should only be done on login cancellation
    /// and inside [LoginProvButton]
    /// stores the current provider handling the login
    pub processing: ReadSignal<Option<ProviderKind>>,
    pub set_processing: SignalSetter<Option<ProviderKind>>,
    pub login_complete: SignalSetter<NewIdentity>,
}

/// Login providers must use this button to trigger the login action
/// automatically sets the processing state to true
#[component]
fn LoginProvButton<Cb: Fn(ev::MouseEvent) + 'static>(
    prov: ProviderKind,
    #[prop(into)] class: Oco<'static, str>,
    on_click: Cb,
    #[prop(optional, into)] disabled: Signal<bool>,
    children: Children,
) -> impl IntoView {
    let ctx: LoginProvCtx = expect_context();

    let click_action = Action::new(move |()| async move {});

    view! {
        <button
            disabled=move || ctx.processing.get().is_some() || disabled.get()
            class=class
            on:click=move |ev| {
                ctx.set_processing.set(Some(prov));
                on_click(ev);
                click_action.dispatch(());
            }
        >

            {children()}
        </button>
    }
}

/// on_resolve -> a callback that returns the new principal
#[component]
pub fn LoginProviders(
    show_modal: RwSignal<bool>,
    lock_closing: RwSignal<bool>,
    redirect_to: Option<String>,
    #[prop(optional, into)] reload_window: bool,
    #[prop(optional)] text: String,
) -> impl IntoView {
    let auth = auth_state();

    let processing = RwSignal::new(None);

    let nav = use_navigate();

    let login_action = Action::new(move |new_id: &NewIdentity| {
        // Clone the necessary parts
        let new_id = new_id.clone();
        let redirect_to = redirect_to.clone();

        let nav = nav.clone();
        // let start = start.clone();
        // Capture the context signal setter
        send_wrap(async move {
            let canisters = auth
                .set_new_identity_and_wait_for_authentication(new_id.clone(), true)
                .await?;

            if let Err(e) = handle_user_login(
                canisters.clone(),
                new_id.email,
            )
            .await
            {
                log::warn!("failed to handle user login, err {e}. skipping");
            }

            if reload_window {
                let res = window().location().reload();
                logging::log!("Reloading window after login: {:#?}", res);
            }
            show_modal.set(false);

            if let Some(redir_loc) = redirect_to {
                nav(&redir_loc, Default::default());
            }

            Ok::<_, ServerFnError>(())
        })
    });

    let ctx = LoginProvCtx {
        processing: processing.read_only(),
        set_processing: SignalSetter::map(move |val: Option<ProviderKind>| {
            lock_closing.set(val.is_some());
            processing.set(val);
        }),
        login_complete: SignalSetter::map(move |val: NewIdentity| {
            // Dispatch just the DelegatedIdentityWire
            logging::log!("email: {:?}", val.email);
            login_action.dispatch(val);
        }),
    };
    provide_context(ctx);

    view! {
        <div class="flex justify-center items-center py-6 px-4 w-full h-full cursor-auto">
            <div class="overflow-hidden relative items-center w-full max-w-md rounded-md cursor-auto h-fit bg-neutral-950">
                <img
                    src="/img/common/refer-bg.webp"
                    class="object-cover absolute inset-0 z-0 w-full h-full opacity-40"
                />
                <div
                    style="background: radial-gradient(circle, rgba(226, 1, 123, 0.4) 0%, rgba(255,255,255,0) 50%);"
                    class="absolute z-[1] size-[50rem] -left-[75%] -top-[50%]"
                ></div>
                <button
                    on:click=move |_| show_modal.set(false)
                    class="flex absolute top-4 right-4 justify-center items-center text-lg text-center text-white rounded-full md:text-xl size-6 bg-neutral-600 z-[3]"
                >
                    <Icon icon=icondata::ChCross />
                </button>
                <div class="flex relative flex-col gap-8 justify-center items-center py-10 px-12 text-white z-[2]">
                    <img src="/img/common/join-yral.webp" class="object-contain h-52" />
                    <div class="text-base font-bold text-center">
                        {if text.is_empty() {
                            view! {
                                <span>
                                    "Login in to watch, play & earn Bitcoin"
                                </span>
                            }.into_any()
                        } else {
                            view! {
                                <span>
                                    {text}
                                </span>
                            }.into_any()
                        }}
                    </div>
                    <div class="flex flex-col gap-4 items-center w-full">
                        {
                            #[cfg(any(feature = "oauth-ssr", feature = "oauth-hydrate"))]
                            view! { <yral::YralAuthProvider /> }
                        }
                    </div>
                    <div class="flex flex-col items-center text-center text-md">
                        <div>"By signing up, you agree to our"</div>
                        <a class="font-bold text-pink-300" target="_blank" href="https://yral.com/terms-android">
                            "Terms of Service"
                        </a>
                    </div>
                </div>
            </div>
        </div>
    }
}
