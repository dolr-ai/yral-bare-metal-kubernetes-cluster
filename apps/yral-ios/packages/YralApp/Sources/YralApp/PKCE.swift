import Foundation
import CryptoKit

/// PKCE (RFC 7636, S256) + JWT payload parsing — 1:1 port of the Kotlin
/// `OAuthUtils`/`IosOAuthUtils` contract.
///
/// Kotlin contract details preserved exactly:
///   - Verifier: 64 random bytes → base64url, `=` padding stripped (86 chars).
///   - Challenge: SHA256(verifier utf8) → base64url, padding stripped.
///   - `state` is NOT a separate random value in this app — the auth flow
///     always sets `state == codeChallenge` and the callback check compares
///     against the stored challenge.
public enum PKCE {

    /// Random bytes per count, via `SecRandomCopyBytes` (cryptographically
    /// secure — same source as Kotlin's kSecRandomDefault).
    private static func secureRandomBytes(_ count: Int) throws -> [UInt8] {
        var bytes = [UInt8](repeating: 0, count: count)
        let status = bytes.withUnsafeMutableBytes { buffer in
            SecRandomCopyBytes(kSecRandomDefault, count, buffer.baseAddress!)
        }
        guard status == errSecSuccess else {
            throw AuthError.randomGenerationFailed(status: Int(status))
        }
        return bytes
    }

    /// base64url WITHOUT padding (Kotlin: `Base64.UrlSafe.encode().trimEnd('=')`).
    private static func base64URLEncode(_ data: Data) -> String {
        data.base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }

    /// Generates the PKCE code verifier: 64 random bytes, base64url, unpadded.
    public static func generateCodeVerifier() throws -> String {
        base64URLEncode(Data(try secureRandomBytes(64)))
    }

    /// Derives the S256 code challenge from a verifier.
    public static func generateCodeChallenge(codeVerifier: String) -> String {
        let digest = SHA256.hash(data: Data(codeVerifier.utf8))
        return base64URLEncode(Data(digest))
    }
}

/// JWT claim payload — port of Kotlin `TokenClaims`. Payload-only parsing
/// (NO signature verification — token trust is established server-side).
public struct TokenClaims: Equatable {
    /// `aud` — audience (client id). Absent → empty, single string → 1-item,
    /// array → as-is (polymorphic, matching `parseAudience`).
    public let audience: [String]
    /// `exp` — epoch seconds. Validity is `expiry > now` (strictly greater).
    public let expiry: Int64
    /// `iat` — epoch seconds.
    public let issuedAtTime: Int64
    /// `iss` — issuer host ("auth.yral.com").
    public let issuerHost: String
    /// `sub` — the user's principal (SpacetimeDB identity source).
    public let principal: String
    /// `nonce` — optional.
    public let nonce: String?
    /// `ext_is_anonymous` — default false.
    public let isAnonymous: Bool
    /// `email` — optional.
    public let email: String?
    /// `ext_ai_account_ids` — bot account ids; null elements dropped.
    public let botAccountIds: [String]?

    /// Matches Kotlin `TokenClaims.isValid`: expiry strictly greater than now.
    public func isValid(currentTimeInEpochSeconds: Int64) -> Bool {
        expiry > currentTimeInEpochSeconds
    }
}

/// JWT payload parsing — port of Kotlin `IosOAuthUtils.parseOAuthToken`.
///
/// Decoding: split on ".", require exactly 3 segments, base64url-decode
/// segment 1 with `=` re-padding to a multiple of 4, UTF-8 JSON object.
/// Required claims (throw when missing): `exp`, `iat`, `iss`, `sub`.
public enum JWTParser {

    /// Claim keys (mirroring the Kotlin constants).
    private enum ClaimKey {
        static let audience = "aud"
        static let expiry = "exp"
        static let issuedAt = "iat"
        static let issuer = "iss"
        static let subject = "sub"
        static let nonce = "nonce"
        static let isAnonymous = "ext_is_anonymous"
        static let botAccountIds = "ext_ai_account_ids"
        static let email = "email"
    }

