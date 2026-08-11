use candid::Principal;
use consts::WHITELIST_FOR_SATS_CLEARING;
use hon_worker_common::{SatsBalanceInfo, SatsBalanceUpdateRequestV2, WORKER_URL};
use leptos::prelude::*;
use num_bigint::Sign;
use reqwest::Url;
use state::{canisters::auth_state, server::HonWorkerJwt};
use utils::{send_wrap, try_or_redirect_opt};

/// Fetch sats balance from the hon-worker (HTTP, not IC).
async fn load_sats_balance(user_principal: Principal) -> Result<SatsBalanceInfo, ServerFnError> {
    let url: Url = WORKER_URL.parse().expect("url to be valid");
    let balance_url = url
        .join(&format!("/balance/{user_principal}"))
        .expect("url to be valid");
    let res: SatsBalanceInfo = reqwest::get(balance_url).await?.json().await?;
    Ok(res)
}

#[server(input = server_fn::codec::Json)]
pub async fn clear_sats(
    _user_canister: Principal,
    user_principal: Principal,
) -> Result<(), ServerFnError> {
    if !WHITELIST_FOR_SATS_CLEARING.contains(user_principal.to_text().as_str()) {
        leptos::logging::log!("sats clearing({user_principal}): not whitelisted");
        return Err(ServerFnError::new(""));
    }

    let balance = load_sats_balance(user_principal)
        .await?
        .balance;

    let Some(jwt): Option<HonWorkerJwt> = use_context() else {
        leptos::logging::log!("sats clearing({user_principal}): no token");
        return Err(ServerFnError::new(""));
    };

    let req_url: Url = WORKER_URL.parse().expect("url to be valid");
    let req_url = req_url
        .join(&format!("/v2/update_balance/{user_principal}"))
        .expect("url to be valid");
    let delta = num_bigint::BigInt::from_biguint(Sign::Minus, balance.clone());
    if delta > 0.into() {
        leptos::logging::log!("sats clearing({user_principal}): balance is negative?");
        return Err(ServerFnError::new(""));
    }
    let worker_req = SatsBalanceUpdateRequestV2 {
        previous_balance: balance,
        delta,
        is_airdropped: false,
    };
    let client = reqwest::Client::new();
    let res = client
        .post(req_url)
        .json(&worker_req)
        .header("Authorization", format!("Bearer {}", jwt.0))
        .send()
        .await?;

    if !res.status().is_success() {
        let (status, text) = (res.status().as_u16(), res.text().await?);
        leptos::logging::log!("sats clearing({user_principal}): worker error({status}): {text}");
        return Err(ServerFnError::new(""));
    }

    Ok(())
}

#[component]
pub fn ClearSats() -> impl IntoView {
    let auth = auth_state();
    let balance = Resource::new_blocking(
        || (),
        move |_| async move {
            let cans = send_wrap(auth.auth_cans()).await?;
            let user_canister = cans.user_canister();
            let user_principal = cans.user_principal();
            if !WHITELIST_FOR_SATS_CLEARING.contains(user_principal.to_text().as_str()) {
                return Err(ServerFnError::new("who dis?"));
            }
            let sats_info: SatsBalanceInfo =
                load_sats_balance(user_principal).await?;

            let res = (sats_info.balance, user_canister, user_principal);

            Ok::<_, ServerFnError>(res)
        },
    );

    let action = Action::new_unsync(
        move |&(user_canister, user_principal): &(Principal, Principal)| async move {
            clear_sats(user_canister, user_principal).await
        },
    );

    let value = action.value();

    use leptos::html::*;
    use leptos::ev;
    use leptos::suspense::{Suspense, SuspenseProps};
    use leptos::children::ToChildren;

    div()
        .attr("class", "text-white")
        .child(
            Suspense(SuspenseProps::builder()
                .children(ToChildren::to_children(move || {
                    Suspend::new(async move {
                        let (balance, user_canister, user_principal) =
                            try_or_redirect_opt!(balance.await.inspect_err(|err| {
                                leptos::logging::log!(
                                    "balance fetching for sat clears failed: {err:?}"
                                );
                            })
                            .map_err(|_| "not found"));

                        Some(
                            div()
                                .child(p().child(format!("user principal: {}", user_principal.to_text())))
                                .child(p().child(format!("user canister : {}", user_canister.to_text())))
                                .child(p().child(format!("balance: {}", balance.to_string())))
                                .child(move || {
                                    value.get().map(|res| match res {
                                        Ok(_) => p().child("balance was cleared").into_any(),
                                        Err(err) => p()
                                            .child(format!("Couldnt clear balance: {err:#?}"))
                                            .into_any(),
                                    })
                                })
                                .child(
                                    button()
                                        .attr("class", "bg-red text-white border-white")
                                        .on(ev::click, move |_| {
                                            action.dispatch((user_canister, user_principal));
                                        })
                                        .child("Clear My Sats Balance"),
                                ),
                        )
                    })
                }))
                .build()),
        )
}