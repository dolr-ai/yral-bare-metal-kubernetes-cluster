import SwiftUI

/// Menu — the current Settings surface (switch account, notifications,
/// sign out, delete account). The tab's anchor; extra menu rows arrive
/// with their features.
struct MenuView: View {

    let authClient: AuthClient
    let sessionStore: SessionStore

    var body: some View {
        NavigationStack {
            SettingsView(authClient: authClient, sessionStore: sessionStore)
        }
    }
}

#Preview {
    let sessionStore = SessionStore()
    MenuView(
        authClient: AuthClient(
            authDataSource: AuthDataSource(),
            redirectScheme: "com.yral.iosApp",
            sessionStore: sessionStore
        ),
        sessionStore: sessionStore
    )
}
