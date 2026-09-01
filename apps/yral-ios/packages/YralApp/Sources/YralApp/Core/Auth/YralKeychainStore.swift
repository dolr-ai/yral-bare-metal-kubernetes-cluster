import Foundation
import Security

/// Keychain-backed token/session storage.
///
/// The legacy Kotlin app stored tokens in NSUserDefaults (suite YRAL_PREF) —
/// plaintext and excluded from device backups only by OS convention. This
/// port uses the Keychain with `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`
/// (survives app updates, never leaves the device, unavailable while locked).
///
/// One decision differs from the legacy app by design: no migration from
/// NSUserDefaults — users re-authenticate once at first launch of the new
/// app (per the Phase 0 plan decision).
public struct YralKeychainStore: Sendable {

    /// Keychain service name (scoping all entries to this app).
    private let service: String

    public init(service: String = "com.yral.iosApp") {
        self.service = service
    }

    // MARK: - Keys (mirroring the legacy PrefKeys the new app needs)

    /// Primary authentication tokens.
    public enum Key: String, Sendable {
        case idToken = "ID_TOKEN"
        case accessToken = "ACCESS_TOKEN"
        case refreshToken = "REFRESH_TOKEN"
        /// OAuth subject of the active (possibly bot) identity.
        case lastActivePrincipal = "LAST_ACTIVE_PRINCIPAL"
        /// OAuth subject of the main (human) account.
        case mainPrincipal = "MAIN_PRINCIPAL"
    }

    // MARK: - CRUD

    /// Reads a string value.
    public func string(forKey key: Key) -> String? {
        var query = baseQuery(for: key)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess, let data = result as? Data else { return nil }
        return String(data: data, encoding: .utf8)
    }

    /// Writes a string value (upsert — replaces existing).
    public func setString(_ value: String, forKey key: Key) {
        let data = Data(value.utf8)
        var query = baseQuery(for: key)
        let update: [String: Any] = [kSecValueData as String: data]
        let updateStatus = SecItemUpdate(query as CFDictionary, update as CFDictionary)
        if updateStatus == errSecSuccess { return }
        SecItemDelete(query as CFDictionary)
        query[kSecValueData as String] = data
        SecItemAdd(query as CFDictionary, nil)
    }

    /// Removes a value (tolerates absence).
    public func removeValue(forKey key: Key) {
        SecItemDelete(baseQuery(for: key) as CFDictionary)
    }

    /// Removes every value stored by this service (logout).
    public func removeAll() {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service
        ]
        SecItemDelete(query as CFDictionary)
    }

    /// Common Keychain query attributes.
    private func baseQuery(for key: Key) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key.rawValue,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        ]
    }
}
