use std::fmt::Display;

#[macro_export]
macro_rules! try_or_redirect {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                use utils::route::failure_redirect;
                failure_redirect(e);
                return;
            }
        }
    };
}

#[macro_export]
macro_rules! try_or_redirect_opt {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                use utils::route::failure_redirect;
                failure_redirect(e);
                return None;
            }
        }
    };
}

pub fn failure_redirect<E: Display>(err: E) {
    // Each branch below is feature-gated, so without `hydrate` or `ssr` the
    // binding (and the parameter) would go unused. Bind only when something
    // can read it.
    #[cfg(any(feature = "hydrate", feature = "ssr"))]
    let path = format!("/error?err={err}");
    #[cfg(not(any(feature = "hydrate", feature = "ssr")))]
    let _ = err;

    #[cfg(feature = "hydrate")]
    {
        let nav = leptos_router::hooks::use_navigate();
        nav(&path, Default::default());
    }
    #[cfg(feature = "ssr")]
    {
        use leptos_axum::redirect;
        redirect(&path);
    }
}

pub fn go_to_root() {
    #[cfg(any(feature = "hydrate", feature = "ssr"))]
    let path = "/";

    #[cfg(feature = "hydrate")]
    {
        let nav = leptos_router::hooks::use_navigate();
        nav(path, Default::default());
    }
    #[cfg(feature = "ssr")]
    {
        use leptos_axum::redirect;
        redirect(&path);
    }
}
