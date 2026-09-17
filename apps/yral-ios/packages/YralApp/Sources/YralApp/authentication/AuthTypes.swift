import Foundation

/// Social providers — port of Kotlin `SocialProvider`.
public enum SocialProvider: String, Sendable {
  case google
  case apple
  case phone

  /// Wire value of the OAuth `provider` query parameter.
  public var wireValue: String { rawValue }

  /// Kotlin `responseMode()`: Apple uses form_post; Google uses query.
  public var responseMode: String {
    self == .apple ? "form_post" : "query"
  }

  /// Kotlin `authScope()`: Apple gets name+email; others get openid.
  public var authScope: String {
    self == .apple ? "name email" : "openid"
  }
}

/// Token-expiry logout causes — Kotlin `AuthSessionCause`. Carried as the
/// payload of `AuthMachine.State.signedOut`, so the reason a session ended
/// travels with the state rather than in a parallel field (the analytics
/// event lands with the analytics phase; tests assert on the cause).
public enum AuthExpiryCause: String, Sendable {
  case refreshTokenMissing = "REFRESH_TOKEN_MISSING"
  case refreshTokenExpiredOrInvalid = "REFRESH_TOKEN_EXPIRED_OR_INVALID"
  case refreshAccessTokenFailed = "REFRESH_ACCESS_TOKEN_FAILED"
}
