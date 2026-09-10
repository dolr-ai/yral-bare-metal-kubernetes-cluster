import Foundation

/// Auth session machine — port of Kotlin `DefaultAuthClient`.
///
/// Responsibilities (faithful to the Kotlin contract):
///   - `initialize()`: cold-start decision tree (restore cached AI account session /
///     validate ID token / refresh / anonymous identity).
///   - `logout()`: clear tokens + cached session + subject prefs, reset, reset
///     session properties, return to `.initial`.
///   - Token pipeline: `handleToken` → claims-based session or refresh.
///
/// Social/phone sign-in flows live in `AuthClient+SignIn.swift`;
/// cached-session persistence in `AuthClient+Persistence.swift` —
/// the Kotlin god class (917 lines) split across files by concern for the
/// 400-line lint limit. They are the SAME class (extensions), not layers.
///
/// Differences from Kotlin, by design:
///   - No analytics/telemetry dependency (Firebase Analytics wiring lands
///     with the analytics phase; call sites are documented inline).
///   - Token storage is the Keychain (not NSUserDefaults) via
///     `KeychainStore` — see its header comment.
///   - AI account-identity persistence (`ext_ai_account_ids` merge) lands with
///     the account-switcher phase that consumes it.
///
/// Storage layout (Kotlin `PrefKeys` split by secrecy):
///   - Keychain: ID_TOKEN / ACCESS_TOKEN / REFRESH_TOKEN,
///     LAST_ACTIVE_PRINCIPAL, MAIN_PRINCIPAL.
///   - UserDefaults (display data, not secrets): CANISTER_ID,
///     USER_PRINCIPAL, PROFILE_PIC, USERNAME,
///     IS_CREATED_FROM_SERVICE_CANISTER, PHONE_NUMBER,
///     SOCIAL_SIGN_IN_SUCCESSFUL.
@MainActor @Observable
public final class AuthClient {

    /// Auth data source (yral-auth + metadata + off-chain endpoints).
    let authDataSource: AuthDataSource

    /// SpacetimeDB data plane — used to hydrate AI-identity avatar URLs
    /// (bots created before the URL was persisted locally have no
    /// hostedAvatarURL; their real picture lives in the profile table).
    /// Injectable for tests; defaults to the real client reading the
    /// current id token.
    let spacetimeDataSource: SpacetimeDBRemoteDataSource

    /// Rishi-agent data plane — the bots' REAL names live in the creator
    /// listing (`GET /api/v1/creator/influencers`), not in SpacetimeDB or
    /// local storage. Injectable for tests.
    let influencerDataSource: AIInfluencerDataSource?

    /// Redirect scheme (from Info.plist via the app shell).
    let redirectScheme: String

    /// Keychain-backed token cache.
    let keychain: KeychainStore

    /// Cached session display fields (non-secret) — Kotlin's Preferences.
    let defaults: UserDefaults

    /// Session state (the app's observable session).
    let sessionStore: SessionStore

    /// PKCE verifier of the in-flight flow — needed at code exchange
    /// (Kotlin holds it on `AuthRepositoryImpl`).
    var pendingCodeVerifier: String?

    /// Code challenge of the in-flight flow; doubles as the anti-CSRF
    /// `state` (social) / `client_state` (phone).
    var currentCodeChallenge: String?

    /// Provider of the in-flight social flow (callback handling + analytics).
    var currentProvider: SocialProvider?

    /// Most recent token-expiry logout cause (nil after a user logout) —
    /// test hook mirroring Kotlin's telemetry cause parameter. Internal:
    /// tests read it via @testable; the persistence extension writes it.
    internal var lastLogoutCause: AuthExpiryCause?

