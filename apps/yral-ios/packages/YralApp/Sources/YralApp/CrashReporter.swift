import Foundation
import FirebaseCore
import FirebaseCrashlytics

/// The app's single error-reporting entry point — thin facade over
/// Crashlytics non-fatal recording (Apple-canonical `record(error:)`,
/// Firebase docs "Report non-fatal exceptions").
///
/// Why a facade: (1) every HANDLED API error must reach Crashlytics
/// (Hard Rule: upstream errors are surfaced AND reported, never
/// swallowed into a UI label only); (2) `record(error:)` groups issues
/// by NSError `domain`+`code`, so the mapping must be STABLE and
/// low-cardinality — unique values (status codes, subjects, timestamps)
/// go in `userInfo`, which Crashlytics shows in the issue's Keys tab;
/// (3) no-op when Firebase is not configured (unit tests, previews) so
/// call sites never need `#if` gates.
///
/// The current thread's stack trace is captured by `record(error:)`
/// itself; `log(_:)` adds breadcrumb context. Non-fatals are buffered
/// on-device and delivered on the next app launch.
enum CrashReporter {

    /// Stable domain for ALL recorded app errors — grouping happens on
    /// domain+code, so this never varies per call site.
    private static let errorDomain = "com.yral.iosApp"

    /// Records a handled error as a Crashlytics non-fatal.
    ///
    /// - Parameters:
    ///   - error: the error being handled (its description — including
    ///     the upstream API status/body per the verbatim-errors rule —
    ///     lands in `userInfo` and the issue's Keys/logs tabs).
    ///   - context: the operation that failed, e.g. "persona-generation".
    static func record(
        _ error: any Error,
        context: String,
        file: String = #fileID,
        function: String = #function
    ) {
        guard FirebaseApp.app() != nil else { return }
        let nsError = NSError(
            domain: errorDomain,
            code: stableCode(for: error),
            userInfo: [
                NSLocalizedDescriptionKey: String(describing: error),
                "context": context,
                "site": "\(file) \(function)"
            ]
        )
        Crashlytics.crashlytics().record(error: nsError)
    }

    /// Adds a breadcrumb log associated with subsequent recorded events.
    /// (64 kB ring buffer per session per the Firebase docs.)
    static func log(_ message: String) {
        guard FirebaseApp.app() != nil else { return }
        Crashlytics.crashlytics().log(message)
    }

    /// Stable, low-cardinality issue-grouping codes — one per error
    /// KIND, never per instance (per-instance values like status codes
    /// live in `userInfo`; docs: unique domain/code values cause high
    /// cardinality and Crashlytics limits reporting).
    static func stableCode(for error: any Error) -> Int {
        switch error as? NetworkError {
        case .transport: return 1
        case .http: return 2
        case .notAuthenticated: return 3
        case .none: return 0
        }
    }
}
