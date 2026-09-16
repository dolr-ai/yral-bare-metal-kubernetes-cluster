import Foundation

/// Account-switcher lifecycle as a finite state machine.
///
/// The view used to hold two independent `@State` values (`entries` and
/// `isSwitching`), which made illegal states representable — e.g.
/// `isSwitching == true` while `entries == nil`, or a tap landing while a
/// previous switch was still in flight. Now the view observes one value and
/// sends events; every transition is total.
///
/// Mirrors the XState model (see the root AGENTS.md "Finite State Machines
/// for Stateful Logic" rule): the enum is the finite state and each variant
/// carries its own payload. There is deliberately no separate context bag —
/// no data is shared across states, so adding one would be an empty struct.
/// `transition` is pure: it returns the `Effect` to run instead of doing I/O.
enum AccountSwitcherMachine {

    // MARK: - Finite state

    /// What the switcher is doing. Payload lives in the variant, never in
    /// a shared optional that only makes sense in some modes.
    enum State: Equatable, Sendable {
        /// Local rows are already computed and on screen; the remote
        /// overlays are still in flight. The list is usable — the overlays
        /// are best-effort upgrades (hosted avatars, real bot names).
        case showingLocal(AccountSwitcherEntries)
        /// Overlays resolved. Carries the final list.
        case ready(AccountSwitcherEntries)
        /// Nothing to show — no main subject (signed out).
        case empty
        /// A row was tapped; its identity is captured so the effect can
        /// switch without re-reading it from a possibly-changed list.
        case switching(subject: String, avatarURL: String, username: String)
        /// Switch completed; the sheet dismisses. Terminal for this sheet.
        case dismissed

        static let initial = State.empty

        /// The rows to render, when this state has any.
        var entries: AccountSwitcherEntries? {
            switch self {
            case .showingLocal(let entries), .ready(let entries): entries
            case .empty, .switching, .dismissed: nil
            }
        }
    }

    // MARK: - Events

    /// Every way the switcher's state can change. The view emits these; it
    /// never mutates state directly.
    enum Event: Equatable, Sendable {
        /// The sheet appeared: local rows computed (nil → no main subject).
        case appeared(localEntries: AccountSwitcherEntries?)
        /// The remote overlays finished (or gave up — best-effort).
        case overlaysResolved(AccountSwitcherEntries?)
        /// A row was tapped.
        case rowTapped(subject: String, avatarURL: String, username: String)
        /// The switch action finished.
        case switchCompleted
    }

    // MARK: - Effects

    /// Side effects the caller must perform. The transition function is
    /// pure, so it *describes* I/O instead of doing it.
    enum Effect: Equatable, Sendable {
        /// Run the best-effort overlays and send back `overlaysResolved`.
        case loadOverlays
        /// Perform the session switch for this row, then send
        /// `switchCompleted`.
        case performSwitch(subject: String, avatarURL: String, username: String)
        /// Close the sheet.
        case dismiss
        case none
    }

    // MARK: - Transition

    /// Pure: `(state, event) -> (nextState, effect)`.
    ///
    /// Only the transitions that DO something are listed. Anything else
    /// falls to `default`, which is XState's documented behaviour for an
    /// event with no enabled transition: **no transition is taken and the
    /// state does not change**. Enumerating the no-ops individually would
    /// add no safety — `default` satisfies exhaustiveness just as well —
    /// so it would be ceremony, not rigour.
    static func transition(_ state: State, _ event: Event) -> (state: State, effect: Effect) {
        switch (state, event) {

        // The sheet appeared: show local rows immediately, overlays in
        // flight. Presentation-scoped, so it is accepted from any state and
        // is idempotent — a re-presented sheet starts here again.
        case (_, .appeared(let localEntries)):
            guard let localEntries else { return (.empty, .none) }
            return (.showingLocal(localEntries), .loadOverlays)

        // Overlays landed while local rows are showing → upgrade them. A
        // nil result means the overlays could not even build local rows
        // (signed out meanwhile).
        case (.showingLocal, .overlaysResolved(let entries)):
            guard let entries else { return (.empty, .none) }
            return (.ready(entries), .none)

        // A tap is legal only with rows on screen. This guard is what
        // makes a double-tap impossible: `.switching` is unreachable from
        // `.switching`, so two switches can never overlap.
        case (.showingLocal, .rowTapped(let subject, let avatarURL, let username)),
             (.ready, .rowTapped(let subject, let avatarURL, let username)):
            return (
                .switching(subject: subject, avatarURL: avatarURL, username: username),
                .performSwitch(subject: subject, avatarURL: avatarURL, username: username)
            )

        // Switch finished → dismiss.
        case (.switching, .switchCompleted):
            return (.dismissed, .dismiss)

        // No transition taken — the state does not change.
        default:
            return (state, .none)
        }
    }
}
