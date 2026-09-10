import Testing
import Foundation
@testable import YralApp

/// Tests for `AuthDataSource` redirect construction — 1:1 with the
/// Kotlin `AuthEnv.RedirectUri` shape (`<scheme>://oauth/callback`) —
/// plus the account-deletion contract: deletion goes through the
/// SpacetimeDB `delete_user_info` reducer (the off-chain-agent endpoint
/// is decommissioned — it was a stub that deleted nothing).
/// @MainActor: the delete tests construct AuthClient/SessionStore. Each
/// test stubs HTTP on its own `ChannelURLProtocol` channel — no shared
/// state, fully parallel-safe.
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

    // MARK: - Account deletion (the SpacetimeDB delete_user_info contract)

    /// Everything a delete test needs: the signed-in client, its
    /// stores, and the stub channel (tests `defer` its unregistration).
    private struct SignedInClient {
        let client: AuthClient
        let sessionStore: SessionStore
        let keychain: KeychainStore
        let channel: String
    }

    /// Builds a signed-in client with the given session + tokens. The
    /// channel-scoped handler captures the reducer call into `recorder`.
    @discardableResult
    private func makeSignedInClient(
        activeSubject: String,
        isAIAccount: Bool,
        mainSubject: String,
        idToken: String,
        recorder: RequestRecorder
    ) -> SignedInClient {
        let channel = UUID().uuidString
        ChannelURLProtocol.register(
            { request in
                recorder.record(request)
                // The delete_user PROCEDURE returns DeleteUserResult as
                // a positional array: [error, deleted_subjects,
                // backend_deletions]. Success-shaped stub: echo the
                // requested subject as deleted, no backend failures.
                var deletedSubjectsJSON = "[]"
                if request.url?.path.contains("/call/delete_user") == true {
                    let body = request.httpBody ?? request.bodyStreamData
                    if let body,
                       let arguments = try? JSONSerialization.jsonObject(with: body) as? [Any],
                       let target = arguments.first as? String {
                        deletedSubjectsJSON = "[\"\(target)\"]"
                    }
                }
                let deleteResult = "[\"\",\(deletedSubjectsJSON),[]]"
                let response = HTTPURLResponse(
                    url: request.url!, statusCode: 200,
                    httpVersion: "HTTP/1.1", headerFields: nil
                )!
                return (response, Data(deleteResult.utf8))
            },
            forChannel: channel
        )
        let mockSession = ChannelURLProtocol.makeSession(channel: channel)
        let keychain = KeychainStore(service: "delete-account-tests-\(UUID().uuidString)")
        keychain.setString(idToken, forKey: .idToken)
        keychain.setString(mainSubject, forKey: .mainSubject)
        let sessionStore = SessionStore()
        let session = Session(
            userSubject: activeSubject,
            profilePic: ProfilePicture.url(fromSubject: activeSubject),
            username: "test-user",
            isAIAccount: isAIAccount
        )
        sessionStore.updateState(.signedIn(session))
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
        return SignedInClient(
            client: client,
            sessionStore: sessionStore,
            keychain: keychain,
            channel: channel
        )
    }

    /// Extracts the procedure target from the captured delete_user call.
    private func capturedProcedureTarget(recorder: RequestRecorder) throws -> String {
        guard let deleteCall = recorder.requests.first(where: {
            $0.url?.path.contains("/call/delete_user") == true
        }) else {
            Issue.record("expected a delete_user call")
            return ""
        }
        let body = deleteCall.httpBody ?? deleteCall.bodyStreamData
        let arguments = try body.flatMap { try JSONSerialization.jsonObject(with: $0) as? [Any] }
        return arguments?.first as? String ?? ""
    }

    /// THE prod bug this pins: deleting a BOT must target THE BOT's
    /// subject — never the token's sub. Bot sessions carry the
    /// PARENT's tokens (sub = main subject), so the token-based
    /// version cascaded the whole main account when a bot was active.
    @Test("deleting an active AI account targets the bot subject, then switches to main")
    func deleteAIAccountTargetsBotAndSwitchesToMain() async throws {
        let now = Int64(Date.now.timeIntervalSince1970)
        let mainSubject = "main-owner-sub"
        let botSubject = "3324e51a-6379-4eb0-a7ec-cde85897081f"
        // Bot session: tokens are the PARENT's (sub = main subject).
        let idToken = makeJWT(claims: [
            "exp": now + 3_600, "iat": now - 60,
            "iss": "auth.yral.com", "sub": mainSubject
        ])

        let recorder = RequestRecorder()
        let signedIn = makeSignedInClient(
            activeSubject: botSubject,
            isAIAccount: true,
            mainSubject: mainSubject,
            idToken: idToken,
            recorder: recorder
        )
        defer { signedIn.keychain.removeAll() }
        defer { ChannelURLProtocol.unregister(channel: signedIn.channel) }

        try await signedIn.client.deleteAccount()

        // The procedure target is THE BOT — not the token's (main) sub.
        #expect(try capturedProcedureTarget(recorder: recorder) == botSubject)
        // UI switched back to the main account (still signed in).
        #expect(signedIn.sessionStore.userSubject == mainSubject)
        #expect(signedIn.sessionStore.isAIAccount == false)
        #expect(signedIn.keychain.string(forKey: .idToken) != nil)
    }

    @Test("deleting the main account targets the main subject and logs out")
    func deleteMainAccountTargetsMainAndLogsOut() async throws {
        let now = Int64(Date.now.timeIntervalSince1970)
        let mainSubject = "main-owner-sub"
        // Main session: token sub IS the main subject.
        let idToken = makeJWT(claims: [
            "exp": now + 3_600, "iat": now - 60,
            "iss": "auth.yral.com", "sub": mainSubject
        ])

        let recorder = RequestRecorder()
        let signedIn = makeSignedInClient(
            activeSubject: mainSubject,
            isAIAccount: false,
            mainSubject: mainSubject,
            idToken: idToken,
            recorder: recorder
        )
        defer { signedIn.keychain.removeAll() }
        defer { ChannelURLProtocol.unregister(channel: signedIn.channel) }

        try await signedIn.client.deleteAccount()

        // The procedure target is the main subject (cascades all bots
        // server-side).
        #expect(try capturedProcedureTarget(recorder: recorder) == mainSubject)
        // Full logout — the account (and every bot) is gone.
        #expect(signedIn.sessionStore.userSubject == nil)
        #expect(signedIn.keychain.string(forKey: .idToken) == nil)
    }
}
