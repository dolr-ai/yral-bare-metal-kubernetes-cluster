import Foundation
import Testing
@testable import YralApp

/// Tests for `AccountSwitcherMachine` — the switcher's lifecycle.
///
/// The machine is pure, so these drive it with events and assert the
/// resulting `(state, effect)` with no network, no database, no view.
struct AccountSwitcherMachineTests {

    private let localEntries = AccountSwitcherEntries(
        mainAccount: AccountSwitcherEntry(
            subject: "main-subject", username: "Main", avatarURL: "https://a/main",
            isBot: false, isActive: true
        ),
        aiAccounts: [
            AccountSwitcherEntry(
                subject: "bot-subject", username: "Bot", avatarURL: "https://a/bot",
                isBot: true, isActive: false
            )
        ]
    )

    private func initial() -> AccountSwitcherMachine.Snapshot {
        AccountSwitcherMachine.Snapshot.initial
    }

    // MARK: - Happy path

    @Test("appearing shows local rows immediately and starts the overlays")
    func appearingShowsLocalAndLoadsOverlays() {
        let (next, effect) = AccountSwitcherMachine.transition(
            initial(), .appeared(localEntries: localEntries)
        )
        #expect(next.state == .showingLocal(localEntries))
        #expect(effect == .loadOverlays)
        // The list is usable before the overlays land — that is the point
        // of the two-layer design.
        #expect(next.state.entries?.aiAccounts.count == 1)
    }

    @Test("overlays upgrade the showing state to ready")
    func overlaysUpgradeToReady() {
        let first = AccountSwitcherMachine.transition(
            initial(), .appeared(localEntries: localEntries)
        ).snapshot
        let (next, effect) = AccountSwitcherMachine.transition(
            first, .overlaysResolved(localEntries)
        )
        #expect(next.state == .ready(localEntries))
        #expect(effect == .none)
    }

    @Test("appearing with no main subject goes to empty")
    func appearingWithoutMainSubjectIsEmpty() {
        let (next, effect) = AccountSwitcherMachine.transition(
            initial(), .appeared(localEntries: nil)
        )
        #expect(next.state == .empty)
        #expect(effect == .none)
        #expect(next.state.entries == nil)
    }

    @Test("tapping a row switches and carries that row's identity")
    func tapSwitchesWithRowIdentity() {
        let showing = AccountSwitcherMachine.transition(
            initial(), .appeared(localEntries: localEntries)
        ).snapshot
        let (next, effect) = AccountSwitcherMachine.transition(
            showing,
            .rowTapped(subject: "bot-subject", avatarURL: "https://a/bot", username: "Bot")
        )
        #expect(
            next.state == .switching(
                subject: "bot-subject", avatarURL: "https://a/bot", username: "Bot"
            )
        )
        #expect(
            effect == .performSwitch(
                subject: "bot-subject", avatarURL: "https://a/bot", username: "Bot"
            )
        )
    }

    @Test("a completed switch dismisses")
    func switchCompletedDismisses() {
        let switching = AccountSwitcherMachine.Snapshot(
            state: .switching(subject: "bot", avatarURL: "u", username: "n"),
            context: .init()
        )
        let (next, effect) = AccountSwitcherMachine.transition(
            switching, .switchCompleted(failed: false)
        )
        #expect(next.state == .dismissed)
        #expect(effect == .dismiss)
        #expect(next.context.lastSwitchFailed == false)
    }

    @Test("a failed switch still dismisses but is recorded")
    func failedSwitchDismissesAndRecords() {
        let switching = AccountSwitcherMachine.Snapshot(
            state: .switching(subject: "bot", avatarURL: "u", username: "n"),
            context: .init()
        )
        let (next, effect) = AccountSwitcherMachine.transition(
            switching, .switchCompleted(failed: true)
        )
        #expect(effect == .dismiss)
        #expect(next.context.lastSwitchFailed == true)
    }

    // MARK: - Illegal states are unreachable

    @Test("a second tap while switching is a no-op — no overlapping switches")
    func doubleTapIsNoOp() {
        let showing = AccountSwitcherMachine.transition(
            initial(), .appeared(localEntries: localEntries)
        ).snapshot
        let switching = AccountSwitcherMachine.transition(
            showing,
            .rowTapped(subject: "bot-subject", avatarURL: "https://a/bot", username: "Bot")
        ).snapshot

        let (next, effect) = AccountSwitcherMachine.transition(
            switching,
            .rowTapped(subject: "main-subject", avatarURL: "https://a/main", username: "Main")
        )
        // Still on the FIRST switch — the second never took effect.
        #expect(
            next.state == .switching(
                subject: "bot-subject", avatarURL: "https://a/bot", username: "Bot"
            )
        )
        #expect(effect == .none)
    }

    @Test("a tap with no rows on screen is a no-op")
    func tapWhenEmptyIsNoOp() {
        let (next, effect) = AccountSwitcherMachine.transition(
            initial(), .rowTapped(subject: "bot", avatarURL: "u", username: "n")
        )
        #expect(next.state == .empty)
        #expect(effect == .none)
    }

    // MARK: - Late events must not clobber a leaving sheet

    @Test("overlays landing mid-switch do not overwrite the switch state")
    func overlaysDuringSwitchAreIgnored() {
        let switching = AccountSwitcherMachine.Snapshot(
            state: .switching(subject: "bot", avatarURL: "u", username: "n"),
            context: .init()
        )
        let (next, effect) = AccountSwitcherMachine.transition(
            switching, .overlaysResolved(localEntries)
        )
        #expect(
            next.state == .switching(subject: "bot", avatarURL: "u", username: "n")
        )
        #expect(effect == .none)
    }

    @Test("overlays landing after dismissal are ignored")
    func overlaysAfterDismissAreIgnored() {
        let dismissed = AccountSwitcherMachine.Snapshot(state: .dismissed, context: .init())
        let (next, effect) = AccountSwitcherMachine.transition(
            dismissed, .overlaysResolved(localEntries)
        )
        #expect(next.state == .dismissed)
        #expect(effect == .none)
    }

    @Test("an unhandled event leaves the snapshot untouched")
    func unhandledEventDoesNotChangeState() {
        let ready = AccountSwitcherMachine.Snapshot(state: .ready(localEntries), context: .init())
        let (next, effect) = AccountSwitcherMachine.transition(
            ready, .switchCompleted(failed: true)
        )
        // switchCompleted outside .switching takes no transition.
        #expect(next == ready)
        #expect(effect == .none)
    }

    // MARK: - Snapshot shape

    @Test("entries are only exposed by the content-bearing states")
    func entriesOnlyForContentStates() {
        #expect(AccountSwitcherMachine.State.empty.entries == nil)
        #expect(AccountSwitcherMachine.State.dismissed.entries == nil)
        #expect(
            AccountSwitcherMachine.State
                .switching(subject: "s", avatarURL: "u", username: "n").entries == nil
        )
        #expect(AccountSwitcherMachine.State.showingLocal(localEntries).entries != nil)
        #expect(AccountSwitcherMachine.State.ready(localEntries).entries != nil)
    }

    @Test("isSwitching is readable without pattern-matching at the call site")
    func isSwitchingReflectsState() {
        #expect(!AccountSwitcherMachine.State.ready(localEntries).isSwitching)
        #expect(
            AccountSwitcherMachine.State
                .switching(subject: "s", avatarURL: "u", username: "n").isSwitching
        )
    }
}
