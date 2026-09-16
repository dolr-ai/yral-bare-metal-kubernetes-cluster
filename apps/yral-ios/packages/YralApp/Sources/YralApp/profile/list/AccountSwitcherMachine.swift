import Foundation

/// Account-switcher lifecycle as a finite state machine.
///
/// The view used to hold two independent `@State` values (`entries` and
/// `isSwitching`), which made illegal states representable — e.g.
/// `isSwitching == true` while `entries == nil`, or a tap landing while a
/// previous switch was still in flight. Every transition below is total:
/// the view sends events, the machine decides, and the view renders
/// whatever `(state, context)` it observes.
///
/// Mirrors the XState model (see the root AGENTS.md "Finite State
/// Machines for Stateful Logic" rule): `State` is the finite state, the
/// enum variant carries state-specific payload, and `Context` holds the
/// data every state shares. `transition` is pure — it returns the
/// `Effect` to run rather than performing I/O itself.
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

        /// The rows to render, when this state has any.
        var entries: AccountSwitcherEntries? {
            switch self {
            case .showingLocal(let entries), .ready(let entries): entries
            case .empty, .switching, .dismissed: nil
            }
        }

        /// True while a switch is in flight — disables further taps.
        var isSwitching: Bool {
            if case .switching = self { return true }
            return false
        }
    }

    /// Data shared across every state. Kept deliberately small: anything
    /// only meaningful in one state belongs in that variant instead.
    struct Context: Equatable, Sendable {
        /// The sheet is presented/dismissed. Tracked separately from
        /// `State` because presentation outlives the sheet's own content
        /// states (an empty switcher is still a presented sheet).
        var lastSwitchFailed: Bool = false
    }

    /// A machine snapshot — exactly `(state, context)`, matching XState.
    struct Snapshot: Equatable, Sendable {
        var state: State
        var context: Context

        static let initial = Snapshot(state: .empty, context: Context())
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
        /// The switch action finished (success or reported failure).
        case switchCompleted(failed: Bool)
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

    /// Pure: `(state, event) -> (nextState, context, effect)`.
    ///
    /// Only the transitions that DO something are listed. Anything else
    /// falls to `default`, which is XState's documented behaviour for an
    /// event with no enabled transition: **no transition is taken and the
    /// state does not change**. Enumerating the no-ops individually would
    /// add no safety — `default` satisfies exhaustiveness just as well —
    /// so it would be ceremony, not rigour.
    static func transition(
        _ snapshot: Snapshot,
        _ event: Event
    ) -> (snapshot: Snapshot, effect: Effect) {
        var context = snapshot.context
        switch (snapshot.state, event) {

        // The sheet appeared: show local rows immediately, overlays in
        // flight. Recomputed from any state — it is idempotent, and a
        // re-presented sheet legitimately starts here again.
        case (_, .appeared(let localEntries)):
            guard let localEntries else {
                return (Snapshot(state: .empty, context: context), .none)
            }
            return (
                Snapshot(state: .showingLocal(localEntries), context: context),
                .loadOverlays
            )

        // Overlays landed while local rows are showing → upgrade them.
        case (.showingLocal, .overlaysResolved(let entries)):
            guard let entries else {
                return (Snapshot(state: .empty, context: context), .none)
            }
            return (Snapshot(state: .ready(entries), context: context), .none)

        // A tap is legal only with rows on screen. This guard is what
        // makes a double-tap impossible: `.switching` is unreachable from
        // `.switching`, so two switches can never overlap.
        case (.showingLocal, .rowTapped(let subject, let avatarURL, let username)),
             (.ready, .rowTapped(let subject, let avatarURL, let username)):
            return (
                Snapshot(
                    state: .switching(subject: subject, avatarURL: avatarURL, username: username),
                    context: context
                ),
                .performSwitch(subject: subject, avatarURL: avatarURL, username: username)
            )

        // Switch finished → dismiss, recording the outcome for callers.
        case (.switching, .switchCompleted(let failed)):
            context.lastSwitchFailed = failed
            return (Snapshot(state: .dismissed, context: context), .dismiss)

        // No transition taken — the state does not change.
        default:
            return (snapshot, .none)
        }
    }
}
