import Foundation
import Observation

/// Signed-in identity — port of Kotlin `Session` (core/session/Session.kt).
/// The Kotlin original carried an ICP canister ID; in the JWT-only world
/// that field duplicated userSubject and is removed (ICP legacy purge —
/// same treatment as the principal→subject rename).
public struct Session: Equatable, Sendable {
    public var userSubject: String?
    public var profilePic: String?
    public var username: String?
    public var bio: String?
    public var isAIAccount: Bool

    public init(
        userSubject: String? = nil,
        profilePic: String? = nil,
        username: String? = nil,
        bio: String? = nil,
        isAIAccount: Bool = false
    ) {
        self.userSubject = userSubject
        self.profilePic = profilePic
        self.username = username
        self.bio = bio
        self.isAIAccount = isAIAccount
    }
}

/// Auth lifecycle — the machine's finite state. Port of Kotlin
/// `SessionState`, extended with the states that distinction needed (see
/// `AuthMachine` for why `.initial` had to split from `.signedOut`).
public typealias SessionState = AuthMachine.State

/// Session-adjacent state — port of Kotlin `SessionProperties`. Fields whose
/// consumers land in later phases (follow sets, pro details, mandatory
/// login) are added with their phases.
public struct SessionProperties: Equatable, Sendable {
    public var coinBalance: Int64?
    public var isSocialSignIn: Bool?
    public var profileVideosCount: Int?
    public var botCount: Int?
    public var accountDirectory: AccountDirectory?
    public var emailID: String?
    public var isFirebaseLoggedIn: Bool
    public var phoneNumber: String?
    public var isYralProAvailable: Bool?

    public init(
        coinBalance: Int64? = nil,
        isSocialSignIn: Bool? = nil,
        profileVideosCount: Int? = nil,
        botCount: Int? = nil,
        accountDirectory: AccountDirectory? = nil,
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
public struct ProDetails: Equatable, Sendable {
    public static let defaultTotalCredits = 30

    public var isProPurchased: Bool
    public var availableCredits: Int
    public var totalCredits: Int

    public init(
        isProPurchased: Bool = false,
        availableCredits: Int = 0,
        totalCredits: Int = ProDetails.defaultTotalCredits
    ) {
        self.isProPurchased = isProPurchased
        self.availableCredits = availableCredits
        self.totalCredits = totalCredits
    }
}

/// Main + AI account accounts for the account switcher — port of Kotlin
/// `AccountDirectory`/`AccountDirectoryProfile` (consumed by the switcher
/// phase; the type ships now because `SessionProperties` holds it and
/// Kotlin's `updateState` preserves it across session resets).
public struct AccountDirectoryProfile: Codable, Equatable, Sendable {
    public var subject: String
    public var username: String
    public var avatarURL: String
    public var isBot: Bool

    public init(subject: String, username: String, avatarURL: String, isBot: Bool) {
        self.subject = subject
        self.username = username
        self.avatarURL = avatarURL
        self.isBot = isBot
    }
}

public struct AccountDirectory: Codable, Equatable, Sendable {
    public var mainSubject: String?
    public var botSubjects: [String]
    public var profilesBySubject: [String: AccountDirectoryProfile]

    public init(
        mainSubject: String?,
        botSubjects: [String],
        profilesBySubject: [String: AccountDirectoryProfile]
    ) {
        self.mainSubject = mainSubject
        self.botSubjects = botSubjects
        self.profilesBySubject = profilesBySubject
    }
}

/// Observable auth session — drives the UI from `AuthMachine`. Port of
/// Kotlin `SessionManager` (MutableStateFlow state + properties →
/// `@Observable`).
///
/// The store OWNS the snapshot and is the only thing that mutates it, but
/// it never decides the next state itself: every change goes through
/// `AuthMachine.transition`, so the state graph and the property-reset rule
/// stay pure and testable (see that file for the defects this replaced).
@MainActor @Observable
public final class SessionStore {

    public private(set) var snapshot = AuthMachine.Snapshot.initial

    public var state: SessionState { snapshot.state }
    public var properties: SessionProperties { snapshot.context.properties }

    public init() {}

    // MARK: - Signed-in session accessors

    public var userSubject: String? { state.session?.userSubject }

    public var profilePic: String? { state.session?.profilePic }

    public var username: String? { state.session?.username }

    /// True only for a bot session (`nil` when not signed in) — previously
    /// a `Bool?` read off the payload, now off the state, so it cannot
    /// disagree with the session it describes.
    public var isBotSession: Bool? { state.session == nil ? nil : state.isBotSession }

    // MARK: - Events

    /// The machine's I/O effects, performed by `AuthClient` — the store
    /// holds no keychain reference.
    ///
    /// Set once at construction (`AuthClient` owns the credential store,
    /// the store owns the state; neither reaches into the other). Defaults
    /// to a no-op so tests can drive transitions without a keychain.
    var effectHandler: (AuthMachine.Effect) -> Void = { _ in }

    /// Sends an event through the machine and applies whatever it decides.
    /// The single entry point for state change — no caller sets state.
    func send(_ event: AuthMachine.Event) {
        let (next, effect) = AuthMachine.transition(snapshot, event)
        snapshot = next
        effectHandler(effect)
    }

    public func updateCoinBalance(_ newBalance: Int64) {
        snapshot.context.properties.coinBalance = newBalance
    }

    public func updateSocialSignInStatus(_ isSocialSignIn: Bool) {
        snapshot.context.properties.isSocialSignIn = isSocialSignIn
    }

    public func updateLoggedInUserEmail(_ email: String?) {
        snapshot.context.properties.emailID = email
    }

    public func updatePhoneNumber(_ phoneNumber: String?) {
        snapshot.context.properties.phoneNumber = phoneNumber
    }

    public func updateFirebaseLoginState(_ isLoggedIn: Bool) {
        snapshot.context.properties.isFirebaseLoggedIn = isLoggedIn
    }

    /// Logout-scoped property reset — port of Kotlin
    /// `resetSessionProperties` (coin balance 0, counts cleared, social
    /// sign-in off; pro availability is device-level and survives).
    public func resetSessionProperties() {
        let preserved = snapshot.context.properties.isYralProAvailable
        snapshot.context.properties = SessionProperties(
            coinBalance: 0,
            isSocialSignIn: false,
            profileVideosCount: 0,
            botCount: nil,
            accountDirectory: nil,
            isYralProAvailable: preserved
        )
    }
}
