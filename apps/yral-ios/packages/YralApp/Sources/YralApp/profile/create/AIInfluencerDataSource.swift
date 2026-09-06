import Foundation
import HTTPTypes
import OpenAPIRuntime
import OpenAPIURLSession

/// AI-influencer backend calls — TYPED via the swift-openapi-generator
/// client, generated at build time from `openapi.json` (byte-verbatim
/// snapshot of https://agent.rishi.yral.com/openapi.json, refreshed by
/// `mise run yral-ios-sync-rishi-openapi` before every build). Same
/// principle as the SpacetimeDB bindings: the live spec is the source
/// of truth, drift becomes a compile error.
///
/// Auth: bearer yral-auth id_token, injected per call via the
/// `BearerAuthenticationMiddleware` below (Apple's documented
/// ClientMiddleware pattern — the generator has no security-scheme
/// support, this is the canonical mechanism).
///
/// KNOWN SPEC GAP (tracked upstream): the generator (1.13.x) drops
/// OpenAPI 3.1 `anyOf: [T, {type: null}]` properties entirely
/// (apple/swift-openapi-generator#817) — so the generated
/// `Components.Schemas.CreateInfluencerRequest` only carries the 5
/// non-nullable fields today; avatar_url/description/category/
/// initial_greeting/suggested_messages/personality_traits/source are
/// absent until the upstream spec PR lands. Until then the create call
/// sends what the generated type can carry, and the server generates
/// greeting/suggestions when absent.
///
/// Persona generation is LLM-backed and slow (gemini thinking: 7s..>30s)
/// — both generation endpoints get the 90-second timeout.
///
/// Avatar lifecycle (per the server source): the metadata response's
/// `avatar_url` is a SHORT-LIVED Replicate delivery URL (reaped after
/// ~2h, may be null when generation fails) — display only. The durable
/// Storj URL comes from the profile-image upload step, which the
/// creation pipeline runs right after.
public struct AIInfluencerDataSource: Sendable {

    private let serverURL: URL
    private let session: URLSession

    public init(session: URLSession = .shared) {
        self.session = session
        self.serverURL = URL(string: "https://\(AppConfiguration.chatBaseURL)")!
    }

    /// A client with this call's bearer token. The generated Input has
    /// no middlewares parameter (the spec declares no security schemes)
    /// — the Client init is the injection point; constructing one per
    /// call is cheap (a thin struct over the transport).
    private func authenticatedClient(bearerToken: String) -> Client {
        Client(
            serverURL: serverURL,
            transport: URLSessionTransport(
                configuration: .init(
                    session: Self.personaGenerationSession
                )
            ),
            middlewares: [BearerAuthenticationMiddleware(bearerToken: bearerToken)]
        )
    }

    /// Kotlin `PERSONA_GEN_TIMEOUT_MS = 90s` — LLM-backed generation
    /// (gemini thinking: 7s..>30s, spikes beyond) exceeds URLSession's
    /// 60s default, so ALL generation-endpoint calls ride this
    /// dedicated session. (Create/upload calls share it too — one
    /// session keeps connection reuse simple; their latency is
    /// dominated by the same backend anyway.)
    private static let personaGenerationSession: URLSession = {
        let configuration = URLSessionConfiguration.default
        configuration.timeoutIntervalForRequest = 90
        configuration.timeoutIntervalForResource = 120
        return URLSession(configuration: configuration)
    }()

    // MARK: - Persona generation (Kotlin PERSONA_GEN_TIMEOUT_MS = 90s)

    /// `POST /api/v1/influencers/generate-prompt` — description →
    /// persona system instructions.
    public func generatePrompt(
        prompt: String,
        idToken: String
    ) async throws -> String {
        let response = try await authenticatedClient(bearerToken: idToken)
            .generate_prompt_api_v1_influencers_generate_prompt_post(
                .init(body: .json(.init(concept: prompt)))
            )
        switch response {
        case .ok(let payload):
            return try payload.body.json.system_instructions
        case .unprocessableContent(let payload):
            throw Self.validationError(try payload.body.json.detail)
        case .undocumented:
            throw Self.undocumented(operation: "generate-prompt")
        }
    }

    /// `POST /api/v1/influencers/validate-and-generate-metadata` — persona
    /// instructions → validated metadata (name, greeting, suggested
    /// messages, traits, avatar URL, category).
    public func validateAndGenerateMetadata(
        systemInstructions: String,
        idToken: String
    ) async throws -> AIInfluencerMetadata {
        let response = try await authenticatedClient(bearerToken: idToken)
            .validate_and_generate_api_v1_influencers_validate_and_generate_metadata_post(
                .init(body: .json(.init(concept: systemInstructions)))
            )
        switch response {
        case .ok(let payload):
            return AIInfluencerMetadata(
                from: try payload.body.json
            )
        case .unprocessableContent(let payload):
            throw Self.validationError(try payload.body.json.detail)
        case .undocumented:
            throw Self.undocumented(operation: "validate-and-generate-metadata")
        }
    }

