import Foundation

/// The typed analytics event catalog — one case per event we actually emit.
///
/// **Structured events only.** The Snowplow Enrich resolver in our cluster
/// registers Iglu Central as its sole schema repository
/// (`kubernetes/infrastructure/snowplow/iglu-resolver-config.yaml`), and Iglu
/// Central only holds Snowplow's own schemas. A custom self-describing event
/// (`iglu:com.yral/...`) would therefore fail schema resolution mid-pipeline
/// and land in `snowplow-enrich-bad` instead of `snowplow-enriched`. Every
/// event here is a Snowplow atomic `Structured` event (`e: "se"`), whose
/// schema IS on Iglu Central.
///
/// **Wire shape matches the legacy Kotlin app**, so existing dashboards and
/// downstream consumers keep working across the migration:
///
/// | Structured field | Value |
/// |---|---|
/// | `se_ca` (category) | the feature name — `Features.getFeatureName()` |
/// | `se_ac` (action) | the event name — `FeatureEvents.getEventName()` |
/// | `se_pr` (property) | this event's fields as a JSON **string** |
///
/// Two constraints on `se_pr`, both from Snowplow's `atomic` schema:
///
///  * it is `maxLength: 1000`, so the encoded payload is truncated to fit —
///    over-length events become `atomic_field_length_exceeded` bad rows, and
///    truncation is what the Kotlin provider does (`MAX_SE_PROPERTY_LENGTH`).
///  * Snowplow applies NO parsing to it, so any consumer reads it as a JSON
///    string and parses it. That is intentional: it keeps the field count
///    stable on the wire while letting events differ in shape.
///
/// The `cx` context array carries session/device/platform data automatically
/// (the tracker attaches it), which is why `base64Encoding` must be `false` —
/// with base64 on, contexts land in the opaque `co` field instead of `cx`.
public enum AnalyticsEvent: Equatable, Sendable {

  /// First launch of an installed app — the funnel's top.
  ///
  /// The Kotlin original sends this with a `date_time`; we send the ISO-8601
  /// timestamp of the launch instead, because the collector stamps every
  /// event with `collector_tstamp` anyway and a client clock is the less
  /// trustworthy of the two. `device_timestamp` still carries the client
  /// time.
  case appLaunch

  /// A sign-in completed. Fired after the token exchange succeeded, so it
  /// means "we hold a session", not "the user tapped the button".
  case loginSuccess(provider: SocialProvider)

  /// A session ended — user-initiated sign-out, expiry, or account deletion.
  ///
  /// `cause` is nil for a deliberate sign-out (mirroring
  /// `AuthMachine.State.signedOut(cause:)`, where nil and a value are the
  /// two distinct cases) and the expiry reason otherwise.
  case logout(cause: AuthExpiryCause?)

  /// A sign-in attempt failed.
  case authFailed(provider: SocialProvider)

  // MARK: - Wire mapping

  /// The `se_ca` value. Matches Kotlin `Features.getFeatureName()`.
  public var featureName: String {
    switch self {
    case .appLaunch:
      return "app"
    case .loginSuccess, .logout, .authFailed:
      return "auth"
    }
  }

  /// The `se_ac` value. Matches Kotlin `FeatureEvents.getEventName()`.
  public var eventName: String {
    switch self {
    case .appLaunch: return "first_app_launch"
    case .loginSuccess: return "login_success"
    case .logout: return "auth_session_state_changed"
    case .authFailed: return "auth_failed"
    }
  }

  /// The fields that become `se_pr`, in a stable order.
  ///
  /// Values are strings because `se_pr` is a single JSON string field — but
  /// they are built as a real dictionary and serialized with
  /// `JSONSerialization`, NOT by hand-concatenating `"key":"value"` pairs.
  /// The Kotlin provider hand-rolls that join, which is why it needs manual
  /// backslash/quote escaping; `JSONSerialization` cannot emit malformed JSON
  /// from a Swift value, so that whole class of bug is designed out.
  ///
  /// Pure — no clock, no environment. `appLaunch` carries no fields: the
  /// collector stamps every event with `collector_tstamp` and the tracker
  /// adds `dvce_created_tstamp`, so the Kotlin `date_time` field (a client
  /// clock, the least trustworthy of the three) is not worth reproducing.
  public var properties: [String: String] {
    switch self {
    case .appLaunch:
      return [:]
    case .loginSuccess(let provider):
      return ["auth_journey": provider.wireValue]
    case .logout(let cause):
      var fields = [
        "from_state": "authenticated",
        "to_state": "unauthenticated",
        "initiator": cause == nil ? "user" : "system",
      ]
      // Omitted entirely when nil, matching Kotlin's
      // `.filterValues { it != "null" }` — an absent `cause` is more
      // honest downstream than a literal "null" string.
      if let cause { fields["cause"] = cause.analyticsValue }
      return fields
    case .authFailed(let provider):
      return ["auth_journey": provider.wireValue]
    }
  }
}

extension AuthExpiryCause {
  /// The `cause` value on `auth_session_state_changed`. Kotlin
  /// `AuthSessionCause` serializes lowercase (`refresh_token_missing`), not
  /// the raw SCREAMING_CASE token that `AuthExpiryCause.rawValue` holds.
  var analyticsValue: String { rawValue.lowercased() }
}
