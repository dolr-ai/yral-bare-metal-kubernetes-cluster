import Foundation

/// AI-influencer backend calls — port of Kotlin `AiInfluencerRemoteDataSource`
/// (agent.rishi.yral.com, Bearer id_token) plus the avatar upload endpoint
/// from `AiInfluencerViewModel.uploadProfileImage` (STORAGE_INTERFACE host).
///
/// Persona generation is LLM-backed and slow (gemini thinking: 7s..>30s) —
/// both generation endpoints get the Kotlin 90-second timeout.
public struct AIInfluencerDataSource: Sendable {

    private let session: URLSession

    public init(session: URLSession = .shared) {
        self.session = session
    }

    // MARK: - Persona generation (Kotlin PERSONA_GEN_TIMEOUT_MS = 90s)

    /// `POST /api/v1/influencers/generate-prompt` — description →
    /// persona system instructions.
    public func generatePrompt(
        prompt: String,
        idToken: String
    ) async throws -> String {
        let object: [String: Any] = ["prompt": prompt]
        let data = try await post(
            path: "api/v1/influencers/generate-prompt",
            body: object,
            idToken: idToken,
            timeout: 90
        )
        return try stringField(data, "system_instructions")
    }

    /// `POST /api/v1/influencers/validate-and-generate-metadata` — persona
    /// instructions → validated metadata (name, greeting, suggested
    /// messages, traits, avatar URL, category).
    public func validateAndGenerateMetadata(
        systemInstructions: String,
        idToken: String
    ) async throws -> AIInfluencerMetadata {
        let object: [String: Any] = ["system_instructions": systemInstructions]
        let data = try await post(
            path: "api/v1/influencers/validate-and-generate-metadata",
            body: object,
            idToken: idToken,
            timeout: 90
        )
        return try AIInfluencerMetadata(json: data)
    }

    /// `POST /api/v1/influencers/create` — the AI account's backend record.
    public func createInfluencer(
        request: CreateInfluencerRequest,
        idToken: String
    ) async throws -> CreateInfluencerResponse {
        let data = try await post(
            path: "api/v1/influencers/create",
            body: request.jsonObject,
            idToken: idToken,
            timeout: 30
        )
        return try CreateInfluencerResponse(json: data)
    }

    // MARK: - Avatar (Kotlin uploadProfileImage: STORAGE_INTERFACE host)

    /// `POST /api/v1/user/profile-image` — uploads base64 image bytes,
    /// returns the hosted URL.
    public func uploadProfileImage(
        imageBase64: String,
        idToken: String
    ) async throws -> String {
        let object: [String: Any] = ["image_data": imageBase64]
        let data = try await post(
            host: AppConfiguration.storageInterfaceBaseURL,
            path: "api/v1/user/profile-image",
            body: object,
            idToken: idToken,
            timeout: 30
        )
        return try stringField(data, "profile_image_url")
    }

    /// Downloads the generated avatar bytes (Kotlin `downloadAvatar`).
    public func downloadAvatar(url: URL) async throws -> Data {
        let (data, response) = try await session.data(from: url)
        guard let httpResponse = response as? HTTPURLResponse,
              (200..<300).contains(httpResponse.statusCode)
        else {
            throw NetworkError.http(
                statusCode: (response as? HTTPURLResponse)?.statusCode ?? 0,
                body: nil
            )
        }
        return data
    }

    // MARK: - Request plumbing (inline — no client wrapper)

    private func post(
        host: String = AppConfiguration.chatBaseURL,
        path: String,
        body: [String: Any],
        idToken: String,
        timeout: TimeInterval
    ) async throws -> [String: Any] {
        var request = URLRequest(url: URL(string: "https://\(host)/\(path)")!)
        request.httpMethod = "POST"
        request.timeoutInterval = timeout
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("Bearer \(idToken)", forHTTPHeaderField: "Authorization")
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        let (data, response) = try await session.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse else {
            throw NetworkError.transport(underlying: "Non-HTTP response")
        }
        guard (200..<300).contains(httpResponse.statusCode) else {
            // The server's error body is the user-facing message (Kotlin
            // extractServerMessage).
            throw NetworkError.http(
                statusCode: httpResponse.statusCode,
                body: String(data: data, encoding: .utf8)
            )
        }
        guard let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw NetworkError.transport(underlying: "Response is not a JSON object")
        }
        return object
    }

    private func stringField(_ object: [String: Any], _ key: String) throws -> String {
        guard let value = object[key] as? String, !value.isEmpty else {
            throw NetworkError.transport(underlying: "Response missing '\(key)'")
        }
        return value
    }
}

// MARK: - DTOs (Kotlin ValidateAndGenerateMetadataResponseDto etc.)

/// Kotlin `GeneratedInfluencerMetadata`.
public struct AIInfluencerMetadata: Equatable, Sendable {
    public var isValid: Bool
    public var validationReason: String?
    public var name: String?
    public var displayName: String?
    public var description: String?
    public var initialGreeting: String?
    public var suggestedMessages: [String]
    public var personalityTraits: [String: String]
    public var category: String?
    public var avatarURL: String?
    public var systemInstructions: String?
    public var isNSFW: Bool

    init(json: [String: Any]) {
        isValid = json["is_valid"] as? Bool ?? false
        validationReason = json["reason"] as? String
        name = json["name"] as? String
        displayName = json["display_name"] as? String
        description = json["description"] as? String
        initialGreeting = json["initial_greeting"] as? String
        suggestedMessages = (json["suggested_messages"] as? [String]) ?? []
        personalityTraits = (json["personality_traits"] as? [String: String]) ?? [:]
        category = json["category"] as? String
        avatarURL = json["avatar_url"] as? String
        systemInstructions = json["system_instructions"] as? String
        isNSFW = json["is_nsfw"] as? Bool ?? false
    }
}

/// Kotlin `CreateInfluencerRequestDto` (snake_case wire keys).
public struct CreateInfluencerRequest: Sendable {
    public var name: String
    public var displayName: String
    public var description: String
    public var systemInstructions: String
    public var initialGreeting: String
    public var suggestedMessages: [String]
    public var personalityTraits: [String: String]
    public var category: String
    public var avatarURL: String
    public var isNSFW: Bool
    public var aiPrincipalID: String
    public var parentPrincipalID: String

    public var jsonObject: [String: Any] {
        [
            "name": name,
            "display_name": displayName,
            "description": description,
            "system_instructions": systemInstructions,
            "initial_greeting": initialGreeting,
            "suggested_messages": suggestedMessages,
            "personality_traits": personalityTraits,
            "category": category,
            "avatar_url": avatarURL,
            "is_nsfw": isNSFW,
            // Wire key is Kotlin's DTO field name — the server schema calls
            // the AI account a "bot" (kept verbatim on the wire).
            "bot_principal_id": aiPrincipalID,
            "parent_principal_id": parentPrincipalID
        ]
    }
}

/// Kotlin `CreateInfluencerResponseDto`.
public struct CreateInfluencerResponse: Sendable {
    public var id: String
    public var starterVideoPrompt: String?

    init(json: [String: Any]) throws {
        guard let id = json["id"] as? String else {
            throw NetworkError.transport(underlying: "Create response missing 'id'")
        }
        self.id = id
        starterVideoPrompt = json["starter_video_prompt"] as? String
    }
}
