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

- **Kotlin is reference material, never a spec (Hard Rule).** The legacy
  Kotlin app is consulted ONLY to learn which backend endpoints a flow
  calls, with what payloads, and in what order — that wire behavior is worth
  matching. Its *architecture* is not: do NOT reproduce Kotlin's layers
  (repositories, managers, service classes, DTO mapping tiers), its
  `Yral`-prefixed names, its `Dto` suffixes, or its state model. This app
  follows the minimal architecture set out in this file — inline by default,
  folder-per-feature, native SwiftUI, FSMs for stateful logic. When porting,
  port the *behavior over the wire*, then express it the Swift way. A port
  that "matches Kotlin" by adding a layer this file forbids is a bug, not
  fidelity.

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

- **Stateful logic is a finite state machine (Hard Rule).** Any type with
  more than one meaningful mode — session, account deletion, upload job,
  AI-account creation, the account switcher — is an FSM: top-level state as
  an enum with state-specific payloads inside the variants, mutated only
  through one pure `transition(_ event:)`, wrapped in an `actor` when it has
  concurrent work. Add a shared `context` only when several states genuinely
  need the same datum — a machine whose states share nothing has none. See
  the "Finite State Machines for Stateful Logic" rule in the root AGENTS.md
  for the full contract and rationale. Convert flat-state types when you
  next touch them — no big-bang rewrite.

- **Folder per top-level screen/feature (Hard Rule).** Sources live under
  `Sources/YralApp/<feature>/` — ONE folder per top-level screen/feature
  (currently `authentication/`, `profile/`, `settings/`), with the logic
  that screen uses colocated INSIDE the folder (view + browser session +
  data source + validator together — no per-layer folders). A feature may
  have ROUTE subfolders named after its screens (`profile/create/`,
  `profile/list/`, later `profile/detail/` — one per screen, code
  colocated in each). Files that don't fit an existing feature stay at
  the ROOT (`Sources/YralApp/`) — cross-feature infrastructure (Session,
  AppConfiguration, NetworkError, ProfilePicture, Spacetime trio,
  AppRoot/RootScene) — until a feature owns them or a new feature folder
  earns its place. Do NOT create a feature folder ahead of its first
  real screen, and do NOT pre-create route subfolders with no code in
  them (profile/detail/ appears when the detail screen does).

- **Every UI file declares its kind with a filename suffix (Hard Rule).**
  The suffix says what the file IS, so a filename is self-describing
  without opening it. Four suffixes, and every Swift file has exactly one:

  | Suffix | What it is | Lives in | Has a preview? |
  |---|---|---|---|
  | `Screen` | A screen — a routable surface (a tab, a sheet, a full-screen flow) | `<feature>/` or `<feature>/<route>/` | Yes |
  | `Component` | A reusable, presentation-only view piece | `<feature>/` or `components/<tier>/` | Yes |
  | `Machine` | A finite state machine (the `State`/`Event`/`Effect` types + pure `transition`) | Beside its UI | No (pure — tested instead) |
  | *(none)* | Logic: clients, data sources, parsers/validation, persistence, wire models, config | Beside the UI that calls it | No |

  So `SignInScreen.swift` is a screen, `AICreationHeaderComponent.swift` is a
  piece of UI, `AuthMachine.swift` is a state machine, and `AuthClient.swift`
  is logic. Rename on touch — no big-bang rewrite. The name itself stays the
  domain noun (`AccountSwitcherScreen`, `AuthClient`), never the `Yral`
  prefix (see the no-prefix rule above).

  **A machine never carries a `Screen`/`Component` suffix**, and a
  `Screen`/`Component` file never holds a machine. This is what the FSM rule's
  "state lives in one transition function" looks like on disk: the machine
  is its own file precisely so its purity is visible.

  **One type per file, filename == type name.** Where a screen's private
  helper views would push it past the lint file-length bound, split them into
  their own `Component` files rather than letting one file hold several
  unrelated things (this is why `CountryPickerButtonComponent.swift` exists
  separately from `SignInScreen.swift`).

- **Every `Screen` and `Component` file ships a preview (Hard Rule).** A UI
  file is openable in Xcode and immediately inspectable — `#Preview` (the
  iOS 17+ macro, not the legacy `PreviewProvider`) with enough state wired
  that the component renders without a running app. A UI file with no
  preview is not finished. Previews are **not** dead code: they are the
  component's manual test, and they are the only place a component's
  variations (empty / populated / error / long-content) should be
  enumerated — use named previews (`#Preview("error")`) for those
  variations.

  Previews must not require the network or a real `KeychainStore` — inject
  stubs/fixtures, exactly as the tests do. A preview that cannot render
  offline is a preview nobody runs.

