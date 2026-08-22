//! Vendored IC ingress-message signature verification.
//!
//! Replaces the `ic-validator-ingress-message` + `ic-types` git dependencies
//! from `dfinity/ic.git` with a self-contained implementation using only
//! crates.io dependencies (`ic-agent` / `ic-transport-types` + `k256`).
//!
//! The verification follows the [IC interface spec](https://internetcomputer.org/docs/current/references/ic-interface-spec#authentication):
//! 1. Reconstruct the `EnvelopeContent::Call` from `Signature` + `Message`.
//! 2. Compute `request_id` via representation-independent hashing.
//! 3. Verify the ECDSA (secp256k1) signature over `"\x0Aic-request" || request_id`.
//! 4. Walk the delegation chain, verifying each link over `"\x1Aic-request-auth-delegation" || hash`.
//! 5. Derive the sender principal from the head public key and compare to the claimed principal.

use candid::Principal;
use ic_agent::agent::EnvelopeContent;
use ic_agent::identity::Delegation as AgentDelegation;

use super::{
    Error, Result, Signature, SignedDelegation, msg_builder::Message,
    current_epoch,
};

/// Domain separator prepended to the request_id hash before signing/verifying.
const IC_REQUEST_DOMAIN_SEPARATOR: &[u8] = b"\x0Aic-request";

impl Signature {
    /// Verify that this signature was produced by the holder of `principal`'s
    /// private key, over the given `Message`.
    ///
    /// This is the replacement for the old `ic_git::verify_identity`.
    pub fn verify_identity(self, principal: Principal, mut msg: Message) -> Result<()> {
        msg.sender = self.sender;
        msg.ingress_expiry = self.ingress_expiry;

        // 1. Check ingress expiry hasn't passed.
        let now_ns = current_epoch().as_nanos();
        let expiry_ns = self.ingress_expiry.as_nanos();
        if now_ns > expiry_ns {
            return Err(Error::SignatureVerification("ingress message expired".into()));
        }

        // 2. Reconstruct the EnvelopeContent::Call and compute request_id.
        let envelope: EnvelopeContent = msg.into();
        let request_id = envelope.to_request_id();

        // 3. Determine the effective signing key + signature.
        //    - No delegation: the head pubkey signs directly.
        //    - With delegation chain [d0, d1, ..., dN]:
        //      head pubkey = d0.pubkey, the user's key.
        //      The final signature (self.sig) is made by the *last* delegation's key
        //      (or by the head key if there's only one delegation entry which is
        //      actually the signing key itself).
        //
        // In ic-agent's model, when delegating:
        //   - `sender_pubkey` = the head (user's) public key
        //   - `sender_sig` = signature by the *delegate* (the last key in the chain)
        //   - `sender_delegation` = [d0 signed by head, d1 signed by d0, ...]
        //
        // The sender principal is derived from the head pubkey.

        let head_pubkey = self
            .public_key
            .as_deref()
            .ok_or(Error::MissingSignature)?;

        // 4. Derive the sender principal from the head public key.
        let derived_sender = Principal::self_authenticating(head_pubkey);
        if derived_sender != principal {
            return Err(Error::IdentityMismatch);
        }

        // 5. Verify the delegation chain (if present) and find the effective signing key.
        let signing_pubkey = if let Some(delegations) = &self.delegations {
            verify_delegation_chain(delegations, head_pubkey)?
        } else {
            head_pubkey.to_vec()
        };

        // 6. Verify the main signature over "\x0Aic-request" || request_id.
        let sig = self
            .sig
            .as_deref()
            .ok_or(Error::MissingSignature)?;

        let mut signable = Vec::with_capacity(IC_REQUEST_DOMAIN_SEPARATOR.len() + 32);
        signable.extend_from_slice(IC_REQUEST_DOMAIN_SEPARATOR);
        signable.extend_from_slice(request_id.as_slice());

        verify_secp256k1_signature(&signing_pubkey, sig, &signable)?;

        Ok(())
    }

