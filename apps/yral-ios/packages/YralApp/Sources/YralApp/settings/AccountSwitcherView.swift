import SwiftUI

/// Account switcher sheet — SwiftUI port of Kotlin `AccountSwitchSheet`
/// (`RootScreen.kt`): "Main Profile" + "AI Influencer profiles" sections,
/// one row per account (avatar, name, active check); tap switches the
/// client-side session. State inline (@State), actions via `AuthClient` —
/// same pattern as the other screens.
struct AccountSwitcherView: View {

    let authClient: AuthClient
    @State private var entries: AccountSwitcherEntries?
    @State private var isSwitching = false
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Switch account")
                .font(.headline)
                .foregroundStyle(.white)
                .padding(.top, 26)

            if let entries {
                if !entries.mainAccount.isPlaceholder {
                    section(title: "Main Profile", accounts: [entries.mainAccount])
                }
                if !entries.botAccounts.isEmpty {
                    section(title: "AI Influencer profiles", accounts: entries.botAccounts)
                }
            } else {
                Text("No other accounts")
                    .font(.subheadline)
                    .foregroundStyle(.white.opacity(0.5))
                    .padding(.top, 12)
            }

            Spacer(minLength: 0)
        }
        .padding(.horizontal, 16)
        .padding(.bottom, 20)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(Color(red: 0.04, green: 0.04, blue: 0.06))
        .onAppear { entries = authClient.accountSwitcherEntries() }
    }

    /// Kotlin `SheetSection` — section title + rows.
    @ViewBuilder
    private func section(title: String, accounts: [AccountSwitcherEntry]) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.white)
            VStack(spacing: 0) {
                ForEach(Array(accounts.enumerated()), id: \.element.principal) { index, account in
                    accountRow(account)
                    if index < accounts.count - 1 {
                        Divider().overlay(Color(white: 0.28))
                    }
                }
            }
            .background(Color(white: 0.13), in: RoundedRectangle(cornerRadius: 12))
        }
    }

    /// Kotlin `AccountRow` — avatar, name, active checkmark.
    private func accountRow(_ account: AccountSwitcherEntry) -> some View {
        Button {
            guard !isSwitching else { return }
            isSwitching = true
            authClient.switchToAccount(principal: account.principal)
            dismiss()
        } label: {
            HStack(spacing: 10) {
                AsyncImage(url: URL(string: account.avatarURL)) { image in
                    image.resizable().scaledToFill()
                } placeholder: {
                    Color(white: 0.2)
                }
                .frame(width: 36, height: 36)
                .clipShape(Circle())

                Text(account.username)
                    .font(.subheadline)
                    .foregroundStyle(.white)
                    .lineLimit(1)
                    .truncationMode(.middle)

                Spacer()

                if account.isActive {
                    Image(systemName: "checkmark")
                        .font(.subheadline.weight(.bold))
                        .foregroundStyle(Color(red: 0.95, green: 0.25, blue: 0.55))
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(account.isActive)
    }
}

/// The switcher's data — Kotlin `AccountDialogInfo`/`AccountUi`.
struct AccountSwitcherEntries: Equatable, Sendable {
    var mainAccount: AccountSwitcherEntry
    var botAccounts: [AccountSwitcherEntry]
}

struct AccountSwitcherEntry: Equatable, Identifiable, Sendable {
    var principal: String
    var username: String
    var avatarURL: String
    var isBot: Bool
    var isActive: Bool
    var id: String { principal }
}

private extension AccountSwitcherEntry {
    /// Signed-out sentinel: entries() returns nil then, but keep the main
    /// row's placeholder handling honest if a nil sneaks through.
    var isPlaceholder: Bool { principal.isEmpty }
}
