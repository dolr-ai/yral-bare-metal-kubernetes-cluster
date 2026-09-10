import Testing
import Foundation
@testable import YralApp

/// Tests for `AuthDataSource` redirect construction — 1:1 with the
/// Kotlin `AuthEnv.RedirectUri` shape (`<scheme>://oauth/callback`) —
/// plus the account-deletion contract: deletion goes through the
/// SpacetimeDB `delete_user_info` reducer (the off-chain-agent endpoint
/// is decommissioned — it was a stub that deleted nothing).
/// @MainActor: the delete test constructs AuthClient/SessionStore.
@MainActor
struct AuthDataSourceTests {

    @Test("redirect URI is scheme://oauth/callback")
    func redirectURI() {
        #expect(AuthDataSource.redirectURI(scheme: "com.yral.iosApp")
                == "com.yral.iosApp://oauth/callback")
    }

    // MARK: - delete-account contract (SpacetimeDB reducer)

    /// JWT fixture — mirror of `AuthClientTests.makeJWT` (payload-only
    /// parsing; no signature verification in the client).
    private func makeJWT(claims: [String: Any]) -> String {
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

    /// Deletion targets the CALLER'S OWN sub (from the id token — the
    /// reducer enforces self-or-admin server-side). The reducer call
    /// must carry the token subject, never a locally stored principal.
    @Test("delete account calls delete_user_info with the token subject")
    func deleteAccountCallsReducerWithTokenSubject() async throws {
        let now = Int64(Date.now.timeIntervalSince1970)
        let subject = "test-user-sub-123"
        let idToken = makeJWT(claims: [
            "exp": now + 3_600, "iat": now - 60,
            "iss": "auth.yral.com", "sub": subject
        ])

        // Reference-boxed capture — the @Sendable URLProtocol handler
        // runs off the MainActor (same shape as AuthClientTests' recorders).
        final class CallBox: @unchecked Sendable {
            var url: URL?
            var body: Data?
        }
        let box = CallBox()
        RecordingURLProtocol.handler = { request in
            box.url = request.url
            box.body = request.httpBody ?? request.bodyStreamData
            let response = HTTPURLResponse(
                url: request.url!, statusCode: 200,
                httpVersion: "HTTP/1.1", headerFields: nil
            )!
            return (response, Data("[]".utf8))
        }
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [RecordingURLProtocol.self]
        let mockSession = URLSession(configuration: configuration)
        let keychain = KeychainStore(service: "delete-account-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        keychain.setString(idToken, forKey: .idToken)
        let sessionStore = SessionStore()
        let client = AuthClient(
            authDataSource: AuthDataSource(session: mockSession),
            redirectScheme: "com.yral.iosApp",
            keychain: keychain,
            spacetimeDataSource: SpacetimeDBRemoteDataSource(
                idTokenProvider: { idToken },
                session: mockSession
            ),
            sessionStore: sessionStore
        )

        try await client.deleteAccount()

        // The reducer endpoint, not the off-chain one.
        #expect(box.url?.path.contains("/call/delete_user_info") == true)
        guard let bodyData = box.body else {
            Issue.record("expected a request body")
            return
        }
        let arguments = try JSONSerialization.jsonObject(with: bodyData) as? [Any]
        #expect(arguments?.first as? String == subject)
        // Deletion logs the caller out (session torn down).
        #expect(sessionStore.userPrincipal == nil)
        #expect(keychain.string(forKey: .idToken) == nil)
    }
}
