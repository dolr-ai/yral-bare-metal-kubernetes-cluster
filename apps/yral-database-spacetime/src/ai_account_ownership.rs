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
//! # Why this reads the database and not the token
//!
//! It used to read the `ext_ai_account_ids` claim out of the caller's token.
//! That was wrong, and it made creating a bot and immediately using it
//! impossible.
//!
//! The claim is stamped into a token when the token is **granted**. A bot
//! created afterwards cannot appear in a token that was already issued, and the
//! app creates a bot and generates its first video about nine seconds later, in
//! the same session, with the same token. So the check refused every brand-new
//! bot — which is exactly the case the whole mechanism exists to allow. There
//! is no ordering that fixes this from the client: the token is older than the
//! bot by construction.
//!
//! `user_profiles_2` already holds the answer, and holds it *correctly*.
//! `accept_new_user_registration` records the bot against its owner —
//! `UserAccountType::MainAccount { bots }` — and the app calls that **before**
//! it does anything as the bot. So by the time any reducer asks "does this
//! caller own that bot?", the row has been written.
//!
//! Reading it here means ownership is answered from the same table that created
//! the relationship, is correct the instant a bot exists, and needs no token
//! refresh, no client change, and no second login.

use spacetimedb::{ReducerContext, Table};

// `user_profiles_2` is the generated table-accessor trait; it must be in
// scope for `ctx.db.user_profiles_2()` to resolve.
use crate::user_info::{user_profiles_2, UserAccountType};

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

/// Whether `ai_account_id` is one of the bots this account owns.
///
/// Split out from the lookup so the rule can be tested without a
/// `ReducerContext`. A bot may not act as another bot.
fn account_owns_bot(account_type: &UserAccountType, ai_account_id: &str) -> bool {
    match account_type {
        UserAccountType::MainAccount { bots } => bots.iter().any(|bot| bot == ai_account_id),
        UserAccountType::BotAccount { .. } => false,
    }
}

/// Whether the caller owns this AI account.
///
/// Looks the caller up in `user_profiles_2` and asks whether the account is in
/// their `bots` list. Returns `false` if the caller has no profile — someone who
/// has never registered owns nothing, which is the safe answer.
///
/// This does NOT consider admins; callers that want to allow admins should check
/// `constants::ADMINS` separately, so that reading the call site makes both
/// rules obvious.
pub fn caller_owns_ai_account(ctx: &ReducerContext, ai_account_id: &str) -> bool {
    let caller = caller_oauth_subject(ctx);
    ctx.db
        .user_profiles_2()
        .iter()
        .find(|profile| profile.oauth_subject == caller)
        .is_some_and(|profile| account_owns_bot(&profile.account_type, ai_account_id))
}

#[cfg(test)]
mod tests {
    use super::account_owns_bot;
    use crate::user_info::UserAccountType;

    fn owner_of(bots: &[&str]) -> UserAccountType {
        UserAccountType::MainAccount {
            bots: bots.iter().map(|b| b.to_string()).collect(),
        }
    }

    #[test]
    fn owner_can_act_as_their_own_ai_account() {
        let owner = owner_of(&["bot-a", "bot-b"]);
        assert!(account_owns_bot(&owner, "bot-a"));
        assert!(account_owns_bot(&owner, "bot-b"));
    }

    #[test]
    fn cannot_act_as_someone_elses_ai_account() {
        let owner = owner_of(&["bot-a"]);
        assert!(!account_owns_bot(&owner, "bot-owned-by-someone-else"));
    }

    #[test]
    fn an_account_with_no_bots_owns_nothing() {
        assert!(!account_owns_bot(&owner_of(&[]), "bot-a"));
    }

    #[test]
    fn a_bot_cannot_act_as_another_bot() {
        let bot = UserAccountType::BotAccount {
            owner: "owner-1".to_string(),
        };
        assert!(!account_owns_bot(&bot, "bot-a"));
    }

    /// The regression this module was rewritten for: a bot registered against
    /// its owner is usable immediately, with no token refresh. Under the old
    /// token-claim check this was refused until the caller signed in again.
    #[test]
    fn a_just_created_bot_is_usable_immediately() {
        let owner = owner_of(&["bot-a"]);
        let after_creating_another = owner_of(&["bot-a", "brand-new-bot"]);
        assert!(!account_owns_bot(&owner, "brand-new-bot"));
        assert!(account_owns_bot(&after_creating_another, "brand-new-bot"));
    }
}
