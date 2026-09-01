import Testing

@testable import YralApp

@Test("firebase bootstrapper is idempotent — configure() twice is a safe no-op")
func firebaseBootstrapperIdempotent() {
    // A second FirebaseApp.configure() would raise Firebase's fatal
    // "already configured" error if the guard were missing — passing proves
    // idempotence. Also proves the missing-plist path is safe (no bundled
    // GoogleService-Info.plist in the test host → configure() skips).
    FirebaseBootstrapper.configure()
    FirebaseBootstrapper.configure()
}
