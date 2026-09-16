import SwiftUI

/// Menu — the current Settings surface (switch account, notifications,
/// sign out, delete account). The tab's anchor; extra menu rows arrive
/// with their features.
struct MenuScreen: View {

    let authClient: AuthClient
    let sessionStore: SessionStore

    var body: some View {
        NavigationStack {
            SettingsScreen(authClient: authClient, sessionStore: sessionStore)
        }
    }
}

#Preview {
    let sessionStore = SessionStore()
    MenuScreen(
        authClient: AuthClient(
            authDataSource: AuthDataSource(),
            redirectScheme: "com.yral.iosApp",
            sessionStore: sessionStore
        ),
        sessionStore: sessionStore
    )
}
