import Foundation

/// Social + phone sign-in flows for `AuthClient` — Kotlin
/// `DefaultAuthClient`'s `signInWithSocial`/`handleOAuthCallback`/
/// `authenticate`/`phoneAuthLogin`/`verifyPhoneAuth`, kept in an extension
/// of the same class (not a layer). The browser-auth session
/// (ASWebAuthenticationSession) is driven by the UI layer, which calls
/// `socialAuthorizationURL` then `handleOAuthCallbackResult`.
extension AuthClient {

    /// Builds the OAuth authorization URL for the provider — Kotlin
    /// `AuthRepositoryImpl.getOAuthUrl` (state == code challenge). The
    /// verifier stays on the client for the code exchange.
    public func socialAuthorizationURL(provider: SocialProvider) throws -> URL {
        let codeVerifier = try PKCE.generateCodeVerifier()
        let codeChallenge = PKCE.generateCodeChallenge(codeVerifier: codeVerifier)
        pendingCodeVerifier = codeVerifier
        currentCodeChallenge = codeChallenge
        currentProvider = provider

        var components = URLComponents()
        components.scheme = "https"
        components.host = AppConfiguration.oauthBaseURL
        components.path = "/oauth/auth"
        components.queryItems = [
            URLQueryItem(name: "provider", value: provider.wireValue),
            URLQueryItem(name: "client_id", value: AuthDataSource.clientID),
            URLQueryItem(name: "response_type", value: "code"),
            URLQueryItem(name: "response_mode", value: provider.responseMode),
            URLQueryItem(
                name: "redirect_uri",
                value: AuthDataSource.redirectURI(scheme: redirectScheme)
            ),
            URLQueryItem(name: "scope", value: provider.authScope),
            URLQueryItem(name: "code_challenge", value: codeChallenge),
            URLQueryItem(name: "code_challenge_method", value: "S256"),
            URLQueryItem(name: "login_hint", value: ""),
            URLQueryItem(name: "state", value: codeChallenge)
        ]
        guard let url = components.url else {
            throw AuthError.oauthFailed(errorDescription: "OAuth URL construction failed")
        }
        return url
    }

    /// Handles the redirect result from the browser auth session — Kotlin
    /// `handleOAuthCallback`. `state` mismatch → error (possible CSRF).
    public func handleOAuthCallbackResult(_ result: OAuthResult) async throws {
        switch result {
        case let .success(code, state):
            guard state == currentCodeChallenge else {
                currentProvider = nil
                throw AuthError.stateMismatch
            }
            sessionStore.updateState(.loading)
            let previousPrincipal = sessionStore.userPrincipal
            try await authenticate(code: code, currentUserPrincipal: previousPrincipal)
        case let .failure(error, errorDescription):
            currentProvider = nil
            throw AuthError.oauthFailed(
                errorDescription: errorDescription.map { "\(error): \($0)" } ?? error
            )
        case .cancelled:
            currentProvider = nil
            throw AuthError.oauthFailed(
                errorDescription: "OAuth authentication was cancelled by user"
            )
        case .timedOut:
            currentProvider = nil
            throw AuthError.oauthFailed(
                errorDescription: "OAuth authentication timed out - please try again"
            )
        }
    }

    /// Code → token exchange + session build — Kotlin `authenticate`.
    func authenticate(code: String, currentUserPrincipal: String?) async throws {
        guard let codeVerifier = pendingCodeVerifier else {
            throw AuthError.oauthFailed(errorDescription: "No in-flight OAuth flow")
        }
        pendingCodeVerifier = nil
        let tokenResponse = try await authDataSource.authenticateToken(
            code: code,
            codeVerifier: codeVerifier,
            redirectScheme: redirectScheme
        )
        defaults.set(true, forKey: CachedSessionKey.socialSignInSuccessful.rawValue)
        sessionStore.updateSocialSignInStatus(true)
        if let phone = defaults.string(forKey: CachedSessionKey.phoneNumber.rawValue) {
            sessionStore.updatePhoneNumber(phone)
        }
        let provider = currentProvider ?? .google
        currentProvider = nil
        currentCodeChallenge = nil

        await handleToken(
            idToken: tokenResponse.idToken,
            accessToken: tokenResponse.accessToken,
            refreshToken: tokenResponse.refreshToken,
            resetCanister: true
        )
        if let cached = cachedSession() {
            await updateYralSession(cached)
        }
        // Analytics events (onAuthSuccess with new-user flag; provider)
        // land with the analytics phase.
        _ = provider
        _ = currentUserPrincipal
    }

    // MARK: - Phone OTP (Kotlin phoneAuthLogin/verifyPhoneAuth)

    /// Starts phone OTP. Stores the challenge as `client_state` — the
    /// verify call must echo it (Kotlin contract).
    public func phoneAuthLogin(phoneNumber: String) async throws -> String {
        let codeVerifier = try PKCE.generateCodeVerifier()
        let codeChallenge = PKCE.generateCodeChallenge(codeVerifier: codeVerifier)
        pendingCodeVerifier = codeVerifier
        currentCodeChallenge = codeChallenge

        switch try await authDataSource.phoneAuthLogin(
            phoneNumber: phoneNumber,
            codeChallenge: codeChallenge,
            redirectScheme: redirectScheme
        ) {
        case .success:
            return codeChallenge
        case let .error(errorPayload):
            throw AuthError.oauthFailed(
                errorDescription: "\(errorPayload.error) - \(errorPayload.errorDescription)"
            )
        }
    }

    /// Verifies the OTP and completes sign-in through the code exchange —
    /// Kotlin `verifyPhoneAuth`.
    public func verifyPhoneAuth(phoneNumber: String, code: String) async throws {
        guard let clientState = currentCodeChallenge else {
            throw AuthError.oauthFailed(
                errorDescription: "Phone auth verification failed - no state found"
            )
        }
        switch try await authDataSource.verifyPhoneAuth(
            phoneNumber: phoneNumber,
            code: code,
            clientState: clientState
        ) {
        case let .success(idTokenCode, _):
            defaults.set(phoneNumber, forKey: CachedSessionKey.phoneNumber.rawValue)
            sessionStore.updatePhoneNumber(phoneNumber)
            currentProvider = .phone
            guard let userPrincipal = sessionStore.userPrincipal else {
                throw AuthError.oauthFailed(
                    errorDescription: "Phone auth verification failed - user principal not found"
                )
            }
            try await authenticate(code: idTokenCode, currentUserPrincipal: userPrincipal)
        case let .error(errorPayload):
            throw AuthError.oauthFailed(
                errorDescription:
                    "Phone auth verification failed - \(errorPayload.error) - \(errorPayload.errorDescription)"
            )
        }
    }
}
