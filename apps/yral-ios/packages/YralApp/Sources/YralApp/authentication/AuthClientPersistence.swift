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
            isAIAccount: resolvedIsBotAccount
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
    /// MAIN_PRINCIPAL/LAST_ACTIVE_PRINCIPAL only for non-AI account sessions; a
    /// AI account session never overwrites the main principal).
    func cacheSession(
        canisterID: String,
        userPrincipal: String,
        profilePic: String,
        username: String?,
        isAIAccount: Bool
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
        if !isAIAccount {
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
        // Kotlin merges the JWT's `ext_ai_account_ids` into
        // AIIdentitiesStore here when `persistBotIdentities` is true —
        // that list feeds the account switcher's AI section.
        if persistBotIdentities,
           let claims = try? JWTParser.parsePayload(of: idToken),
           let aiAccountIds = claims.aiAccountIds {
            AIIdentitiesStore.mergeFromTokenAIAccountIds(aiAccountIds, defaults: defaults)
        }
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

    // MARK: - Account switching (Kotlin RootViewModel.switchToAccount)

    /// The switcher's list — Kotlin `seedAccountDialogFromLocalData`:
    /// main account (from MAIN_PRINCIPAL) + AI entries (from
    /// AIIdentitiesStore), each with resolved username + propic + the
    /// active flag. Nil when no main principal exists (signed out).
    func accountSwitcherEntries() -> AccountSwitcherEntries? {
        guard let mainPrincipal = keychain.string(forKey: .mainPrincipal) else {
            return nil
        }
        let activePrincipal = sessionStore.userPrincipal
        let mainEntry = AccountSwitcherEntry(
            principal: mainPrincipal,
            username: mainPrincipal,
            avatarURL: sessionStore.profilePic
                ?? ProfilePicture.url(fromPrincipal: mainPrincipal),
            isBot: false,
            isActive: mainPrincipal == activePrincipal
        )
        let botEntries = AIIdentitiesStore.entries(defaults: defaults)
            .filter { $0.principal != mainPrincipal }
            .map { entry in
                AccountSwitcherEntry(
                    principal: entry.principal,
                    username: UsernameGenerator.resolveUsername(
                        preferred: entry.username, principal: entry.principal
                    ) ?? entry.principal,
                    avatarURL: ProfilePicture.url(fromPrincipal: entry.principal),
                    isBot: true,
                    isActive: entry.principal == activePrincipal
                )
            }
        return AccountSwitcherEntries(mainAccount: mainEntry, aiAccounts: botEntries)
    }

    /// Switches the active account — Kotlin `switchToAccount` (CLIENT-SIDE
    /// session construction; no network): build the session directly from
    /// the principal (propic + username derived), update the store,
    /// persist the cached session fields, and set LAST_ACTIVE_PRINCIPAL.
    /// AI switches skip token refresh (the parent's tokens stay active);
    /// switching back to main refreshes + reauthorizes.
    func switchToAccount(principal: String) {
        // No-op when already active (Kotlin returns early).
        guard sessionStore.userPrincipal != principal else { return }

        let storedMainPrincipal = keychain.string(forKey: .mainPrincipal)
        var isBot = true
        var botUsername: String?
        if principal == storedMainPrincipal {
            isBot = false
        } else {
            let storedBots = AIIdentitiesStore.entries(defaults: defaults)
            guard let match = storedBots.first(where: { $0.principal == principal }) else {
                return
            }
            botUsername = match.username
        }

        let profilePic = ProfilePicture.url(fromPrincipal: principal)
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
            // AI accounts share the parent's tokens — do NOT overwrite the session
            // with parent-token auth state (Kotlin parity).
            sessionStore.updateFirebaseLoginState(false)
        } else {
            Task {
                await refreshTokens()
                sessionStore.updateFirebaseLoginState(true)
            }
        }
    }

    /// Kotlin `postLogin` — notification-token registration; the push
    /// phase wires this to Firebase Messaging.
    func postLogin() {}

    var currentEpochSeconds: Int64 {
        Int64(Date.now.timeIntervalSince1970)
    }

    // MARK: - Logout + account deletion

    /// User-initiated logout.
    public func logout() async {
        await logoutInternal()
    }

    /// Current ID token, or nil when signed out — settings flows (delete
    /// account) need it for the Bearer-authenticated off-chain call.
    public var idToken: String? {
        keychain.string(forKey: .idToken)
    }

    /// Delete the account via off-chain-agent (Kotlin
    /// `DeleteAccountUseCase` main-account path) then logout. AI account
    /// accounts additionally need the soft-delete-on-AI account-server step —
    /// that lands with the AI accounts phase.
    public func deleteAccount() async throws {
        guard let idToken else {
            throw AuthError.oauthFailed(errorDescription: "Not signed in")
        }
        try await authDataSource.deleteAccount(idToken: idToken)
        await logoutInternal()
    }

    /// Kotlin `trackAndLogoutForTokenExpiry` — the token-expiry logout
    /// path with its cause (analytics event lands with the analytics phase).
    func trackAndLogoutForTokenExpiry(cause: AuthExpiryCause) async {
        lastLogoutCause = cause
        await logoutInternal()
    }

    func logoutInternal() async {
        keychain.removeValue(forKey: .refreshToken)
        keychain.removeValue(forKey: .accessToken)
        keychain.removeValue(forKey: .idToken)
        defaults.removeObject(forKey: CachedSessionKey.socialSignInSuccessful.rawValue)
        defaults.removeObject(forKey: CachedSessionKey.username.rawValue)
        defaults.removeObject(forKey: CachedSessionKey.phoneNumber.rawValue)

        // Kotlin also deregisters the push token here; the push phase adds
        // deregister_notification_token when Firebase Messaging lands.

        resetCachedCanisterData()
        sessionStore.resetSessionProperties()
        sessionStore.updateFirebaseLoginState(false)
        sessionStore.updateState(.initial)
    }
}