- **Reusable UI lives in `components/`, tiered by atomic design (Hard Rule).**
  Anything reused across features goes to
  `Sources/YralApp/components/<atoms|molecules|organisms>/`, named with the
  `Component` suffix. Feature-local pieces stay in the feature folder — the
  promotion trigger is a SECOND feature consuming it, never anticipation
  (same rule as every other shared module).
  
  The tiers, per Brad Frost's *Atomic Design* (chapter 2 — the canonical
  source; read it before arguing about which tier something is):
  - **atoms** — the indivisible primitives. A button, an icon, a label, a
    text style. Cannot be broken down further without ceasing to function.
    Atoms in SwiftUI are often close to the platform built-ins plus our
    styling — that is fine; the value is that all base styling is
    reviewable at a glance in one place.
  - **molecules** — a few atoms bonded into a simple unit with its own
    behaviour: a label + field + button forming a search form. Single
    responsibility; reusable wherever that functionality is needed.
  - **organisms** — relatively complex assemblies forming a *discrete
    section* of an interface: a header (logo + nav + search), a feed row, a
    card. May compose molecules, atoms, and other organisms.

  **The three tiers are a mental model, not a build order and not a
  maturity ladder.** Frost is explicit: "It would be foolish to design
  buttons and other elements in isolation, then cross our fingers and hope
  everything comes together" — the stages work *concurrently*. Atoms are not
  "more reusable" than organisms, and an organism is not a promoted
  molecule. Place a piece by **what it is**, never by "how shared" it feels.
  If a piece resists categorisation, that is usually a sign it is doing two
  jobs — split it rather than inventing a fourth tier.

  **We deliberately stop at organisms.** Frost's taxonomy continues with
  *templates* (layout skeletons) and *pages* (templates + real content).
  We already have both, and they are the feature folders: a `View` file IS
  the page (a specific instance with real content), and its body is the
  template (the layout skeleton). Adding `templates/` and `pages/` would
  create two homes for the same concept — exactly the structure the
  "inline by default / folder-per-feature" rules exist to prevent. Do not
  add those two folders.

  **Atomic design is technology-agnostic** (Frost applies it to native
  Instagram). It is not a CSS or Swift-specific technique — for us it is
  purely a placement and naming taxonomy for reusable view code.

- **Tests mirror their subjects by filename AND folder (Hard Rule).** SPM
  requires one directory per target — a test file CANNOT live inside
  `Sources/YralApp/` alongside its subject (unlike Rust's
  `#[cfg(test)] mod tests`). The canonical Swift answer is the standard
  `Tests/YralAppTests/` layout, kept as a disciplined MIRROR of the source
  tree: ONE test file per tested source file, in the SAME subfolder as its
  subject, named after it (`authentication/PKCEAndJWTParserTests.swift`
  tests `authentication/PKCE.swift` + `JWTParser`). A test file covering
  two subjects splits when either grows. Struct name == file name. No
  shared-helpers file — test fixtures live beside the tests that use
  them. When porting a Kotlin test file, its tests land in the mirror
  file of the Swift file that now holds the ported code.

- **No Yral prefix on types (Hard Rule).** The `YralApp` module already
  namespaces everything — `YralAuthClient`, `YralSession`, `YralPKCE` are
  redundant (`YralApp.AuthClient` is unambiguous). Types are bare:
  `AuthClient`, `Session`, `SessionStore`, `PKCE`, `PhoneValidator`,
  `OAuthCallbackParser`, `NetworkError`… matching the Kotlin originals
  (`SocialProvider`, `SessionState`, `TokenClaims` were never
  `YralSocialProvider` there either). The ONLY Yral names allowed:
  the package/module (`YralApp`), the app-shell entry points
  (`YralAppRoot`), and file-scoped test struct names mirroring their
  test files. Filenames follow their type (`AuthClient.swift`,
  `Session.swift`).

- **Thin Xcode shell** (`iosApp.xcodeproj`) — targets, signing, assets, and
  the Crashlytics dSYM build phase only. Contains no product code. The
  pbxproj uses folder-synchronized groups: adding a file under `iosApp/`
  or the SPM package requires **zero** pbxproj edits.
- **Single SPM package** (`packages/YralApp/`) — ALL product code. One target
  (`YralApp`), one folder per top-level screen/feature under
  `Sources/YralApp/` (see the folder rule above). No multi-package split
  until a concrete need emerges (build times, team boundaries).
- One target, one bundle id (`com.yral.iosApp`). TestFlight and App Store are
  distribution channels on the same App Store Connect app record — not
  separate apps. Version numbers continue past the legacy app (3.4.5/24).
- Deployment target: iOS 26. Swift 6 language mode, `@Observable` throughout.
  iOS-only APIs used outside the test host are gated `#if canImport(UIKit)`
  (the macOS host exists only for `swift test`).
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

## Design rules

- **Native Liquid Glass everywhere (Hard Rule).** The app targets iOS 26 and
  adopts its system look wholesale — no custom re-implementations of system
  materials. Standard containers (`TabView`, `NavigationStack`, sheets,
  alerts) render Liquid Glass for free; where a custom surface needs the
  material, use the system glass effect APIs rather than hand-rolled
  translucency. Do not build a bespoke design system on top of the native one.