    /// Parses the JWT payload into typed claims.
    public static func parsePayload(of token: String) throws -> TokenClaims {
        let payloadJSON = try decodePayloadObject(token)

        let audience = parseAudience(payloadJSON[ClaimKey.audience])
        let expiry = try requireLongClaim(payloadJSON, ClaimKey.expiry)
        let issuedAt = try requireLongClaim(payloadJSON, ClaimKey.issuedAt)
        let issuerHost = try requireStringClaim(payloadJSON, ClaimKey.issuer)
        let principal = try requireStringClaim(payloadJSON, ClaimKey.subject)
        let nonce = optionalString(payloadJSON[ClaimKey.nonce])
        let isAnonymous = (payloadJSON[ClaimKey.isAnonymous] as? Bool) ?? false
        let email = optionalString(payloadJSON[ClaimKey.email])
        let botAccountIds = parseStringList(payloadJSON[ClaimKey.botAccountIds])

        return TokenClaims(
            audience: audience,
            expiry: expiry,
            issuedAtTime: issuedAt,
            issuerHost: issuerHost,
            principal: principal,
            nonce: nonce,
            isAnonymous: isAnonymous,
            email: email,
            botAccountIds: botAccountIds
        )
    }

    /// Decodes the payload segment (base64url with `=` re-padding).
    private static func decodePayloadObject(_ token: String) throws -> [String: Any] {
        let segments = token.split(separator: ".", omittingEmptySubsequences: false)
        guard segments.count == 3 else {
            throw AuthError.malformedJWT(reason: "expected 3 segments, got \(segments.count)")
        }
        let encodedPayload = String(segments[1])
        let paddedLength = (4 - encodedPayload.count % 4) % 4
        let padded = encodedPayload + String(repeating: "=", count: paddedLength)
        let normalized = padded
            .replacingOccurrences(of: "-", with: "+")
            .replacingOccurrences(of: "_", with: "/")
        guard let payloadData = Data(base64Encoded: normalized),
              let object = try? JSONSerialization.jsonObject(with: payloadData),
              let payloadJSON = object as? [String: Any]
        else {
            throw AuthError.malformedJWT(reason: "payload is not a JSON object")
        }
        return payloadJSON
    }

    /// `aud` polymorphic decode (absent → [], string → [it], array → items).
    private static func parseAudience(_ value: Any?) -> [String] {
        switch value {
        case let array as [Any]:
            return array.compactMap { $0 as? String }.filter { !$0.isBlank }
        case let string as String:
            return string.isBlank ? [] : [string]
        default:
            return []
        }
    }

    /// `exp`/`iat` coercion: number → int, double-truncated → int,
    /// string → parsed (Kotlin: `longOrNull ?: doubleOrNull?.toLong() ?:
    /// content.toLong()`).
    private static func requireLongClaim(_ json: [String: Any], _ key: String) throws -> Int64 {
        guard let value = json[key] else {
            throw AuthError.missingRequiredClaim(key)
        }
        switch value {
        case let number as NSNumber:
            if let integer = number.int64ValueExact { return integer }
            throw AuthError.malformedClaim(key: key, reason: "not an integer")
        case let string as String:
            guard let integer = Int64(string) else {
                throw AuthError.malformedClaim(key: key, reason: "unparseable number")
            }
            return integer
        default:
            throw AuthError.malformedClaim(key: key, reason: "unexpected type")
        }
    }

    private static func requireStringClaim(_ json: [String: Any], _ key: String) throws -> String {
        guard let value = json[key] as? String, !value.isEmpty else {
            throw AuthError.missingRequiredClaim(key)
        }
        return value
    }

    private static func optionalString(_ value: Any?) -> String? {
        guard let value = value as? String else { return nil }
        return value
    }

    /// `ext_ai_account_ids` — array of strings, null elements dropped.
    private static func parseStringList(_ value: Any?) -> [String]? {
        guard let array = value as? [Any] else { return nil }
        let strings = array.compactMap { $0 as? String }
        return strings
    }
}

/// Auth-domain failures.
public enum AuthError: Error, Equatable {
    case randomGenerationFailed(status: Int)
    case malformedJWT(reason: String)
    case missingRequiredClaim(String)
    case malformedClaim(key: String, reason: String)
    /// OAuth callback `state` did not match the in-flight code challenge —
    /// possible CSRF (Kotlin: SecurityException).
    case stateMismatch
    /// OAuth/phone flow failed at any step after URL construction.
    case oauthFailed(errorDescription: String)
}

extension String {
    /// Whitespace-or-empty check (Kotlin `isBlank`).
    var isBlank: Bool { allSatisfy(\.isWhitespace) }
}

extension NSNumber {
    /// Exact Int64 extraction — rejects fractional doubles (JSONSerialization
    /// may produce NSNumber-backed doubles for integer tokens).
    var int64ValueExact: Int64? {
        if CFGetTypeID(self) == CFGetTypeID(true as NSNumber) { return nil }
        let objCTypeChar = objCType[0]
        guard objCTypeChar != UInt8(0x64) /* 'd' */ else {
            let doubleValue = self.doubleValue
            guard doubleValue.rounded() == doubleValue else { return nil }
            return Int64(doubleValue)
        }
        return int64Value
    }
}