    /// Session-storage keys in UserDefaults for the cached session fields.
    /// The rawValue strings keep the legacy PRINCIPAL naming — they are
    /// the persisted keys on existing installs (written by the Kotlin
    /// app's PrefKeys); only the Swift case names read as `subject`.
    enum CachedSessionKey: String {
        case canisterID = "CANISTER_ID"
        case userSubject = "USER_PRINCIPAL"
        case profilePic = "PROFILE_PIC"
        case username = "USERNAME"
        case isCreatedFromServiceCanister = "IS_CREATED_FROM_SERVICE_CANISTER"
        case phoneNumber = "PHONE_NUMBER"
        case socialSignInSuccessful = "SOCIAL_SIGN_IN_SUCCESSFUL"
    }

    public init(
        authDataSource: AuthDataSource,
        redirectScheme: String,
        keychain: KeychainStore = KeychainStore(),
        defaults: UserDefaults = .standard,
        spacetimeDataSource: SpacetimeDBRemoteDataSource? = nil,
        influencerDataSource: AIInfluencerDataSource? = nil,
        sessionStore: SessionStore
    ) {
        self.authDataSource = authDataSource
        self.redirectScheme = redirectScheme
        self.keychain = keychain
        self.defaults = defaults
        // Default: the real client reading the current id token. Built
        // BEFORE the property is assigned — the closure captures self.
        self.spacetimeDataSource = spacetimeDataSource
            ?? {
                let keychain = keychain
                return SpacetimeDBRemoteDataSource(
                    idTokenProvider: { keychain.string(forKey: .idToken) }
                )
            }()
        // Nil default: the creator-name overlay degrades to the fallback
        // names; tests inject a stub-backed source when needed.
        self.influencerDataSource = influencerDataSource
            ?? AIInfluencerDataSource()
        self.sessionStore = sessionStore
    }

    // MARK: - Cold start (initialize + refreshAuthIfNeeded)

    /// Restores or obtains a session. Decision tree, Kotlin-faithful:
    ///
    ///  1. lastActive == cached AI account (≠ main) → restore immediately, then
    ///     validate/refresh AI account tokens.
    ///  2. ID token present → validate; if expired use the refresh token
    ///     (valid refresh → refresh; else logout with cause).
    ///  3. Nothing cached → anonymous identity.
    public func initialize() async {
        sessionStore.updateState(.loading)
        await refreshAuthIfNeeded()
    }

    private func refreshAuthIfNeeded() async {
        let lastActiveSubject = keychain.string(forKey: .lastActiveSubject)
        let mainSubject = keychain.string(forKey: .mainSubject)

        // 1. AI account (or non-main) last-active → restore from cache directly.
        if let lastActiveSubject, lastActiveSubject != mainSubject {
            if let cached = cachedSession(),
               cached.userSubject == lastActiveSubject {
                sessionStore.updateState(.signedIn(cached))
                await refreshBotColdStartTokensIfNeeded()
                return
            }
        }

        // 2. Main-account ID token → validate/refresh through handleToken.
        if let idToken = keychain.string(forKey: .idToken) {
            let shouldPersistTokenState =
                lastActiveSubject == nil || lastActiveSubject == mainSubject
            await handleToken(
                idToken: idToken,
                accessToken: "",
                refreshToken: "",
                persistTokenState: shouldPersistTokenState,
                persistBotIdentities: false
            )
            return
        }

        // 3. Fresh install → anonymous identity.
        await obtainAnonymousIdentity()
    }

    /// AI account cold-start token hygiene — Kotlin
    /// `refreshBotColdStartTokensIfNeeded` verbatim.
    private func refreshBotColdStartTokensIfNeeded() async {
        let now = currentEpochSeconds
        let idToken = keychain.string(forKey: .idToken)
        let shouldRefresh = idToken.map { !isTokenValid($0, now: now) } ?? true
        guard shouldRefresh else { return }

        guard let refreshToken = keychain.string(forKey: .refreshToken),
              !refreshToken.isEmpty
        else {
            await trackAndLogoutForTokenExpiry(cause: .refreshTokenMissing)
            return
        }
        guard isTokenValid(refreshToken, now: now) else {
            await trackAndLogoutForTokenExpiry(cause: .refreshTokenExpiredOrInvalid)
            return
        }
        await refreshBotColdStartTokens(refreshToken: refreshToken)
    }

