import Foundation

/// Authentication lifecycle as a finite state machine.
///
/// Replaces `SessionState` (`initial` / `loading` / `signedIn`), which was
/// an enum but not a machine: nothing owned the transitions, `updateState`
/// was called from seven files, and any caller could set any state at any
/// time. Two concrete consequences drove this:
///
///  * `.initial` did double duty — "never had a session" AND "had one, it
///    expired" — so the `AuthExpiryCause` that the token paths compute had
///    nowhere to live.
///  * `updateState` took a `SessionState` but also reset `SessionProperties`
///    as a hidden side effect, even for `.loading`/`.initial`, which are not
///    session changes.
///
/// Mirrors the XState model (root AGENTS.md, "Finite State Machines for
/// Stateful Logic"): `State` is the finite state and each variant carries
/// its own payload; `Context` holds the data every state shares. There is no
/// separate "loading" bool and no `isAIAccount` flag to compare — the bot
/// case is a state.
public enum AuthMachine {

  // MARK: - Finite state

  /// Where authentication is. Payload lives in the variant.
  public enum State: Equatable, Sendable {
    /// No session has ever existed on this device. Cold start before
    /// the first restore, or a fresh install. Routes to sign-in.
    case initial
    /// A session is being restored or obtained — cold-start keychain
    /// read, token refresh, OAuth code exchange. Routes to the splash.
    case restoring
    /// A session was held and is now gone. `cause` records why and is
    /// `nil` for a deliberate user sign-out — this is the distinction
    /// `.initial` used to swallow (it is the analytics dimension the
    /// token paths already compute).
    case signedOut(cause: AuthExpiryCause?)
    /// Signed in to the main account.
    case signedIn(Session)
    /// Signed in as one of the main account's bots.
    ///
    /// A distinct state, not a flag: a bot session shares the parent's
    /// tokens (so `updateFirebaseLoginState` must not be set), deleting
    /// a bot returns to the main account rather than logging out, and
    /// the profile shows the AI badge.
    case signedInAsBot(Session)

    public static let start = State.initial

    /// The session, under either identity. Nil when not signed in.
    public var session: Session? {
      switch self {
      case .signedIn(let session), .signedInAsBot(let session): session
      case .initial, .restoring, .signedOut: nil
      }
    }

    /// True while a restore/sign-in is genuinely in flight.
    public var isRestoring: Bool {
      if case .restoring = self { return true }
      return false
    }

    /// True for a bot session — the branch the deletion flow needs.
    public var isBotSession: Bool {
      if case .signedInAsBot = self { return true }
      return false
    }
  }

  // MARK: - Context

  /// Session-adjacent data shared across states (Kotlin
  /// `SessionProperties`). Genuinely shared — a signed-in state reads it,
  /// leaving one clears it.
  ///
  /// Public because `Snapshot` is public API of the machine and this is
  /// its second component; the field itself stays internal so only the
  /// machine (via `SessionStore`) can assign it.
  public struct Context: Equatable, Sendable {
    public internal(set) var properties = SessionProperties()

    public init(properties: SessionProperties = SessionProperties()) {
      self.properties = properties
    }
  }

  /// A machine snapshot — exactly `(state, context)`, matching XState.
  public struct Snapshot: Equatable, Sendable {
    public var state: State
    public var context: Context

    public static let initial = Snapshot(state: .initial, context: Context())
  }

  // MARK: - Events

  /// Every way auth state changes. `AuthClient` emits these from its I/O
  /// paths; nothing sets the state directly.
  public enum Event: Equatable, Sendable {
    /// Cold start began.
    case restoreStarted
    /// A session became current — restored from cache, refreshed, or
    /// obtained via sign-in/OAuth/anonymous identity/account switch.
    /// One event for all of them: the transition is identical, so a
    /// separate "restored" variant would be ceremony.
    case sessionEstablished(Session)
    /// Cold start found nothing cached — a fresh install, not a logout.
    case nothingCached
    /// The session ended without the user asking (token missing,
    /// expired, or refresh failed).
    case sessionExpired(cause: AuthExpiryCause)
    /// The user signed out.
    case userSignedOut
  }

  // MARK: - Effects

  /// Side effects the caller must perform. `transition` is pure, so it
  /// *describes* I/O instead of doing it.
  public enum Effect: Equatable, Sendable {
    /// Clear the keychain tokens and cached session fields.
    case clearStoredSession
    case none
  }

  // MARK: - Transition

  /// Pure: `(snapshot, event) -> (snapshot, effect)`.
  ///
  /// Only the transitions that DO something are listed; everything else
  /// falls to `default`, which is XState's documented "no transition is
  /// taken" behaviour.
  public static func transition(
    _ snapshot: Snapshot,
    _ event: Event
  ) -> (snapshot: Snapshot, effect: Effect) {
    switch (snapshot.state, event) {

    // Cold start — accept from any state: a re-launch legitimately
    // restores again, and it is idempotent.
    case (_, .restoreStarted):
      return (commit(.restoring, snapshot), .none)

    // A session became current. One arm covers restore, sign-in,
    // anonymous identity, and account switching; the bot variant is
    // chosen by the payload.
    case (_, .sessionEstablished(let session)):
      return (
        commit(session.isAIAccount ? .signedInAsBot(session) : .signedIn(session), snapshot), .none
      )

    // Cold start found nothing — back to the sign-in surface, and NOT
    // a signed-out case (there was never a session to expire).
    case (.restoring, .nothingCached):
      return (commit(.initial, snapshot), .none)

    // The session ended without the user asking. `.initial` is
    // included: an anonymous identity can expire before the first
    // real sign-in, and that is still an expiry, not a fresh install.
    case (_, .sessionExpired(let cause)):
      return (commit(.signedOut(cause: cause), snapshot), .clearStoredSession)

    // The user signed out — `nil` cause distinguishes it from expiry.
    case (_, .userSignedOut):
      return (commit(.signedOut(cause: nil), snapshot), .clearStoredSession)

    // No transition taken — the state does not change. `commit` is not
    // called, so the context is untouched too.
    default:
      return (snapshot, .none)
    }
  }

  /// Applies a state change and the context update that belongs with it.
  ///
  /// Kotlin's `updateState` reset `SessionProperties` on every call; here
  /// it is an explicit part of the transition, so the reset rule is pure
  /// and testable, and only fires when the session actually *changes* —
  /// a no-op event (the `default` arm) never clears properties.
  private static func commit(_ state: State, _ snapshot: Snapshot) -> Snapshot {
    Snapshot(
      state: state,
      context: Context(
        properties: resetProperties(
          onSessionChange: snapshot.state, next: state, from: snapshot.context.properties)
      ))
  }

  /// Per-session values are cleared, device-level values preserved —
  /// exactly the Kotlin `updateState` reset (botCount, accountDirectory,
  /// isYralProAvailable survive a session change). Pure.
  ///
  /// The trigger is the SESSION changing, not the state changing:
  /// `.initial → .restoring` is a different state but the same (absent)
  /// session, and wiping a balance there would be the very bug
  /// `updateState` had (it reset on every call, including `.loading`).
  static func resetProperties(
    onSessionChange previous: State,
    next: State,
    from properties: SessionProperties
  ) -> SessionProperties {
    guard previous.session != next.session else { return properties }
    return SessionProperties(
      botCount: properties.botCount,
      accountDirectory: properties.accountDirectory,
      isYralProAvailable: properties.isYralProAvailable
    )
  }
}
