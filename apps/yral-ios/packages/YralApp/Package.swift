// swift-tools-version:6.2
// YralApp — the single SPM package holding all Yral iOS app code.
//
// Conventions (see apps/yral-ios/AGENTS.md):
//   - ALL product code lives in this package; the committed iosApp.xcodeproj
//     is a thin shell (targets + signing + assets) with no product code.
//   - One folder per top-level screen/feature under Sources/YralApp/
//     (authentication/, settings/…), logic colocated inside the feature
//     folder; cross-feature infrastructure stays at the root. SPM
//     requires one directory per target, so test files cannot share the
//     source folder; instead Tests/YralAppTests/ mirrors the source
//     tree's folders with one test file per tested file, named after it
//     (e.g. authentication/PKCEAndJWTParserTests.swift tests PKCE.swift).
//   - Day-to-day editing happens in VS Code via the Swift extension.
//   - Third-party dependencies are declared HERE, not in the Xcode project.
//
// Dependency policy: exact-pinned versions. Bump deliberately; never floating.
// Firebase Apple SDK release notes: https://firebase.google.com/support/release-notes/ios
import PackageDescription

let firebaseAppleSdkVersion: Version = "12.18.0"

let package = Package(
    name: "YralApp",
    platforms: [
        .iOS(.v18),
        // macOS is declared ONLY so `swift test` can run on the host (Swift
        // Testing executes on macOS). The app ships iOS 18 exclusively; the
        // macOS floor exists to satisfy Firebase's minimum. iOS-only code must
        // be gated with `#if canImport(UIKit)` so host-side tests keep working.
        .macOS(.v15)
    ],
    products: [
        .library(name: "YralApp", targets: ["YralApp"])
    ],
    dependencies: [
        // Firebase: Crashlytics + Analytics from day one (Phase 0) so every
        // shipped build reports crashes. Messaging/RemoteConfig are added in
        // later feature phases (push, feature flags).
        .package(
            url: "https://github.com/firebase/firebase-ios-sdk.git",
            exact: firebaseAppleSdkVersion
        )
    ],
    targets: [
        .target(
            name: "YralApp",
            dependencies: [
                // FirebaseCore is declared explicitly: SPM does not expose a
                // target's transitive dependencies for import to consumers,
                // and FirebaseBootstrapper calls FirebaseApp.configure().
                .product(name: "FirebaseCore", package: "firebase-ios-sdk"),
                .product(
                    name: "FirebaseCrashlytics",
                    package: "firebase-ios-sdk"
                ),
                .product(
                    name: "FirebaseAnalytics",
                    package: "firebase-ios-sdk"
                )
            ],
            swiftSettings: [
                .enableUpcomingFeature("StrictConcurrency")
            ]
        ),
        .testTarget(
            name: "YralAppTests",
            dependencies: ["YralApp"],
            swiftSettings: [
                .enableUpcomingFeature("StrictConcurrency")
            ]
        )
    ]
)
