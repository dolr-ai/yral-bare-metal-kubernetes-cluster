//! Checks whether a caller owns an AI account, so they can act as it.
//!
//! # Why this exists
//!
//! One person signs in once with Google and may then create many AI accounts
//! ("bots"). Each bot is meant to behave like its own user: its own posts, its
//! own followers, its own username. But a bot has no login of its own —
//! everything it does is done by its owner, in the owner's session.
//!
//! So when the owner does something on a bot's behalf, the reducer has to know
//! two different things:
//!
//!   1. Who is calling?          -> the `sub` claim of their token
//!   2. Who are they acting as?  -> named by the caller, and checked here
//!
//! The IC delegated identity used to carry that second part. When it was
//! removed nothing replaced it, so every write a bot makes is now recorded
//! against its owner instead of against the bot.
//!
//! # Why the token is enough
//!
//! yral-auth already puts the list of AI accounts a person owns into every
//! id_token, as the `ext_ai_account_ids` claim. That same id_token is the
//! SpacetimeDB token (it carries `ext_spacetimedb_token: true`), so the list is
//! sitting right there in the credential the caller already presented.
//!
//! That means no second login, no separate bot token to mint or refresh, and
//! nothing a client can fake — the claim is inside a signature we verify.
//!
//! `JwtClaims::raw_payload()` is the SDK's documented way to read claims beyond
//! `sub` / `iss` / `aud`.

use spacetimedb::ReducerContext;

/// The OAuth subject of whoever made this call.
///
/// This is the `sub` claim of their token — for a person signed in with Google,
/// their Google account id. It is what `posts_3.creator_oauth_subject` holds
/// for a post someone made as themselves.
pub fn caller_oauth_subject(ctx: &ReducerContext) -> String {
    ctx.sender_auth()
        .jwt()
        .expect("JWT required")
        .subject()
        .to_string()
}

/// Whether the caller's token says they own this AI account.
///
/// Reads the `ext_ai_account_ids` claim and looks for `ai_account_id` in it.
///
/// Returns `false` if the claim is missing or malformed — an older token simply
/// cannot act as a bot, which is the safe answer. It does NOT consider admins;
/// callers that want to allow admins should check `constants::ADMINS`
/// separately, so that reading the call site makes both rules obvious.
pub fn caller_owns_ai_account(ctx: &ReducerContext, ai_account_id: &str) -> bool {
    let Some(jwt) = ctx.sender_auth().jwt() else {
        return false;
    };
    let Ok(claims) = serde_json::from_str::<serde_json::Value>(jwt.raw_payload()) else {
        return false;
    };
    claims
        .get("ext_ai_account_ids")
        .and_then(|ai_account_ids| ai_account_ids.as_array())
        .is_some_and(|ai_account_ids| {
            ai_account_ids
                .iter()
                .any(|owned_id| owned_id.as_str() == Some(ai_account_id))
        })
}

#[cfg(test)]
mod tests {
    /// The claim-reading half of `caller_owns_ai_account`, split out so it can
    /// be tested without a `ReducerContext`.
    fn token_lists_ai_account(raw_payload: &str, ai_account_id: &str) -> bool {
        let Ok(claims) = serde_json::from_str::<serde_json::Value>(raw_payload) else {
            return false;
        };
        claims
            .get("ext_ai_account_ids")
            .and_then(|ids| ids.as_array())
            .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(ai_account_id)))
    }

    #[test]
    fn owner_can_act_as_their_own_ai_account() {
        let token = r#"{"sub":"owner-1","ext_ai_account_ids":["bot-a","bot-b"]}"#;
        assert!(token_lists_ai_account(token, "bot-a"));
        assert!(token_lists_ai_account(token, "bot-b"));
    }

    #[test]
    fn cannot_act_as_someone_elses_ai_account() {
        let token = r#"{"sub":"owner-1","ext_ai_account_ids":["bot-a"]}"#;
        assert!(!token_lists_ai_account(token, "bot-owned-by-someone-else"));
    }

    #[test]
    fn token_without_the_claim_owns_nothing() {
        assert!(!token_lists_ai_account(r#"{"sub":"owner-1"}"#, "bot-a"));
        assert!(!token_lists_ai_account(
            r#"{"ext_ai_account_ids":"not-an-array"}"#,
            "bot-a"
        ));
        assert!(!token_lists_ai_account("not json at all", "bot-a"));
    }
}
