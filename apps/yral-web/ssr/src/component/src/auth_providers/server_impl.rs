use leptos::prelude::*;

pub async fn mark_user_registered(user_id: String) -> Result<bool, ServerFnError> {
    ensure_user_logged_in_with_oauth(user_id.clone()).await?;

    // Check if user already exists in SpacetimeDB.
    #[cfg(feature = "ssr")]
    {
        use yral_database_spacetime_bindings::get_user_profile_details_v_7;
        use tokio::sync::oneshot;
        use state::spacetime::spacetime_conn;

        let conn = spacetime_conn();
        let (tx, rx) = oneshot::channel();
        conn.procedures.get_user_profile_details_v_7_then(
            user_id.clone(),
            move |_ctx, result| { let _ = tx.send(result.ok().flatten()); },
        );
        let existing = rx.await.unwrap_or(None);
        if existing.is_some() {
            return Ok(false); // returning user
        }

        // New user — register via SpacetimeDB reducer.
        use yral_database_spacetime_bindings::accept_new_user_registration_v_2;
        conn.reducers.accept_new_user_registration_v_2(
            user_id,
            true,
            None,
        )?;
        Ok(true)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Ok(true)
    }
}

async fn ensure_user_logged_in_with_oauth(user_id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "oauth-ssr")]
    {
        use std::env;

        use auth::server_impl::yral::YralAuthRefreshTokenClaims;
        use axum_extra::extract::{SignedCookieJar, cookie::Key};
        use consts::{
            auth::REFRESH_TOKEN_COOKIE,
            yral_auth::{YRAL_AUTH_CLIENT_ID_ENV, YRAL_AUTH_ISSUER_URL, YRAL_AUTH_TRUSTED_KEY},
        };
        use jsonwebtoken::Validation;
        use leptos_axum::extract_with_state;

        let key: Key = expect_context();
        let jar: SignedCookieJar = extract_with_state(&key).await?;

        let Some(refresh_token) = jar.get(REFRESH_TOKEN_COOKIE) else {
            return Err(ServerFnError::new("not logged in"));
        };

        let client_id = env::var(YRAL_AUTH_CLIENT_ID_ENV).expect("expected to have client id");

        let mut token_validation = Validation::new(jsonwebtoken::Algorithm::ES256);
        token_validation.set_audience(&[client_id]);
        token_validation.set_issuer(&[YRAL_AUTH_ISSUER_URL]);

        let decoded = jsonwebtoken::decode::<YralAuthRefreshTokenClaims>(
            refresh_token.value(),
            &YRAL_AUTH_TRUSTED_KEY,
            &token_validation,
        )?;
        if decoded.claims.ext_is_anonymous || decoded.claims.sub != user_id {
            Err(ServerFnError::new("not logged in"))
        } else {
            Ok(())
        }
    }
    #[cfg(not(feature = "oauth-ssr"))]
    {
        _ = user_id;
        Err(ServerFnError::new("not logged in"))
    }
}
