import Foundation

/// Cached-session persistence for `AuthClient` — Kotlin
/// `DefaultAuthClient`'s `getCachedSession`/`cacheSession`/
/// `resetCachedCanisterData`/`saveTokens`/`updateYralSession`/`postLogin`,
/// kept as an extension of the same class (not a layer).
extension AuthClient {

    /// Rebuilds the cached session — Kotlin `getCachedSession` verbatim
    /// (single-slot cache: PROFILE_PIC/USERNAME are trusted only when
    /// USER_PRINCIPAL == LAST_ACTIVE_PRINCIPAL).
    func cachedSession() -> Session? {
        let mainPrincipal = keychain.string(forKey: .mainPrincipal)
        let lastActivePrincipal = keychain.string(forKey: .lastActivePrincipal)

        let preferredPrincipal = defaults.string(forKey: CachedSessionKey.userPrincipal.rawValue)
        let usePreferred = lastActivePrincipal != nil
            && preferredPrincipal == lastActivePrincipal

        let canisterID = defaults.string(forKey: CachedSessionKey.canisterID.rawValue)
        let userPrincipal = usePreferred
            ? preferredPrincipal
            : mainPrincipal ?? preferredPrincipal
        let profilePic = cachedProfilePic(
            userPrincipal: userPrincipal, preferredPrincipal: preferredPrincipal
        )
        let username = cachedUsername(
            userPrincipal: userPrincipal, preferredPrincipal: preferredPrincipal
        )
        let isCreatedFromServiceCanister = defaults.bool(
            forKey: CachedSessionKey.isCreatedFromServiceCanister.rawValue
        )
        let resolvedIsBotAccount = mainPrincipal.map { main in
            userPrincipal != nil && userPrincipal != main
        } ?? false

        guard let canisterID, let userPrincipal, let profilePic else { return nil }
        return Session(
            canisterID: canisterID,
            userPrincipal: userPrincipal,
            profilePic: profilePic,
            username: UsernameGenerator.resolveUsername(
                preferred: username, principal: userPrincipal
            ),
            isCreatedFromServiceCanister: isCreatedFromServiceCanister,
            isBotAccount: resolvedIsBotAccount
        )
    }

    /// Kotlin `getCachedProfilePic`: cached pic trusted only when the
    /// preferred principal matches; else derived from the principal.
    private func cachedProfilePic(
        userPrincipal: String?,
        preferredPrincipal: String?
    ) -> String? {
        let cached = defaults.string(forKey: CachedSessionKey.profilePic.rawValue)
        guard preferredPrincipal == userPrincipal else {
            return userPrincipal.map { ProfilePicture.url(fromPrincipal: $0) }
        }
        return cached ?? userPrincipal.map { ProfilePicture.url(fromPrincipal: $0) }
    }

    /// Kotlin `getCachedUsername`: cached username trusted only when the
    /// preferred principal matches.
    private func cachedUsername(
        userPrincipal: String?,
        preferredPrincipal: String?
    ) -> String? {
        let cached = defaults.string(forKey: CachedSessionKey.username.rawValue)
        guard preferredPrincipal == userPrincipal else { return nil }
        return cached
    }

    /// Persists the session fields — Kotlin `cacheSession` (writes
    /// MAIN_PRINCIPAL/LAST_ACTIVE_PRINCIPAL only for non-bot sessions; a
    /// bot session never overwrites the main principal).
    func cacheSession(
        canisterID: String,
        userPrincipal: String,
        profilePic: String,
        username: String?,
        isBotAccount: Bool
    ) {
        defaults.set(canisterID, forKey: CachedSessionKey.canisterID.rawValue)
        defaults.set(userPrincipal, forKey: CachedSessionKey.userPrincipal.rawValue)
        defaults.set(profilePic, forKey: CachedSessionKey.profilePic.rawValue)
        let resolvedUsername = UsernameGenerator.resolveUsername(
            preferred: username, principal: userPrincipal
        )
        if let resolvedUsername {
            defaults.set(resolvedUsername, forKey: CachedSessionKey.username.rawValue)
        } else {
            defaults.removeObject(forKey: CachedSessionKey.username.rawValue)
        }
        defaults.set(true, forKey: CachedSessionKey.isCreatedFromServiceCanister.rawValue)
        if !isBotAccount {
            let storedMainPrincipal = keychain.string(forKey: .mainPrincipal)
            if storedMainPrincipal == nil || storedMainPrincipal == userPrincipal {
                keychain.setString(userPrincipal, forKey: .mainPrincipal)
                keychain.setString(userPrincipal, forKey: .lastActivePrincipal)
            }
        }
    }

    /// Kotlin `resetCachedCanisterData` — logout clears the cached session
    /// fields and principal prefs.
    func resetCachedCanisterData() {
        for key in [
            CachedSessionKey.canisterID,
            CachedSessionKey.userPrincipal,
            CachedSessionKey.profilePic,
            CachedSessionKey.username,
            CachedSessionKey.isCreatedFromServiceCanister
        ] {
            defaults.removeObject(forKey: key.rawValue)
        }
        keychain.removeValue(forKey: .mainPrincipal)
        keychain.removeValue(forKey: .lastActivePrincipal)
    }

    /// Kotlin `saveTokens` — writes the three OAuth tokens; the
    /// empty-string guards keep absent fields from overwriting existing
    /// values.
    func saveTokens(
        idToken: String,
        refreshToken: String,
        accessToken: String,
        persistBotIdentities: Bool = true
    ) {
        keychain.setString(idToken, forKey: .idToken)
        if !refreshToken.isEmpty {
            keychain.setString(refreshToken, forKey: .refreshToken)
        }
        if !accessToken.isEmpty {
            keychain.setString(accessToken, forKey: .accessToken)
        }
        // Kotlin merges JWT `ext_ai_account_ids` into BotIdentitiesStore
        // here when `persistBotIdentities` is true; that store is consumed
        // by the account-switcher phase, which will add its persistence
        // alongside its UI.
        _ = persistBotIdentities
    }

    /// Kotlin `updateYralSession` — fire-and-forget registration call.
    func updateYralSession(_ session: Session) async {
        guard let idToken = keychain.string(forKey: .idToken),
              let canisterID = session.canisterID,
              let userPrincipal = session.userPrincipal
        else { return }
        _ = try? await authDataSource.updateSessionAsRegistered(
            idToken: idToken,
            canisterID: canisterID,
            userPrincipal: userPrincipal
        )
    }

    /// Kotlin `postLogin` — notification-token registration; the push
    /// phase wires this to Firebase Messaging.
    func postLogin() {}

    var currentEpochSeconds: Int64 {
        Int64(Date.now.timeIntervalSince1970)
    }
}
