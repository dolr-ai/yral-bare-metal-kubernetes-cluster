import Testing
import Foundation
@testable import YralApp

/// Tests for the auth lifecycle — `AuthMachine` (the pure transition
/// function) and `SessionStore` (the `@Observable` holder that drives it).
///
/// The machine is pure, so the transition tests need no keychain, no
/// network, and no view.
@MainActor
struct SessionTests {

    private func snapshot(_ state: AuthMachine.State) -> AuthMachine.Snapshot {
        AuthMachine.Snapshot(state: state, context: AuthMachine.Context())
    }

    private let mainSession = Session(
        userSubject: "main-subject",
        profilePic: "https://example.com/pic.png",
        username: "sunnyotter",
        isAIAccount: false
    )

    private let botSession = Session(
        userSubject: "bot-subject",
        profilePic: "https://example.com/bot.png",
        username: "dekuizuku",
        isAIAccount: true
    )

    // MARK: - Store accessors

    @Test("signed-in accessors read the session under either identity")
    func signedInAccessorsReadSession() {
        let store = SessionStore()
        #expect(store.state == .initial)
        #expect(store.userSubject == nil)
        #expect(store.profilePic == nil)
        #expect(store.isBotSession == nil)

        store.send(.sessionEstablished(mainSession))
        #expect(store.userSubject == "main-subject")
        #expect(store.profilePic == "https://example.com/pic.png")
        #expect(store.username == "sunnyotter")
        #expect(store.isBotSession == false)
    }

    @Test("a bot session is its own state, not a flag on the main state")
    func botSessionIsDistinctState() {
        let store = SessionStore()
        store.send(.sessionEstablished(botSession))

        #expect(store.state == .signedInAsBot(botSession))
        #expect(store.state != .signedIn(botSession))
        // Still a session for the shared accessors...
        #expect(store.userSubject == "bot-subject")
        // ...and the bot branch is readable without inspecting the payload.
        #expect(store.isBotSession == true)
        #expect(store.state.isBotSession)
    }

    // MARK: - Initial vs signed-out (the distinction `.initial` swallowed)

    @Test("never-signed-in and logged-out are different states")
    func initialAndSignedOutDiffer() {
        let fresh = SessionStore()
        #expect(fresh.state == .initial)

        // A session that expired is signed out WITH a cause.
        let expired = SessionStore()
        expired.send(.sessionEstablished(mainSession))
        expired.send(.sessionExpired(cause: .refreshTokenExpiredOrInvalid))
        #expect(expired.state == .signedOut(cause: .refreshTokenExpiredOrInvalid))
        #expect(expired.state != .initial)

        // A deliberate sign-out carries NO cause — same state, different
        // payload, which is exactly what `.initial` could not express.
        let signedOut = SessionStore()
        signedOut.send(.sessionEstablished(mainSession))
        signedOut.send(.userSignedOut)
        #expect(signedOut.state == .signedOut(cause: nil))
        #expect(signedOut.state != expired.state)
    }

    @Test("cold start with nothing cached returns to initial, not signedOut")
    func nothingCachedStaysInitial() {
        let (next, effect) = AuthMachine.transition(
            snapshot(.restoring), .nothingCached
        )
        #expect(next.state == .initial)
        #expect(effect == .none)
    }

    // MARK: - Transitions

    @Test("restore start moves to restoring from any state")
    func restoreStartedFromAnyState() {
        let states: [AuthMachine.State] = [
            .initial,
            .signedOut(cause: nil),
            .signedIn(mainSession),
            .signedInAsBot(botSession)
        ]
        for state in states {
            let (next, effect) = AuthMachine.transition(snapshot(state), .restoreStarted)
            #expect(next.state == .restoring)
            #expect(effect == .none)
        }
    }

    @Test("either identity routes to its own signed-in state")
    func sessionEstablishedRoutesByIdentity() {
        let (main, _) = AuthMachine.transition(
            snapshot(.restoring), .sessionEstablished(mainSession)
        )
        #expect(main.state == .signedIn(mainSession))

        let (bot, _) = AuthMachine.transition(
            snapshot(.restoring), .sessionEstablished(botSession)
        )
        #expect(bot.state == .signedInAsBot(botSession))
    }

    @Test("expiry and sign-out both clear credentials")
    func logoutPathsRequestCredentialClearing() {
        let (expired, expiryEffect) = AuthMachine.transition(
            snapshot(.signedIn(mainSession)), .sessionExpired(cause: .refreshTokenMissing)
        )
        #expect(expiryEffect == .clearStoredSession)
        #expect(expired.state == .signedOut(cause: .refreshTokenMissing))

        let (signedOut, signOutEffect) = AuthMachine.transition(
            snapshot(.signedIn(mainSession)), .userSignedOut
        )
        #expect(signOutEffect == .clearStoredSession)
        #expect(signedOut.state == .signedOut(cause: nil))
    }

