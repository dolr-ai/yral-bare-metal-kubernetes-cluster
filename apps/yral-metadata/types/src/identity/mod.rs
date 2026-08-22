//! Identity types and signature verification, inlined from the former
//! `yral-identity` crate. Provides `Signature`, `Message`, `Error`,
//! delegation types, and IC ingress-message signing/verification.

mod error;
mod msg_builder;
#[cfg(feature = "ic-agent")]
mod ic_agent;
#[cfg(feature = "verify")]
mod verify;

pub use error::*;

use candid::Principal;
use serde::{Deserialize, Serialize};
use web_time::{Duration, SystemTime};

pub use msg_builder::Message;

fn current_epoch() -> Duration {
    web_time::SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}

/// A signature, interoperable with ic-agent & yral-identity
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
pub struct Signature {
    pub sig: Option<Vec<u8>>,
    pub public_key: Option<Vec<u8>>,
    pub ingress_expiry: Duration,
    pub delegations: Option<Vec<SignedDelegation>>,
    pub sender: Principal,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
pub struct Delegation {
    pub pubkey: Vec<u8>,
    pub expiration_ns: u64,
    pub targets: Option<Vec<Principal>>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
pub struct SignedDelegation {
    pub delegation: Delegation,
    pub signature: Vec<u8>,
}

/// Re-export the ic_agent signing module when the feature is enabled.
#[cfg(feature = "ic-agent")]
pub use ic_agent::sign_message;