    /// Verify the signature without checking the sender principal.
    /// (Kept for API compatibility; currently unused in production.)
    pub fn verify(self, msg: Message) -> Result<()> {
        let sender = self.sender;
        self.verify_identity(sender, msg)
    }
}

/// Verify a delegation chain and return the effective signing public key.
///
/// The chain is ordered head → tail: `delegations[0]` is signed by the head
/// (user) key, `delegations[1]` is signed by `delegations[0].pubkey`, etc.
/// The final (tail) delegation's pubkey is the key that made `self.sig`.
fn verify_delegation_chain(
    delegations: &[SignedDelegation],
    head_pubkey: &[u8],
) -> Result<Vec<u8>> {
    if delegations.is_empty() {
        return Err(Error::SignatureVerification("empty delegation chain".into()));
    }

    let mut current_key: Vec<u8> = head_pubkey.to_vec();

    for sd in delegations {
        let agent_delegation = AgentDelegation {
            pubkey: sd.delegation.pubkey.clone(),
            expiration: sd.delegation.expiration_ns,
            targets: sd.delegation.targets.clone(),
            permissions: None,
        };

        // Verify the delegation signature: current_key signs the delegation's signable bytes.
        let signable = agent_delegation.signable();
        verify_secp256k1_signature(&current_key, &sd.signature, &signable)?;

        // Check delegation hasn't expired.
        let now_ns = current_epoch().as_nanos();
        if now_ns > u64::try_from(sd.delegation.expiration_ns).unwrap_or(u64::MAX) as u128 {
            return Err(Error::SignatureVerification(
                "delegation expired".into(),
            ));
        }

        current_key = sd.delegation.pubkey.clone();
    }

    Ok(current_key)
}

/// Verify a secp256k1 ECDSA signature over `message` using a DER-encoded public key.
///
/// The IC uses DER-encoded SubjectPublicKeyInfo for secp256k1 keys (the same
/// format that `k256::PublicKey::from_sec1_bytes` / `from_public_key_der`
/// accepts). We try SEC1 raw format first, then DER.
fn verify_secp256k1_signature(
    pubkey_der: &[u8],
    signature: &[u8],
    message: &[u8],
) -> Result<()> {
    use k256::ecdsa::{VerifyingKey, signature::Verifier};

    // Try SEC1 compressed/uncompressed first, then DER SPKI.
    let vk = VerifyingKey::from_sec1_bytes(pubkey_der)
        .or_else(|_| {
            // DER-encoded SubjectPublicKeyInfo (ic-agent's Secp256k1Identity::public_key
            // returns DER SPKI bytes).
            use k256::elliptic_curve::pkcs8::DecodePublicKey;
            VerifyingKey::from_public_key_der(pubkey_der)
        })
        .map_err(|e| Error::SignatureVerification(format!("invalid public key: {e}")))?;

    // ic-agent produces recoverable signatures (65 bytes: r || s || recovery_id).
    // The `Signature` struct stores the raw 64-byte (r || s) or 65-byte recoverable
    // signature. k256's `Signature::from_slice` expects 64-byte fixed-size.
    // ic-agent's Secp256k1Identity signs with `k256::ecdsa::SigningKey` and stores
    // `signature: Option<Vec<u8>>` — the `sign_recoverable` produces 65 bytes.
    // We need to handle both 64 and 65 byte signatures.
    let sig = if signature.len() == 65 {
        // Recoverable signature: strip the recovery byte.
        &signature[..64]
    } else {
        signature
    };

    let sig = k256::ecdsa::Signature::from_slice(sig)
        .map_err(|e| Error::SignatureVerification(format!("invalid signature format: {e}")))?;

    vk.verify(message, &sig)
        .map_err(|e| Error::SignatureVerification(format!("signature verification failed: {e}")))
}