import FirebaseCore
import Foundation
import SwiftUI
import os

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

  /// The process-wide analytics facade, or nil when analytics was never
  /// configured (unit-test hosts that never call `configureAnalytics()`).
  ///
  /// Held here because the TRACKER's lifecycle is process-wide — the SDK
  /// keeps one event store per namespace in `Snowplow`, and creating a second
  /// tracker for the same namespace reconfigures the first — while the
  /// MACHINE that decides attribution lives inside this client as instance
  /// state. The `if let` guard in `configureAnalytics()` is the substance: it
  /// makes repeated setup reuse one tracker. SwiftUI calls a `View`'s `init`
  /// more than once, so a per-scene client would call `createTracker`
  /// repeatedly and reset live tracker configuration each time.
  ///
  /// `@MainActor` because it holds a main-actor-isolated type (Swift 6
  /// requires the isolation to be stated, not inferred, for mutable static
  /// state).
  @MainActor private(set) static var analytics: AnalyticsClient?

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

  /// Installs the analytics tracker and returns the facade the app records
  /// through. Idempotent — a second call returns the existing client rather
  /// than reconfiguring the tracker.
  ///
  /// Separate from `configureFirebase()` on purpose: Firebase and Snowplow
  /// fail independently, and a tracker problem must not prevent crash
  /// reporting from starting (Crashlytics is the thing that would tell us
  /// about it).
  @MainActor
  public static func configureAnalytics() -> AnalyticsClient {
    if let analytics { return analytics }
    let analytics = AnalyticsClient()
    self.analytics = analytics
    return analytics
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
