pub mod info;
mod popups;
pub mod transfer;
pub mod types;

use leptos::prelude::*;
use leptos_router::params::Params;
use yral_canisters_common::utils::token::RootType;

#[derive(Params, PartialEq, Clone)]
struct TokenParams {
    token_root: Option<RootType>,
}

#[derive(Params, PartialEq, Clone)]
struct TokenInfoParams {
    token_root: Option<RootType>,
}
