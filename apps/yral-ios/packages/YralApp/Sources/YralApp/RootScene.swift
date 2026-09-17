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

  init() {
    let sessionStore = SessionStore()
    let authClient = AuthClient(
      authDataSource: AuthDataSource(),
      redirectScheme: "com.yral.iosApp",
      sessionStore: sessionStore
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
    .task { await authClient.initialize() }
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
