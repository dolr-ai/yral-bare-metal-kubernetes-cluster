import Testing
import Foundation
@testable import YralApp

/// Tests for `SessionStore` (the @Observable session machine) —
/// Kotlin `SessionManager` behavior contracts.
@MainActor
struct SessionTests {

    @Test("signed-in accessors read the session; updateState resets properties")
    func signedInAccessorsAndPropertyReset() {
        let store = SessionStore()

        // Initial state.
        #expect(store.state == .initial)
        #expect(store.userPrincipal == nil)
        #expect(store.profilePic == nil)
        #expect(store.isBotAccount == nil)

        // Signed in with properties set.
        let session = Session(
            canisterID: "canister-1",
            userPrincipal: "auth0|user-77",
            profilePic: "https://example.com/pic.png",
            username: "sunnyotter",
            isCreatedFromServiceCanister: true,
            isBotAccount: false
        )
        store.updateState(.signedIn(session))
        store.updateCoinBalance(250)
        store.updateSocialSignInStatus(true)
        store.updatePhoneNumber("+15551234567")

        #expect(store.canisterID == "canister-1")
        #expect(store.userPrincipal == "auth0|user-77")
        #expect(store.profilePic == "https://example.com/pic.png")
        #expect(store.username == "sunnyotter")
        #expect(store.isBotAccount == false)
        #expect(store.properties.coinBalance == 250)
        #expect(store.properties.isSocialSignIn == true)
        #expect(store.properties.phoneNumber == "+15551234567")

        // updateState resets per-session properties (botCount, directory,
        // pro availability preserved — Kotlin parity).
        store.updateState(.loading)
        #expect(store.properties.coinBalance == nil)
        #expect(store.properties.isSocialSignIn == nil)
    }

    @Test("resetSessionProperties zeroes balance and clears social sign-in")
    func resetSessionProperties() {
        let store = SessionStore()
        let session = Session(userPrincipal: "p", profilePic: "pic")
        store.updateState(.signedIn(session))
        store.updateCoinBalance(99)
        store.updateSocialSignInStatus(true)
        store.updateLoggedInUserEmail("user@example.com")

        store.resetSessionProperties()

        #expect(store.properties.coinBalance == 0)
        #expect(store.properties.isSocialSignIn == false)
        #expect(store.properties.emailID == nil)
    }

    @Test("firebase login state tracks independently of session state")
    func firebaseLoginState() {
        let store = SessionStore()
        #expect(store.properties.isFirebaseLoggedIn == false)
        store.updateFirebaseLoginState(true)
        #expect(store.properties.isFirebaseLoggedIn == true)
    }
}
