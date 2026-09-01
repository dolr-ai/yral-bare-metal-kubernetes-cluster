import FirebaseCore
import Foundation
import os
import SwiftUI

/// Root entry surface of the Yral iOS app package.
///
/// All product code lives in the `YralApp` SPM package; the committed
/// `iosApp.xcodeproj` is a thin shell with no product code of its own.
/// The shell's `AppDelegate` calls these entry points at launch.
///
/// TODO(deep-links): Branch SDK wiring (the legacy app used
/// ios-branch-sdk-spm + the `yral://` scheme + applinks:link.yral.com
/// already in the entitlements) — deferred pending research on the
/// dependency; the Branch key lives in the legacy Info.plist
/// (branch_key) when we get there. Also note the native-auth TODOs on
/// BrowserAuthSession.
public enum YralAppRoot {

    /// Creates the root SwiftUI scene content for the app.
    @MainActor
    public static func makeRootScene() -> some View {
        RootScene()
    }

    /// Initializes Firebase (Core — which readies Analytics + Crashlytics).
    ///
    /// Idempotent: repeated calls are a no-op. Safe in environments without a
    /// bundled `GoogleService-Info.plist` (unit tests, previews).
    @MainActor
    public static func configureFirebase() {
        FirebaseBootstrapper.configure()
    }
}

/// Bootstraps Firebase SDKs at launch.
///
/// Wrapping the global `FirebaseApp.configure()` call behind a dedicated,
/// documented entry point keeps the shell's `AppDelegate` thin and makes the
/// "already configured" guard unit-testable without SDK side effects.
enum FirebaseBootstrapper {

    /// Tracks whether `FirebaseApp.configure()` has already run in this process.
    private static let isInitialized = OSAllocatedUnfairLock(initialState: false)

    /// Configures `FirebaseCore` — which transitively readies Analytics and
    /// Crashlytics. Idempotent, and a no-op when no Google service plist is
    /// bundled (unit-test hosts, SwiftUI previews).
    static func configure() {
        let alreadyConfigured = isInitialized.withLock { state in
            defer { state = true }
            return state
        }
        guard !alreadyConfigured else { return }

        // `FirebaseApp.configure()` requires a `GoogleService-Info.plist` in the
        // calling bundle. The app shell bundles one; test hosts do not — so
        // probe for the file first and skip configuration when absent rather
        // than crash with Firebase's fatal error.
        guard
            Bundle.main.path(
                forResource: "GoogleService-Info",
                ofType: "plist"
            ) != nil
        else { return }

        FirebaseApp.configure()
    }
}
