use crate::nav_icons::*;
use codee::string::FromToStringCodec;
use consts::{
    AUTH_UTIL_COOKIES_MAX_AGE_MS,
    USER_PRINCIPAL_STORE,
};
use leptos::prelude::*;
use leptos_icons::*;
use leptos_router::hooks::use_location;
use leptos_use::{use_cookie_with_options, UseCookieOptions};

#[derive(Clone)]
struct NavItem {
    render_data: NavItemRenderData,
    cur_selected: Signal<bool>,
}

#[derive(Debug, Clone)]
enum NavItemRenderData {
    Icon {
        icon: icondata_core::Icon,
        filled_icon: Option<icondata_core::Icon>,
        href: Signal<String>,
    },
}

fn yral_nav_items() -> Vec<NavItem> {
    let cur_location = use_location();
    let path = cur_location.pathname;
    let (user_id, _) = use_cookie_with_options::<String, FromToStringCodec>(
        USER_PRINCIPAL_STORE,
        UseCookieOptions::default()
            .path("/")
            .max_age(AUTH_UTIL_COOKIES_MAX_AGE_MS),
    );

    vec![
        NavItem {
            render_data: NavItemRenderData::Icon {
                icon: WalletSymbol,
                filled_icon: Some(WalletSymbolFilled),
                href: "/wallet".into(),
            },
            cur_selected: Signal::derive(move || {
                // is selected only if the user is viewing their own wallet
                let Some(user_id) = user_id.get() else {
                    return false;
                };
                path.get().starts_with(&format!("/wallet/{user_id}"))
            }),
        },
    ]
}

fn get_nav_items() -> Vec<NavItem> {
    yral_nav_items()
}

#[component]
pub fn NavBar() -> impl IntoView {
    let items = get_nav_items();

    view! {
        <Suspense>
            <div class="flex fixed bottom-0 left-0 z-50 flex-row justify-between items-center px-6 w-full bg-black/80">
                {items
                    .iter()
                    .map(|item| {
                        let cur_selected = item.cur_selected;
                        let NavItemRenderData::Icon { icon, filled_icon, href } = item.render_data.clone();
                        view! { <NavIcon href icon filled_icon cur_selected /> }
                    })
                    .collect::<Vec<_>>()}
            </div>
        </Suspense>
    }
}

#[component]
fn NavIcon(
    #[prop(into)] href: Signal<String>,
    #[prop(into)] icon: icondata_core::Icon,
    #[prop(into)] filled_icon: Option<icondata_core::Icon>,
    #[prop(into)] cur_selected: Signal<bool>,
) -> impl IntoView {
    let on_click = move |_ev: leptos::ev::MouseEvent| {
        // Navigation click
    };
    view! {
        <a href=move || href.get() on:click=on_click class="flex justify-center items-center">
            <Show
                when=move || cur_selected.get()
                fallback=move || {
                    view! {
                        <div class="py-5">
                            <Icon icon=icon attr:class="text-2xl text-white md:text-3xl" />
                        </div>
                    }
                }
            >

                <div class="py-5 border-t-2 border-t-pink-500">
                    <Icon
                        icon=filled_icon.unwrap_or(icon)
                        attr:class="text-2xl text-white md:text-3xl aspect-square"
                    />
                </div>
            </Show>
        </a>
    }
}
