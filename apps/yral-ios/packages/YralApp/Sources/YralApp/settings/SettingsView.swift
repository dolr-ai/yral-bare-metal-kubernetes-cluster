import SwiftUI

/// Account / settings screen — SwiftUI port of the essential surface of
/// Kotlin `AccountScreen` (the menu the profile opens): account header,
/// notification-alerts toggle, logout, and delete account with a
/// confirmation sheet. Pro subscription cards, the account switcher, and
/// help-link lists land with their phases (subscription, bots, settings
/// links respectively) — this slice gives logout its proper home and
/// wires the already-ported delete endpoint.
///
/// No view model — state is @State here; actions call `AuthClient` /
/// `AuthDataSource` directly (same inline pattern as the sign-in screens).
struct SettingsView: View {

    let authClient: AuthClient
    let sessionStore: SessionStore

    @State private var isAlertsEnabled = false
    @State private var isDeleteSheetShown = false
    @State private var isDeletingAccount = false
    @State private var isAccountSwitcherShown = false
    @State private var actionError: String?
    @Environment(\.openURL) private var openURL

    /// Kotlin PrefKeys.NOTIFICATION_ALERTS_ENABLED — display-data
    /// persistence (the actual push registration lands with the push
    /// phase; the toggle is stored now so it controls that flow later).
    private let alertsDefaultsKey = "NOTIFICATION_ALERTS_ENABLED"

    // TODO(push-notifications): wire APNS → Firebase Messaging → the
    // SpacetimeDB register_notification_token reducer on sign-in
    // (AuthClient.postLogin stub), and deregister on logout. Requires
    // pinning the FirebaseMessaging product of the already-pinned
    // firebase-ios-sdk in Package.swift + the aps-environment entitlement
    // (already present: development) + server-side APNS key upload to
    // Firebase console. HOLD: research pending before committing to the
    // dependency set.

    var body: some View {
        VStack(spacing: 0) {
            header

            List {
                accountSection
                togglesSection
                dangerSection
            }
            #if canImport(UIKit)
                .listStyle(.insetGrouped)
            #else
                .listStyle(.automatic)
            #endif
            .scrollContentBackground(.hidden)
        }
        .background(Color(red: 0.04, green: 0.04, blue: 0.06))
        #if canImport(UIKit)
            .toolbar(.hidden, for: .navigationBar)
        #endif
        .confirmationDialog(
            "Delete your account?",
            isPresented: $isDeleteSheetShown,
            titleVisibility: .visible
        ) {
            Button("Yes, delete", role: .destructive) {
                Task { await deleteAccount() }
            }
            Button("No, take me back", role: .cancel) {}
        } message: {
            Text("This permanently removes your account, posts, and data. This cannot be undone.")
        }
    }

    // MARK: - Header (Kotlin AccountsTitle)

    private var header: some View {
        ZStack {
            Text("Settings")
                .font(.title3.bold())
                .foregroundStyle(.white)
        }
        .padding(.vertical, 12)
    }

    // MARK: - Account info (Kotlin AccountInfoView)

    private var accountSection: some View {
        Section {
            HStack(spacing: 12) {
                if let profilePicURL = sessionStore.profilePic {
                    AsyncImage(url: URL(string: profilePicURL)) { image in
                        image.resizable().scaledToFill()
                    } placeholder: {
                        Color(white: 0.2)
                    }
                    .frame(width: 44, height: 44)
                    .clipShape(Circle())
                }
                VStack(alignment: .leading, spacing: 2) {
                    Text(sessionStore.username ?? "Anonymous")
                        .font(.headline)
                        .foregroundStyle(.white)
                    Text(sessionStore.userPrincipal ?? "")
                        .font(.caption)
                        .foregroundStyle(.white.opacity(0.5))
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
            }
            .listRowBackground(Color(white: 0.13))
        }
    }

    // MARK: - Toggles (Kotlin alerts row)

    private var togglesSection: some View {
        Section {
            Toggle(isOn: $isAlertsEnabled) {
                Text("Notifications")
                    .foregroundStyle(.white)
            }
            .tint(Color(red: 0.95, green: 0.25, blue: 0.55))
            .listRowBackground(Color(white: 0.13))
            .onChange(of: isAlertsEnabled) { _, enabled in
                UserDefaults.standard.set(enabled, forKey: alertsDefaultsKey)
            }
            .onAppear {
                isAlertsEnabled = UserDefaults.standard.bool(forKey: alertsDefaultsKey)
            }
        }
    }

    // MARK: - Logout + delete (Kotlin HelpLinks logout row + delete sheet)

    private var dangerSection: some View {
        Section {
            Button {
                isAccountSwitcherShown = true
            } label: {
                Text("Switch account")
                    .foregroundStyle(.white)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .listRowBackground(Color(white: 0.13))
            .sheet(isPresented: $isAccountSwitcherShown) {
                AccountSwitcherView(authClient: authClient)
            }

            Button {
                Task { await authClient.logout() }
            } label: {
                Text("Sign out")
                    .foregroundStyle(.white)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .listRowBackground(Color(white: 0.13))

            Button {
                isDeleteSheetShown = true
            } label: {
                Text("Delete account")
                    .foregroundStyle(Color(red: 1.0, green: 0.45, blue: 0.6))
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .listRowBackground(Color(white: 0.13))

            if let actionError {
                Text(actionError)
                    .font(.footnote)
                    .foregroundStyle(Color(red: 1.0, green: 0.45, blue: 0.6))
                    .listRowBackground(Color(white: 0.13))
            }
        }
    }

    /// Kotlin `AccountsViewModel.deleteAccount` (main-account path): call
    /// the off-chain delete endpoint via the auth client, then logout
    /// (the client method handles both). Bot accounts need the
    /// soft-delete-on-bot-server path — that lands with the bots phase.
    private func deleteAccount() async {
        isDeletingAccount = true
        defer { isDeletingAccount = false }
        do {
            try await authClient.deleteAccount()
        } catch {
            let reason = (error as? LocalizedError)?.errorDescription
                ?? String(describing: error)
            actionError = "Failed to delete account: \(reason)"
        }
    }
}

#Preview {
    let sessionStore = SessionStore()
    let authClient = AuthClient(
        authDataSource: AuthDataSource(),
        redirectScheme: "com.yral.iosApp",
        sessionStore: sessionStore
    )
    return SettingsView(authClient: authClient, sessionStore: sessionStore)
}
