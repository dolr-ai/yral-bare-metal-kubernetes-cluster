import Foundation

/// Outcome of an OAuth redirect or of the browser-auth session itself — 1:1
/// port of Kotlin `OAuthResult` (features/auth/utils/OAuthResult.kt).
/// `cancelled` and `timedOut` are produced by the browser-auth session
/// (user dismisses the sheet / the 5-minute callback window elapses), not
/// by URL parsing.
public enum OAuthResult: Equatable, Sendable {
    /// The redirect carried a valid `code` + `state` pair.
    case success(code: String, state: String)
    /// The redirect carried an `error` (+ optional description).
    case failure(error: String, errorDescription: String?)
    /// The user dismissed the browser-auth session (errSecCanceled).
    case cancelled
    /// The browser-auth session expired before any redirect arrived.
    case timedOut
}

/// Maps an OAuth callback URL to a typed result — 1:1 port of Kotlin
/// `IosOAuthUtilsHelper.mapUriToOAuthResult`. The redirect shape is
/// `<scheme>://oauth/callback` with `error`/`error_description` or
/// `code`+`state` query parameters. Non-matching URLs (some other app or
/// universal link) map to nil — ignored, not an error.
public enum OAuthCallbackParser {

    /// Parses a callback URL string. Returns nil when the URL doesn't match
    /// the expected scheme/host/path (or is not a valid URL).
    public static func parse(
        callbackURL: String,
        redirectScheme: String,
        redirectHost: String = "oauth",
        redirectPath: String = "/callback"
    ) -> OAuthResult? {
        guard let url = URL(string: callbackURL) else { return nil }
        return parse(
            callbackURL: url,
            redirectScheme: redirectScheme,
            redirectHost: redirectHost,
            redirectPath: redirectPath
        )
    }

    /// URL-typed variant (the system hands us a URL, e.g. via `onOpenURL`).
    public static func parse(
        callbackURL: URL,
        redirectScheme: String,
        redirectHost: String = "oauth",
        redirectPath: String = "/callback"
    ) -> OAuthResult? {
        // THE invalid_callback fix. RFC 3986: scheme (§3.1) and host (§3.2.2)
        // are case-INSENSITIVE; the path (§3.3) is case-sensitive. Foundation's
        // URL parser (WHATWG-based since iOS 17) lowercases the scheme while
        // parsing, so a redirect registered as "com.yral.iosApp://…" arrives
        // with `.scheme == "com.yral.iosapp"` while `redirectScheme` (verbatim
        // from Info.plist) keeps the mixed case — the previous exact `==`
        // rejected EVERY callback (the `invalid_callback` failure). The Kotlin
        // original matched raw strings, where case survives parsing, so
        // case-insensitive comparison is the faithful behavior here.
        guard let scheme = callbackURL.scheme,
              scheme.lowercased() == redirectScheme.lowercased(),
              let host = callbackURL.host,
              host.lowercased() == redirectHost.lowercased(),
              callbackURL.path == redirectPath
        else { return nil }

        // OAuth redirects are application/x-www-form-urlencoded (RFC 6749
        // §4.1.2.1): '+' encodes a space and only then percent-decoding
        // applies. URLComponents.queryItems decodes per RFC 3986, where '+'
        // is literal — so parse the percent-encoded query manually.
        let percentEncodedQuery = URLComponents(
            url: callbackURL, resolvingAgainstBaseURL: false
        )?.percentEncodedQuery ?? ""
        func queryValue(_ name: String) -> String? {
            for pair in percentEncodedQuery.split(separator: "&") {
                let parts = pair.split(
                    separator: "=",
                    maxSplits: 1,
                    omittingEmptySubsequences: false
                )
                guard parts.count == 2, parts[0] == name else { continue }
                return parts[1]
                    .replacingOccurrences(of: "+", with: " ")
                    .removingPercentEncoding
            }
            return nil
        }

        let error = queryValue("error")
        let code = queryValue("code")
        let state = queryValue("state")

        if let error, !error.trimmingCharacters(in: .whitespaces).isEmpty {
            return .failure(
                error: error,
                errorDescription: queryValue("error_description")
            )
        }
        if let code, let state,
           !code.trimmingCharacters(in: .whitespaces).isEmpty,
           !state.trimmingCharacters(in: .whitespaces).isEmpty {
            return .success(code: code, state: state)
        }
        return .failure(
            error: "unknown_error",
            errorDescription: "Missing required parameters"
        )
    }
}
