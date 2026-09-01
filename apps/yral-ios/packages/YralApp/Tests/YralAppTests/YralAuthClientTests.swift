import Testing
import Foundation
@testable import YralApp

/// Routes every request through a per-test handler. FILE SCOPE (not
/// nested in the @MainActor suite): URLProtocol.startLoading runs on
/// URLSession's queue, and a nested class would inherit MainActor
/// isolation under Swift 6 — trapping on executor mismatch (seen as
/// `dispatch_assert_queue` SIGTRAPs). Declaring it here keeps the
/// protocol nonisolated; the suite's `.serialized` run order makes the
/// static handler safe (one test installs its own before running).
final class RecordingURLProtocol: URLProtocol, @unchecked Sendable {
    nonisolated(unsafe) static var handler:
        (@Sendable (URLRequest) throws -> (HTTPURLResponse, Data))?

    static override func canInit(with request: URLRequest) -> Bool { true }
    static override func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        guard let handler = Self.handler else {
            client?.urlProtocol(
                self, didFailWithError: URLError(.unsupportedURL)
            )
            return
        }
        do {
            let (response, data) = try handler(request)
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: data)
            client?.urlProtocolDidFinishLoading(self)
        } catch {
            client?.urlProtocol(self, didFailWithError: error)
        }
    }

    override func stopLoading() {}
}

/// Refresh-call recorder — reference-boxed so the @Sendable URLProtocol
/// handler (running off the MainActor) can mutate it. File scope for the
/// same isolation reason as `RecordingURLProtocol`.
final class RefreshRecorder: @unchecked Sendable {
    private let lock = NSLock()
    private var storedRefreshCalls = 0
    private var storedLastRefreshToken: String?

    var refreshCalls: Int {
        lock.lock(); defer { lock.unlock() }
        return storedRefreshCalls
    }

    var lastRefreshToken: String? {
        lock.lock(); defer { lock.unlock() }
        return storedLastRefreshToken
    }

    func recordRefresh(refreshToken: String) {
        lock.lock(); defer { lock.unlock() }
        storedRefreshCalls += 1
        storedLastRefreshToken = refreshToken
    }
}

/// Total-call counter for the "must NOT refresh" contracts — same
/// reference-boxed, locked shape as `RefreshRecorder`.
final class RefreshCounter: @unchecked Sendable {
    private let lock = NSLock()
    private var storedCalls = 0

    var calls: Int {
        lock.lock(); defer { lock.unlock() }
        return storedCalls
    }

    func recordCall() {
        lock.lock(); defer { lock.unlock() }
        storedCalls += 1
    }
}

/// Cold-start + token-expiry contracts for `YralAuthClient` — ports of the
/// three verified `DefaultAuthClientTest.kt` cases. HTTP is stubbed via
/// `URLProtocol` (Apple-canonical seam) driving the real data source; the
/// Keychain/UserDefaults use isolated per-test instances.
///
/// `.serialized`: the shared static URLProtocol handler is per-test state
/// (each test installs its own); parallel execution would race on it.
@Suite(.serialized)
@MainActor
struct YralAuthClientTests {

    /// JWT fixture builder — real JWTs so `YralJWTParser` exercises its
    /// actual decode path (the Kotlin test faked the parser; here the
    /// parser is the code under test too).
    static func makeJWT(claims: [String: Any]) -> String {
        func base64URL(_ data: Data) -> String {
            data.base64EncodedString()
                .replacingOccurrences(of: "+", with: "-")
                .replacingOccurrences(of: "/", with: "_")
                .replacingOccurrences(of: "=", with: "")
        }
        let header = #"{"alg":"ES256","typ":"JWT"}"#
        let payloadData = try? JSONSerialization.data(withJSONObject: claims)
        let payload = payloadData.flatMap { String(data: $0, encoding: .utf8) } ?? "{}"
        return [header, payload, "signature"]
            .map { base64URL(Data($0.utf8)) }
            .joined(separator: ".")
    }

    // MARK: - Fixtures

    static let mainPrincipal = "main-principal"
    static let botPrincipal = "bot-principal"
    static let now = Int64(Date.now.timeIntervalSince1970)

    /// Kotlin `storeCachedBotSession` — cached bot session + tokens.
    @discardableResult
    func storeCachedBotSession(
        keychain: YralKeychainStore,
        defaults: UserDefaults,
        idToken: String,
        refreshToken: String
    ) -> (main: String, bot: String) {
        keychain.setString(Self.mainPrincipal, forKey: .mainPrincipal)
        keychain.setString(Self.botPrincipal, forKey: .lastActivePrincipal)
        defaults.set("bot-canister", forKey: "CANISTER_ID")
        defaults.set(Self.botPrincipal, forKey: "USER_PRINCIPAL")
        defaults.set("https://example.com/bot.png", forKey: "PROFILE_PIC")
        defaults.set("bot-user", forKey: "USERNAME")
        defaults.set(true, forKey: "IS_CREATED_FROM_SERVICE_CANISTER")
        keychain.setString(idToken, forKey: .idToken)
        keychain.setString(refreshToken, forKey: .refreshToken)
        return (Self.mainPrincipal, Self.botPrincipal)
    }

