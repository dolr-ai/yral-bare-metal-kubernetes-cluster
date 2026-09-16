import Testing
import Foundation
@testable import YralApp

/// Tests for `AIIdentitiesStore` — the merge semantics from Kotlin
/// `AIIdentitiesStore.mergeFromTokenAIAccountIds` (union by subject,
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

    @Test("blank subjects are filtered before merging")
    func blankSubjectsFiltered() {
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
        #expect(entries.map(\.subject).sorted() == ["auth0|AI account-1", "auth0|AI account-2"])
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
        let stored = [AIIdentityEntry(subject: "auth0|AI account-1", username: "cutie-AI account")]
        let merged = AIIdentitiesStore.merge(
            existing: stored,
            additions: [AIIdentityEntry(subject: "auth0|AI account-1", username: nil)]
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
            [AIIdentityEntry(subject: "auth0|AI account-1", username: nil)],
            defaults: defaults
        )
        AIIdentitiesStore.remove(defaults: defaults)
        #expect(AIIdentitiesStore.entries(defaults: defaults).isEmpty)
    }

    // MARK: - Deleted-bot resurrection (regression)

    /// The bug: a token minted BEFORE a deletion still carries the deleted
    /// bot in `ext_ai_account_ids`, so the union merge re-added it to the
    /// switcher as a zombie. `removeIdentity` now tombstones the subject and
    /// the merge filters tombstones from both sides.

    @Test("a deleted subject is not resurrected by a stale token")
    func deletedSubjectNotResurrected() {
        let defaults = freshDefaults()
        let deletedBot = "auth0|deleted-bot"

        // The bot exists locally, then is deleted.
        AIIdentitiesStore.saveIdentity(subject: deletedBot, username: "gone", defaults: defaults)
        AIIdentitiesStore.removeIdentity(subject: deletedBot, defaults: defaults)
        #expect(AIIdentitiesStore.entries(defaults: defaults).isEmpty)

        // A stale token still names it — the merge must NOT re-add it.
        _ = AIIdentitiesStore.mergeFromTokenAIAccountIds(
            [deletedBot, "auth0|live-bot"], defaults: defaults
        )
        let subjects = AIIdentitiesStore.entries(defaults: defaults).map(\.subject)
        #expect(!subjects.contains(deletedBot))
        #expect(subjects == ["auth0|live-bot"])
    }

    @Test("the tombstone survives an empty token merge")
    func tombstonePersists() {
        let defaults = freshDefaults()
        AIIdentitiesStore.removeIdentity(subject: "auth0|deleted-bot", defaults: defaults)
        // Merging nothing is a no-op, but must not clear the tombstone.
        #expect(AIIdentitiesStore.mergeFromTokenAIAccountIds([], defaults: defaults) == nil)
        #expect(AIIdentitiesStore.deletedSubjects(defaults: defaults) == ["auth0|deleted-bot"])
    }

    @Test("a deleted subject already persisted is pruned on the next merge")
    func persistedDeletedSubjectIsPruned() {
        let defaults = freshDefaults()
        // Simulates state written before the tombstone existed: the bot is
        // still in the stored entries AND tombstoned.
        AIIdentitiesStore.put(
            [
                AIIdentityEntry(subject: "auth0|deleted-bot", username: "gone"),
                AIIdentityEntry(subject: "auth0|live-bot", username: nil)
            ],
            defaults: defaults
        )
        AIIdentitiesStore.markDeleted(subject: "auth0|deleted-bot", defaults: defaults)

        _ = AIIdentitiesStore.mergeFromTokenAIAccountIds(
            ["auth0|live-bot"], defaults: defaults
        )

        let subjects = AIIdentitiesStore.entries(defaults: defaults).map(\.subject)
        #expect(subjects == ["auth0|live-bot"])
    }

    @Test("marking the same subject twice does not duplicate the tombstone")
    func tombstoneIsIdempotent() {
        let defaults = freshDefaults()
        AIIdentitiesStore.markDeleted(subject: "auth0|bot", defaults: defaults)
        AIIdentitiesStore.markDeleted(subject: "auth0|bot", defaults: defaults)
        #expect(AIIdentitiesStore.deletedSubjects(defaults: defaults).count == 1)
    }

    @Test("remove on an absent subject still tombstones it")
    func removeAbsentSubjectTombstones() {
        // The delete flow may run for a bot that was never persisted locally
        // (created on another device) — the tombstone must still be written,
        // or a stale token would add it later.
        let defaults = freshDefaults()
        AIIdentitiesStore.removeIdentity(subject: "auth0|never-stored", defaults: defaults)
        #expect(AIIdentitiesStore.entries(defaults: defaults).isEmpty)
        #expect(AIIdentitiesStore.deletedSubjects(defaults: defaults) == ["auth0|never-stored"])
    }
}
