import Foundation

/// AI identities persisted locally — port of Kotlin `AIIdentitiesStore`.
/// The JWT's `ext_ai_account_ids` claim seeds the list on every social
/// sign-in / token refresh (`AuthClient.saveTokens` merges here); AI account
/// usernames update as profiles load. The account switcher reads this to
/// build the AI-influencer section (the main account comes from the
/// Keychain's MAIN_PRINCIPAL).
struct AIIdentityEntry: Codable, Equatable, Sendable {
    let principal: String
    var username: String?
}

/// UserDefaults-backed store (Kotlin used its Preferences — display data,
/// not secrets). JSON-encoded array under BOT_IDENTITIES.
enum AIIdentitiesStore {

    private static let storageKey = "BOT_IDENTITIES"

    /// Decode errors (corrupt/corrupt-by-upgrade data) load as empty —
    /// the next token merge re-seeds the list.
    static func entries(defaults: UserDefaults = .standard) -> [AIIdentityEntry] {
        guard let data = defaults.data(forKey: storageKey),
              let decoded = try? JSONDecoder().decode(
                  [AIIdentityEntry].self, from: data
              )
        else { return [] }
        return decoded
    }

    static func put(_ entries: [AIIdentityEntry], defaults: UserDefaults = .standard) {
        guard let data = try? JSONEncoder().encode(entries) else { return }
        defaults.set(data, forKey: storageKey)
    }

    static func remove(defaults: UserDefaults = .standard) {
        defaults.removeObject(forKey: storageKey)
    }

    /// Kotlin `BotIdentityStorage.saveBotIdentity` — upsert a single AI
    /// identity (with its username) after creation.
    static func saveIdentity(
        principal: String,
        username: String?,
        defaults: UserDefaults = .standard
    ) {
        var entries = Self.entries(defaults: defaults)
        if let index = entries.firstIndex(where: { $0.principal == principal }) {
            entries[index].username = username ?? entries[index].username
        } else {
            entries.append(AIIdentityEntry(principal: principal, username: username))
        }
        put(entries, defaults: defaults)
    }

    /// Kotlin `mergeFromTokenBotAccountIds`: union of stored + token-claimed
    /// identities, keyed by principal; the most recent non-blank username
    /// wins. Returns nil when the merge would change nothing (empty input).
    @discardableResult
    static func mergeFromTokenAIAccountIds(
        _ aiAccountIds: [String],
        defaults: UserDefaults = .standard
    ) -> MergeResult? {
        let newEntries = aiAccountIds
            .filter { !$0.isBlank }
            .map { AIIdentityEntry(principal: $0, username: nil) }
        guard !newEntries.isEmpty else { return nil }
        let existing = entries(defaults: defaults)
        let merged = merge(existing: existing, additions: newEntries)
        guard merged != existing else {
            return MergeResult(
                existingCount: existing.count,
                addedCount: 0,
                mergedCount: existing.count
            )
        }
        put(merged, defaults: defaults)
        return MergeResult(
            existingCount: existing.count,
            addedCount: newEntries.count,
            mergedCount: merged.count
        )
    }

    /// Pure union — existing entries keep their usernames; the LATEST
    /// entry (token order) wins for duplicate principals; a stored
    /// non-blank username is preserved.
    static func merge(
        existing: [AIIdentityEntry],
        additions: [AIIdentityEntry]
    ) -> [AIIdentityEntry] {
        let grouped = Dictionary(grouping: existing + additions, by: \.principal)
        return grouped.values.map { group in
            let latest = group.last!
            let username = group
                .reversed()
                .first(where: { !($0.username ?? "").isBlank })?
                .username
            var entry = latest
            entry.username = username
            return entry
        }
    }

    struct MergeResult: Equatable {
        let existingCount: Int
        let addedCount: Int
        let mergedCount: Int
    }
}
