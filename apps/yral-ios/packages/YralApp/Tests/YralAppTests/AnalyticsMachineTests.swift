import Foundation
import Testing

@testable import YralApp

/// Tests for the analytics identity machine and its projection from
/// `SessionStore`.
///
/// Two layers, deliberately:
///
///  1. `AnalyticsMachine.transition` — pure, so every transition is asserted
///     directly with no session, no network, and no tracker.
///  2. `SessionStore`'s projection — asserted by driving the REAL auth
///     transitions and reading `analyticsUserId`, so the projection cannot
///     drift from the auth lifecycle without a test failing.
///
/// No test here constructs an `AnalyticsClient`. Doing so would install a real
/// Snowplow tracker in the test process — creating the SDK's SQLite event store
/// and pointing it at the live collector. The machine and the projection are
/// pure, which is exactly why they can be tested without one.
@MainActor
struct AnalyticsMachineTests {

  // MARK: - Pure machine

  @Test("starts anonymous")
  func startsAnonymous() {
    #expect(AnalyticsMachine.Snapshot.initial.state == .anonymous)
    #expect(AnalyticsMachine.Snapshot.initial.state.attributionUserId == nil)
    #expect(AnalyticsMachine.Snapshot.initial.state.canSend)
  }

  @Test("identityEstablished attributes events, and re-attributes on a switch")
  func identityEstablishedAttributes() {
    let first = AnalyticsMachine.transition(.initial, .identityEstablished(userId: "user-a"))
    #expect(first.state == .identified(userId: "user-a"))
    #expect(first.state.attributionUserId == "user-a")

    // The switch: same state variant, different payload. This is why the
    // payload is the id itself and not a Bool.
    let second = AnalyticsMachine.transition(first, .identityEstablished(userId: "user-b"))
    #expect(second.state == .identified(userId: "user-b"))
    #expect(second.state.attributionUserId == "user-b")
  }

  @Test("identityCleared returns to anonymous and drops attribution")
  func identityClearedReturnsToAnonymous() {
    let identified = AnalyticsMachine.Snapshot(state: .identified(userId: "user-a"))
    let cleared = AnalyticsMachine.transition(identified, .identityCleared)
    #expect(cleared.state == .anonymous)
    #expect(cleared.state.attributionUserId == nil)
  }

  @Test("identityCleared while already anonymous is a no-op")
  func identityClearedWhileAnonymousIsNoOp() {
    let next = AnalyticsMachine.transition(.initial, .identityCleared)
    #expect(next.state == .anonymous)
  }

  @Test("trackerFailed disables delivery from any state and is final")
  func trackerFailedDisables() {
    let states: [AnalyticsMachine.State] = [
      .anonymous,
      .identified(userId: "user-a"),
    ]
    for state in states {
      let start = AnalyticsMachine.Snapshot(state: state)
      let failed = AnalyticsMachine.transition(start, .trackerFailed(cause: "install failed"))
      #expect(failed.state == .disabled(cause: "install failed"))
      #expect(!failed.state.canSend)
      #expect(failed.state.attributionUserId == nil)

      // Final: a later identity event does not revive delivery.
      let afterIdentity = AnalyticsMachine.transition(
        failed, .identityEstablished(userId: "user-b"))
      #expect(afterIdentity.state == .disabled(cause: "install failed"))
      #expect(!afterIdentity.state.canSend)
    }
  }

  @Test("a disabled tracker stays disabled through identityCleared")
  func disabledStaysDisabled() {
    let disabled = AnalyticsMachine.Snapshot(state: .disabled(cause: "install failed"))
    let next = AnalyticsMachine.transition(disabled, .identityCleared)
    #expect(next.state == .disabled(cause: "install failed"))
  }

  // MARK: - Projection from the auth lifecycle

  @Test("signing in projects attribution from the session subject")
  func signingInProjectsAttribution() {
    let store = SessionStore()
    store.send(.sessionEstablished(Session(userSubject: "user-a", isAIAccount: false)))

    #expect(store.analyticsUserId == "user-a")
  }

  /// The regression that motivated the machine: sign out of A, sign in as B,
  /// and attribution afterwards must be B. The defect was a tracker that kept
  /// A's id because nothing owned the transition.
  @Test("switching accounts re-attributes — the previous user's id never persists")
  func switchingAccountsReAttributes() {
    let store = SessionStore()

    store.send(.sessionEstablished(Session(userSubject: "user-a", isAIAccount: false)))
    #expect(store.analyticsUserId == "user-a")

    store.send(.userSignedOut)
    #expect(store.analyticsUserId == nil)

    store.send(.sessionEstablished(Session(userSubject: "user-b", isAIAccount: false)))
    #expect(store.analyticsUserId == "user-b")
    #expect(store.analyticsUserId != "user-a")
  }

