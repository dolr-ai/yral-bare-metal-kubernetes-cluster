import Foundation
import SnowplowTracker

/// The app's single analytics entry point — thin facade over the Snowplow iOS
/// tracker, mirroring `CrashReporter`'s shape.
///
/// **Where events go.** The tracker POSTs to our self-hosted Scala Stream
/// Collector at `AppConfiguration.snowplowCollectorURL`
/// (`snowplow-collector.yral.com`, path
/// `/com.snowplowanalytics.snowplow/tp2`, appended by the SDK), which writes
/// them straight to the Kafka `snowplow-raw` topic. That collector is the HTTP
/// entry point into the analytics pipeline and is intentionally public —
/// unauthenticated, no oauth2-proxy in front (`kubernetes/networking/routes/
/// snowplow-collector.yaml`), because the Snowplow tracker protocol is not a
/// browser-origin-authenticated flow. The Kafka Bridge
/// (`kafka-bridge.yral.com`) is a DIFFERENT service and is read-only: its
/// `KafkaUser` carries no write ACL, so it can read events back but can never
/// be used to send them.
///
/// **Division of labour.** This type does I/O and nothing else. Attribution
/// decisions live in `AnalyticsMachine` (pure, tested); the event shapes live
/// in `AnalyticsEvent` (pure, tested). Both are consulted here and nowhere
/// else, so there is exactly one place an event can be built or attributed.
///
/// **Dependencies are injected, not reached for.** This type is constructed
/// once at launch by `YralAppRoot.configureAnalytics()` and handed to the
/// objects that emit events; `SessionStore` reaches it only through its
/// `identityChangeHandler` closure, `AuthClient` through a defaulted-nil
/// parameter. That is what keeps tests and previews away from the real
/// tracker — no test ever constructs a client, because the machine
/// (`AnalyticsMachine`) and the projection (`SessionStore.analyticsUserId`)
/// are both pure and testable without one.
///
/// `@MainActor` because the SDK reads app state on the main thread at tracker
/// creation (`AppStateProvider.ensureInitialized`) and because every caller
/// (`SessionStore`, `AuthClient`) is already main-actor isolated.
@MainActor
public final class AnalyticsClient {

  /// Tracker namespace. The SDK keys its persistent event store and its
  /// per-namespace configuration on this string, so it is an opaque,
  /// stable identifier — not a display name.
  private static let trackerNamespace = "yral-ios"

  /// `app_id` on every event. Single-environment app, so this never varies
  /// by build configuration (see apps/yral-ios/AGENTS.md, "one environment").
  private static let applicationIdentifier = "yral-ios"

  /// Snowplow's `atomic` schema caps `se_pr` at 1000 characters; exceeding it
  /// yields `atomic_field_length_exceeded` bad rows. Same bound and same
  /// suffix as the Kotlin provider, so truncated events look identical on
  /// both platforms. The raw topic remains the reprocessing source of truth.
  private static let maximumPropertyLength = 1000
  private static let truncationSuffix = "..."

  private let tracker: TrackerController?
  private var snapshot = AnalyticsMachine.Snapshot.initial

  /// Creates the client and installs the tracker.
  ///
  /// `Snowplow.createTracker` is synchronous and returns a non-optional
  /// controller, so there is no async installation phase to model — hence no
  /// `.initializing` state in the machine. A tracker that fails to install does
  /// so by being inert rather than by throwing (the SDK's failure channel is
  /// its own logging, not a Swift error), so `.disabled` is entered explicitly
  /// via `disable(cause:)` when a caller observes that — see `isEnabled`. It is
  /// never entered silently.
  public init() {
    let network = NetworkConfiguration(
      endpoint: "https://\(AppConfiguration.snowplowCollectorURL)"
    )
    let configuration = TrackerConfiguration()
      .appId(Self.applicationIdentifier)
      .devicePlatform(.mobile)
      // Contexts must land in the `cx` array (parseable JSON) rather than the
      // opaque base64 `co` field — our enrich pipeline expects `cx`, and the
      // Kotlin provider sets this too. The SDK default is `true`.
      .base64Encoding(false)
    // Every other autotracking option is left at the SDK default ON PURPOSE
    // (root AGENTS.md, "Default-First Configuration"): screen views, session,
    // lifecycle, install, application and platform contexts are all enabled by
    // default in tracker 6.3.0 (`TrackerDefaults`), so re-stating them would be
    // noise that re-creates the drift this repo's rules exist to prevent.
    // Screen views in particular are already automatic — which is why this app
    // hand-tracks no screen events at all.
    tracker = Snowplow.createTracker(
      namespace: Self.trackerNamespace,
      network: network,
      configurations: [configuration]
    )
  }