    func isTokenValid(_ token: String, now: Int64) -> Bool {
        guard let claims = try? JWTParser.parsePayload(of: token) else { return false }
        return claims.isValid(currentTimeInEpochSeconds: now)
    }

    private func refreshBotColdStartTokens(refreshToken: String) async {
        do {
            let tokenResponse = try await authDataSource.refreshToken(refreshToken)
            saveTokens(
                idToken: tokenResponse.idToken,
                refreshToken: tokenResponse.refreshToken,
                accessToken: tokenResponse.accessToken,
                persistBotIdentities: false
            )
            if let cached = cachedSession() {
                await updateYralSession(cached)
            }
        } catch {
            await trackAndLogoutForTokenExpiry(cause: .refreshAccessTokenFailed)
        }
    }

    private func obtainAnonymousIdentity() async {
        do {
            let tokenResponse = try await authDataSource.obtainAnonymousIdentity()
            await handleToken(
                idToken: tokenResponse.idToken,
                accessToken: tokenResponse.accessToken,
                refreshToken: tokenResponse.refreshToken,
                resetCanister: true
            )
            sessionStore.updateSocialSignInStatus(false)
        } catch {
            // Kotlin rethrows YralAuthException; without a session there is
            // nothing to restore — back to .initial for a retry on next
            // launch (surfaced to the UI by the state machine).
            sessionStore.updateState(.initial)
        }
    }

    // MARK: - Token handling pipeline

    /// Kotlin `handleToken` — the single entry for every token arrival
    /// (cold start, refresh, social exchange, phone exchange). Branches:
    /// valid → claims-based session; expired → refresh-token path.
    ///
    /// A malformed persisted ID token is treated like an expired one (and
    /// so recovers via the refresh path). Kotlin instead lets the parse
    /// error propagate out of `initialize()` — a cold-start crash; this is
    /// an intentional improvement over the Kotlin behavior.
    func handleToken(
        idToken: String,
        accessToken: String,
        refreshToken: String,
        resetCanister: Bool = false,
        persistTokenState: Bool = true,
        persistBotIdentities: Bool = true
    ) async {
        if persistTokenState {
            saveTokens(
                idToken: idToken,
                refreshToken: refreshToken,
                accessToken: accessToken,
                persistBotIdentities: persistBotIdentities
            )
        }

        guard let tokenClaims = try? JWTParser.parsePayload(of: idToken),
              tokenClaims.isValid(currentTimeInEpochSeconds: currentEpochSeconds)
        else {
            guard let refreshToken = keychain.string(forKey: .refreshToken),
                  !refreshToken.isEmpty
            else {
                await trackAndLogoutForTokenExpiry(cause: .refreshTokenMissing)
                return
            }
            if isTokenValid(refreshToken, now: currentEpochSeconds) {
                await refreshAccessToken(refreshToken: refreshToken)
            } else {
                await trackAndLogoutForTokenExpiry(cause: .refreshTokenExpiredOrInvalid)
            }
            return
        }

        if resetCanister { resetCachedCanisterData() }
        handleTokenClaims(tokenClaims)
        if let email = tokenClaims.email {
            sessionStore.updateLoggedInUserEmail(email)
        }
    }

