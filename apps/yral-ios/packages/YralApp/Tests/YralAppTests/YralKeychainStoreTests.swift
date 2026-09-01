import Testing
import Foundation
@testable import YralApp

/// Tests for `YralKeychainStore` — the macOS test host has a real Keychain,
/// so these exercise the actual SecItem round-trip (not a stub).
struct YralKeychainStoreTests {

    @Test("set/get/remove round-trips")
    func roundTrip() {
        let store = YralKeychainStore(service: "yral-tests-\(UUID().uuidString)")
        defer { store.removeAll() }

        #expect(store.string(forKey: .idToken) == nil)
        store.setString("token-1", forKey: .idToken)
        #expect(store.string(forKey: .idToken) == "token-1")

        // Upsert replaces.
        store.setString("token-2", forKey: .idToken)
        #expect(store.string(forKey: .idToken) == "token-2")

        store.removeValue(forKey: .idToken)
        #expect(store.string(forKey: .idToken) == nil)
    }

    @Test("removeAll clears every key")
    func removeAll() {
        let store = YralKeychainStore(service: "yral-tests-\(UUID().uuidString)")
        defer { store.removeAll() }
        store.setString("id", forKey: .idToken)
        store.setString("access", forKey: .accessToken)
        store.setString("refresh", forKey: .refreshToken)
        store.setString("last-active", forKey: .lastActivePrincipal)
        store.setString("main", forKey: .mainPrincipal)
        store.removeAll()
        #expect(store.string(forKey: .idToken) == nil)
        #expect(store.string(forKey: .accessToken) == nil)
        #expect(store.string(forKey: .refreshToken) == nil)
        #expect(store.string(forKey: .lastActivePrincipal) == nil)
        #expect(store.string(forKey: .mainPrincipal) == nil)
    }
}
