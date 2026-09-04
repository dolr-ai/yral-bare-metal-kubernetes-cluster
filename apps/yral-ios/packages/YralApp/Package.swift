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

// swift-openapi-generator toolchain — same principle as the SpacetimeDB
// bindings: the OpenAPI spec IS the API contract; generated types make
// drift a compile error. Versions exact-pinned (repo rule) — latest
// official releases as of 2026-09-03.
let swiftOpenAPIGeneratorVersion: Version = "1.13.1"
let swiftOpenAPIRuntimeVersion: Version = "1.12.1"
let swiftOpenAPIURLSessionVersion: Version = "1.3.1"

let package = Package(
    name: "YralApp",
    platforms: [
        // iOS 26: adopt the native Liquid Glass look (system chrome
        // renders glass automatically; custom styles dropped in favor of
        // system materials). Pre-launch, so dropping iOS 18–25 devices
        // costs nothing.
        .iOS(.v26),
        // macOS is declared ONLY so `swift test` can run on the host (Swift
        // Testing executes on macOS). The app ships iOS 26 exclusively; the
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
        ),
        // swift-openapi toolchain: the generator plugin runs at build time,
        // the runtime provides Codable conformances, and the URLSession
        // transport wires the generated client to the network.
        .package(
            url: "https://github.com/apple/swift-openapi-generator.git",
            exact: swiftOpenAPIGeneratorVersion
        ),
        .package(
            url: "https://github.com/apple/swift-openapi-runtime.git",
            exact: swiftOpenAPIRuntimeVersion
        ),
        .package(
            url: "https://github.com/apple/swift-openapi-urlsession.git",
            exact: swiftOpenAPIURLSessionVersion
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
                ),
                // swift-openapi runtime + URLSession transport for the
                // generated Rishi API client.
                .product(
                    name: "OpenAPIRuntime",
                    package: "swift-openapi-runtime"
                ),
                .product(
                    name: "OpenAPIURLSession",
                    package: "swift-openapi-urlsession"
                )
            ],
            swiftSettings: [
                .enableUpcomingFeature("StrictConcurrency")
            ],
            plugins: [
                // Generates Sources/YralApp/GeneratedOpenAPI/ at build
                // time from the byte-verbatim live-spec snapshot
                // (openapi.json + openapi-generator-config.yaml in the
                // source dir; the snapshot is refreshed before every
                // build/test by mise run yral-ios-sync-rishi-openapi —
                // see AGENTS.md "API bindings come from the live contract").
                .plugin(
                    name: "OpenAPIGenerator",
                    package: "swift-openapi-generator"
                )
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
