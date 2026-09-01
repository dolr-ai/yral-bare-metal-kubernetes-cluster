import Foundation

/// Typed network errors, mirroring the Kotlin `NetworkException` +
/// `HTTPResponseStatus` taxonomy. Every transport in this package throws
/// these — call sites inline their own URLSession status checks and funnel
/// failures into these cases.
public enum NetworkError: Error, Equatable {
    /// Any transport-level failure (connection, TLS, timeout, decoding).
    case transport(underlying: String)
    /// Non-success HTTP status (Ktor `expectSuccess = true` equivalent).
    case http(statusCode: Int, body: String?)
    /// A write was attempted without an ID token.
    case notAuthenticated(description: String)
}

/// Matches Kotlin's `AuthDnsFailureDetector` — recognizes DNS resolution
/// failures in transport errors so callers can report them to Crashlytics
/// without misclassifying ordinary connectivity issues.
public func isDnsLookupFailure(_ error: Error) -> Bool {
    let nsError = error as NSError
    return nsError.domain == NSURLErrorDomain
        && (nsError.code == NSURLErrorCannotFindHost || nsError.code == NSURLErrorCannotConnectToHost)
}
