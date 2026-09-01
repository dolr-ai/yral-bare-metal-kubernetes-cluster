import Testing
import Foundation
@testable import YralApp

/// Tests for `BotIdentitiesStore` — the merge semantics from Kotlin
/// `BotIdentitiesStore.mergeFromTokenBotAccountIds` (union by principal,
/// latest token entry wins, non-blank usernames preserved).
struct AccountSwitcherTests {

    private func freshDefaults() -> UserDefaults {
        let name = "account-switcher-tests-\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: name)!
        defaults.removePersistentDomain(forName: name)
        return defaults
    }

    @Test("empty token list merges nothing and returns nil")
    func emptyMerge() {
        let defaults = freshDefaults()
        #expect(BotIdentitiesStore.mergeFromTokenBotAccountIds([], defaults: defaults) == nil)
        #expect(BotIdentitiesStore.entries(defaults: defaults).isEmpty)
    }

    @Test("blank principals are filtered before merging")
    func blankPrincipalsFiltered() {
        let defaults = freshDefaults()
        #expect(
            BotIdentitiesStore.mergeFromTokenBotAccountIds(
                ["", "   "], defaults: defaults
            ) == nil
        )
        #expect(BotIdentitiesStore.entries(defaults: defaults).isEmpty)
    }

    @Test("token identities merge into an empty store")
    func mergeIntoEmpty() {
        let defaults = freshDefaults()
        let result = BotIdentitiesStore.mergeFromTokenBotAccountIds(
            ["auth0|bot-1", "auth0|bot-2"], defaults: defaults
        )
        #expect(result?.addedCount == 2)
        #expect(result?.mergedCount == 2)
        let entries = BotIdentitiesStore.entries(defaults: defaults)
        #expect(entries.map(\.principal).sorted() == ["auth0|bot-1", "auth0|bot-2"])
    }

    @Test("re-merging the same identities changes nothing (idempotent)")
    func idempotentMerge() {
        let defaults = freshDefaults()
        _ = BotIdentitiesStore.mergeFromTokenBotAccountIds(
            ["auth0|bot-1"], defaults: defaults
        )
        let before = BotIdentitiesStore.entries(defaults: defaults)
        _ = BotIdentitiesStore.mergeFromTokenBotAccountIds(
            ["auth0|bot-1"], defaults: defaults
        )
        #expect(BotIdentitiesStore.entries(defaults: defaults) == before)
    }

    @Test("merge preserves a stored non-blank username over a blank token entry")
    func mergePreservesUsernames() {
        let stored = [BotIdentityEntry(principal: "auth0|bot-1", username: "cutie-bot")]
        let merged = BotIdentitiesStore.merge(
            existing: stored,
            additions: [BotIdentityEntry(principal: "auth0|bot-1", username: nil)]
        )
        #expect(merged.count == 1)
        #expect(merged.first?.username == "cutie-bot")
    }

    @Test("corrupt persisted data loads as empty (next merge re-seeds)")
    func corruptDataLoadsEmpty() {
        let defaults = freshDefaults()
        defaults.set(Data("not json".utf8), forKey: "BOT_IDENTITIES")
        #expect(BotIdentitiesStore.entries(defaults: defaults).isEmpty)
    }

    @Test("remove clears the store")
    func removeClears() {
        let defaults = freshDefaults()
        BotIdentitiesStore.put(
            [BotIdentityEntry(principal: "auth0|bot-1", username: nil)],
            defaults: defaults
        )
        BotIdentitiesStore.remove(defaults: defaults)
        #expect(BotIdentitiesStore.entries(defaults: defaults).isEmpty)
    }
}
