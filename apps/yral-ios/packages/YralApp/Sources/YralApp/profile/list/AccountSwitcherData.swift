import Foundation

/// Account-switcher data for `AuthClient` — feature-local (the switcher
/// lives in `profile/list/`); this extension carries its loading +
/// overlay logic so `AuthClientPersistence.swift` stays within lint
/// bounds.
///
/// The switcher's list has three layers, each best-effort:
///   1. INSTANT local rows — subjects from the JWT-seeded
///      `AIIdentitiesStore`, pseudonym-name fallbacks, GobGob avatars.
///   2. AVATAR overlay — one batch read of ALL row subjects (main +
///      bots) from the SpacetimeDB profile table overlays the DURABLE
///      hosted URLs (written at creation). The main row gets its OWN
///      picture — never the active bot's (the double-avatar bug).
///   3. NAME overlay — one creator call (`GET /api/v1/creator/
///      influencers`, `id == subject` verified live) overlays the
///      bots' real names. The profile table carries no displayable
///      name, and the locally stored username exists only on the
///      creating device — without this overlay every device but the
///      creating one shows pseudonyms.
/// Nothing is duplicated into local storage; the remote stores are the
/// sources of truth. A failed overlay keeps the previous layer's rows.
extension AuthClient {

    /// The switcher's instant local list — main account (from
    /// MAIN_SUBJECT) + AI entries (from AIIdentitiesStore), each with
    /// resolved username + propic + the active flag. Nil when no main
    /// subject exists (signed out).
    func accountSwitcherEntries() -> AccountSwitcherEntries? {
        guard let mainSubject = keychain.string(forKey: .mainSubject) else {
            return nil
        }
        let activeSubject = sessionStore.userSubject
        // Main row: its OWN session pic when the main account is the
        // active one; the GobGob fallback otherwise (never the active
        // bot's picture — that was the double-avatar bug).
        let mainAvatarURL: String
        if mainSubject == activeSubject, let sessionPic = sessionStore.profilePic {
            mainAvatarURL = sessionPic
        } else {
            mainAvatarURL = ProfilePicture.url(fromSubject: mainSubject)
        }
        let mainEntry = AccountSwitcherEntry(
            subject: mainSubject,
            // No server-side display name in the profile table yet —
            // deterministic pseudonym fallback (never the raw identifier
            // as a name).
            username: UsernameGenerator.resolveUsername(
                preferred: nil, subject: mainSubject
            ) ?? mainSubject,
            avatarURL: mainAvatarURL,
            isBot: false,
            isActive: mainSubject == activeSubject
        )
        let botEntries = AIIdentitiesStore.entries(defaults: defaults)
            .filter { $0.subject != mainSubject }
            .map { entry in
                AccountSwitcherEntry(
                    subject: entry.subject,
                    // Stored username (creation device only) when we have
                    // it; deterministic pseudonym fallback otherwise. The
                    // creator-name overlay replaces pseudonyms with the
                    // real names once loaded.
                    username: UsernameGenerator.resolveUsername(
                        preferred: entry.username, subject: entry.subject
                    ) ?? entry.subject,
                    // GobGob deterministic fallback — `refreshedAccountSwitcherEntries()`
                    // overlays the hosted URL from the profile table when
                    // the switcher is opened.
                    avatarURL: ProfilePicture.url(fromSubject: entry.subject),
                    isBot: true,
                    isActive: entry.subject == activeSubject
                )
            }
        return AccountSwitcherEntries(mainAccount: mainEntry, aiAccounts: botEntries)
    }

    /// The switcher's list with live data — `accountSwitcherEntries()`
    /// (instant local fallbacks) upgraded by the avatar overlay (batch
    /// profile read) and the name overlay (creator API).
    func refreshedAccountSwitcherEntries() async -> AccountSwitcherEntries? {
        guard var entries = accountSwitcherEntries() else { return nil }
        // Overlay 1 — avatars: ALL rows in one batch read (main + bots),
        // so the main row shows its OWN picture, never the active bot's.
        var allSubjects = entries.aiAccounts.map(\.subject)
        if let mainSubject = keychain.string(forKey: .mainSubject) {
            allSubjects.append(mainSubject)
        }
        do {
            let profiles = try await spacetimeDataSource.getUsersProfileDetails(
                oauthSubjects: allSubjects
            )
            entries.aiAccounts = Self.applyProfilePictures(
                to: entries.aiAccounts,
                from: profiles
            )
            if let mainSubject = keychain.string(forKey: .mainSubject),
               let mainProfile = profiles.first(where: { $0.oauthSubject == mainSubject }),
               let mainPicture = mainProfile.profilePicture,
               !mainPicture.url.isEmpty {
                entries.mainAccount.avatarURL = mainPicture.url
            }
        } catch {
            // Best-effort overlay — GobGob fallback rows stay. The
            // failure still reaches Crashlytics (non-fatal).
            CrashReporter.record(error, context: "switcher-avatar-overlay")
        }
        // Overlay 2 — real bot names from the creator API.
        if let idToken = keychain.string(forKey: .idToken) {
            do {
                let creators = try await influencerDataSource?.listMyInfluencers(
                    idToken: idToken
                )
                if let creators {
                    entries.aiAccounts = Self.applyRealNames(
                        to: entries.aiAccounts,
                        from: creators
                    )
                }
            } catch {
                // Best-effort overlay — pseudonym fallback stays; the
                // failure still reaches Crashlytics (non-fatal).
                CrashReporter.record(error, context: "switcher-name-overlay")
            }
        }
        return entries
    }