    @Test("an anonymous identity can expire before any real sign-in")
    func expiryFromInitialIsSignedOut() {
        // Regression: `.initial` used to be both "fresh install" and
        // "logged out", so an anonymous-identity expiry looked like a fresh
        // install and lost its cause.
        let (next, effect) = AuthMachine.transition(
            snapshot(.initial), .sessionExpired(cause: .refreshTokenMissing)
        )
        #expect(next.state == .signedOut(cause: .refreshTokenMissing))
        #expect(effect == .clearStoredSession)
    }

    // MARK: - Property reset is part of the transition, not a side effect

    @Test("a session change resets per-session properties and keeps device-level ones")
    func propertyResetOnSessionChange() {
        let (signedIn, _) = AuthMachine.transition(
            snapshot(.initial), .sessionEstablished(mainSession)
        )
        var context = signedIn.context
        context.properties.coinBalance = 250
        context.properties.isSocialSignIn = true
        context.properties.phoneNumber = "+15551234567"
        context.properties.botCount = 3
        context.properties.isYralProAvailable = true

        let before = AuthMachine.Snapshot(state: signedIn.state, context: context)
        let (after, _) = AuthMachine.transition(before, .userSignedOut)

        // Per-session values cleared...
        #expect(after.context.properties.coinBalance == nil)
        #expect(after.context.properties.isSocialSignIn == nil)
        #expect(after.context.properties.phoneNumber == nil)
        // ...device-level values survive.
        #expect(after.context.properties.botCount == 3)
        #expect(after.context.properties.isYralProAvailable == true)
    }

    @Test("a no-op event never clears properties")
    func noOpEventPreservesProperties() {
        // Regression: `updateState` reset properties on EVERY call,
        // including for states that are not session changes.
        var context = AuthMachine.Context()
        context.properties.coinBalance = 250
        let before = AuthMachine.Snapshot(state: .restoring, context: context)

        // `.nothingCached` from a non-restoring state takes no transition.
        let (after, effect) = AuthMachine.transition(before, .nothingCached)

        #expect(effect == .none)
        #expect(after.context.properties.coinBalance == 250)
    }

    @Test("leaving restoring for a fresh session does not leak the old account's properties")
    func accountSwitchDoesNotLeakProperties() {
        // The invariant that matters: per-session values must not carry
        // across accounts. Enter session A with a balance, leave it, come
        // back as session B — B must start clean.
        let sessionA = mainSession
        let sessionB = Session(
            userSubject: "second-subject", profilePic: "p2", username: "second"
        )

        var context = AuthMachine.Context()
        context.properties.coinBalance = 250
        context.properties.botCount = 3
        let inA = AuthMachine.Snapshot(state: .signedIn(sessionA), context: context)

        // Leave A (restoring clears the session) then establish B.
        let (restoring, _) = AuthMachine.transition(inA, .restoreStarted)
        let (inB, _) = AuthMachine.transition(restoring, .sessionEstablished(sessionB))

        #expect(inB.state == .signedIn(sessionB))
        // A's balance is gone...
        #expect(inB.context.properties.coinBalance == nil)
        // ...but the device-level value survived both transitions.
        #expect(inB.context.properties.botCount == 3)
    }

    @Test("re-entering the same signed-in state does not wipe live properties")
    func reenteringSameStatePreservesProperties() {
        var context = AuthMachine.Context()
        context.properties.coinBalance = 250
        let before = AuthMachine.Snapshot(state: .signedIn(mainSession), context: context)

        let (after, _) = AuthMachine.transition(before, .sessionEstablished(mainSession))

        #expect(after.state == .signedIn(mainSession))
        #expect(after.context.properties.coinBalance == 250)
    }

    @Test("resetSessionProperties zeroes balance and clears social sign-in")
    func resetSessionProperties() {
        let store = SessionStore()
        store.send(.sessionEstablished(mainSession))
        store.updateCoinBalance(99)
        store.updateSocialSignInStatus(true)
        store.updateLoggedInUserEmail("user@example.com")

        store.resetSessionProperties()

        #expect(store.properties.coinBalance == 0)
        #expect(store.properties.isSocialSignIn == false)
        #expect(store.properties.emailID == nil)
    }

    // MARK: - State shape

    @Test("session is exposed by both signed-in states and no other")
    func sessionOnlyWhenSignedIn() {
        #expect(AuthMachine.State.initial.session == nil)
        #expect(AuthMachine.State.restoring.session == nil)
        #expect(AuthMachine.State.signedOut(cause: nil).session == nil)
        #expect(AuthMachine.State.signedIn(mainSession).session == mainSession)
        #expect(AuthMachine.State.signedInAsBot(botSession).session == botSession)
    }

    @Test("isRestoring is true only while restoring")
    func isRestoringReflectsState() {
        #expect(AuthMachine.State.restoring.isRestoring)
        #expect(!AuthMachine.State.initial.isRestoring)
        #expect(!AuthMachine.State.signedIn(mainSession).isRestoring)
    }
}
