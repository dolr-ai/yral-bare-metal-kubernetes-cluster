import SwiftUI
import YralApp

@main
struct YralApp: App {
    /// `UIApplicationDelegateAdaptor` keeps UIKit lifecycle callbacks (push
    /// notifications, deep links, background tasks) available while the app
    /// itself is a pure SwiftUI `App`. Firebase bootstrap happens here, before
    /// the first scene is presented.
    @UIApplicationDelegateAdaptor(YralAppDelegate.self)
    private var appDelegate

    var body: some Scene {
        WindowGroup {
            YralAppRoot.makeRootScene()
        }
    }
}
