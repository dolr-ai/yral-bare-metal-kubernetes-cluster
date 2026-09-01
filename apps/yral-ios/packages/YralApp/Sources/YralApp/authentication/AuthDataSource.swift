import Foundation

/// yral-auth HTTP data source — port of Kotlin `AuthDataSourceImpl`.
///
/// Endpoint table (all on `auth.yral.com` unless noted):
///   - POST `oauth/token` (form-urlencoded): anonymous (client_credentials),
///     authorization-code exchange, refresh-token grant.
///   - POST `api/phone_auth_login` (JSON), POST `api/verify_phone_auth` (JSON)
///   - POST `api/create_ai_account` (JSON)
///   - POST `metadata.yral.com/v2/update_session_as_registered` (JSON, Bearer)
///   - DELETE `offchain.yral.com/api/v1/user` (JSON, Bearer)
///
/// Kotlin-faithful details:
///   - Form bodies are RAW string interpolation (`grant_type=x&client_id=y`)
///     with NO URL-encoding — values are safe (UUIDs, base64url tokens).
///   - `redirect_uri` is sent unencoded (`com.yral.iosApp://oauth/callback`).
///   - `update_session_as_registered` is fire-and-forget (status NOT checked).
///   - `verify_phone_auth` returns a 2-string JSON ARRAY on 200 —
///     `[id_token_code, redirect_uri]` — not an object.
public struct AuthDataSource: Sendable {

    /// Default URLSession — no custom client needed (Kotlin's 30s timeout
    /// policy matches URLSession's own default; per-endpoint overrides set
    /// `request.timeoutInterval` inline where needed).
    private let session: URLSession

    /// iOS OAuth client id (registered with yral-auth; Android's differs).
    public static let clientID = "e1a6a7fb-8a1d-42dc-87b4-13ff94ecbe34"

    /// Redirect URI host/path constants (scheme comes from Info.plist).
    private static let redirectHost = "oauth"
    private static let redirectPath = "/callback"

    public init(session: URLSession = .shared) {
        self.session = session
    }

    /// Builds `com.yral.iosApp://oauth/callback` from the Info.plist scheme
    /// (`YRAL_REDIRECT_URI_SCHEME` — set by the app shell's Info.plist).
    public static func redirectURI(scheme: String) -> String {
        "\(scheme)://\(redirectHost)\(redirectPath)"
    }

    // MARK: - Token endpoint (all three grants)

    /// `client_credentials` grant → anonymous identity tokens.
    public func obtainAnonymousIdentity() async throws -> TokenResponse {
        let formData = [
            "grant_type=client_credentials",
            "client_id=\(Self.clientID)"
        ].joined(separator: "&")
        return try await postTokenEndpoint(formData: formData)
    }

    /// `authorization_code` exchange (social sign-in callback completion).
    public func authenticateToken(
        code: String,
        codeVerifier: String,
        redirectScheme: String
    ) async throws -> TokenResponse {
        let formData = [
            "grant_type=authorization_code",
            "client_id=\(Self.clientID)",
            "code=\(code)",
            "code_verifier=\(codeVerifier)",
            "redirect_uri=\(Self.redirectURI(scheme: redirectScheme))"
        ].joined(separator: "&")
        return try await postTokenEndpoint(formData: formData)
    }

    /// `refresh_token` grant.
    public func refreshToken(_ refreshToken: String) async throws -> TokenResponse {
        let formData = [
            "grant_type=refresh_token",
            "refresh_token=\(refreshToken)",
            "client_id=\(Self.clientID)"
        ].joined(separator: "&")
        return try await postTokenEndpoint(formData: formData)
    }

    /// Shared POST to `oauth/token` (form-urlencoded). Inline status check —
    /// non-2xx throws `NetworkError.http` (the Kotlin `expectSuccess =
    /// true` semantic).
    private func postTokenEndpoint(formData: String) async throws -> TokenResponse {
        var request = URLRequest(
            url: URL(string: "https://\(AppConfiguration.oauthBaseURL)/oauth/token")!)
        request.httpMethod = "POST"
        request.setValue("application/x-www-form-urlencoded", forHTTPHeaderField: "Content-Type")
        request.httpBody = Data(formData.utf8)
        let (data, response) = try await session.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse else {
            throw NetworkError.transport(underlying: "Non-HTTP response")
        }
        guard (200..<300).contains(httpResponse.statusCode) else {
            throw NetworkError.http(
                statusCode: httpResponse.statusCode,
                body: String(data: data, encoding: .utf8)
            )
        }
        return try TokenResponse.fromJSONBody(String(data: data, encoding: .utf8) ?? "")
    }