- **Native SwiftUI primitives over a design language (Hard Rule).** No
  design system of our own — no custom color palette, no spacing scale, no
  typography tokens, no custom component library. Use native colors
  (`Color.black`, `Color.gray`, `.primary`/`.secondary`/`.tertiary`, `.pink`),
  native fonts (`Font.system`/`.title`/`.headline`/`.caption`), and system
  defaults inline at the call site. **If nothing matches exactly, pick the
  NEAREST native thing** — color, size, spacing, corner radius, animation,
  whatever — rather than inventing a custom value or a named constant for
  it. Rationale: this kills bikeshedding over design-language adherence;
  native primitives are inline (maximal convenience, minimum confusion),
  and the app inherits platform evolution (new OS look, Dark Mode,
  accessibility) for free. An earlier custom `SystemColors` helper was
  created and deleted same-day for exactly this reason.

- **Previews for every screen (Hard Rule).** Every screen/view ships with a
  `#Preview` in the same file, covering its meaningful variants (idle,
  working, empty, error — whatever applies; named variants via
  `#Preview("name")`). Previews use the REAL view and realistic fixture
  data (`.constant` bindings where needed), never stripped-down stand-ins —
  the operator previews each screen in Xcode's canvas to check exactly what
  ships. Sound-producing previews play their sound (do not mute them in
  previews). A new screen without a preview is incomplete.

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

**Deploys run LOCALLY: `fnox exec -- mise run yral-ios-upload-testflight`.**
The CI deploy job is DISABLED (2026-09-01): the free public macOS runner pool
queues 1h+ and starved the deploy job entirely; a local deploy (~2.5 min)
is the iteration loop. CI runs CHECKS ONLY (lint + tests + simulator build)
— the identical `yral-ios-checks` mise task both places. The deploy job is
kept as a commented block in `yral-ios-ci.yml` for easy re-enabling.

**Checks parity:** CI needs only a working mise + the root repo's
pre-existing `ANSIBLE_VAULT_PASSWORD` GitHub secret; `mise run bootstrap`
extracts the age key from the vault and fnox decrypts the Firebase plist +
signing secrets from `fnox.toml` — exactly like local. No repo-scoped iOS
GitHub secrets, no fastlane, no CocoaPods.

- `.github/workflows/yral-ios-ci.yml` — checks only (lint + package tests
  + simulator build). The TestFlight deploy job is commented out — deploys
  run locally via `fnox exec -- mise run yral-ios-upload-testflight`.
- `.github/workflows/yral-ios-app-store.yml` — release tags containing
  `iOS`: sets `MARKETING_VERSION` (Apple's fixed build-setting name for the
  user-facing version, `CFBundleShortVersionString`) from the tag, runs the
  same upload task, commits the version bump back to main via the default
  `GITHUB_TOKEN` (`permissions: contents: write` + checkout's persisted
  credentials — GitHub's documented pattern; no deploy key). NOTE: this
  workflow's macOS-runner dependency makes it subject to the same queue
  starvation as CI deploys — if it proves unreliable, run the upload task
  locally against the release tag instead (same mise task).

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

**Report every handled error to Crashlytics — never swallow into a UI label
only (Hard Rule).** Every `catch` of an API/network/persistence failure
records the error via the `CrashReporter` facade
(`packages/YralApp/Sources/YralApp/CrashReporter.swift`, thin wrapper over
`Crashlytics.crashlytics().record(error:)` — the documented non-fatal
mechanism per Firebase's "Report non-fatal exceptions" docs; no-op when
Firebase isn't configured so tests/previews stay clean). No `try?` on
network calls in feature flows — a best-effort UI enhancement that fails
still reports (`try?` hides the failure from both the user AND the
dashboard; use do/catch + record + fallback behavior instead). Grouping is
by NSError `domain`+`code`, so codes are STABLE per error kind (one code
per `NetworkError` case via `CrashReporter.stableCode` — never per-instance
values; the docs warn high-cardinality domains/codes get rate-limited).
Per-instance detail (upstream status + body per the verbatim-errors rule,
operation context, call site) lives in `userInfo` keys (`context`, `site`)
and shows in the issue's Keys/logs tabs; the full `String(describing:)` of
the error is in `NSLocalizedDescriptionKey`. Breadcrumbs via
`CrashReporter.log(_:)` (64 kB ring buffer per session). Non-fatals buffer
on-device and are delivered on the next app launch.

## Phase status

- [x] Phase 0 — scaffold + CI/distribution + Crashlytics
- [x] Phase 1 — core foundation (config, SpacetimeDB client, networking, auth client, analytics providers)
- [ ] Phase 2 — auth + account/settings + deep links + push
- [ ] Phase 3 — video feed
- [ ] Phase 4+ — profile, chat, upload/videogen, wallet, ai-influencer, subscriptions