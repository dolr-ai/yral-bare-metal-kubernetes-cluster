import Foundation

/// AI account creation pipeline — port of Kotlin
/// `AiInfluencerViewModel.createBotAccount` + `completeBotSetup`.
///
/// The sequence (each step guarded by the progress record, so a retry
/// resumes where it failed):
///  1. Register the OWNER (main account) in SpacetimeDB — a MainAccount row
///     must exist before an AI account can attach ("Owner not found"
///     otherwise). Same reducer with `mainAccount = null`; idempotent.
///  2. Mint the AI account via yral-auth (`create_ai_account`).
///  3. Attach it (`accept_new_user_registration` with
///     `mainAccount = <owner principal>`).
///  4. Profile: update bio + hosted avatar via `update_profile_details`.
///  5. Backend record: `create` on the influencer API.
///  6. Finalize: persist the AI identity, switch the active session.
///
/// The username update (Kotlin tryUpdateUsername/set_user_metadata) is a
/// no-op in the Kotlin source too (the TODO'd SpacetimeDB REST call is
/// commented out — it just normalizes and returns `updated = true`), so
/// this port normalizes only, matching shipped behavior.
struct AICreationProgress {
    let profileKey: String
    var aiPrincipal: String?
    var ownerRegistered = false
    var registrationAccepted = false
    var avatarBytes: Data?
    var hostedAvatarURL: String?
    var profileUpdated = false
    var influencerCreated = false
    var finalized = false
}

enum AIAccountCreator {

    /// Context — the collaborators of one creation run (thin struct so
    /// each step function takes a single parameter).
    @MainActor
    struct CreationContext {
        let authClient: AuthClient
        let sessionStore: SessionStore
        let influencerDataSource: AIInfluencerDataSource
        let spacetime: SpacetimeDBRemoteDataSource
    }

    /// Kotlin `createBotAccount` + `completeBotSetup` end-to-end. Returns
    /// the created AI principal; the session switch happens on success
    /// (Kotlin setActiveBotSession).
    @MainActor
    static func create(
        profile: AIProfileDetails,
        progress: inout AICreationProgress,
        context: CreationContext
    ) async throws -> String {
        guard let ownerPrincipal = context.sessionStore.userPrincipal else {
            throw AuthError.oauthFailed(errorDescription: "Not signed in")
        }

        if !progress.ownerRegistered {
            try await registerOwner(ownerPrincipal: ownerPrincipal, context: context)
            progress.ownerRegistered = true
        }

        if progress.aiPrincipal == nil {
            progress.aiPrincipal = try await mintAIAccount(
                ownerPrincipal: ownerPrincipal, context: context
            )
        }
        let aiPrincipal = try require(progress.aiPrincipal)

        if !progress.registrationAccepted {
            try await attachAIAccount(
                ownerPrincipal: ownerPrincipal,
                aiPrincipal: aiPrincipal,
                context: context
            )
            progress.registrationAccepted = true
        }

        let hostedAvatarURL = try await updateProfile(
            profile: profile,
            aiPrincipal: aiPrincipal,
            progress: &progress,
            context: context
        )

        if !progress.influencerCreated {
            try await createInfluencerRecord(
                profile: profile,
                ownerPrincipal: ownerPrincipal,
                aiPrincipal: aiPrincipal,
                hostedAvatarURL: hostedAvatarURL,
                context: context
            )
            progress.influencerCreated = true
        }

        if !progress.finalized {
            try finalize(
                profile: profile,
                ownerPrincipal: ownerPrincipal,
                aiPrincipal: aiPrincipal,
                hostedAvatarURL: hostedAvatarURL,
                context: context
            )
            progress.finalized = true
        }

        return aiPrincipal
    }

    // MARK: - Steps (each idempotent-guarded by the progress record)

    /// Step 1 — register the OWNER (main account): a MainAccount row must
    /// exist before an AI account can attach ("Owner not found"
    /// otherwise). Same reducer with `mainAccount = nil`; idempotent.
    @MainActor
    private static func registerOwner(
        ownerPrincipal: String,
        context: CreationContext
    ) async throws {
        try await context.spacetime.acceptNewUserRegistration(
            newPrincipalText: ownerPrincipal,
            authenticated: true,
            mainAccountText: nil
        )
    }

    /// Step 2 — mint the AI identity via yral-auth.
    @MainActor
    private static func mintAIAccount(
        ownerPrincipal: String,
        context: CreationContext
    ) async throws -> String {
        guard let idToken = context.authClient.idToken else {
            throw AuthError.oauthFailed(errorDescription: "Not signed in")
        }
        return try await context.authClient.authDataSource.createAiAccount(
            userID: ownerPrincipal,
            idToken: idToken
        )
    }

    /// Step 3 — attach the AI account under the owner.
    @MainActor
    private static func attachAIAccount(
        ownerPrincipal: String,
        aiPrincipal: String,
        context: CreationContext
    ) async throws {
        try await context.spacetime.acceptNewUserRegistration(
            newPrincipalText: aiPrincipal,
            authenticated: true,
            mainAccountText: ownerPrincipal
        )
    }

