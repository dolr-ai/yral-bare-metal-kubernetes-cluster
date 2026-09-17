import Foundation

/// AI identities persisted locally — port of Kotlin `AIIdentitiesStore`.
/// The JWT's `ext_ai_account_ids` claim seeds the list on every social
/// sign-in / token refresh (`AuthClient.saveTokens` merges here); AI account
/// usernames update as profiles load. The account switcher reads this to
/// build the AI-influencer section (the main account comes from the
/// Keychain's MAIN_PRINCIPAL).
///
/// Display data ONLY: this stores WHICH bots exist (subjects) and their
/// usernames. The bots' avatar URLs are NOT duplicated here — they live in
/// the SpacetimeDB profile table (written at creation) and are read from
/// there when the switcher opens (`refreshedAccountSwitcherEntries`).
struct AIIdentityEntry: Codable, Equatable, Sendable {
  let subject: String
  var username: String?
}

/// UserDefaults-backed store (Kotlin used its Preferences — display data,
/// not secrets). JSON-encoded array under BOT_IDENTITIES, tombstones under
/// DELETED_BOT_SUBJECTS.
enum AIIdentitiesStore {

  private static let storageKey = "BOT_IDENTITIES"
  private static let deletedSubjectsKey = "DELETED_BOT_SUBJECTS"

  /// Every subject deleted on this device.
  ///
  /// The local store is display-only, but it is *seeded* from the JWT's
  /// `ext_ai_account_ids` on each refresh (`saveTokens`). A token minted
  /// before a deletion still names the deleted bot, so without a
  /// tombstone the union merge in `mergeFromTokenAIAccountIds` re-adds it
  /// to the switcher as a zombie. The server prunes the KV list that
  /// feeds that claim, but this device's cached token can be stale — the
  /// tombstone makes the client self-consistent regardless.
  static func deletedSubjects(defaults: UserDefaults = .standard) -> Set<String> {
    Set(defaults.stringArray(forKey: deletedSubjectsKey) ?? [])
  }

  /// Remember a deletion so the token merge can never re-add the subject.
  static func markDeleted(subject: String, defaults: UserDefaults = .standard) {
    var deleted = deletedSubjects(defaults: defaults)
    guard deleted.insert(subject).inserted else { return }
    defaults.set(Array(deleted), forKey: deletedSubjectsKey)
  }

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

  /// Drops one AI identity and tombstones it. The account-deletion flow
  /// calls this so the deleted bot disappears from the switcher's AI
  /// section — and stays gone even if a stale token still claims it
  /// (`mergeFromTokenAIAccountIds` filters tombstoned subjects).
  static func removeIdentity(
    subject: String,
    defaults: UserDefaults = .standard
  ) {
    markDeleted(subject: subject, defaults: defaults)
    let current = entries(defaults: defaults)
    let remaining = current.filter { $0.subject != subject }
    if remaining.count == current.count { return }
    put(remaining, defaults: defaults)
  }

  /// Kotlin `BotIdentityStorage.saveBotIdentity` — upsert a single AI
  /// identity (with its username) after creation.
  static func saveIdentity(
    subject: String,
    username: String?,
    defaults: UserDefaults = .standard
  ) {
    var entries = Self.entries(defaults: defaults)
    if let index = entries.firstIndex(where: { $0.subject == subject }) {
      entries[index].username = username ?? entries[index].username
    } else {
      entries.append(AIIdentityEntry(subject: subject, username: username))
    }
    put(entries, defaults: defaults)
  }

  /// Kotlin `mergeFromTokenBotAccountIds`: union of stored + token-claimed
  /// identities, keyed by subject; the most recent non-blank username
  /// wins. Returns nil when the merge would change nothing (empty input).
  ///
  /// Tombstoned subjects are dropped from BOTH sides: a token minted
  /// before a deletion still lists the deleted bot, so the merge would
  /// otherwise resurrect it (see `markDeleted`).
  @discardableResult
  static func mergeFromTokenAIAccountIds(
    _ aiAccountIds: [String],
    defaults: UserDefaults = .standard
  ) -> MergeResult? {
    let deleted = deletedSubjects(defaults: defaults)
    let newEntries =
      aiAccountIds
      .filter { !$0.isBlank && !deleted.contains($0) }
      .map { AIIdentityEntry(subject: $0, username: nil) }
    let stored = entries(defaults: defaults)
    // Prune stored entries BEFORE the empty-input early return: a
    // tombstoned subject already persisted (written before the
    // tombstone existed, or by an older build) must be cleared out even
    // when this refresh carries no new ids.
    let existing = stored.filter { !deleted.contains($0.subject) }
    let prunedStored = existing.count != stored.count

    let merged = merge(existing: existing, additions: newEntries)
    // Write when the merge changed something OR the prune did — the
    // latter is the only way a tombstone takes effect for an entry that
    // is still on disk.
    guard merged != existing || prunedStored else {
      return newEntries.isEmpty
        ? nil
        : MergeResult(
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
  /// entry (token order) wins for duplicate subjects; a stored
  /// non-blank username is preserved.
  static func merge(
    existing: [AIIdentityEntry],
    additions: [AIIdentityEntry]
  ) -> [AIIdentityEntry] {
    let grouped = Dictionary(grouping: existing + additions, by: \.subject)
    return grouped.values.map { group in
      let latest = group.last!
      let username =
        group
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
