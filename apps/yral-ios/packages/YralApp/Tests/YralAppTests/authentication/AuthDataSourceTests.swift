import Testing
import Foundation
@testable import YralApp

/// Tests for `AuthDataSource` redirect construction — 1:1 with the
/// Kotlin `AuthEnv.RedirectUri` shape (`<scheme>://oauth/callback`).
struct AuthDataSourceTests {

    @Test("redirect URI is scheme://oauth/callback")
    func redirectURI() {
        #expect(AuthDataSource.redirectURI(scheme: "com.yral.iosApp")
                == "com.yral.iosApp://oauth/callback")
    }
}