    /// Step 4 — avatar download → upload → bio + picture URL
    /// (Kotlin downloadAvatar + uploadProfileImage + update_profile_details).
    @MainActor
    private static func updateProfile(
        profile: AIProfileDetails,
        aiPrincipal: String,
        progress: inout AICreationProgress,
        context: CreationContext
    ) async throws -> String {
        if let hostedAvatarURL = progress.hostedAvatarURL {
            return hostedAvatarURL
        }
        guard let idToken = context.authClient.idToken else {
            throw AuthError.oauthFailed(errorDescription: "Not signed in")
        }
        let avatarBytes = try await downloadAvatarBytes(
            profile: profile, progress: &progress, context: context
        )
        let hostedAvatarURL = try await context.influencerDataSource.uploadProfileImage(
            imageBase64: avatarBytes.base64EncodedString(),
            idToken: idToken
        )
        try await context.spacetime.updateProfileDetails(
            bio: profile.description,
            websiteURL: nil,
            profilePictureURL: hostedAvatarURL
        )
        progress.profileUpdated = true
        progress.hostedAvatarURL = hostedAvatarURL
        return hostedAvatarURL
    }

    /// Step 5 — the backend influencer record.
    @MainActor
    private static func createInfluencerRecord(
        profile: AIProfileDetails,
        ownerPrincipal: String,
        aiPrincipal: String,
        hostedAvatarURL: String,
        context: CreationContext
    ) async throws {
        guard let idToken = context.authClient.idToken else {
            throw AuthError.oauthFailed(errorDescription: "Not signed in")
        }
        _ = try await context.influencerDataSource.createInfluencer(
            request: CreateInfluencerRequest(
                name: profile.name,
                displayName: profile.displayName,
                description: profile.description,
                systemInstructions: profile.systemInstructions,
                initialGreeting: profile.initialGreeting,
                suggestedMessages: profile.suggestedMessages,
                personalityTraits: profile.personalityTraits,
                category: profile.category,
                avatarURL: hostedAvatarURL,
                isNSFW: profile.isNSFW,
                aiPrincipalID: aiPrincipal,
                parentPrincipalID: ownerPrincipal
            ),
            idToken: idToken
        )
    }

    /// Step 6 — persist the AI identity + switch the active session
    /// (Kotlin setActiveBotSession; the AI account becomes last-active so
    /// relaunches continue as it until switched).
    @MainActor
    private static func finalize(
        profile: AIProfileDetails,
        ownerPrincipal: String,
        aiPrincipal: String,
        hostedAvatarURL: String,
        context: CreationContext
    ) {
        AIIdentitiesStore.saveIdentity(
            principal: aiPrincipal,
            username: profile.name,
            defaults: context.authClient.defaults
        )
        let aiSession = Session(
            canisterID: aiPrincipal,
            userPrincipal: aiPrincipal,
            profilePic: hostedAvatarURL,
            username: profile.name,
            bio: profile.description,
            isCreatedFromServiceCanister: true,
            isAIAccount: true
        )
        context.sessionStore.updateState(.signedIn(aiSession))
        context.authClient.cacheSession(
            canisterID: aiPrincipal,
            userPrincipal: aiPrincipal,
            profilePic: hostedAvatarURL,
            username: profile.name,
            isAIAccount: true
        )
        context.authClient.keychain.setString(aiPrincipal, forKey: .lastActivePrincipal)
    }

    /// Avatar bytes — generated URL download (Kotlin downloadAvatar),
    /// memoized in the progress record.
    @MainActor
    private static func downloadAvatarBytes(
        profile: AIProfileDetails,
        progress: inout AICreationProgress,
        context: CreationContext
    ) async throws -> Data {
        if let bytes = progress.avatarBytes {
            return bytes
        }
        guard let avatarURL = URL(string: profile.avatarURL) else {
            throw NetworkError.transport(underlying: "Profile has a malformed avatar URL")
        }
        let bytes = try await context.influencerDataSource.downloadAvatar(url: avatarURL)
        progress.avatarBytes = bytes
        return bytes
    }

    private static func require<T>(_ value: T?) throws -> T {
        guard let value else {
            throw NetworkError.transport(underlying: "Creation state was reset mid-flow")
        }
        return value
    }
}

/// Kotlin `AiInfluencerStep.ProfileDetails` — the validated creation
/// payload (everything the backend record + profile need).
struct AIProfileDetails {
    var systemInstructions: String
    var name: String
    var displayName: String
    var description: String
    var avatarURL: String
    var initialGreeting: String
    var suggestedMessages: [String]
    var personalityTraits: [String: String]
    var category: String
    var isNSFW: Bool

    /// Kotlin progressKey — identity of the creation attempt for resume.
    var profileKey: String {
        "\(name)|\(displayName)|\(description.hashValue)"
    }
}