    // MARK: - Phone OTP

    /// Starts phone/WhatsApp OTP: the response carries its own PKCE state
    /// (`client_state` == code challenge — same as the social flow's `state`).
    public func phoneAuthLogin(
        phoneNumber: String,
        codeChallenge: String,
        redirectScheme: String
    ) async throws -> PhoneAuthLoginOutcome {
        let request = try phoneAuthBaseRequest(
            path: "api/phone_auth_login",
            phoneNumber: phoneNumber,
            codeChallenge: codeChallenge,
            redirectScheme: redirectScheme
        )
        // Status handled manually here (Kotlin: expectSuccess = false).
        let (data, response) = try await session.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse,
              httpResponse.statusCode == 200
        else {
            return .error(try AuthErrorPayload.fromJSONBody(
                String(data: data, encoding: .utf8) ?? ""
            ))
        }
        return .success
    }

    /// Verifies the OTP. On 200 the body is a 2-string JSON ARRAY:
    /// `[id_token_code, redirect_uri]` — the code then goes through the
    /// standard authorization-code exchange.
    public func verifyPhoneAuth(
        phoneNumber: String,
        code: String,
        clientState: String
    ) async throws -> PhoneAuthVerifyOutcome {
        var request = URLRequest(
            url: URL(string: "https://\(AppConfiguration.oauthBaseURL)/api/verify_phone_auth")!)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        let bodyObject: [String: [String: String]] = [
            "verify_request": [
                "phone_number": phoneNumber,
                "code": code,
                "client_state": clientState
            ]
        ]
        request.httpBody = try JSONSerialization.data(withJSONObject: bodyObject)
        let (data, response) = try await session.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse,
              httpResponse.statusCode == 200
        else {
            return .error(try AuthErrorPayload.fromJSONBody(
                String(data: data, encoding: .utf8) ?? ""
            ))
        }
        guard let array = try? JSONSerialization.jsonObject(with: data) as? [String],
            array.count == 2
        else {
            return .error(
                AuthErrorPayload(
                    error: "Missing all required keys",
                    errorDescription: String(data: data, encoding: .utf8) ?? ""
                ))
        }
        return .success(idTokenCode: array[0], redirectURI: array[1])
    }

    private func phoneAuthBaseRequest(
        path: String,
        phoneNumber: String,
        codeChallenge: String,
        redirectScheme: String
    ) throws -> URLRequest {
        var request = URLRequest(
            url: URL(string: "https://\(AppConfiguration.oauthBaseURL)/\(path)")!)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        let bodyObject: [String: Any] = [
            "auth_client_query": [
                "response_type": "code",
                "client_id": Self.clientID,
                "redirect_uri": Self.redirectURI(scheme: redirectScheme),
                "state": codeChallenge,
                "code_challenge": codeChallenge,
                "code_challenge_method": "S256",
                "login_hint": ""
            ],
            "phone_number": phoneNumber
        ]
        request.httpBody = try JSONSerialization.data(withJSONObject: bodyObject)
        return request
    }

    // MARK: - AI account creation (bot identity)

    /// Mints a delegated bot identity under the caller. No Bearer header —
    /// the Kotlin client sends the user id in the body only.
    public func createAiAccount(
        userID: String,
        idToken: String
    ) async throws -> String {
        var request = URLRequest(
            url: URL(string: "https://\(AppConfiguration.oauthBaseURL)/api/create_ai_account")!)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("Bearer \(idToken)", forHTTPHeaderField: "Authorization")
        let bodyObject: [String: String] = ["user_id": userID]
        request.httpBody = try JSONSerialization.data(withJSONObject: bodyObject)
        let (data, response) = try await session.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse,
            (200..<300).contains(httpResponse.statusCode)
        else {
            throw NetworkError.http(
                statusCode: (response as? HTTPURLResponse)?.statusCode ?? 0,
                body: String(data: data, encoding: .utf8)
            )
        }
        return try JSONDecoder().decode(CreateAiAccountResponse.self, from: data).aiAccountId
    }

    // MARK: - Registration side effects

