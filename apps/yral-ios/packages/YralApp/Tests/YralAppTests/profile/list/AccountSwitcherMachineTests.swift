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

  private func showing() -> AccountSwitcherMachine.State {
    AccountSwitcherMachine.transition(
      .initial, .appeared(localEntries: localEntries)
    ).state
  }

  // MARK: - Happy path

  @Test("appearing shows local rows immediately and starts the overlays")
  func appearingShowsLocalAndLoadsOverlays() {
    let (next, effect) = AccountSwitcherMachine.transition(
      .initial, .appeared(localEntries: localEntries)
    )
    #expect(next == .showingLocal(localEntries))
    #expect(effect == .loadOverlays)
    // The list is usable before the overlays land — that is the point
    // of the two-layer design.
    #expect(next.entries?.aiAccounts.count == 1)
  }

  @Test("overlays upgrade the showing state to ready")
  func overlaysUpgradeToReady() {
    let (next, effect) = AccountSwitcherMachine.transition(
      showing(), .overlaysResolved(localEntries)
    )
    #expect(next == .ready(localEntries))
    #expect(effect == .none)
  }

  @Test("appearing with no main subject goes to empty")
  func appearingWithoutMainSubjectIsEmpty() {
    let (next, effect) = AccountSwitcherMachine.transition(
      .initial, .appeared(localEntries: nil)
    )
    #expect(next == .empty)
    #expect(effect == .none)
    #expect(next.entries == nil)
  }

  @Test("overlays resolving to nil fall back to empty")
  func overlaysResolvingNilFallsBackToEmpty() {
    let (next, effect) = AccountSwitcherMachine.transition(
      showing(), .overlaysResolved(nil)
    )
    #expect(next == .empty)
    #expect(effect == .none)
  }

  @Test("tapping a row switches and carries that row's identity")
  func tapSwitchesWithRowIdentity() {
    let (next, effect) = AccountSwitcherMachine.transition(
      showing(),
      .rowTapped(subject: "bot-subject", avatarURL: "https://a/bot", username: "Bot")
    )
    #expect(
      next
        == .switching(
          subject: "bot-subject", avatarURL: "https://a/bot", username: "Bot"
        )
    )
    #expect(
      effect
        == .performSwitch(
          subject: "bot-subject", avatarURL: "https://a/bot", username: "Bot"
        )
    )
  }

  @Test("a tap works from the ready state too — overlays are not a barrier")
  func tapWorksFromReady() {
    let (next, effect) = AccountSwitcherMachine.transition(
      .ready(localEntries),
      .rowTapped(subject: "bot-subject", avatarURL: "https://a/bot", username: "Bot")
    )
    #expect(
      next
        == .switching(
          subject: "bot-subject", avatarURL: "https://a/bot", username: "Bot"
        )
    )
    #expect(
      effect
        == .performSwitch(
          subject: "bot-subject", avatarURL: "https://a/bot", username: "Bot"
        ))
  }

  @Test("a completed switch dismisses")
  func switchCompletedDismisses() {
    let switching = AccountSwitcherMachine.State
      .switching(subject: "bot", avatarURL: "u", username: "n")
    let (next, effect) = AccountSwitcherMachine.transition(switching, .switchCompleted)
    #expect(next == .dismissed)
    #expect(effect == .dismiss)
  }

  // MARK: - Illegal states are unreachable

  @Test("a second tap while switching is a no-op — no overlapping switches")
  func doubleTapIsNoOp() {
    let switching = AccountSwitcherMachine.transition(
      showing(),
      .rowTapped(subject: "bot-subject", avatarURL: "https://a/bot", username: "Bot")
    ).state

    let (next, effect) = AccountSwitcherMachine.transition(
      switching,
      .rowTapped(subject: "main-subject", avatarURL: "https://a/main", username: "Main")
    )
    // Still on the FIRST switch — the second never took effect, so two
    // switchToAccount calls cannot overlap.
    #expect(
      next
        == .switching(
          subject: "bot-subject", avatarURL: "https://a/bot", username: "Bot"
        )
    )
    #expect(effect == .none)
  }

  @Test("a tap with no rows on screen is a no-op")
  func tapWhenEmptyIsNoOp() {
    let (next, effect) = AccountSwitcherMachine.transition(
      .initial, .rowTapped(subject: "bot", avatarURL: "u", username: "n")
    )
    #expect(next == .empty)
    #expect(effect == .none)
  }

  @Test("a tap after dismissal is a no-op — the sheet is on its way out")
  func tapAfterDismissIsNoOp() {
    let (next, effect) = AccountSwitcherMachine.transition(
      .dismissed, .rowTapped(subject: "bot", avatarURL: "u", username: "n")
    )
    #expect(next == .dismissed)
    #expect(effect == .none)
  }

  // MARK: - Late events must not clobber a leaving sheet

  @Test("overlays landing mid-switch do not overwrite the switch state")
  func overlaysDuringSwitchAreIgnored() {
    let switching = AccountSwitcherMachine.State
      .switching(subject: "bot", avatarURL: "u", username: "n")
    let (next, effect) = AccountSwitcherMachine.transition(
      switching, .overlaysResolved(localEntries)
    )
    #expect(next == switching)
    #expect(effect == .none)
  }

  @Test("overlays landing after dismissal are ignored")
  func overlaysAfterDismissAreIgnored() {
    let (next, effect) = AccountSwitcherMachine.transition(
      .dismissed, .overlaysResolved(localEntries)
    )
    #expect(next == .dismissed)
    #expect(effect == .none)
  }

  @Test("a switch completing outside .switching is a no-op")
  func completionOutsideSwitchingIsNoOp() {
    let ready = AccountSwitcherMachine.State.ready(localEntries)
    let (next, effect) = AccountSwitcherMachine.transition(ready, .switchCompleted)
    #expect(next == ready)
    #expect(effect == .none)
  }

  @Test("a late overlay result cannot override a captured switch")
  func lateOverlayCannotOverrideCapturedSwitch() {
    // A tap already captured its row: the switch must win over a
    // duplicate/empty overlay result.
    let switching = AccountSwitcherMachine.State
      .switching(subject: "bot", avatarURL: "u", username: "n")
    let (next, effect) = AccountSwitcherMachine.transition(
      switching, .overlaysResolved(nil)
    )
    #expect(next == switching)
    #expect(effect == .none)
  }

  // MARK: - State shape

  @Test("entries are only exposed by the content-bearing states")
  func entriesOnlyForContentStates() {
    #expect(AccountSwitcherMachine.State.initial.entries == nil)
    #expect(AccountSwitcherMachine.State.empty.entries == nil)
    #expect(AccountSwitcherMachine.State.dismissed.entries == nil)
    #expect(
      AccountSwitcherMachine.State
        .switching(subject: "s", avatarURL: "u", username: "n").entries == nil
    )
    #expect(AccountSwitcherMachine.State.showingLocal(localEntries).entries != nil)
    #expect(AccountSwitcherMachine.State.ready(localEntries).entries != nil)
  }
}
