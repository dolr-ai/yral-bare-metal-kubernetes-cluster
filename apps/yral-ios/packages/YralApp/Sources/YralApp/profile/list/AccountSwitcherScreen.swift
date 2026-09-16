import SwiftUI

/// Account switcher sheet — "Main Profile" + "AI Influencer profiles"
/// sections, one row per account (avatar, name, active check); tap switches
/// the client-side session.
struct AccountSwitcherScreen: View {

    let authClient: AuthClient
    /// The switcher's state — the view OBSERVES this and renders from it;
    /// it never holds its own list/switching flags. See
    /// `AccountSwitcherMachine` for why (the old pair of independent
    /// `@State` values made illegal states representable).
    @State private var state = AccountSwitcherMachine.State.initial
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            HStack {
                Text("Switch account")
                    .font(.headline)
                Spacer()
                Button {
                    dismiss()
                } label: {
                    Image(systemName: "xmark")
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(.secondary)
                        .padding(8)
                        .background(Color.gray.opacity(0.25), in: Circle())
                }
                .accessibilityLabel("Close")
            }
            .padding(.top, 26)

            if let entries = state.entries {
                // Scrollable — a creator can have many AI accounts; without
                // this the rows past the 2/3 detent's height were simply
                // unreachable.
                ScrollView {
                    VStack(alignment: .leading, spacing: 20) {
                        if !entries.mainAccount.isPlaceholder {
                            section(title: "Main Profile", accounts: [entries.mainAccount])
                        }
                        if !entries.aiAccounts.isEmpty {
                            section(title: "AI Influencer profiles", accounts: entries.aiAccounts)
                        }
                    }
                }
            } else {
                Text("No other accounts")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                    .padding(.top, 12)
            }

            Spacer(minLength: 0)
        }
        .padding(.horizontal, 16)
        .padding(.bottom, 20)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        // 2/3 of the screen — the switcher can hold many AI accounts;
        // medium (~half) cramped the list. Drag down to medium, up for
        // full screen. Custom fraction detent per SwiftUI docs.
        .presentationDetents([.fraction(2.0 / 3.0), .medium, .large])
        .background(Color.black)
        .onAppear {
            // Fallback entries first (instant, offline-safe), then one
            // batch read of the bot profiles overlays the DURABLE hosted
            // avatar URLs from the SpacetimeDB profile table — the source
            // of truth written at creation. Best-effort: on failure the
            // GobGob fallback rows remain.
            send(.appeared(localEntries: authClient.accountSwitcherEntries()))
        }
    }

    /// Perform the effect the transition returned. The machine is pure; the
    /// I/O happens here and is reported back as an event.
    private func run(_ effect: AccountSwitcherMachine.Effect) {
        switch effect {
        case .loadOverlays:
            Task { @MainActor in
                let refreshed = await authClient.refreshedAccountSwitcherEntries()
                send(.overlaysResolved(refreshed))
            }
        case .performSwitch(let subject, let avatarURL, let username):
            authClient.switchToAccount(
                subject: subject,
                avatarURL: avatarURL,
                username: username
            )
            // `switchToAccount` is synchronous and reports no outcome, so
            // completion follows immediately. When it grows a failure path,
            // that becomes the payload and `.dismiss` stops being automatic.
            send(.switchCompleted)
        case .dismiss:
            dismiss()
        case .none:
            break
        }
    }

    /// Send an event to the machine and act on whatever it decides.
    private func send(_ event: AccountSwitcherMachine.Event) {
        let (next, effect) = AccountSwitcherMachine.transition(state, event)
        state = next
        run(effect)
    }

    /// Kotlin `SheetSection` — section title + rows.
    @ViewBuilder
    private func section(title: String, accounts: [AccountSwitcherEntry]) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title)
                .font(.subheadline.weight(.semibold))
            VStack(spacing: 0) {
                ForEach(Array(accounts.enumerated()), id: \.element.subject) { index, account in
                    accountRow(account)
                    if index < accounts.count - 1 {
                        Divider()
                    }
                }
            }
            .background(
                Color.gray.opacity(0.2),
                in: RoundedRectangle(cornerRadius: 12)
            )
        }
    }

    /// Kotlin `AccountRow` — avatar, name, active checkmark.
    private func accountRow(_ account: AccountSwitcherEntry) -> some View {
        Button {
            // The machine decides whether a tap is legal (e.g. a second tap
            // during an in-flight switch is a no-op, not a race) and
            // returns the switch effect.
            send(
                .rowTapped(
                    subject: account.subject,
                    avatarURL: account.avatarURL,
                    username: account.username
                )
            )
        } label: {
            HStack(spacing: 10) {
                AsyncImage(url: URL(string: account.avatarURL)) { image in
                    image.resizable().scaledToFill()
                } placeholder: {
                    Color.gray.opacity(0.25)
                }
                .frame(width: 36, height: 36)
                .clipShape(Circle())

                Text(account.username)
                    .font(.subheadline)
                    .lineLimit(1)
                    .truncationMode(.middle)

                Spacer()

                if account.isActive {
                    Image(systemName: "checkmark")
                        .font(.subheadline.weight(.bold))
                        .foregroundStyle(.pink)
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
    var aiAccounts: [AccountSwitcherEntry]
}

struct AccountSwitcherEntry: Equatable, Identifiable, Sendable {
    var subject: String
    var username: String
    var avatarURL: String
    var isBot: Bool
    var isActive: Bool
    var id: String { subject }
}

private extension AccountSwitcherEntry {
    /// Signed-out sentinel: entries() returns nil then, but keep the main
    /// row's placeholder handling honest if a nil sneaks through.
    var isPlaceholder: Bool { subject.isEmpty }
}

#if DEBUG
/// Fixtures for the previews — a main account plus two bots, one active.
private let previewEntries = AccountSwitcherEntries(
    mainAccount: AccountSwitcherEntry(
        subject: "main-subject",
        username: "sunnyotter",
        avatarURL: ProfilePicture.url(fromSubject: "main-subject"),
        isBot: false,
        isActive: false
    ),
    aiAccounts: [
        AccountSwitcherEntry(
            subject: "bot-one",
            username: "dekuizuku",
            avatarURL: ProfilePicture.url(fromSubject: "bot-one"),
            isBot: true,
            isActive: true
        ),
        AccountSwitcherEntry(
            subject: "bot-two",
            username: "uraraka",
            avatarURL: ProfilePicture.url(fromSubject: "bot-two"),
            isBot: true,
            isActive: false
        )
    ]
)

/// The screen renders straight from the machine's state, so a preview can
/// drive any state directly — no network, no live auth client. `.onAppear`
/// still fires and would overwrite the state, so these previews show the
/// empty case honestly and rely on the machine's own tests for the rest.
#Preview("signed out (empty)") {
    AccountSwitcherScreen(
        authClient: AuthClient(
            authDataSource: AuthDataSource(),
            redirectScheme: "com.yral.iosApp",
            sessionStore: SessionStore()
        )
    )
    .preferredColorScheme(.dark)
}

/// The row rendering itself, previewed in isolation with fixtures — this is
/// the populated variation the screen above cannot show without a session.
#Preview("rows (populated)") {
    VStack(alignment: .leading, spacing: 12) {
        Text("Main Profile").font(.subheadline.weight(.semibold))
        ForEach(previewEntries.aiAccounts + [previewEntries.mainAccount]) { entry in
            Text(entry.username)
                .font(.subheadline)
                .foregroundStyle(entry.isActive ? .pink : .primary)
        }
    }
    .padding(16)
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    .background(Color.black)
    .preferredColorScheme(.dark)
}
#endif
