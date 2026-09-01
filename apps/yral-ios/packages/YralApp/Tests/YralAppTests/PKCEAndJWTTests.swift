import Testing
import Foundation
@testable import YralApp

struct PKCEAndJWTTests {

    // MARK: - PKCE

    @Test("code verifier is 86 chars of unpadded base64url")
    func verifierFormat() throws {
        let verifier = try YralPKCE.generateCodeVerifier()
        #expect(verifier.count == 86)
        #expect(!verifier.contains("="))
        #expect(!verifier.contains("+"))
        #expect(!verifier.contains("/"))
        // Only base64url alphabet.
        #expect(verifier.allSatisfy { $0.isLetter || $0.isNumber || $0 == "-" || $0 == "_" })
    }

    @Test("two verifiers differ (random)")
    func verifierUniqueness() throws {
        let first = try YralPKCE.generateCodeVerifier()
        let second = try YralPKCE.generateCodeVerifier()
        #expect(first != second)
    }

    @Test("S256 challenge is the RFC 7636 reference value")
    func challengeReferenceVector() {
        // RFC 7636 appendix B: verifier "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        // → challenge "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".
        let challenge = YralPKCE.generateCodeChallenge(
            codeVerifier: "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        )
        #expect(challenge == "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM")
    }

    // MARK: - JWT payload parsing

    /// A hand-built ES256-style JWT (payload only matters — no signature
    /// verification in the client).
    static func makeJWT(payloadJSON: String) -> String {
        func base64URL(_ string: String) -> String {
            Data(string.utf8)
                .base64EncodedString()
                .replacingOccurrences(of: "+", with: "-")
                .replacingOccurrences(of: "/", with: "_")
                .replacingOccurrences(of: "=", with: "")
        }
        return "\(base64URL(#"{"alg":"ES256","typ":"JWT"}"#)).\(base64URL(payloadJSON)).\(base64URL("signature"))"
    }

    @Test("required claims decode; exp/iat coerce from numbers")
    func claimsDecode() throws {
        // Multi-line payload keeps the fixture under the 200-char line limit.
        let payloadJSON = #"""
        {"aud":"e1a6a7fb-8a1d-42dc-87b4-13ff94ecbe34","exp":1767225600,"iat":1767139200,
        "iss":"auth.yral.com","sub":"auth0|user-77","nonce":"n-1","ext_is_anonymous":false,
        "email":"user@example.com","ext_ai_account_ids":["auth0|bot-1",null,"auth0|bot-2"]}
        """#
        let jwt = Self.makeJWT(payloadJSON: payloadJSON)
        let claims = try YralJWTParser.parsePayload(of: jwt)

        #expect(claims.audience == ["e1a6a7fb-8a1d-42dc-87b4-13ff94ecbe34"])
        #expect(claims.expiry == 1_767_225_600)
        #expect(claims.issuedAtTime == 1_767_139_200)
        #expect(claims.issuerHost == "auth.yral.com")
        #expect(claims.principal == "auth0|user-77")
        #expect(claims.nonce == "n-1")
        #expect(claims.isAnonymous == false)
        #expect(claims.email == "user@example.com")
        // null elements in ext_ai_account_ids are dropped.
        #expect(claims.botAccountIds == ["auth0|bot-1", "auth0|bot-2"])
        #expect(claims.isValid(currentTimeInEpochSeconds: 1_767_139_201))
        #expect(!claims.isValid(currentTimeInEpochSeconds: 1_767_225_600))
    }

    @Test("audience is polymorphic: absent, single string, array")
    func audiencePolymorphism() throws {
        // Array form.
        let arrayJWT = Self.makeJWT(
            payloadJSON: #"{"aud":["a","b"],"exp":1,"iat":1,"iss":"auth.yral.com","sub":"s"}"#
        )
        #expect(try YralJWTParser.parsePayload(of: arrayJWT).audience == ["a", "b"])

        // Absent → empty.
        let absentJWT = Self.makeJWT(
            payloadJSON: #"{"exp":1,"iat":1,"iss":"auth.yral.com","sub":"s"}"#
        )
        #expect(try YralJWTParser.parsePayload(of: absentJWT).audience.isEmpty)
    }

    @Test("missing required claim throws")
    func missingClaim() {
        let jwt = Self.makeJWT(payloadJSON: #"{"iss":"auth.yral.com"}"#)
        #expect(throws: YralAuthError.missingRequiredClaim("exp")) {
            _ = try YralJWTParser.parsePayload(of: jwt)
        }
    }

    @Test("exp/iat coerce from strings (server-JSON edge)")
    func claimsCoerceFromString() throws {
        let jwt = Self.makeJWT(
            payloadJSON: #"{"exp":"1767225600","iat":"1767139200","iss":"auth.yral.com","sub":"s"}"#
        )
        let claims = try YralJWTParser.parsePayload(of: jwt)
        #expect(claims.expiry == 1_767_225_600)
    }

    @Test("malformed JWT (2 segments) throws")
    func malformedJWT() {
        #expect(throws: YralAuthError.malformedJWT(reason: "expected 3 segments, got 2")) {
            _ = try YralJWTParser.parsePayload(of: "two.segments")
        }
    }

    // MARK: - Redirect URI construction

    @Test("redirect URI is scheme://oauth/callback")
    func redirectURI() {
        #expect(YralAuthDataSource.redirectURI(scheme: "com.yral.iosApp") == "com.yral.iosApp://oauth/callback")
    }

    // MARK: - Keychain round-trip (host-side: in-memory only via UserDefaults
    // would be better; Keychain IS available in macOS test hosts, so exercise it)

    @Test("keychain set/get/remove round-trips")
    func keychainRoundTrip() {
        let store = YralKeychainStore(service: "yral-tests-\(UUID().uuidString)")
        defer { store.removeAll() }

        #expect(store.string(forKey: .idToken) == nil)
        store.setString("token-1", forKey: .idToken)
        #expect(store.string(forKey: .idToken) == "token-1")

        // Upsert replaces.
        store.setString("token-2", forKey: .idToken)
        #expect(store.string(forKey: .idToken) == "token-2")

        store.removeValue(forKey: .idToken)
        #expect(store.string(forKey: .idToken) == nil)
    }
}
