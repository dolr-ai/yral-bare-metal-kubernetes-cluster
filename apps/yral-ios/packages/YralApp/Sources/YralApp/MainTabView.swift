import SwiftUI

/// The signed-in app shell — a five-tab bottom navigation bar
/// (operator-specified structure, 2026-09-01):
///   home  — the AI-influencer feed (Phase 3 fills it in)
///   chat  — past conversations list (chat phase)
///   create — NEW AI influencer creation (moved here from Settings)
///   profile — the current account's details (Phase 4 fills it in)
///   menu  — what Settings holds today
///
/// Tab content = the five views above; the system tab bar renders as
/// Liquid Glass on iOS 26 (native chrome — no custom styling).
struct MainTabView: View {

    let authClient: AuthClient
    let sessionStore: SessionStore

    @State private var selectedTab: Tab = .home

    enum Tab: Hashable {
        case home, chat, create, profile, menu
    }

    var body: some View {
        TabView(selection: $selectedTab) {
            HomeFeedView()
                .tabItem { Label("Home", systemImage: "house.fill") }
                .tag(Tab.home)

            ChatView()
                .tabItem { Label("Chat", systemImage: "bubble.left.and.bubble.right.fill") }
                .tag(Tab.chat)

            // The create tab is a button, not a destination: tapping it
            // pushes the creation flow as a sheet from anywhere.
            Color.clear
                .tabItem { Label("Create", systemImage: "plus") }
                .tag(Tab.create)

            ProfileView(sessionStore: sessionStore)
                .tabItem { Label("Profile", systemImage: "person.fill") }
                .tag(Tab.profile)

            MenuView(authClient: authClient, sessionStore: sessionStore)
                .tabItem { Label("Menu", systemImage: "line.3.horizontal") }
                .tag(Tab.menu)
        }
        .tint(.pink)
        .sheet(
            isPresented: Binding(
                get: { selectedTab == .create },
                set: { if !$0 && selectedTab == .create { selectedTab = .home } }
            )
        ) {
            NavigationStack {
                AIAccountCreationView(authClient: authClient, sessionStore: sessionStore)
            }
            .presentationDetents([.large])
        }
    }
}

#Preview {
    let sessionStore = SessionStore()
    MainTabView(
        authClient: AuthClient(
            authDataSource: AuthDataSource(),
            redirectScheme: "com.yral.iosApp",
            sessionStore: sessionStore
        ),
        sessionStore: sessionStore
    )
}