  /// Whether events will actually be delivered.
  public var isEnabled: Bool {
    snapshot.state.canSend && tracker?.isTracking == true
  }

  /// The user id events are currently attributed to, or nil when anonymous.
  /// A derived read of the machine — never a second source of truth.
  public var attributionUserId: String? {
    snapshot.state.attributionUserId
  }

  // MARK: - Identity

  /// Attributes subsequent events to `userId` (the session's `userSubject`).
  ///
  /// Called on every session establishment, including an identity SWITCH
  /// (main → bot → main) — the machine's `identified` payload is replaced, so
  /// an event after a switch can never carry the previous user's id.
  public func setIdentity(userId: String) {
    apply(.identityEstablished(userId: userId))
  }

  /// Clears attribution — sign-out, account deletion, or expiry.
  public func clearIdentity() {
    apply(.identityCleared)
  }

  /// Records a tracker that could not be installed, disabling delivery.
  public func disable(cause: String) {
    apply(.trackerFailed(cause: cause))
  }

  // MARK: - Tracking

  /// Sends one event, attributed per the machine.
  public func track(_ event: AnalyticsEvent) {
    guard snapshot.state.canSend, let tracker, tracker.isTracking else {
      // A dropped event is a real gap in the funnel — surface it rather than
      // letting it vanish (Hard Rule: never swallow failures silently).
      CrashReporter.log("analytics dropped (\(event.eventName)): tracker disabled")
      return
    }

    let structured = Structured(category: event.featureName, action: event.eventName)
    structured.property(Self.encodeProperty(event.properties))
    _ = tracker.track(structured)
  }

  // MARK: - Private

  /// Serializes the event's fields to the `se_pr` JSON string.
  ///
  /// Uses `JSONSerialization` rather than string concatenation, so escaping is
  /// handled by the platform and malformed JSON is not representable. (The
  /// Kotlin provider hand-rolls the join and therefore needs manual
  /// backslash/quote escaping — this designs that bug class out.) Keys are
  /// sorted via `.sortedKeys`, so the payload is byte-stable across calls and
  /// thus assertable in tests and diffable in the raw topic.
  static func encodeProperty(_ properties: [String: String]) -> String {
    // A `[String: String]` always serializes; the fallback exists only to
    // avoid a force-unwrap (Hard Rule: `!` needs a reason, and this needs none).
    let data =
      (try? JSONSerialization.data(
        withJSONObject: properties,
        options: [.sortedKeys]
      )) ?? Data("{}".utf8)
    let json = String(decoding: data, as: UTF8.self)
    guard json.count > Self.maximumPropertyLength else { return json }
    return
      String(json.prefix(Self.maximumPropertyLength - Self.truncationSuffix.count))
      + Self.truncationSuffix
  }

  /// Sends an event through the machine and performs whatever it decides.
  /// The single mutation point for `snapshot` — nothing sets state directly.
  private func apply(_ event: AnalyticsMachine.Event) {
    snapshot = AnalyticsMachine.transition(snapshot, event)
    // Setting the id on the tracker is idempotent and cheap; the machine is
    // what decides WHICH id that is, including the nil case on clear.
    tracker?.subject?.userId = snapshot.state.attributionUserId
  }
}
