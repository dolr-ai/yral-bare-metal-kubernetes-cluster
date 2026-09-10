import Foundation

/// Cached-session persistence for `AuthClient` — Kotlin
/// `DefaultAuthClient`'s `getCachedSession`/`cacheSession`/
/// `resetCachedSessionData`/`saveTokens`/`postLogin`, kept as an
/// extension of the same class (not a layer). The Kotlin original's
/// canister fields (ICP legacy) are removed — userSubject is the
/// identity; the canister-era updateYralSession call is dropped (its
/// server endpoint is a documented no-op).
extension AuthClient {

    /// Rebuilds the cached session — Kotlin `getCachedSession` verbatim
    /// (single-slot cache: PROFILE_PIC/USERNAME are trusted only when
    /// the stored USER_PRINCIPAL == LAST_ACTIVE_PRINCIPAL — the
    /// persisted rawValue strings keep the legacy names for device
    /// continuity; the Swift cases read as `subject`).
    func cachedSession() -> Session? {
        let mainSubject = keychain.string(forKey: .mainSubject)
        let lastActiveSubject = keychain.string(forKey: .lastActiveSubject)

        let preferredSubject = defaults.string(forKey: CachedSessionKey.userSubject.rawValue)
        let usePreferred =
            lastActiveSubject != nil
            && preferredSubject == lastActiveSubject

        let userSubject =
            usePreferred
            ? preferredSubject
            : mainSubject ?? preferredSubject
        let profilePic = cachedProfilePic(
            userSubject: userSubject, preferredSubject: preferredSubject
        )
        let username = cachedUsername(
            userSubject: userSubject, preferredSubject: preferredSubject
        )
        let resolvedIsBotAccount =
            mainSubject.map { main in
                userSubject != nil && userSubject != main
            } ?? false

        guard let userSubject, let profilePic else { return nil }
        return Session(
            userSubject: userSubject,
            profilePic: profilePic,
            // Cached username when present; deterministic pseudonym
            // fallback otherwise (never the raw identifier as a name).
            username: UsernameGenerator.resolveUsername(
                preferred: username, subject: userSubject
            ),
            isAIAccount: resolvedIsBotAccount
        )
    }

    /// Kotlin `getCachedProfilePic`: cached pic trusted only when the
    /// preferred subject matches; else derived from the subject.
    private func cachedProfilePic(
        userSubject: String?,
        preferredSubject: String?
    ) -> String? {
        let cached = defaults.string(forKey: CachedSessionKey.profilePic.rawValue)
        guard preferredSubject == userSubject else {
            return userSubject.map { ProfilePicture.url(fromSubject: $0) }
        }
        return cached ?? userSubject.map { ProfilePicture.url(fromSubject: $0) }
    }

    /// Kotlin `getCachedUsername`: cached username trusted only when the
    /// preferred subject matches.
    private func cachedUsername(
        userSubject: String?,
        preferredSubject: String?
    ) -> String? {
        let cached = defaults.string(forKey: CachedSessionKey.username.rawValue)
        guard preferredSubject == userSubject else { return nil }
        return cached
    }

    /// Persists the session fields — Kotlin `cacheSession` (writes the
    /// keychain's LAST_ACTIVE_PRINCIPAL and the defaults' USER_PRINCIPAL
    /// only for non-AI account sessions; an AI account session never
    /// overwrites the main subject — the persisted rawValue strings
    /// keep the legacy names for device continuity).
    func cacheSession(
        userSubject: String,
        profilePic: String,
        username: String?,
        isAIAccount: Bool
    ) {
        defaults.set(userSubject, forKey: CachedSessionKey.userSubject.rawValue)
        defaults.set(profilePic, forKey: CachedSessionKey.profilePic.rawValue)
        // The stored username when present; deterministic pseudonym
        // fallback otherwise.
        let resolvedUsername = UsernameGenerator.resolveUsername(
            preferred: username, subject: userSubject
        )
        if let resolvedUsername {
            defaults.set(resolvedUsername, forKey: CachedSessionKey.username.rawValue)
        } else {
            defaults.removeObject(forKey: CachedSessionKey.username.rawValue)
        }
        if !isAIAccount {
            let storedMainSubject = keychain.string(forKey: .mainSubject)
            if storedMainSubject == nil || storedMainSubject == userSubject {
                keychain.setString(userSubject, forKey: .mainSubject)
                keychain.setString(userSubject, forKey: .lastActiveSubject)
            }
        }
    }

    /// Kotlin `resetCachedCanisterData` — logout clears the cached session
    /// fields and subject prefs. (Kotlin named it for the ICP canister
    /// cache; renamed — no canisters exist in the JWT-only world.)
    func resetCachedSessionData() {
        for key in [
            CachedSessionKey.userSubject,
            CachedSessionKey.profilePic,
            CachedSessionKey.username
        ] {
            defaults.removeObject(forKey: key.rawValue)
        }
        keychain.removeValue(forKey: .mainSubject)
        keychain.removeValue(forKey: .lastActiveSubject)
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
        let parsedClaims = try? JWTParser.parsePayload(of: idToken)
        if persistBotIdentities, let aiAccountIds = parsedClaims?.aiAccountIds {
            AIIdentitiesStore.mergeFromTokenAIAccountIds(aiAccountIds, defaults: defaults)
        }
    }

    /// Kotlin `postLogin` placeholder — the push phase wires this to
    /// Firebase Messaging. (Kotlin's `updateYralSession` registration call
    /// is dropped: its server endpoint is a documented no-op awaiting
    /// deletion — see apps/yral-metadata server/src/session.rs.)
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

    /// Delete the ACTIVE account — ONE transactional SpacetimeDB reducer
    /// (`delete_user_info`) cascading profiles, bots (where applicable),
    /// follows (with counter fixes), notification tokens, posts, and
    /// auth_kv identity mappings. The off-chain-agent's DELETE
    /// /api/v1/user is decommissioned (it was a stub: logged + returned
    /// fake success, deleted nothing).
    ///
    /// Semantics by WHAT is active (the reducer's ownership rule — self
    /// or owner-of-bot — enforces both server-side):
    ///   - AI account active → deletes THAT bot + its data. The UI then
    ///     switches back to the main account (`switchAfterDelete`).
    ///   - Main account active → deletes the main + ALL bots + all data;
    ///     the UI logs out.
    ///
    /// CRITICAL — the target is the ACTIVE SESSION's subject, NOT the
    /// token's sub: bot sessions carry the PARENT's tokens, so the sub
    /// is the main subject — deleting "the token's subject" while a
    /// bot is active would cascade the whole main account (observed in
    /// prod: deleting pure-calm-moose removed every bot).
    public func deleteAccount() async throws {
        guard let idToken else {
            throw AuthError.oauthFailed(errorDescription: "Not signed in")
        }
        guard let activeSubject = sessionStore.userSubject else {
            throw AuthError.oauthFailed(errorDescription: "No active account")
        }
        try await spacetimeDataSource.deleteUserInfo(
            subjectToDelete: activeSubject
        )
        if sessionStore.isAIAccount == true {
            // Deleted a BOT — switch back to the main account rather
            // than logging out (the user is still signed in as main).
            guard let mainSubject = keychain.string(forKey: .mainSubject) else {
                await logoutInternal()
                return
            }
            switchToAccount(subject: mainSubject)
        } else {
            // Deleted the MAIN account (cascades all bots) — full logout.
            await logoutInternal()
        }
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

        resetCachedSessionData()
        sessionStore.resetSessionProperties()
        sessionStore.updateFirebaseLoginState(false)
        sessionStore.updateState(.initial)
    }
}
