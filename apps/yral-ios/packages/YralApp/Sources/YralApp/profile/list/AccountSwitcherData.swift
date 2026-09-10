import Foundation

/// Account-switcher data for `AuthClient` — feature-local (the switcher
/// lives in `profile/list/`); this extension carries its loading +
/// overlay logic so `AuthClientPersistence.swift` stays within lint
/// bounds.
///
/// The switcher's list has three layers, each best-effort:
///   1. INSTANT local rows — principals from the JWT-seeded
///      `AIIdentitiesStore`, pseudonym-name fallbacks, GobGob avatars.
///   2. AVATAR overlay — one batch read of ALL row principals (main +
///      bots) from the SpacetimeDB profile table overlays the DURABLE
///      hosted URLs (written at creation). The main row gets its OWN
///      picture — never the active bot's (the double-avatar bug).
///   3. NAME overlay — one creator call (`GET /api/v1/creator/
///      influencer­ers`, `id == principal` verified live) overlays the
///      bots' real names. The profile table carries no displayable
///      name, and the locally stored username exists only on the
///      creating device — without this overlay every device but the
///      creating one shows pseudonyms.
/// Nothing is duplicated into local storage; the remote stores are the
/// sources of truth. A failed overlay keeps the previous layer's rows.
extension AuthClient {

    /// The switcher's instant local list — main account (from
    /// MAIN_PRINCIPAL) + AI entries (from AIIdentitiesStore), each with
    /// resolved username + propic + the active flag. Nil when no main
    /// principal exists (signed out).
    func accountSwitcherEntries() -> AccountSwitcherEntries? {
        guard let mainPrincipal = keychain.string(forKey: .mainPrincipal) else {
            return nil
        }
        let activePrincipal = sessionStore.userPrincipal
        // Main row: its OWN session pic when the main account is the
        // active one; the GobGob fallback otherwise (never the active
        // bot's picture — that was the double-avatar bug).
        let mainAvatarURL: String
        if mainPrincipal == activePrincipal, let sessionPic = sessionStore.profilePic {
            mainAvatarURL = sessionPic
        } else {
            mainAvatarURL = ProfilePicture.url(fromPrincipal: mainPrincipal)
        }
        let mainEntry = AccountSwitcherEntry(
            principal: mainPrincipal,
            // No server-side display name in the profile table yet —
            // deterministic pseudonym fallback (never the raw identifier
            // as a name).
            username: UsernameGenerator.resolveUsername(
                preferred: nil, principal: mainPrincipal
            ) ?? mainPrincipal,
            avatarURL: mainAvatarURL,
            isBot: false,
            isActive: mainPrincipal == activePrincipal
        )
        let botEntries = AIIdentitiesStore.entries(defaults: defaults)
            .filter { $0.principal != mainPrincipal }
            .map { entry in
                AccountSwitcherEntry(
                    principal: entry.principal,
                    // Stored username (creation device only) when we have
                    // it; deterministic pseudonym fallback otherwise. The
                    // creator-name overlay replaces pseudonyms with the
                    // real names once loaded.
                    username: UsernameGenerator.resolveUsername(
                        preferred: entry.username, principal: entry.principal
                    ) ?? entry.principal,
                    // GobGob deterministic fallback — `refreshedAccountSwitcherEntries()`
                    // overlays the hosted URL from the profile table when
                    // the switcher is opened.
                    avatarURL: ProfilePicture.url(fromPrincipal: entry.principal),
                    isBot: true,
                    isActive: entry.principal == activePrincipal
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
        var allPrincipals = entries.aiAccounts.map(\.principal)
        if let mainPrincipal = keychain.string(forKey: .mainPrincipal) {
            allPrincipals.append(mainPrincipal)
        }
        if let profiles = try? await spacetimeDataSource.getUsersProfileDetails(
            oauthSubjects: allPrincipals
        ) {
            entries.aiAccounts = Self.applyProfilePictures(
                to: entries.aiAccounts,
                from: profiles
            )
            if let mainPrincipal = keychain.string(forKey: .mainPrincipal),
               let mainProfile = profiles.first(where: { $0.oauthSubject == mainPrincipal }),
               let mainPicture = mainProfile.profilePicture,
               !mainPicture.url.isEmpty {
                entries.mainAccount.avatarURL = mainPicture.url
            }
        }
        // Overlay 2 — real bot names from the creator API.
        if let idToken = keychain.string(forKey: .idToken),
           let creators = try? await influencerDataSource?.listMyInfluencers(
               idToken: idToken
           ) {
            entries.aiAccounts = Self.applyRealNames(
                to: entries.aiAccounts,
                from: creators
            )
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
        let pictureURLsByPrincipal: [String: String] = Dictionary(
            uniqueKeysWithValues: profiles.compactMap { profile -> (String, String)? in
                guard let picture = profile.profilePicture,
                      !picture.url.isEmpty
                else { return nil }
                return (profile.oauthSubject, picture.url)
            }
        )
        return entries.map { entry in
            guard let hostedURL = pictureURLsByPrincipal[entry.principal] else { return entry }
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
        let namesByPrincipal = Dictionary(
            uniqueKeysWithValues: influencers.map { ($0.principal, $0.name) }
        )
        return entries.map { entry in
            guard let realName = namesByPrincipal[entry.principal],
                  !realName.isEmpty
            else { return entry }
            var refreshed = entry
            refreshed.username = realName
            return refreshed
        }
    }

    /// Switches the active account — CLIENT-SIDE session construction:
    /// build the session directly from the principal, update the store,
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
        principal: String,
        avatarURL: String? = nil,
        username: String? = nil
    ) {
        // No-op when already active (Kotlin returns early).
        guard sessionStore.userPrincipal != principal else { return }

        let storedMainPrincipal = keychain.string(forKey: .mainPrincipal)
        var isBot = true
        // Live row name first (creator overlay), then the locally stored
        // one; the pseudonym fallback resolves below.
        var botUsername = username
        if principal == storedMainPrincipal {
            isBot = false
        } else {
            let storedBots = AIIdentitiesStore.entries(defaults: defaults)
            guard let match = storedBots.first(where: { $0.principal == principal }) else {
                return
            }
            botUsername = botUsername ?? match.username
        }

        let profilePic = avatarURL
            ?? ProfilePicture.url(fromPrincipal: principal)
        // Live/stored username when present; deterministic pseudonym
        // fallback otherwise.
        let session = Session(
            canisterID: principal,
            userPrincipal: principal,
            profilePic: profilePic,
            username: UsernameGenerator.resolveUsername(
                preferred: botUsername, principal: principal
            ),
            bio: nil,
            isCreatedFromServiceCanister: true,
            isAIAccount: isBot
        )
        sessionStore.updateState(.signedIn(session))
        cacheSession(
            canisterID: principal,
            userPrincipal: principal,
            profilePic: profilePic,
            username: botUsername,
            isAIAccount: isBot
        )
        keychain.setString(principal, forKey: .lastActivePrincipal)
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