    func makeClient(
        keychain: YralKeychainStore,
        defaults: UserDefaults
    ) -> (client: YralAuthClient, sessionStore: YralSessionStore) {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [RecordingURLProtocol.self]
        let dataSource = YralAuthDataSource(session: URLSession(configuration: configuration))
        let sessionStore = YralSessionStore()
        let client = YralAuthClient(
            authDataSource: dataSource,
            redirectScheme: "com.yral.iosApp",
            keychain: keychain,
            defaults: defaults,
            sessionStore: sessionStore
        )
        return (client, sessionStore)
    }

    func freshDefaults() -> UserDefaults {
        let name = "yral-auth-client-tests-\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: name)!
        defaults.removePersistentDomain(forName: name)
        return defaults
    }

    // MARK: - Contract 1: cached bot + valid ID → no refresh

    /// Failing handler: counts any /oauth/token POST then errors — used by
    /// the "must NOT refresh" contracts. @Sendable: it runs on
    /// URLSession's queue.
    private func makeFailingHandler(
        counter: RefreshCounter
    ) -> @Sendable (URLRequest) throws -> (HTTPURLResponse, Data) {
        return { request in
            if request.url?.path == "/oauth/token" { counter.recordCall() }
            throw URLError(.unsupportedURL)
        }
    }

