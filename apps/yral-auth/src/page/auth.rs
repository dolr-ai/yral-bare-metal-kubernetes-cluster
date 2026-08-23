use base64::{prelude::BASE64_URL_SAFE, Engine};
#[cfg(any(feature = "phone-auth", feature = "google-oauth", feature = "apple-oauth"))]
use leptos::ev;
use leptos::{
    children::ToChildren,
    either::Either,
    html,
    prelude::*,
};
#[cfg(any(feature = "phone-auth", feature = "google-oauth", feature = "apple-oauth"))]
use leptos_router::{
    components::{Redirect, RedirectProps},
    hooks::{use_navigate, use_query},
    params::{Params, ParamsError},
    NavigateOptions,
};
#[cfg(not(any(feature = "phone-auth", feature = "google-oauth", feature = "apple-oauth")))]
use leptos_router::{
    components::{Redirect, RedirectProps},
    hooks::use_query,
    params::{Params, ParamsError},
};
use serde::{Deserialize, Serialize};
use url::Url;
use crate::{
    components::spinner::Spinner,
    error::AuthErrorKind,
    oauth::{
        client_validation::{ClientIdValidator, ClientIdValidatorImpl},
        AuthCodeError, AuthQuery, AuthResponse as AuthResponseCode, CodeChallenge,
        CodeChallengeMethod, SupportedOAuthProviders,
    },
};
#[cfg(any(feature = "phone-auth", feature = "google-oauth", feature = "apple-oauth"))]
use crate::components::yral_symbol::{YralSymbol, YralSymbolProps};
#[cfg(feature = "phone-auth")]
use crate::components::whatsapp_symbol::{WhatsAppSymbol, WhatsAppSymbolProps};
#[cfg(feature = "google-oauth")]
use crate::components::google_symbol::{GoogleSymbol, GoogleSymbolProps};
#[cfg(feature = "apple-oauth")]
use crate::components::apple_symbol::{AppleSymbol, AppleSymbolProps};

#[derive(Debug, Clone, Params, PartialEq)]
pub struct RedirectUriQuery {
    redirect_uri: Option<String>,
}

#[derive(Debug, Clone, Params, PartialEq)]
pub struct StateQuery {
    state: Option<String>,
}

#[derive(Debug, Clone, Params, PartialEq)]
pub struct AuthQueryMaybe {
    response_type: Option<AuthResponseCode>,
    client_id: Option<String>,
    code_challenge: Option<CodeChallenge>,
    code_challenge_method: Option<CodeChallengeMethod>,
    nonce: Option<String>,
    provider: Option<SupportedOAuthProviders>,
}

