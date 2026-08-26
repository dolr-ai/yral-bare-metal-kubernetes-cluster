//! Acting as an AI account you own.
//!
//! A user may own many AI accounts ("bots"), and each behaves like its own user:
//! its own posts, followers, username and profile. But it has no separate login
//! — the owner signs in once with Google and everything is done under that one
//! session. So the token proves *who you are*, and each request says *which of
//! your identities it is acting as*.
//!
//! yral-auth already puts the answer in the token. Every id_token carries
//! `ext_ai_account_ids`, the list of AI accounts the signer owns, and that same
//! id_token is the SpacetimeDB token (`ext_spacetimedb_token: true`). So a
//! reducer can check "may this caller act as that account?" from the signed
//! token alone — no second login, no token exchange, nothing to store or
//! refresh, and nothing a client can forge.
//!
//! This replaces what the IC delegated identity used to express. When that was
//! removed, the ability to act as a bot went with it and nothing took its place,
//! so everything a bot did was recorded against its owner instead.

use spacetimedb::ReducerContext;

/// The OAuth subject this call is acting as.
///
/// `None` means "acting as myself" and returns the caller's own subject.
/// `Some(id)` means "acting as this AI account", allowed only if the id appears
/// in the caller's `ext_ai_account_ids` claim.
///
/// Admins may act as any subject: backend services already hold an admin token
/// and act on users' behalf, and requiring the claim would break them.
pub fn acting_subject(ctx: &ReducerContext, as_account: Option<String>) -> Result<String, String> {
    let jwt = ctx.sender_auth().jwt().ok_or("JWT required")?;
    let caller = jwt.subject().to_string();

    let Some(account) = as_account else {
        return Ok(caller);
    };
    if account == caller {
        return Ok(caller);
    }
    if crate::constants::ADMINS.contains(&ctx.sender()) {
        return Ok(account);
    }
    if owns_account(jwt.raw_payload(), &account) {
        Ok(account)
    } else {
        Err("Unauthorized".to_string())
    }
}

/// Whether `ext_ai_account_ids` in this JWT payload contains `account`.
///
/// A malformed or absent claim means "owns nothing" rather than an error: an
/// older token simply cannot act as a bot, which is the safe reading.
fn owns_account(raw_payload: &str, account: &str) -> bool {
    let Ok(claims) = serde_json::from_str::<serde_json::Value>(raw_payload) else {
        return false;
    };
    claims
        .get("ext_ai_account_ids")
        .and_then(|ids| ids.as_array())
        .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(account)))
}

#[cfg(test)]
mod tests {
    use super::owns_account;

    #[test]
    fn recognises_an_owned_account() {
        let payload = r#"{"sub":"owner-1","ext_ai_account_ids":["bot-a","bot-b"]}"#;
        assert!(owns_account(payload, "bot-a"));
        assert!(owns_account(payload, "bot-b"));
    }

    #[test]
    fn rejects_an_account_the_caller_does_not_own() {
        let payload = r#"{"sub":"owner-1","ext_ai_account_ids":["bot-a"]}"#;
        assert!(!owns_account(payload, "someone-elses-bot"));
    }

    #[test]
    fn treats_a_missing_or_malformed_claim_as_owning_nothing() {
        assert!(!owns_account(r#"{"sub":"owner-1"}"#, "bot-a"));
        assert!(!owns_account(
            r#"{"ext_ai_account_ids":"not-an-array"}"#,
            "bot-a"
        ));
        assert!(!owns_account("not json at all", "bot-a"));
    }
}
