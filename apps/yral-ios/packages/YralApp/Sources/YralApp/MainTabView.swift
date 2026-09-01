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
    /// The create wizard's draft — owned HERE, outside the sheet, so
    /// pulling down to leave never loses it: tapping Create again
    /// resumes exactly where the user left off (operator request
    /// 2026-09-01). Reset inside the sheet is the explicit clear.
    @State private var creationDraft = AICreationDraft()

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
                AIAccountCreationView(
                    authClient: authClient,
                    sessionStore: sessionStore,
                    draft: $creationDraft
                )
            }
            .presentationDetents([.large])
            // The system grabber — the visible pull cue. Pulling down
            // LEAVES without losing anything: the draft lives in
            // MainTabView (outside the sheet) and resumes on the next
            // Create tap. Reset inside the sheet is the explicit clear.
            .presentationDragIndicator(.visible)
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
