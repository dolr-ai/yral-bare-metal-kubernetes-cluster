#![recursion_limit = "256"]
pub mod about_us;
pub mod err;
pub mod internal;
pub mod logout;
pub mod menu;
pub mod notifs;
pub mod post_view;
pub mod scrolling_post_view;
pub mod wallet;
#[cfg(any(feature = "oauth-ssr", feature = "oauth-hydrate"))]
pub mod yral_auth_redirect;
