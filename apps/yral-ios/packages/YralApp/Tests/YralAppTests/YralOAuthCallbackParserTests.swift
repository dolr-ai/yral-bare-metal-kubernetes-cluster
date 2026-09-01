import Testing
import Foundation
@testable import YralApp

/// Tests for `YralOAuthCallbackParser` — Kotlin `mapUriToOAuthResult`
/// parity, including the form-urlencoded `+`-as-space decoding (RFC 6749
/// §4.1.2.1 — URLComponents' RFC-3986 decoding does NOT apply here).
struct YralOAuthCallbackParserTests {

    @Test("success redirect parses code + state")
    func callbackSuccess() {
        let result = YralOAuthCallbackParser.parse(
            callbackURL: "com.yral.iosApp://oauth/callback?code=AUTHCODE123&state=st4te",
            redirectScheme: "com.yral.iosApp"
        )
        #expect(result == .success(code: "AUTHCODE123", state: "st4te"))
    }

    @Test("error redirect parses error + description")
    func callbackError() {
        let result = YralOAuthCallbackParser.parse(
            callbackURL: "com.yral.iosApp://oauth/callback?error=access_denied&error_description=user+said+no",
            redirectScheme: "com.yral.iosApp"
        )
        #expect(result == .failure(error: "access_denied", errorDescription: "user said no"))
    }

    @Test("matching URL with no code/state yields unknown_error failure")
    func callbackUnknownError() {
        let result = YralOAuthCallbackParser.parse(
            callbackURL: "com.yral.iosApp://oauth/callback?foo=1",
            redirectScheme: "com.yral.iosApp"
        )
        #expect(result == .failure(error: "unknown_error", errorDescription: "Missing required parameters"))
    }

    @Test("foreign URLs return nil (ignored, not an error)")
    func callbackForeignURL() {
        #expect(YralOAuthCallbackParser.parse(
            callbackURL: "https://yral.com/post/42",
            redirectScheme: "com.yral.iosApp"
        ) == nil)
        #expect(YralOAuthCallbackParser.parse(
            callbackURL: "yral://oauth/callback?code=c&state=s",
            redirectScheme: "com.yral.iosApp"
        ) == nil)
        #expect(YralOAuthCallbackParser.parse(
            callbackURL: "not a url",
            redirectScheme: "com.yral.iosApp"
        ) == nil)
    }

    @Test("URL-typed variant agrees with the string variant")
    func callbackURLTypedVariant() {
        let url = URL(string: "com.yral.iosApp://oauth/callback?code=c&state=s")!
        #expect(
            YralOAuthCallbackParser.parse(
                callbackURL: url, redirectScheme: "com.yral.iosApp"
            ) == .success(code: "c", state: "s")
        )
    }
}