    @Test("cached bot cold start with valid id token skips refresh")
    func cachedBotValidIDTokenSkipsRefresh() async throws {
        let keychain = YralKeychainStore(service: "yral-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = freshDefaults()

        let validIDToken = Self.makeJWT(claims: [
            "exp": Self.now + 3_600, "iat": Self.now - 60,
            "iss": "auth.yral.com", "sub": Self.botPrincipal
        ])
        storeCachedBotSession(
            keychain: keychain, defaults: defaults,
            idToken: validIDToken, refreshToken: "unused-refresh-token"
        )

        let counter = RefreshCounter()
        RecordingURLProtocol.handler = makeFailingHandler(counter: counter)

        let (client, sessionStore) = makeClient(keychain: keychain, defaults: defaults)
        await client.initialize()

        #expect(counter.calls == 0)
        #expect(sessionStore.userPrincipal == Self.botPrincipal)
        #expect(sessionStore.isBotAccount == true)
        #expect(keychain.string(forKey: .idToken) == validIDToken)
    }

    // MARK: - Contract 2: expired ID + valid refresh → exactly one refresh

    /// Refresh handler: counts refresh grants, echoes the refresh token,
    /// returns the refreshed token trio. @Sendable (URLSession queue).
    private func makeRefreshHandler(
        refreshedIDToken: String,
        recorder: RefreshRecorder
    ) -> @Sendable (URLRequest) throws -> (HTTPURLResponse, Data) {
        return { request in
            guard request.url?.path == "/oauth/token",
                  let body = request.httpBody ?? request.bodyStreamData,
                  let form = String(data: body, encoding: .utf8)
            else { throw URLError(.unsupportedURL) }
            if form.contains("grant_type=refresh_token") {
                recorder.recordRefresh(refreshToken: form
                    .split(separator: "&")
                    .compactMap {
                        $0.hasPrefix("refresh_token=")
                            ? String($0.dropFirst("refresh_token=".count)) : nil
                    }
                    .first ?? "")
            }
            let response = HTTPURLResponse(
                url: request.url!, statusCode: 200,
                httpVersion: nil, headerFields: nil
            )!
            let tokenJSON = [
                "id_token": refreshedIDToken,
                "access_token": "refreshed-access-token",
                "expires_in": 3_600,
                "refresh_token": "refreshed-refresh-token",
                "token_type": "Bearer"
            ]
            let responseBody = try JSONSerialization.data(withJSONObject: tokenJSON)
            return (response, responseBody)
        }
    }

    @Test("cached bot cold start with expired id token refreshes with valid refresh token")
    func cachedBotExpiredIDTokenRefreshes() async throws {
        let keychain = YralKeychainStore(service: "yral-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = freshDefaults()

        let expiredIDToken = Self.makeJWT(claims: [
            "exp": Self.now - 60, "iat": Self.now - 7_200,
            "iss": "auth.yral.com", "sub": Self.botPrincipal
        ])
        let validRefreshToken = Self.makeJWT(claims: [
            "exp": Self.now + 3_600, "iat": Self.now - 60,
            "iss": "auth.yral.com", "sub": Self.botPrincipal
        ])
        storeCachedBotSession(
            keychain: keychain, defaults: defaults,
            idToken: expiredIDToken, refreshToken: validRefreshToken
        )

        let refreshedIDToken = Self.makeJWT(claims: [
            "exp": Self.now + 3_600, "iat": Self.now,
            "iss": "auth.yral.com", "sub": Self.botPrincipal
        ])

        let recorder = RefreshRecorder()
        RecordingURLProtocol.handler = makeRefreshHandler(
            refreshedIDToken: refreshedIDToken,
            recorder: recorder
        )

        let (client, sessionStore) = makeClient(keychain: keychain, defaults: defaults)
        await client.initialize()

        #expect(recorder.refreshCalls == 1)
        #expect(recorder.lastRefreshToken == validRefreshToken)
        #expect(sessionStore.userPrincipal == Self.botPrincipal)
        #expect(sessionStore.isBotAccount == true)
        #expect(keychain.string(forKey: .idToken) == refreshedIDToken)
        #expect(keychain.string(forKey: .refreshToken) == "refreshed-refresh-token")
        #expect(keychain.string(forKey: .accessToken) == "refreshed-access-token")
    }

    // MARK: - Contract 3: expired refresh → logout, no refresh call

    @Test("cached bot cold start with expired refresh token logs out")
    func cachedBotExpiredRefreshTokenLogsOut() async throws {
        let keychain = YralKeychainStore(service: "yral-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = freshDefaults()

        let expiredIDToken = Self.makeJWT(claims: [
            "exp": Self.now - 60, "iat": Self.now - 7_200,
            "iss": "auth.yral.com", "sub": Self.botPrincipal
        ])
        let expiredRefreshToken = Self.makeJWT(claims: [
            "exp": Self.now - 60, "iat": Self.now - 7_200,
            "iss": "auth.yral.com", "sub": Self.botPrincipal
        ])
        storeCachedBotSession(
            keychain: keychain, defaults: defaults,
            idToken: expiredIDToken, refreshToken: expiredRefreshToken
        )

        let counter = RefreshCounter()
        RecordingURLProtocol.handler = makeFailingHandler(counter: counter)

        let (client, sessionStore) = makeClient(keychain: keychain, defaults: defaults)
        await client.initialize()

        #expect(counter.calls == 0)
        #expect(sessionStore.userPrincipal == nil)
        #expect(keychain.string(forKey: .idToken) == nil)
        #expect(keychain.string(forKey: .refreshToken) == nil)
        #expect(client.lastLogoutCause == .refreshTokenExpiredOrInvalid)
    }

    // MARK: - CSRF guard

    @Test("OAuth callback with mismatched state throws (CSRF guard)")
    func callbackStateMismatchThrows() async throws {
        let keychain = YralKeychainStore(service: "yral-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let (client, _) = makeClient(keychain: keychain, defaults: freshDefaults())

        _ = try client.socialAuthorizationURL(provider: .google)
        await #expect(throws: YralAuthError.stateMismatch) {
            try await client.handleOAuthCallbackResult(
                .success(code: "auth-code", state: "attacker-state")
            )
        }
    }

    // MARK: - Social authorization URL

    @Test("social authorization URL carries PKCE + provider + client id")
    func socialAuthorizationURL() throws {
        let keychain = YralKeychainStore(service: "yral-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let (client, _) = makeClient(keychain: keychain, defaults: freshDefaults())

        let url = try client.socialAuthorizationURL(provider: .apple)
        let components = URLComponents(url: url, resolvingAgainstBaseURL: false)!
        #expect(components.scheme == "https")
        #expect(components.host == "auth.yral.com")
        #expect(components.path == "/oauth/auth")

        func query(_ name: String) -> String? {
            components.queryItems?.first(where: { $0.name == name })?.value
        }
        #expect(query("provider") == "apple")
        #expect(query("client_id") == YralAuthDataSource.clientID)
        #expect(query("response_type") == "code")
        #expect(query("response_mode") == "form_post")
        #expect(query("redirect_uri") == "com.yral.iosApp://oauth/callback")
        #expect(query("scope") == "name email")
        #expect(query("code_challenge_method") == "S256")
        #expect(query("state") == query("code_challenge"))
        #expect((query("code_challenge") ?? "").count == 43)
    }
}

extension URLRequest {
    /// `httpBody` may be nil when the protocol consumed the body stream —
    /// this reads it back for the recording handlers.
    var bodyStreamData: Data? {
        guard let stream = httpBodyStream else { return nil }
        stream.open()
        defer { stream.close() }
        var data = Data()
        let bufferSize = 4_096
        let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: bufferSize)
        defer { buffer.deallocate() }
        while stream.hasBytesAvailable {
            let read = stream.read(buffer, maxLength: bufferSize)
            if read <= 0 { break }
            data.append(buffer, count: read)
        }
        return data
    }
}