    /// `POST /api/v1/influencers/create` — the AI account's backend
    /// record. The backend derives the owner from the auth token. The
    /// avatar URL passed here is the DURABLE hosted one (from the
    /// profile-image upload step), not the short-lived generated one.
    func createInfluencer(
        profile: AIProfileDetails,
        aiPrincipalID: String,
        hostedAvatarURL: String,
        idToken: String
    ) async throws {
        let response = try await authenticatedClient(bearerToken: idToken)
            .create_influencer_api_v1_influencers_create_post(
                .init(
                    body: .json(
                        .init(
                            name: profile.name,
                            display_name: profile.displayName,
                            system_instructions: profile.systemInstructions,
                            bot_principal_id: aiPrincipalID,
                            avatar_url: hostedAvatarURL,
                            description: profile.description,
                            category: profile.category,
                            personality_traits: .init(
                                additionalProperties: .init(
                                    unvalidatedValue: profile.personalityTraits
                                )
                            ),
                            initial_greeting: profile.initialGreeting,
                            suggested_messages: profile.suggestedMessages,
                            is_nsfw: profile.isNSFW,
                            source: "ios"
                        )))
            )
        switch response {
        case .created:
            // 201 — the created record is not consumed by the flow.
            break
        case .conflict:
            // 409 — the persona name is already taken; the user edits
            // the name on the review form and retries (the progress
            // record keeps steps 1–4, no re-mint).
            throw NetworkError.http(
                statusCode: 409,
                body: "Name '\(profile.name)' is already taken"
            )
        case .unprocessableContent(let payload):
            throw Self.validationError(try payload.body.json.detail)
        case .undocumented:
            throw Self.undocumented(operation: "create-influencer")
        }
    }

    // MARK: - Avatar upload (the durable-URL step)

    /// `POST /api/v1/user/profile-image` — uploads base64 image bytes,
    /// returns the hosted durable public URL (Storj link-share; the
    /// old storage-interface.prakash host is dead — this endpoint lives
    /// on the agent service itself).
    public func uploadProfileImage(
        imageBase64: String,
        idToken: String
    ) async throws -> String {
        let response = try await authenticatedClient(bearerToken: idToken)
            .upload_profile_image_api_v1_user_profile_image_post(
                .init(body: .json(.init(image_data: imageBase64)))
            )
        switch response {
        case .ok(let payload):
            // FULLY TYPED in the spec (response_model declared) — no
            // OpenAPIValueContainer needed here.
            return try payload.body.json.profile_image_url
        case .unprocessableContent(let payload):
            throw Self.validationError(try payload.body.json.detail)
        case .undocumented:
            throw Self.undocumented(operation: "profile-image")
        }
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

    // MARK: - Error mapping

    /// FastAPI validation errors carry the user-facing message under
    /// "detail" — an ARRAY of validation objects, each with a `msg`.
    private static func validationError(
        _ detail: [Components.Schemas.ValidationError]?
    ) -> NetworkError {
        let message = detail?.first?.msg ?? "Validation failed"
        return NetworkError.http(statusCode: 422, body: message)
    }

    private static func undocumented(operation: String) -> NetworkError {
        NetworkError.transport(
            underlying: "\(operation): unexpected response (spec drift)"
        )
    }
}

// MARK: - Bearer auth middleware (Apple's ClientMiddleware pattern)

/// Injects `Authorization: Bearer <token>` on every request. The
/// generator emits no auth code from security schemes (unsupported) —
/// middleware is the Apple-canonical per-call auth injection point
/// (apple/swift-openapi-generator auth-client-middleware-example).
private struct BearerAuthenticationMiddleware: ClientMiddleware {
    let bearerToken: String

    func intercept(
        _ request: HTTPRequest,
        body: HTTPBody?,
        baseURL: URL,
        operationID: String,
        next: @Sendable (HTTPRequest, HTTPBody?, URL) async throws -> (HTTPResponse, HTTPBody?)
    ) async throws -> (HTTPResponse, HTTPBody?) {
        var request = request
        request.headerFields[.authorization] = "Bearer \(bearerToken)"
        return try await next(request, body, baseURL)
    }
}

// MARK: - Generated persona metadata (the wizard's model)

/// The persona metadata from `validate-and-generate-metadata`. The
/// rejected case carries only `is_valid: false` plus a `reason`; the
/// accepted case carries the full persona. `avatarURL` may be empty
/// when avatar generation fails server-side — and when present it is
/// a SHORT-LIVED Replicate delivery URL; the creation pipeline
/// uploads the bytes for the durable copy.
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

    init(from generated: Components.Schemas.ValidateAndGenerateResponse) {
        isValid = generated.is_valid
        validationReason = generated.reason
        name = generated.name
        displayName = generated.display_name
        description = generated.description
        initialGreeting = generated.initial_greeting
        suggestedMessages = generated.suggested_messages ?? []
        // The generated payload is a free-form object container — read
        // the values back as strings (the server stores string traits).
        personalityTraits = (generated.personality_traits?.additionalProperties.value
            .mapValues { "\($0 ?? "")" }) ?? [:]
        category = generated.category
        avatarURL = generated.avatar_url
    }
}
