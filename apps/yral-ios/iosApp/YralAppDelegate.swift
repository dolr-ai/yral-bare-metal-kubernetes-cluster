import UIKit
import YralApp

/// Thin AppDelegate — the *only* UIKit lifecycle code in the shell.
///
/// All product behavior lives in the `YralApp` SPM package; the shell's job is
/// to wire the package's entry points into app lifecycle events. Feature
/// phases will grow this adapter (push notifications in Phase 2, deep links
/// in Phase 2) by delegating to package-side handlers, never by hosting logic
/// here.
final class YralAppDelegate: NSObject, UIApplicationDelegate {

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        // Firebase first: Crashlytics must capture every launch, and Analytics
        // session data must be attributed to the full launch window.
        YralAppRoot.configureFirebase()
        return true
    }
}
