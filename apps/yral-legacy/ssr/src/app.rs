use crate::error_template::{AppError, ErrorTemplate};
use component::{base_route::BaseRoute, nav::NavBar};
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::hooks::use_location;
use leptos_router::{components::*, path, MatchNestedRoutes};
use page::about_us::AboutUs;
use page::post_view::PostDetailsCacheCtx;
// use page::pumpdump; // TODO: re-enable when pumpdump module is restored
use page::terms_android::TermsAndroid;
use page::terms_ios::TermsIos;
use page::{
    err::ServerErrorPage,
    logout::Logout,
    menu::Menu,
    post_view::single_post::SinglePost,
    privacy::PrivacyPolicy,
    refer_earn::ReferEarn,
    terms::TermsOfService,
    wallet::Wallet,
};
use state::app_state::AppState;
use state::app_type::AppType;
use state::audio_state::AudioState;
use utils::event_streaming::events::HistoryCtx;
use utils::mixpanel::state::MixpanelState;
use utils::types::PostParams;

#[component]
fn NotFound() -> impl IntoView {
    let mut outside_errors = Errors::default();
    outside_errors.insert_with_default_key(AppError::NotFound);
    view! { <ErrorTemplate outside_errors /> }
}

#[component(transparent)]
fn GoogleAuthRedirectHandlerRoute() -> impl MatchNestedRoutes + Clone {
    let path = path!("/auth/google_redirect");
    #[cfg(any(feature = "oauth-ssr", feature = "oauth-hydrate"))]
    {
        use page::yral_auth_redirect::YralAuthRedirectHandler;
        view! { <Route path view=YralAuthRedirectHandler /> }.into_inner()
    }
    #[cfg(not(any(feature = "oauth-ssr", feature = "oauth-hydrate")))]
    {
        view! { <Route path view=NotFound /> }.into_inner()
    }
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <meta name="facebook-domain-verification" content="sqtv2sr90ar0ck7t7zcklos44fw8t3" />
                <script fetchpriority="low" type="module" src="/js/sentry-init.js" async></script>
                <script
                    fetchpriority="low"
                    type="module"
                    src="/js/store-initial-url.js"
                    async
                ></script>

                <AutoReload options=options.clone() />
                <HashedStylesheet id="leptos" options=options.clone() />
                <Meta property="og:title" content="YRAL - World's first social on Bitcoin" />
                <Meta property="og:image" content="/img/common/preview.webp" />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let app_type = AppType::select();
    let app_state = AppState::from_type(&app_type);
    provide_context(app_state.clone());

    // Existing context providers
    provide_context(AudioState::default());
    provide_context(PostDetailsCacheCtx::default());

    // History Tracking
    let history_ctx = HistoryCtx::default();
    provide_context(history_ctx.clone());

    let _ = MixpanelState::init();

    let current_post_params = RwSignal::new(None::<PostParams>);
    provide_context(current_post_params);

    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| {
            let loc = use_location();
            history_ctx.push(&loc.pathname.get());
        });
    }

    view! {
        <Title text=app_state.name />

        // Favicon
        <Link
            rel="icon"
            type_="image/svg+xml"
            href=format!("/{}/favicon.svg", app_state.asset_path())
        />
        <Link rel="shortcut icon" href=format!("/{}/favicon.ico", app_state.asset_path()) />
        <Link
            rel="apple-touch-icon"
            sizes="180x180"
            href=format!("/{}/favicon-apple.png", app_state.asset_path())
        />

        <Link rel="preconnect" href="https://customer-2p3jflss4r4hmpnz.cloudflarestream.com" />
        <Link rel="preconnect" href="https://imagedelivery.net" />

        // Meta
        <Meta name="apple-mobile-web-app-title" content=app_state.name />

        // App manifest
        <Link rel="manifest" href=format!("/{}/manifest.json", app_state.asset_path()) />

        <Router>
            <main class="bg-black" id="body">
                <Routes fallback=|| view! { <NotFound /> }.into_view()>
                    // auth redirect routes exist outside main context
                    <GoogleAuthRedirectHandlerRoute />
                    <ParentRoute path=path!("") view=BaseRoute>
                        <Route path=path!("/post/:canister_id/:post_id") view=SinglePost />
                        <Route path=path!("/error") view=ServerErrorPage />
                        <Route path=path!("/menu") view=Menu />
                        <Route path=path!("/refer-earn") view=ReferEarn />
                        <Route path=path!("/terms-of-service") view=TermsOfService />
                        <Route path=path!("/privacy-policy") view=PrivacyPolicy />
                        <Route path=path!("/about-us") view=AboutUs />
                        <Route path=path!("/wallet/:id") view=Wallet />
                        <Route path=path!("/wallet") view=Wallet />
                        <Route path=path!("/logout") view=Logout />
                        <Route path=path!("/terms-ios") view=TermsIos />
                        <Route path=path!("/terms-android") view=TermsAndroid />
                    </ParentRoute>
                </Routes>

            </main>
            <nav>
                <NavBar />
            </nav>
        </Router>
    }
}
