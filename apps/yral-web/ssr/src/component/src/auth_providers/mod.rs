#[cfg(feature = "ssr")]
mod server_impl;
#[cfg(any(feature = "oauth-ssr", feature = "oauth-hydrate"))]
pub mod yral;

use leptos::logging;
use leptos::prelude::ServerFnError;
use leptos::{ev, html, prelude::*, reactive::wrappers::write::SignalSetter};
use leptos_icons::{Icon, IconProps};
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
#[cfg(any(feature = "oauth-ssr", feature = "oauth-hydrate"))]
fn login_prov_button<Cb: Fn(ev::MouseEvent) + 'static>(
    prov: ProviderKind,
    class: Oco<'static, str>,
    on_click: Cb,
    disabled: Signal<bool>,
    children: impl IntoView,
) -> impl IntoView {
    let ctx: LoginProvCtx = expect_context();

    let click_action = Action::new(move |()| async move {});

    html::button()
        .attr("disabled", move || ctx.processing.get().is_some() || disabled.get())
        .attr("class", class)
        .on(ev::click, move |event| {
            ctx.set_processing.set(Some(prov));
            on_click(event);
            click_action.dispatch(());
        })
        .child(children)
}

pub fn login_providers(
    show_modal: RwSignal<bool>,
    lock_closing: RwSignal<bool>,
    redirect_to: Option<String>,
    reload_window: bool,
    text: String,
) -> impl IntoView {
    let auth = auth_state();

    let processing = RwSignal::new(None);

    let nav = use_navigate();

    let login_action = Action::new(move |new_id: &NewIdentity| {
        let new_id = new_id.clone();
        let redirect_to = redirect_to.clone();
        let nav = nav.clone();
        send_wrap(async move {
            let canisters = auth
                .set_new_identity_and_wait_for_authentication(new_id.clone(), true)
                .await?;

            if let Err(e) = handle_user_login(canisters.clone(), new_id.email).await {
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
            logging::log!("email: {:?}", val.email);
            login_action.dispatch(val);
        }),
    };
    provide_context(ctx);

    let heading_text: AnyView = if text.is_empty() {
        html::span()
            .child("Login in to watch, play & earn Bitcoin")
            .into_any()
    } else {
        html::span().child(text).into_any()
    };

    html::div()
        .attr("class", "flex justify-center items-center py-6 px-4 w-full h-full cursor-auto")
        .child(
            html::div()
                .attr("class", "overflow-hidden relative items-center w-full max-w-md rounded-md cursor-auto h-fit bg-neutral-950")
                .child(
                    html::img()
                        .attr("src", "/img/common/refer-bg.webp")
                        .attr("class", "object-cover absolute inset-0 z-0 w-full h-full opacity-40"),
                )
                .child(
                    html::div()
                        .attr("style", "background: radial-gradient(circle, rgba(226, 1, 123, 0.4) 0%, rgba(255,255,255,0) 50%);")
                        .attr("class", "absolute z-[1] size-[50rem] -left-[75%] -top-[50%]"),
                )
                .child(
                    html::button()
                        .attr("class", "flex absolute top-4 right-4 justify-center items-center text-lg text-center text-white rounded-full md:text-xl size-6 bg-neutral-600 z-[3]")
                        .on(ev::click, move |_| show_modal.set(false))
                        .child(Icon(IconProps::builder().icon(icondata::ChCross).build())),
                )
                .child(
                    html::div()
                        .attr("class", "flex relative flex-col gap-8 justify-center items-center py-10 px-12 text-white z-[2]")
                        .child(
                            html::img()
                                .attr("src", "/img/common/join-yral.webp")
                                .attr("class", "object-contain h-52"),
                        )
                        .child(
                            html::div()
                                .attr("class", "text-base font-bold text-center")
                                .child(heading_text),
                        )
                        .child(
                            html::div()
                                .attr("class", "flex flex-col gap-4 items-center w-full")
                                .child({
                                    #[cfg(any(feature = "oauth-ssr", feature = "oauth-hydrate"))]
                                    { yral::yral_auth_provider().into_any() }
                                    #[cfg(not(any(feature = "oauth-ssr", feature = "oauth-hydrate")))]
                                    { ().into_any() }
                                }),
                        )
                        .child(
                            html::div()
                                .attr("class", "flex flex-col items-center text-center text-md")
                                .child(html::div().child("By signing up, you agree to our"))
                                .child(
                                    html::a()
                                        .attr("class", "font-bold text-pink-300")
                                        .attr("target", "_blank")
                                        .attr("href", "https://yral.com/terms-android")
                                        .child("Terms of Service"),
                                ),
                        ),
                ),
        )
}
