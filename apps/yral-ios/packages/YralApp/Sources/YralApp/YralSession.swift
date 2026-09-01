import Foundation
import Observation

/// Signed-in identity — port of Kotlin `Session` (core/session/Session.kt).
/// `canisterID == userPrincipal` in the JWT-only world (no IC canisters);
/// the field exists because the rest of the app (profile URLs, balance
/// calls) keys off it.
public struct YralSession: Equatable, Sendable {
    public var canisterID: String?
    public var userPrincipal: String?
    public var profilePic: String?
    public var username: String?
    public var bio: String?
    public var isCreatedFromServiceCanister: Bool
    public var isBotAccount: Bool

    public init(
        canisterID: String? = nil,
        userPrincipal: String? = nil,
        profilePic: String? = nil,
        username: String? = nil,
        bio: String? = nil,
        isCreatedFromServiceCanister: Bool = true,
        isBotAccount: Bool = false
    ) {
        self.canisterID = canisterID
        self.userPrincipal = userPrincipal
        self.profilePic = profilePic
        self.username = username
        self.bio = bio
        self.isCreatedFromServiceCanister = isCreatedFromServiceCanister
        self.isBotAccount = isBotAccount
    }
}

/// Auth lifecycle — port of Kotlin `SessionState`.
public enum YralSessionState: Equatable, Sendable {
    case initial
    case loading
    case signedIn(YralSession)
}

/// Session-adjacent state — port of Kotlin `SessionProperties`. Fields whose
/// consumers land in later phases (follow sets, pro details, mandatory
/// login) are added with their phases.
public struct YralSessionProperties: Equatable, Sendable {
    public var coinBalance: Int64?
    public var isSocialSignIn: Bool?
    public var profileVideosCount: Int?
    public var botCount: Int?
    public var accountDirectory: YralAccountDirectory?
    public var emailID: String?
    public var isFirebaseLoggedIn: Bool
    public var phoneNumber: String?
    public var isYralProAvailable: Bool?

    public init(
        coinBalance: Int64? = nil,
        isSocialSignIn: Bool? = nil,
        profileVideosCount: Int? = nil,
        botCount: Int? = nil,
        accountDirectory: YralAccountDirectory? = nil,
        emailID: String? = nil,
        isFirebaseLoggedIn: Bool = false,
        phoneNumber: String? = nil,
        isYralProAvailable: Bool? = nil
    ) {
        self.coinBalance = coinBalance
        self.isSocialSignIn = isSocialSignIn
        self.profileVideosCount = profileVideosCount
        self.botCount = botCount
        self.accountDirectory = accountDirectory
        self.emailID = emailID
        self.isFirebaseLoggedIn = isFirebaseLoggedIn
        self.phoneNumber = phoneNumber
        self.isYralProAvailable = isYralProAvailable
    }
}

/// Pro subscription snapshot — port of Kotlin `ProDetails`.
public struct YralProDetails: Equatable, Sendable {
    public static let defaultTotalCredits = 30

    public var isProPurchased: Bool
    public var availableCredits: Int
    public var totalCredits: Int

    public init(
        isProPurchased: Bool = false,
        availableCredits: Int = 0,
        totalCredits: Int = YralProDetails.defaultTotalCredits
    ) {
        self.isProPurchased = isProPurchased
        self.availableCredits = availableCredits
        self.totalCredits = totalCredits
    }
}

/// Main + bot accounts for the account switcher — port of Kotlin
/// `AccountDirectory`/`AccountDirectoryProfile` (consumed by the switcher
/// phase; the type ships now because `YralSessionProperties` holds it and
/// Kotlin's `updateState` preserves it across session resets).
public struct YralAccountDirectoryProfile: Codable, Equatable, Sendable {
    public var principal: String
    public var username: String
    public var avatarURL: String
    public var isBot: Bool

    public init(principal: String, username: String, avatarURL: String, isBot: Bool) {
        self.principal = principal
        self.username = username
        self.avatarURL = avatarURL
        self.isBot = isBot
    }
}

public struct YralAccountDirectory: Codable, Equatable, Sendable {
    public var mainPrincipal: String?
    public var botPrincipals: [String]
    public var profilesByPrincipal: [String: YralAccountDirectoryProfile]

    public init(
        mainPrincipal: String?,
        botPrincipals: [String],
        profilesByPrincipal: [String: YralAccountDirectoryProfile]
    ) {
        self.mainPrincipal = mainPrincipal
        self.botPrincipals = botPrincipals
        self.profilesByPrincipal = profilesByPrincipal
    }
}

/// Observable session state — port of Kotlin `SessionManager`
/// (MutableStateFlow state + properties → `@Observable`).
/// Kotlin's VideoGenerationTracker reset in `updateState` lands with the
/// video phase.
@MainActor @Observable
public final class YralSessionStore {

    public private(set) var state: YralSessionState = .initial
    public private(set) var properties = YralSessionProperties()

    public init() {}

    // MARK: - Signed-in session accessors

    public var canisterID: String? {
        if case let .signedIn(session) = state { return session.canisterID }
        return nil
    }

    public var userPrincipal: String? {
        if case let .signedIn(session) = state { return session.userPrincipal }
        return nil
    }

    public var profilePic: String? {
        if case let .signedIn(session) = state { return session.profilePic }
        return nil
    }

    public var username: String? {
        if case let .signedIn(session) = state { return session.username }
        return nil
    }

    public var isBotAccount: Bool? {
        if case let .signedIn(session) = state { return session.isBotAccount }
        return nil
    }

    // MARK: - State updates

    /// Replaces the session state and resets per-session properties
    /// (preserving device-level values, exactly as Kotlin does).
    public func updateState(_ newState: YralSessionState) {
        state = newState
        properties = YralSessionProperties(
            botCount: properties.botCount,
            accountDirectory: properties.accountDirectory,
            isYralProAvailable: properties.isYralProAvailable
        )
    }

    public func updateCoinBalance(_ newBalance: Int64) {
        properties.coinBalance = newBalance
    }

    public func updateSocialSignInStatus(_ isSocialSignIn: Bool) {
        properties.isSocialSignIn = isSocialSignIn
    }

    public func updateLoggedInUserEmail(_ email: String?) {
        properties.emailID = email
    }

    public func updatePhoneNumber(_ phoneNumber: String?) {
        properties.phoneNumber = phoneNumber
    }

    public func updateFirebaseLoginState(_ isLoggedIn: Bool) {
        properties.isFirebaseLoggedIn = isLoggedIn
    }

    /// Logout-scoped property reset — port of Kotlin
    /// `resetSessionProperties` (coin balance 0, counts cleared, social
    /// sign-in off; pro availability is device-level and survives).
    public func resetSessionProperties() {
        properties = YralSessionProperties(
            coinBalance: 0,
            isSocialSignIn: false,
            profileVideosCount: 0,
            botCount: nil,
            accountDirectory: nil,
            isYralProAvailable: properties.isYralProAvailable
        )
    }
}