    /// Pure — overlay the fetched profile pictures onto the switcher
    /// rows. A row keeps its fallback when the server row is missing or
    /// carries a blank URL (the bot's write may never have landed — e.g.
    /// the pre-wire-fix creations).
    nonisolated static func applyProfilePictures(
        to entries: [AccountSwitcherEntry],
        from profiles: [SpacetimeUserProfile]
    ) -> [AccountSwitcherEntry] {
        let pictureURLsBySubject: [String: String] = Dictionary(
            uniqueKeysWithValues: profiles.compactMap { profile -> (String, String)? in
                guard let picture = profile.profilePicture,
                      !picture.url.isEmpty
                else { return nil }
                return (profile.oauthSubject, picture.url)
            }
        )
        return entries.map { entry in
            guard let hostedURL = pictureURLsBySubject[entry.subject] else { return entry }
            var refreshed = entry
            refreshed.avatarURL = hostedURL
            return refreshed
        }
    }

    /// Pure — overlay the creator API's real names onto the switcher
    /// rows. Rows without a creator record keep their fallback name.
    nonisolated static func applyRealNames(
        to entries: [AccountSwitcherEntry],
        from influencers: [CreatorInfluencer]
    ) -> [AccountSwitcherEntry] {
        let namesBySubject = Dictionary(
            uniqueKeysWithValues: influencers.map { ($0.subject, $0.name) }
        )
        return entries.map { entry in
            guard let realName = namesBySubject[entry.subject],
                  !realName.isEmpty
            else { return entry }
            var refreshed = entry
            refreshed.username = realName
            return refreshed
        }
    }

    /// Switches the active account — CLIENT-SIDE session construction:
    /// build the session directly from the subject, update the store,
    /// persist the cached session fields, and set LAST_ACTIVE_PRINCIPAL.
    /// AI switches skip token refresh (the parent's tokens stay active);
    /// switching back to main refreshes + reauthorizes.
    ///
    /// `avatarURL`: the tapped switcher row's URL — after
    /// `refreshedAccountSwitcherEntries()` this is the bot's HOSTED
    /// avatar from the profile table (durable Storj URL). Falls back to
    /// the GobGob deterministic URL when absent (offline switch, main
    /// account, or a bot whose write never landed).
    ///
    /// `username`: the tapped row's display name — after the name
    /// overlay this is the bot's REAL name from the creator API. It
    /// feeds the session + PROFILE_PIC-cache so the Settings/Profile
    /// headers show it too (the local store only knows names on the
    /// creating device). Falls back to the stored/pseudonym name.
    func switchToAccount(
        subject: String,
        avatarURL: String? = nil,
        username: String? = nil
    ) {
        // No-op when already active (Kotlin returns early).
        guard sessionStore.userSubject != subject else { return }

        let storedMainSubject = keychain.string(forKey: .mainSubject)
        var isBot = true
        // Live row name first (creator overlay), then the locally stored
        // one; the pseudonym fallback resolves below.
        var botUsername = username
        if subject == storedMainSubject {
            isBot = false
        } else {
            let storedBots = AIIdentitiesStore.entries(defaults: defaults)
            guard let match = storedBots.first(where: { $0.subject == subject }) else {
                return
            }
            botUsername = botUsername ?? match.username
        }

        let profilePic = avatarURL
            ?? ProfilePicture.url(fromSubject: subject)
        // Live/stored username when present; deterministic pseudonym
        // fallback otherwise.
        let session = Session(
            canisterID: subject,
            userSubject: subject,
            profilePic: profilePic,
            username: UsernameGenerator.resolveUsername(
                preferred: botUsername, subject: subject
            ),
            bio: nil,
            isCreatedFromServiceCanister: true,
            isAIAccount: isBot
        )
        sessionStore.updateState(.signedIn(session))
        cacheSession(
            canisterID: subject,
            userSubject: subject,
            profilePic: profilePic,
            username: botUsername,
            isAIAccount: isBot
        )
        keychain.setString(subject, forKey: .lastActiveSubject)
        if isBot {
            // AI accounts share the parent's tokens — do NOT overwrite the
            // session with parent-token auth state.
            sessionStore.updateFirebaseLoginState(false)
        } else {
            Task {
                await refreshTokens()
                sessionStore.updateFirebaseLoginState(true)
            }
        }
    }
}
