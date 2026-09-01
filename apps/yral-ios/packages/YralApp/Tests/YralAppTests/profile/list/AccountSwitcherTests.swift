import Testing
import Foundation
@testable import YralApp

/// Tests for `AIIdentitiesStore` — the merge semantics from Kotlin
/// `AIIdentitiesStore.mergeFromTokenAIAccountIds` (union by principal,
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
        #expect(AIIdentitiesStore.mergeFromTokenAIAccountIds([], defaults: defaults) == nil)
        #expect(AIIdentitiesStore.entries(defaults: defaults).isEmpty)
    }

    @Test("blank principals are filtered before merging")
    func blankPrincipalsFiltered() {
        let defaults = freshDefaults()
        #expect(
            AIIdentitiesStore.mergeFromTokenAIAccountIds(
                ["", "   "], defaults: defaults
            ) == nil
        )
        #expect(AIIdentitiesStore.entries(defaults: defaults).isEmpty)
    }

    @Test("token identities merge into an empty store")
    func mergeIntoEmpty() {
        let defaults = freshDefaults()
        let result = AIIdentitiesStore.mergeFromTokenAIAccountIds(
            ["auth0|AI account-1", "auth0|AI account-2"], defaults: defaults
        )
        #expect(result?.addedCount == 2)
        #expect(result?.mergedCount == 2)
        let entries = AIIdentitiesStore.entries(defaults: defaults)
        #expect(entries.map(\.principal).sorted() == ["auth0|AI account-1", "auth0|AI account-2"])
    }

    @Test("re-merging the same identities changes nothing (idempotent)")
    func idempotentMerge() {
        let defaults = freshDefaults()
        _ = AIIdentitiesStore.mergeFromTokenAIAccountIds(
            ["auth0|AI account-1"], defaults: defaults
        )
        let before = AIIdentitiesStore.entries(defaults: defaults)
        _ = AIIdentitiesStore.mergeFromTokenAIAccountIds(
            ["auth0|AI account-1"], defaults: defaults
        )
        #expect(AIIdentitiesStore.entries(defaults: defaults) == before)
    }

    @Test("merge preserves a stored non-blank username over a blank token entry")
    func mergePreservesUsernames() {
        let stored = [AIIdentityEntry(principal: "auth0|AI account-1", username: "cutie-AI account")]
        let merged = AIIdentitiesStore.merge(
            existing: stored,
            additions: [AIIdentityEntry(principal: "auth0|AI account-1", username: nil)]
        )
        #expect(merged.count == 1)
        #expect(merged.first?.username == "cutie-AI account")
    }

    @Test("corrupt persisted data loads as empty (next merge re-seeds)")
    func corruptDataLoadsEmpty() {
        let defaults = freshDefaults()
        defaults.set(Data("not json".utf8), forKey: "BOT_IDENTITIES")
        #expect(AIIdentitiesStore.entries(defaults: defaults).isEmpty)
    }

    @Test("remove clears the store")
    func removeClears() {
        let defaults = freshDefaults()
        AIIdentitiesStore.put(
            [AIIdentityEntry(principal: "auth0|AI account-1", username: nil)],
            defaults: defaults
        )
        AIIdentitiesStore.remove(defaults: defaults)
        #expect(AIIdentitiesStore.entries(defaults: defaults).isEmpty)
    }
}