    /// Fire-and-forget session registration on yral-metadata (status is NOT
    /// checked — Kotlin sets `expectSuccess = false` here; the server made
    /// this a no-op, kept for compat).
    public func updateSessionAsRegistered(
        idToken: String,
        canisterID: String,
        userPrincipal: String
    ) async throws {
        var request = URLRequest(
            url: URL(
                string:
                    "https://\(AppConfiguration.metadataBaseURL)/v2/update_session_as_registered"
            )!)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("Bearer \(idToken)", forHTTPHeaderField: "Authorization")
        let bodyObject: [String: String] = [
            "user_canister": canisterID,
            "user_principal": userPrincipal
        ]
        request.httpBody = try JSONSerialization.data(withJSONObject: bodyObject)
        // Fire-and-forget: the Kotlin client sets expectSuccess = false and
        // never inspects the result — the server made this endpoint a no-op.
        _ = try? await session.data(for: request)
    }

    /// Deletes the account via off-chain-agent.
    public func deleteAccount(idToken: String) async throws {
        var request = URLRequest(
            url: URL(string: "https://\(AppConfiguration.offChainBaseURL)/api/v1/user")!)
        request.httpMethod = "DELETE"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("Bearer \(idToken)", forHTTPHeaderField: "Authorization")
        // Kotlin sends {"dummy": ""} (encodeDefaults = true).
        request.httpBody = Data(#"{"dummy":""}"#.utf8)
        let (data, response) = try await session.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse,
            (200..<300).contains(httpResponse.statusCode)
        else {
            throw NetworkError.http(
                statusCode: (response as? HTTPURLResponse)?.statusCode ?? 0,
                body: String(data: data, encoding: .utf8)
            )
        }
    }
}

// MARK: - DTOs

/// `create_ai_account` response — `{"ai_account_id": "…"}`.
/// File-scope (not nested in the data source) per the nesting lint rule.
private struct CreateAiAccountResponse: Decodable {
    let aiAccountId: String
    private enum CodingKeys: String, CodingKey {
        case aiAccountId = "ai_account_id"
    }
}

/// `TokenResponseDto` — exact key names from the yral-auth token endpoint.
public struct TokenResponse: Equatable, Sendable {
    public let idToken: String
    public let accessToken: String
    public let expiresIn: Int64
    public let refreshToken: String
    public let tokenType: String

    /// JSON key mapping: `id_token`, `access_token`, `expires_in`,
    /// `refresh_token`, `token_type`.
    static func fromJSONBody(_ body: String) throws -> TokenResponse {
        let data = Data(body.utf8)
        guard let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            throw NetworkError.transport(underlying: "Token response is not JSON: \(body)")
        }
        func requireString(_ key: String) throws -> String {
            guard let value = object[key] as? String, !value.isEmpty else {
                throw NetworkError.transport(underlying: "Token response missing '\(key)'")
            }
            return value
        }
        guard let expiresIn = (object["expires_in"] as? NSNumber)?.int64ValueExact else {
            throw NetworkError.transport(underlying: "Token response missing 'expires_in'")
        }
        return TokenResponse(
            idToken: try requireString("id_token"),
            accessToken: try requireString("access_token"),
            expiresIn: expiresIn,
            refreshToken: try requireString("refresh_token"),
            tokenType: try requireString("token_type")
        )
    }
}

/// `{"error": "…", "error_description": "…"}` error payload shape.
public struct AuthErrorPayload: Equatable, Sendable {
    public let error: String
    public let errorDescription: String

    static func fromJSONBody(_ body: String) throws -> AuthErrorPayload {
        let data = Data(body.utf8)
        guard let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
            let error = object["error"] as? String
        else {
            return AuthErrorPayload(error: "unknown_error", errorDescription: body)
        }
        return AuthErrorPayload(
            error: error,
            errorDescription: object["error_description"] as? String ?? ""
        )
    }
}

/// `phone_auth_login` outcome (unit success or typed error).
public enum PhoneAuthLoginOutcome: Equatable, Sendable {
    case success
    case error(AuthErrorPayload)
}

/// `verify_phone_auth` outcome — success carries the auth code + redirect URI.
public enum PhoneAuthVerifyOutcome: Equatable, Sendable {
    case success(idTokenCode: String, redirectURI: String)
    case error(AuthErrorPayload)
}
