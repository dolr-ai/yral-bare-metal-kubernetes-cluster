import SwiftUI

/// The root SwiftUI scene content for the Yral app.
///
/// Session-driven: `SessionStore.state` decides the surface —
/// `.loading` shows the splash (cold start restores the cached session
/// via `AuthClient.initialize()`), `.initial` shows the sign-in
/// screen (an anonymous session signs in silently, so `.initial` here
/// means the sign-in surface is wanted — e.g. a fresh install before
/// the first anonymous identity, or a logged-out state), `.signedIn`
/// shows the app placeholder (feed phase replaces it).
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
            case .initial:
                SignInView(authClient: authClient)
            case .loading:
                splash
            case .signedIn:
                NavigationStack {
                    signedInPlaceholder
                }
            }
        }
        .task { await authClient.initialize() }
    }

    /// Splash — cold-start session restore in flight.
    private var splash: some View {
        VStack(spacing: 16) {
            Image(systemName: "sparkles")
                .font(.system(size: 48))
                .foregroundStyle(.white)
            ProgressView()
                .tint(.white)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(red: 0.04, green: 0.04, blue: 0.06))
    }

    /// Signed-in placeholder — the video-feed phase replaces this surface.
    private var signedInPlaceholder: some View {
        VStack(spacing: 16) {
            Image(systemName: "sparkles")
                .font(.system(size: 48))
                .foregroundStyle(.white)
            Text("Yral")
                .font(.largeTitle.bold())
                .foregroundStyle(.white)
            Text("Signed in — feed arrives in the next phase")
                .font(.subheadline)
                .foregroundStyle(.white.opacity(0.6))
            if let username = sessionStore.username {
                Text("Hello, \(username)")
                    .font(.headline)
                    .foregroundStyle(.white)
            }
            if let principal = sessionStore.userPrincipal {
                Text(principal)
                    .font(.caption)
                    .foregroundStyle(.white.opacity(0.4))
            }

            NavigationLink {
                SettingsView(authClient: authClient, sessionStore: sessionStore)
            } label: {
                Text("Settings")
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.white)
            }
            .padding(.top, 24)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(red: 0.04, green: 0.04, blue: 0.06))
    }
}

#Preview {
    RootScene()
}