    /// Kotlin `handleTokenClaims` — builds the session from claims. The
    /// foreign-subject guard: if the main account is active and this
    /// token's subject differs, the cached MAIN session is restored
    /// instead (a stale token for another account must not hijack the
    /// active session).
    private func handleTokenClaims(_ tokenClaims: TokenClaims) {
        let storedMainSubject = keychain.string(forKey: .mainSubject)
        let lastActiveSubject = keychain.string(forKey: .lastActiveSubject)
        let subject = tokenClaims.subject

        if let storedMainSubject,
           lastActiveSubject == storedMainSubject,
           subject != storedMainSubject {
            if let cachedMain = cachedSession(),
               cachedMain.userSubject == storedMainSubject {
                sessionStore.updateState(.signedIn(cachedMain))
                sessionStore.updateFirebaseLoginState(true)
            }
            return
        }

        let profilePic = ProfilePicture.url(fromSubject: subject)
        cacheSession(
            canisterID: subject,
            userSubject: subject,
            profilePic: profilePic,
            username: nil,
            isAIAccount: false
        )

        let session = Session(
            canisterID: subject,
            userSubject: subject,
            profilePic: profilePic,
            // Deterministic pseudonym for a fresh main account (no server
            // username yet) — the fallback tier, per the display rules.
            username: UsernameGenerator.resolveUsername(
                preferred: nil, subject: subject
            ),
            isCreatedFromServiceCanister: true,
            isAIAccount: false
        )
        sessionStore.updateCoinBalance(0)
        sessionStore.updateState(.signedIn(session))
        sessionStore.updateFirebaseLoginState(true)
        postLogin()
    }

    // MARK: - Manual token refresh (Kotlin refreshTokens)

    /// Kotlin `refreshAccessToken` — the main-account refresh path (distinct
    /// from `refreshBotColdStartTokens`, which only persists tokens without
    /// re-running the claims pipeline). Refresh failure logs the caller
    /// out with `REFRESH_ACCESS_TOKEN_FAILED`.
    private func refreshAccessToken(refreshToken: String) async {
        do {
            let tokenResponse = try await authDataSource.refreshToken(refreshToken)
            await handleToken(
                idToken: tokenResponse.idToken,
                accessToken: tokenResponse.accessToken,
                refreshToken: tokenResponse.refreshToken
            )
            if let cached = cachedSession() {
                await updateYralSession(cached)
            }
        } catch {
            await trackAndLogoutForTokenExpiry(cause: .refreshAccessTokenFailed)
            CrashReporter.record(error, context: "cold-start-token-refresh")
        }
    }

    /// Kotlin `refreshTokens` — manual refresh used by account-management
    /// flows (AI account deletion). Missing refresh token → no-op; refresh failure
    /// → logged-only no-op (Kotlin parity: it never throws).
    public func refreshTokens() async {
        guard let refreshToken = keychain.string(forKey: .refreshToken),
              !refreshToken.isEmpty
        else { return }
        do {
            let tokenResponse = try await authDataSource.refreshToken(refreshToken)
            saveTokens(
                idToken: tokenResponse.idToken,
                refreshToken: tokenResponse.refreshToken,
                accessToken: tokenResponse.accessToken
            )
        } catch {
            // Kotlin logs and returns; the failure now reaches
            // Crashlytics as a non-fatal.
            CrashReporter.record(error, context: "manual-token-refresh")
        }
    }
}

// MARK: - Supporting types

/// Social providers — port of Kotlin `SocialProvider`.
public enum SocialProvider: String, Sendable {
    case google
    case apple
    case phone

    /// Wire value of the OAuth `provider` query parameter.
    public var wireValue: String { rawValue }

    /// Kotlin `responseMode()`: Apple uses form_post; Google uses query.
    public var responseMode: String {
        self == .apple ? "form_post" : "query"
    }

    /// Kotlin `authScope()`: Apple gets name+email; others get openid.
    public var authScope: String {
        self == .apple ? "name email" : "openid"
    }
}

/// Token-expiry logout causes — Kotlin `AuthSessionCause` (the analytics
/// event lands with the analytics phase; tests assert on the cause).
public enum AuthExpiryCause: String, Sendable {
    case refreshTokenMissing = "REFRESH_TOKEN_MISSING"
    case refreshTokenExpiredOrInvalid = "REFRESH_TOKEN_EXPIRED_OR_INVALID"
    case refreshAccessTokenFailed = "REFRESH_ACCESS_TOKEN_FAILED"
}
