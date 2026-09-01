import Testing
import Foundation
@testable import YralApp

/// Tests for `YralAuthDataSource` redirect construction — 1:1 with the
/// Kotlin `AuthEnv.RedirectUri` shape (`<scheme>://oauth/callback`).
struct YralAuthDataSourceTests {

    @Test("redirect URI is scheme://oauth/callback")
    func redirectURI() {
        #expect(YralAuthDataSource.redirectURI(scheme: "com.yral.iosApp")
                == "com.yral.iosApp://oauth/callback")
    }
}
