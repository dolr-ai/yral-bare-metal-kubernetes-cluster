import Foundation

/// Analytics identity + tracker lifecycle, as a finite state machine.
///
/// Why this exists at all: the tracker attributes EVERY event to whichever
/// user was set last, and nothing else in the app owns that. The failure it
/// prevents is concrete — sign out of account A, sign in as B, and without a
/// single owner of attribution you get B's session carrying A's `user_id`
/// until something remembers to clear it. Account switching (main → bot → main)
/// multiplies the same hazard. So the machine's job is not ceremony: it is the
/// one place that decides *who an event belongs to*, and it makes the illegal
/// combination (an event attributed to a previously-signed-in user)
/// unrepresentable rather than merely not-yet-observed.
///
/// Shape (verified against Snowplow iOS tracker 6.3.0, not assumed):
///
///  * `Snowplow.createTracker` is SYNCHRONOUS and returns a non-optional
///    `TrackerController` — so there is no observable "initializing" phase.
///    An `initializing` state would have zero duration and zero events, i.e.
///    be unreachable; it is therefore not modelled. The client installs the
///    tracker and reports the outcome as the first event instead.
///  * The tracker starts with tracking ENABLED (`TrackerDefaults` →
///    `dataCollection = true`), which is why `.trackerFailed` is a real
///    terminal state and not a startup formality: it is only reached when
///    installation genuinely failed.
///
/// The finite state carries its own payload — `identified` holds the
/// `user_id`, because a `Bool` could not tell an A→B switch from a no-op
/// re-assertion of A. Context holds only what is genuinely shared across
/// states, and today that is nothing, so there is no `Context` struct: an
/// empty one would wrap every snapshot in a field nobody reads.
///
/// Reference: root AGENTS.md, "Finite State Machines for Stateful Logic"
/// (XState model — ported, not imported; the SDK has no state machine).
public enum AnalyticsMachine {

  // MARK: - Finite state

  /// Who analytics events are attributed to.
  public enum State: Equatable, Sendable {
    /// No identity has been established — cold start before the first
    /// session restore, or a signed-out user on the sign-in surface.
    /// Events are still valid; they carry no `user_id`.
    case anonymous
    /// Events are attributed to `userId` (the session's `userSubject`).
    ///
    /// The payload is the id itself, NOT a flag: that is what makes an
    /// identity *switch* visible as a transition and lets a test assert
    /// that the id actually changed.
    case identified(userId: String)
    /// The tracker could not be installed, so no event can be sent.
    /// Final — a failed tracker install is not retried within a launch
    /// (a retry loop would duplicate events if the first attempt half
    /// succeeded; the next launch tries again).
    case disabled(cause: String)

    public static let start = State.anonymous

    /// The user id events are attributed to, or nil when anonymous or
    /// disabled. This is the machine's whole outward contract for
    /// attribution — read it, never recompute it.
    public var attributionUserId: String? {
      if case .identified(let userId) = self { return userId }
      return nil
    }

    /// Whether an event should reach the tracker. Disabled is the only
    /// state that suppresses; anonymous events are still sent (they are
    /// the funnel's top, not noise).
    public var canSend: Bool {
      if case .disabled = self { return false }
      return true
    }
  }

  /// A machine snapshot. No context component — see the type doc.
  public struct Snapshot: Equatable, Sendable {
    public var state: State

    public static let initial = Snapshot(state: .start)

    public init(state: State) {
      self.state = state
    }
  }

  // MARK: - Events

  /// Every way analytics identity changes.
  ///
  /// Three events, and each one changes the state — there is deliberately no
  /// "installation succeeded" event, because success needs no transition:
  /// `.anonymous` already means "no identity, tracker healthy", which is
  /// exactly the post-install situation. Only the failure is news.
  public enum Event: Equatable, Sendable {
    /// Tracker installation failed; `cause` is the failure description
    /// (also reported to `CrashReporter` by the caller).
    case trackerFailed(cause: String)
    /// A session became current — restored, signed in, signed in as a bot,
    /// or switched between the main account and a bot. One event for all of
    /// them: the transition is identical, only the id differs.
    case identityEstablished(userId: String)
    /// The session ended (sign-out, account deletion, or expiry).
    case identityCleared
  }

  // MARK: - Transition

  /// Pure: `(snapshot, event) -> snapshot`.
  ///
  /// There are no effects. The tracker itself is the only I/O, and the
  /// client owns it — `Snowplow.createTracker` runs before the machine's
  /// first event, and setting the attribution id is a memory write with no
  /// failure mode. Inventing an effect enum here would be the ceremony the
  /// root AGENTS.md warns against.
  ///
  /// Only transitions that DO something are enumerated; everything else
  /// falls to `default`, which is XState's documented "no transition is
  /// taken" (the state does not change). A no-op is deliberately NOT
  /// enumerated per state: that would add a line every time a state is added
  /// without forcing any real decision. The one exception is the final-state
  /// guard below, which is XState's documented *forbidden transition* — it
  /// exists to BLOCK the wildcard arms, not to record that nothing happened.
  public static func transition(
    _ snapshot: Snapshot,
    _ event: Event
  ) -> Snapshot {
    // `.disabled` is FINAL, and a final state has no outgoing transitions —
    // enforced here, once, rather than by remembering to exclude `.disabled`
    // from every arm below. Without this the wildcard `identityEstablished`
    // arm revives a dead tracker (a defect the tests caught).
    if case .disabled = snapshot.state { return snapshot }

    switch (snapshot.state, event) {

    // Installation failed. Reachable from the non-final states, because a
    // tracker can be re-installed after a reconfiguration and a failure there
    // is just as final.
    case (_, .trackerFailed(let cause)):
      return Snapshot(state: .disabled(cause: cause))

    // A session became current. `identified` replaces whatever id was
    // there — this arm is what makes an A→B switch carry B, not A.
    case (_, .identityEstablished(let userId)):
      return Snapshot(state: .identified(userId: userId))

    // The session ended. `.disabled` is deliberately absent: a disabled
    // tracker stays disabled (nothing can be sent either way), so the
    // `default` arm preserves it rather than silently reviving it.
    case (.identified, .identityCleared):
      return Snapshot(state: .anonymous)

    // No transition taken — the state does not change.
    default:
      return snapshot
    }
  }
}
