import SwiftUI

/// The root SwiftUI scene content for the Yral app.
///
/// Session-driven: `SessionStore.state` (the `AuthMachine`) decides the
/// surface — `.initial` (never signed in) and `.signedOut` (session ended)
/// both show the sign-in screen, `.restoring` shows the splash, and either
/// signed-in variant shows the app. `isRestoring` drives the splash so the
/// two sign-in states do not need a third route.
struct RootScene: View {

  @State private var authClient: AuthClient
  @State private var sessionStore: SessionStore
  private let analytics: AnalyticsClient

  init() {
    // Launch-time setup: install the tracker, then hand the facade to the
    // objects that emit events. `configureAnalytics()` is idempotent, so a
    // scene rebuilt mid-process reuses the same tracker (the SDK keys its
    // event store by namespace — a second tracker on the same namespace
    // would reconfigure the first).
    let analytics = YralAppRoot.configureAnalytics()
    self.analytics = analytics
    let sessionStore = SessionStore()
    // The store projects attribution; the analytics layer owns the tracker.
    // Wiring them here (not inside the store) is what keeps `SessionStore`
    // free of any analytics dependency.
    sessionStore.identityChangeHandler = { [weak analytics] userId in
      guard let analytics else { return }
      if let userId {
        analytics.setIdentity(userId: userId)
      } else {
        analytics.clearIdentity()
      }
    }
    let authClient = AuthClient(
      authDataSource: AuthDataSource(),
      redirectScheme: "com.yral.iosApp",
      sessionStore: sessionStore,
      analytics: analytics
    )
    _sessionStore = State(initialValue: sessionStore)
    _authClient = State(initialValue: authClient)
  }

  var body: some View {
    Group {
      switch sessionStore.state {
      case .initial, .signedOut:
        SignInScreen(authClient: authClient)
      case .restoring:
        splash
      case .signedIn, .signedInAsBot:
        MainTabScreen(authClient: authClient, sessionStore: sessionStore)
      }
    }
    // Cold-start restore runs for every entry state, so it is not a signal
    // that launch settled. `appLaunch` therefore waits for the session to
    // reach a decision — anonymous or signed in — rather than firing on the
    // `.initial` state that is also the sign-in surface's steady state.
    .task { await authClient.initialize() }
    .onChange(of: sessionStore.state.isRestoring) { _, isRestoring in
      if !isRestoring { analytics.track(.appLaunch) }
    }
  }

  /// Splash — cold-start session restore in flight.
  private var splash: some View {
    VStack(spacing: 16) {
      Image(systemName: "sparkles")
        .font(.system(size: 48))
        .foregroundStyle(.primary)
      ProgressView()
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity)
    .background(Color.black)
  }
}

#Preview {
  RootScene()
}
