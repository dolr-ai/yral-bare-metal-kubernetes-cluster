import Foundation
#if canImport(UIKit)
import AuthenticationServices
import UIKit
#endif

/// Browser-based social sign-in — the yral-auth server-side OIDC flow.
/// Builds the PKCE authorization URL via `AuthClient`, runs
/// `ASWebAuthenticationSession` (ephemeral — no shared cookie state,
/// matching the legacy app), and maps the redirect through
/// `OAuthCallbackParser` into an `OAuthResult` for
/// `AuthClient.handleOAuthCallbackResult`.
///
/// Port of Kotlin `IosOAuthUtils` (`openOAuth`/`handleSessionCompletion`)
/// minus the 5-minute `callbackExpiry` enforcement: ASWebAuthenticationSession
/// delivers exactly one completion (URL, cancel, or system error) — the
/// Kotlin expiry only guarded a double-delivery the native API prevents.
enum BrowserAuthSession {

    // TODO(auth-native-google): replace the browser flow for the Google tile
    // with the GoogleSignIn-iOS SDK (OIDC AppAuth under the hood): obtain the
    // Google identity token natively, then exchange it with yral-auth for our
    // JWT. Requires the app registered as a Google OAuth client + a yral-auth
    // endpoint accepting IdP id_tokens.

    // TODO(auth-native-apple): replace the browser flow for the Apple tile with
    // ASAuthorizationAppleIDButton (AuthenticationServices): obtain the Apple
    // identity token natively, then exchange it with yral-auth. Apple also
    // REQUIRES this native path when third-party sign-in is offered — a
    // compliance item before App Store review.

    /// Runs the browser auth flow for the provider end-to-end (URL build →
    /// session → callback parse), returning the OAuthResult.
    @MainActor
    static func signIn(
        provider: SocialProvider,
        authClient: AuthClient
    ) async throws -> OAuthResult {
        let authorizationURL = try authClient.socialAuthorizationURL(provider: provider)
        let callbackURL = try await runSession(authorizationURL: authorizationURL)
        return OAuthCallbackParser.parse(
            callbackURL: callbackURL,
            redirectScheme: authClient.redirectScheme
        ) ?? .failure(
            error: "invalid_callback",
            errorDescription: "OAuth callback missing required parameters"
        )
    }

    #if canImport(UIKit)
    /// The browser session — Kotlin `IosOAuthUtils.startSession`:
    /// `prefersEphemeralWebBrowserSession = true`, callback scheme =
    /// the app's redirect scheme. Cancel (user dismissed) surfaces as
    /// `ASWebAuthenticationSessionError.canceledLogin`.
    ///
    /// Resume discipline (the crash fix): the completion handler can fire
    /// SYNCHRONOUSLY when start fails — observed on iOS 18 — so both the
    /// handler and the `start() == false` branch guard on `hasResumed`
    /// before touching the continuation. Resuming twice traps with
    /// "SWIFT TASK CONTINUATION MISUSE".
    ///
    /// The session is held for the flow's duration (the Kotlin code kept
    /// it in `authSession`) — a released session cancels its sheet.
    /// The browser session held for the flow's duration (the Kotlin code
    /// kept it in `authSession`) — a released session cancels its sheet.
    /// @MainActor-isolated: the entire flow (create, start, completion,
    /// release) runs on the main actor — ASWebAuthenticationSession's
    /// completion is delivered on the main queue — so no cross-actor access
    /// exists; this is concurrency-safe without a lock.
    @MainActor private static var retainedSession: ASWebAuthenticationSession?

    @MainActor
    private static func runSession(authorizationURL: URL) async throws -> URL {
        // Local (not static) resume flag — captured by both the handler and
        // the start() branch; both run on the main actor, so plain captured
        // var access is race-free.
        final class ResumeFlag: @unchecked Sendable {
            private let lock = NSLock()
            private var value = false
            var hasResumed: Bool {
                lock.lock(); defer { lock.unlock() }
                return value
            }
            func setResumed() {
                lock.lock(); defer { lock.unlock() }
                value = true
            }
        }
        let resumeFlag = ResumeFlag()
        return try await withCheckedThrowingContinuation { continuation in
            let session = ASWebAuthenticationSession(
                url: authorizationURL,
                callbackURLScheme: "com.yral.iosApp"
            ) { callbackURL, error in
                guard !resumeFlag.hasResumed else { return }
                resumeFlag.setResumed()
                Self.retainedSession = nil
                if let callbackURL {
                    continuation.resume(returning: callbackURL)
                } else if let error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume(
                        throwing: AuthError.oauthFailed(
                            errorDescription:
                                "Browser auth session ended without a result"
                        )
                    )
                }
            }
            session.presentationContextProvider = PresentationAnchorProvider()
            session.prefersEphemeralWebBrowserSession = true
            Self.retainedSession = session
            if !session.start() && !resumeFlag.hasResumed {
                // The handler did NOT fire synchronously — start genuinely
                // failed without a callback. This is the ONLY additional
                // resume path (Kotlin's "session_start_failed").
                resumeFlag.setResumed()
                Self.retainedSession = nil
                continuation.resume(
                    throwing: AuthError.oauthFailed(
                        errorDescription: "Unable to launch the browser auth session"
                    )
                )
            }
        }
    }

    /// Presentation anchor — bridges SwiftUI to AuthenticationServices
    /// (Kotlin `OAuthPresentationAnchorProvider` resolving the front
    /// window).
    private final class PresentationAnchorProvider: NSObject,
        ASWebAuthenticationPresentationContextProviding {
        func presentationAnchor(
            for session: ASWebAuthenticationSession
        ) -> ASPresentationAnchor {
            (UIApplication.shared.connectedScenes
                .compactMap { ($0 as? UIWindowScene)?.keyWindow }
                .first) ?? ASPresentationAnchor()
        }
    }
    #else
    /// macOS test host — the browser session is iOS-only; tests drive
    /// `handleOAuthCallbackResult` directly with fabricated results.
    private static func runSession(authorizationURL: URL) async throws -> URL {
        throw AuthError.oauthFailed(
            errorDescription: "Browser auth unavailable on this platform"
        )
    }
    #endif
}
