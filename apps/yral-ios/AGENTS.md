# Agent Guidelines for apps/yral-ios

Native Swift/SwiftUI iOS app for YRAL. Replaces the legacy Kotlin
Multiplatform iOS app (`apps/yral-mobile/iosApp/`, frozen — still builds via
its own CI in the submodule repo).

## Documentation policy (hard rule)

No standalone docs (README, wiki, docs/). They drift from fast-moving source.
Document behavior **inline** — as comments beside/above the source it
describes. This file is the only exception: agent conventions, not product
documentation.

## Architecture rules

- **Inline by default (Hard Rule).** Do not create unnecessary abstractions.
  Most code lives inline at its call site. Introduce a
  helper/wrapper/protocol/manager ONLY when duplication is massive AND an
  abstraction is genuinely required — and ASK the operator before introducing
  it. Concrete applications in Swift:
  - No custom HTTP-client wrappers around `URLSession` — construct
    `URLRequest` and check `URLResponse` statuses inline in each data-source
    method.
  - No repository/manager layers around thin API clients — the data source IS
    the layer.
  - Prefer deleting a wrapper over adding one when its body is a single
    expression repeated a few times.

- **Colocate logic beside its caller (Hard Rule).** Feature code (views,
  data sources, models) lives together in `Sources/YralApp/Features/<Feature>/`
  — NOT in a parallel `Core/` tree. The current `Core/` files are scaffolding
  created before any feature UI existed; as Phase 2+ features land, move each
  data source into the feature that owns it (auth client → `FeatureAuth/`,
  feed client → `FeatureVideoFeed/`…). Promote a piece to a shared location
  ONLY when a second feature consumes it — and ask first. Do not pre-create
  shared/core modules ahead of their first consumer.

- **Thin Xcode shell** (`iosApp.xcodeproj`) — targets, signing, assets, and
  the Crashlytics dSYM build phase only. Contains no product code. The
  pbxproj uses folder-synchronized groups: adding a file under `iosApp/`
  or the SPM package requires **zero** pbxproj edits.
- **Single SPM package** (`packages/YralApp/`) — ALL product code. One target
  (`YralApp`), feature namespacing by folder
  (`Sources/YralApp/Features/<Feature>`). No multi-package split until a
  concrete need emerges (build times, team boundaries).
- One target, one bundle id (`com.yral.iosApp`). TestFlight and App Store are
  distribution channels on the same App Store Connect app record — not
  separate apps. Version numbers continue past the legacy app (3.4.5/24).
- Deployment target: iOS 18. Swift 6 language mode, `@Observable` throughout.
- Third-party deps via SPM only, exact-pinned in
  `packages/YralApp/Package.swift`. No CocoaPods, no fastlane, no Gemfile.
- **Tooling is Apple-canonical**: `xcodebuild archive` →
  `xcodebuild -exportArchive` → `xcrun altool --upload-app`. (Apple is
  deprecating altool in favor of a newer tool; altool is the current
  documented stable path on Xcode 26.)
- `iosApp/YralApp-Info.plist` is the SINGLE source of truth for
  Info-plist content. Do not duplicate its keys as `INFOPLIST_KEY_*` build
  settings.
- Apple ships no CLI generator for pbxproj/xcscheme/Info.plist — the committed
  shell is created once via Xcode GUI and stays static. Do not introduce
  XcodeGen/Tuist (third-party generator dependency for near-zero churn).
  Prefer editing the committed files directly over regenerating.

## Workflow (VS Code-first)

Day-to-day coding happens in VS Code with the Swift extension against
`packages/YralApp`. Xcode is used only for UI previews, signing, and asset
catalog work. Adding a Swift file in the package = zero xcodeproj changes.

## Commands (mise tasks, from repo root)

```sh
mise run yral-ios-setup   # resolve SPM deps
mise run yral-ios-build   # simulator build (Debug, unsigned)
mise run yral-ios-test    # package unit tests (Swift Testing)
mise run yral-ios-lint    # SwiftLint (strict)
mise run yral-ios-clean   # clean build outputs
```

## CI / Distribution

**Local and CI run the SAME mise tasks** (single source of truth). CI needs
only a working mise + the root repo's pre-existing `ANSIBLE_VAULT_PASSWORD`
GitHub secret; `mise run bootstrap` extracts the age key from the vault and
fnox decrypts the signing secrets from `fnox.toml` — exactly like local.
No repo-scoped iOS GitHub secrets, no fastlane, no CocoaPods.

- `.github/workflows/yral-ios-ci.yml` — PR: `mise run yral-ios-checks`.
  Push/merge to main: checks, then `fnox exec -- mise run
  yral-ios-upload-testflight` (archive → export → altool → dSYMs).
- `.github/workflows/yral-ios-app-store.yml` — release tags containing
  `iOS`: sets `MARKETING_VERSION` (Apple's fixed build-setting name for the
  user-facing version, `CFBundleShortVersionString`) from the tag, runs the
  same upload task, commits the version bump back to main via the default
  `GITHUB_TOKEN` (`permissions: contents: write` + checkout's persisted
  credentials — GitHub's documented pattern; no deploy key).

### Signing secrets (fnox.toml — set once via `fnox set <KEY> --provider age`)

| fnox key | Content |
| --- | --- |
| `YRAL_IOS_DIST_CERT_P12_BASE64` | Apple Distribution cert (p12), base64 |
| `YRAL_IOS_DIST_PROFILE_BASE64` | `Yral-Distribution` .mobileprovision, base64 |
| `YRAL_IOS_CERT_PASSWORD` | Password of the distribution p12 |
| `APP_STORE_CONNECT_API_KEY_BASE64` | ASC API key (AuthKey_J52D7789G2.p8), base64 |

Non-secret identifiers live in root `mise.toml [env]`:
`YRAL_ASC_KEY_ID`, `YRAL_ASC_ISSUER_ID`, `YRAL_APPLE_TEAM_ID`.
altool resolves the API key by file convention:
`./private_keys/AuthKey_<KEY_ID>.p8` — the upload task writes it there.

## Firebase

Single Firebase project (`yral-mobile`). **`GoogleService-Info.plist` is
GITIGNORED** — it contains the project's Google Cloud API key, which GitHub
secret scanning flags when committed (an earlier commit leaked it; the key
was rotated 2026-09-01). The plist is stored in fnox
(`YRAL_IOS_FIREBASE_PLIST_BASE64`, base64) and injected at build time:
the `yral-ios-build` and `yral-ios-upload-testflight` tasks materialize it
from the fnox secret when run under `fnox exec --`. Rotating the API key
again: rotate in GCP console → update the fnox secret
(see fnox.toml for the exact command). Crashlytics + Analytics initialize at
launch (`YralAppRoot.configureFirebase()`), so every shipped build reports
crashes from day one. The Crashlytics dSYM upload build phase runs after
every Release build (path resolution documented in `project.pbxproj`).

## Phase status

- [x] Phase 0 — scaffold + CI/distribution + Crashlytics
- [ ] Phase 1 — core foundation (config, SpacetimeDB client, networking, auth client, analytics providers)
- [ ] Phase 2 — auth + account/settings + deep links + push
- [ ] Phase 3 — video feed
- [ ] Phase 4+ — profile, chat, upload/videogen, wallet, ai-influencer, subscriptions