impl AuthQueryMaybe {
    pub async fn validate(
        self,
        validator: &impl ClientIdValidator,
        redirect_uri: String,
        state: String,
    ) -> Result<AuthQuery, AuthErrorKind> {
        let client_id = self
            .client_id
            .ok_or_else(|| AuthErrorKind::missing_param("client_id"))?;
        let redirect_uri =
            Url::parse(&redirect_uri).map_err(|_| AuthErrorKind::InvalidUri(redirect_uri))?;

        validator
            .validate_id_and_redirect(&client_id, &redirect_uri)
            .await?;

        Ok(AuthQuery {
            response_type: self
                .response_type
                .ok_or_else(|| AuthErrorKind::missing_param("response_type"))?,
            client_id,
            state,
            redirect_uri,
            code_challenge: self
                .code_challenge
                .ok_or_else(|| AuthErrorKind::missing_param("code_challenge"))?,
            code_challenge_method: self
                .code_challenge_method
                .ok_or_else(|| AuthErrorKind::missing_param("code_challenge_method"))?,
            nonce: self.nonce,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum AuthKind {
    Default(Box<AuthQuery>),
    Redirect(String),
}

pub fn auth_page() -> impl IntoView {
    let redirect_query = use_query::<RedirectUriQuery>();
    let state_query = use_query::<StateQuery>();
    let auth_query_maybe = use_query::<AuthQueryMaybe>();

    let validator = expect_context::<ClientIdValidatorImpl>();

    let auth_query = Resource::new(
        move || {
            (
                redirect_query.get(),
                auth_query_maybe.get(),
                state_query.get(),
            )
        },
        move |(redirect_query, auth_query_maybe, state_query)| {
            let validator = validator.clone();
            async move {
                let redirect_uri = match redirect_query {
                    Ok(RedirectUriQuery {
                        redirect_uri: Some(uri),
                    }) => uri,
                    _ => {
                        return Err(AuthCodeError::new(
                            AuthErrorKind::missing_param("redirect_uri"),
                            None,
                            "/error",
                        ))
                    }
                };
                let state = match state_query {
                    Ok(StateQuery { state: Some(state) }) => state,
                    _ => {
                        return Err(AuthCodeError::new(
                            AuthErrorKind::missing_param("state"),
                            None,
                            redirect_uri.clone(),
                        ))
                    }
                };

                let res = match auth_query_maybe {
                    Ok(q) => {
                        let provider = q.provider;
                        q.validate(&validator, redirect_uri.clone(), state.clone())
                            .await
                            .map(|q| (q, provider))
                    }
                    Err(ParamsError::MissingParam(param)) => {
                        Err(AuthErrorKind::missing_param(param))
                    }
                    Err(ParamsError::Params(e)) => match e.downcast_ref::<AuthErrorKind>() {
                        Some(e) => Err(e.clone()),
                        None => Err(AuthErrorKind::Unexpected(e.to_string())),
                    },
                };
                let (query, provider) =
                    res.map_err(|e| AuthCodeError::new(e, Some(state), redirect_uri.clone()))?;
                let Some(provider) = provider else {
                    return Ok(AuthKind::Default(Box::new(query)));
                };

                let state_raw = postcard::to_stdvec(&query).unwrap();
                let state = BASE64_URL_SAFE.encode(state_raw);
                let redirect_path = format!("/oauth_redirector?provider={provider}&state={state}");
                Ok(AuthKind::Redirect(redirect_path))
            }
        },
    );

    html::div()
        .attr("class", "w-dvw h-dvh flex justify-center items-center bg-neutral-900")
        .child(Suspense(SuspenseProps::builder().fallback(|| Spinner()).children(ToChildren::to_children(move || {
            Suspend::new(async move {
                let auth = auth_query.await;
                match auth {
                    Ok(AuthKind::Default(auth)) => {
                        #[cfg(any(feature = "phone-auth", feature = "google-oauth", feature = "apple-oauth"))]
                        { Either::Left(login_content(auth)) }
                        #[cfg(not(any(feature = "phone-auth", feature = "google-oauth", feature = "apple-oauth")))]
                        {
                            let _ = auth;
                            Either::Left(html::div().child("No OAuth providers configured"))
                        }
                    }
                    Ok(AuthKind::Redirect(path)) => Either::Right(Redirect(RedirectProps::builder().path(path).build())),
                    Err(e) => Either::Right(Redirect(RedirectProps::builder().path(e.to_redirect()).build())),
                }
            })
        })).build()))
}

#[cfg(any(feature = "phone-auth", feature = "google-oauth", feature = "apple-oauth"))]
pub fn login_content(auth: Box<AuthQuery>) -> impl IntoView {
    let auth_store = StoredValue::new(auth);
    let login_buttons = build_login_buttons(auth_store);

    html::div()
        .attr("class", "flex flex-col items-center text-white cursor-auto")
        .child(YralSymbol(
            YralSymbolProps::builder()
                .class("rounded-full mb-6 text-8xl")
                .build(),
        ))
        .child(html::span().attr("class", "text-2xl mb-4").child("Login to Yral"))
        .child(
            html::div()
                .attr("class", "flex flex-col w-full gap-4 items-center")
                .child(login_buttons),
        )
}

#[cfg(any(feature = "phone-auth", feature = "google-oauth", feature = "apple-oauth"))]
fn build_login_buttons(auth_store: StoredValue<Box<AuthQuery>>) -> Vec<AnyView> {
    let mut login_buttons: Vec<AnyView> = Vec::new();

    #[cfg(feature = "phone-auth")]
    login_buttons.push(
        login_button(
            auth_store,
            SupportedOAuthProviders::Phone,
            "flex flex-row justify-center cursor-pointer items-center justify-between gap-1 rounded-full bg-white pr-4 hover:bg-neutral-200",
            {
                let icon_wrapper = html::div()
                    .attr("class", "grid grid-cols-1 place-items-center pl-2 py-2 rounded-full")
                    .child(WhatsAppSymbol(
                        WhatsAppSymbolProps::builder()
                            .class("text-xl rounded-full")
                            .build(),
                    ));
                let label = html::span()
                    .attr("class", "text-neutral-900")
                    .child("Continue with Whatsapp");
                (icon_wrapper, label)
            },
        )
        .into_any(),
    );

    #[cfg(feature = "google-oauth")]
    login_buttons.push(
        login_button(
            auth_store,
            SupportedOAuthProviders::Google,
            "flex flex-row justify-center cursor-pointer items-center justify-between gap-1 rounded-full bg-white pr-4 hover:bg-neutral-200",
            {
                let icon_wrapper = html::div()
                    .attr("class", "grid grid-cols-1 place-items-center pl-2 py-2 rounded-full")
                    .child(GoogleSymbol(
                        GoogleSymbolProps::builder()
                            .class("text-xl rounded-full")
                            .build(),
                    ));
                let label = html::span()
                    .attr("class", "text-neutral-900")
                    .child("Continue with Google");
                (icon_wrapper, label)
            },
        )
        .into_any(),
    );

    #[cfg(feature = "apple-oauth")]
    login_buttons.push(
        login_button(
            auth_store,
            SupportedOAuthProviders::Apple,
            "flex flex-row justify-center cursor-pointer items-center pr-4 bg-white rounded-full border border-gray-300 hover:bg-neutral-200",
            {
                let icon_wrapper = html::div()
                    .attr("class", "grid grid-cols-1 place-items-center")
                    .child(AppleSymbol(
                        AppleSymbolProps::builder().class("text-4xl").build(),
                    ));
                let label = html::span()
                    .attr("class", "text-black")
                    .child("Continue with Apple");
                (icon_wrapper, label)
            },
        )
        .into_any(),
    );

    login_buttons
}

#[cfg(any(feature = "phone-auth", feature = "google-oauth", feature = "apple-oauth"))]
pub fn login_button(
    auth: StoredValue<Box<AuthQuery>>,
    provider: SupportedOAuthProviders,
    class: &'static str,
    children: impl IntoView,
) -> impl IntoView {
    let redirect_to_oauth = move || {
        let state_raw = auth.with_value(|auth| postcard::to_stdvec(auth).unwrap());
        let state = BASE64_URL_SAFE.encode(state_raw);
        let redirect_path = format!("/oauth_redirector?provider={provider}&state={state}");

        let nav = use_navigate();
        (nav)(&redirect_path, NavigateOptions::default());
    };

    html::button()
        .attr("class", class)
        .on(ev::click, move |_| redirect_to_oauth())
        .child(children)
}