  /// A main → bot switch is a session change whose `isAIAccount` differs while
  /// the state variant has the same shape — attribution must follow the new
  /// subject.
  @Test("switching to a bot account re-attributes")
  func switchingToBotReAttributes() {
    let store = SessionStore()

    store.send(.sessionEstablished(Session(userSubject: "main-subject", isAIAccount: false)))
    store.send(.sessionEstablished(Session(userSubject: "bot-subject", isAIAccount: true)))

    #expect(store.analyticsUserId == "bot-subject")
  }

  @Test("expiry clears attribution as well as a deliberate sign-out")
  func expiryClearsAttribution() {
    let store = SessionStore()

    store.send(.sessionEstablished(Session(userSubject: "user-a", isAIAccount: false)))
    store.send(.sessionExpired(cause: .refreshTokenExpiredOrInvalid))

    #expect(store.analyticsUserId == nil)
  }

  @Test("a session with no subject stays anonymous")
  func sessionWithoutSubjectStaysAnonymous() {
    let store = SessionStore()
    store.send(.sessionEstablished(Session(userSubject: nil, isAIAccount: false)))

    #expect(store.analyticsUserId == nil)
  }

  @Test("cold start sends no identity — anonymous is the start state")
  func coldStartProjectsNil() {
    let store = SessionStore()
    store.send(.restoreStarted)

    #expect(store.analyticsUserId == nil)
  }

  @Test("every transition notifies the handler with the current projection")
  func everyTransitionNotifiesHandler() {
    let store = SessionStore()
    var observed: [String?] = []
    store.identityChangeHandler = { observed.append($0) }

    store.send(.restoreStarted)
    store.send(.sessionEstablished(Session(userSubject: "user-a", isAIAccount: false)))
    store.send(.userSignedOut)

    #expect(observed == [nil, "user-a", nil])
  }

  // MARK: - Wire format

  @Test("se_pr is sorted, compact JSON with no hand-rolled escaping")
  func encodedPropertyIsStableJSON() {
    let encoded = AnalyticsClient.encodeProperty([
      "auth_journey": "google",
      "quote": #"he said "hi" and \ bye"#,
    ])
    // Keys sorted ⇒ byte-stable, which is what makes the wire assertable.
    #expect(encoded == #"{"auth_journey":"google","quote":"he said \"hi\" and \\ bye"}"#)
  }

  @Test("empty properties encode to an empty JSON object")
  func emptyPropertiesEncodeToEmptyObject() {
    #expect(AnalyticsClient.encodeProperty([:]) == "{}")
  }

  @Test("se_pr is truncated to the atomic schema's 1000-character limit")
  func encodedPropertyIsTruncated() {
    let encoded = AnalyticsClient.encodeProperty(["blob": String(repeating: "x", count: 2000)])
    #expect(encoded.count == 1000)
    #expect(encoded.hasSuffix("..."))
  }

  @Test("a payload at exactly the limit is not truncated")
  func encodedPropertyAtLimitIsUntouched() {
    // `{"blob":""}` is 11 characters of JSON overhead, so 989 'x' lands the
    // encoded payload exactly on the 1000-character bound.
    let encoded = AnalyticsClient.encodeProperty(["blob": String(repeating: "x", count: 989)])
    #expect(encoded.count == 1000)
    #expect(!encoded.hasSuffix("..."))
  }

  // MARK: - Event catalog

  @Test("event names and features match the legacy wire values")
  func eventNamesMatchLegacyWireValues() {
    #expect(AnalyticsEvent.appLaunch.featureName == "app")
    #expect(AnalyticsEvent.appLaunch.eventName == "first_app_launch")
    #expect(AnalyticsEvent.loginSuccess(provider: .google).featureName == "auth")
    #expect(AnalyticsEvent.loginSuccess(provider: .google).eventName == "login_success")
    #expect(AnalyticsEvent.logout(cause: nil).eventName == "auth_session_state_changed")
    #expect(AnalyticsEvent.authFailed(provider: .apple).eventName == "auth_failed")
  }

  @Test("appLaunch carries no fields — the collector stamps the time")
  func appLaunchHasNoProperties() {
    #expect(AnalyticsEvent.appLaunch.properties.isEmpty)
  }

  @Test("loginSuccess carries the auth journey")
  func loginSuccessCarriesJourney() {
    #expect(
      AnalyticsEvent.loginSuccess(provider: .phone).properties == ["auth_journey": "phone"])
  }

  @Test("a deliberate logout omits the cause, an expiry includes it")
  func logoutCauseIsOmittedWhenAbsent() {
    let deliberate = AnalyticsEvent.logout(cause: nil).properties
    #expect(deliberate["cause"] == nil)
    #expect(deliberate["initiator"] == "user")

    let expired = AnalyticsEvent.logout(cause: .refreshTokenMissing).properties
    // Lowercase, matching Kotlin's serialized `AuthSessionCause`.
    #expect(expired["cause"] == "refresh_token_missing")
    #expect(expired["initiator"] == "system")
  }

  @Test("authFailed carries the journey that failed")
  func authFailedCarriesJourney() {
    #expect(AnalyticsEvent.authFailed(provider: .apple).properties == ["auth_journey": "apple"])
  }
}
