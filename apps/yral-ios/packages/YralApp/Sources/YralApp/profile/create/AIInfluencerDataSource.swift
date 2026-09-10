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
    /// call is cheap (a thin struct over the transport). Non-generation
    /// calls (create/upload/creator-listing) ride the injected `session`
    /// (default `.shared`) — that's also the URLProtocol test seam.
    private func authenticatedClient(bearerToken: String) -> Client {
        Client(
            serverURL: serverURL,
            transport: URLSessionTransport(
                configuration: .init(session: session)
            ),
            middlewares: [BearerAuthenticationMiddleware(bearerToken: bearerToken)]
        )
    }

    /// The generation endpoints' client — same auth injection, but on
    /// the dedicated 90s LLM-timeout session (gemini thinking exceeds
    /// URLSession's 60s default).
    private func generationClient(bearerToken: String) -> Client {
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
        let response = try await generationClient(bearerToken: idToken)
            .generate_prompt_api_v1_influencers_generate_prompt_post(
                .init(body: .json(.init(concept: prompt)))
            )
        switch response {
        case .ok(let payload):
            return try payload.body.json.system_instructions
        case .unprocessableContent(let payload):
            throw Self.validationError(try payload.body.json.detail)
        case .undocumented(let statusCode, let payload):
            throw await Self.undocumented(
                operation: "generate-prompt", statusCode: statusCode, payload: payload
            )
        }
    }

    /// `POST /api/v1/influencers/validate-and-generate-metadata` — persona
    /// instructions → validated metadata (name, greeting, suggested
    /// messages, traits, avatar URL, category).
    public func validateAndGenerateMetadata(
        systemInstructions: String,
        idToken: String
    ) async throws -> AIInfluencerMetadata {
        let response = try await generationClient(bearerToken: idToken)
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
        case .undocumented(let statusCode, let payload):
            // Undeclared status — the server's known failure mode here
            // is 500 when LLM metadata generation fails (the handler
            // returns None → HTTPException 500). Surface the upstream
            // body verbatim (Hard Rule: never hide upstream errors).
            throw await Self.undocumented(
                operation: "validate-and-generate-metadata",
                statusCode: statusCode,
                payload: payload
            )
        }
    }

    /// `GET /api/v1/creator/influencers` — the authenticated creator's
    /// bots with their real names (`name` = the handle used at creation,
    /// `display_name`, `avatar_url`). This is where a bot's NAME lives —
    /// the SpacetimeDB profile row carries no displayable name.
    ///
    /// SPEC DEFECT (upstream PR): the 200 response declares an empty
    /// schema (`{}`), so the generator emits an untyped
    /// `OpenAPIValueContainer` — decoded here via a JSON round-trip into
    /// the typed `CreatorInfluencer` model. The endpoint is
    /// Bearer-authenticated (the backend derives the creator from the
    /// token).
    public func listMyInfluencers(
        idToken: String
    ) async throws -> [CreatorInfluencer] {
        let response = try await authenticatedClient(bearerToken: idToken)
            .list_my_influencers_api_v1_creator_influencers_get(
                .init()
            )
        switch response {
        case .ok(let payload):
            let body = try payload.body.json
            return try CreatorInfluencerList(
                unvalidated: body.value
            ).influencers
        case .undocumented(let statusCode, let payload):
            throw await Self.undocumented(
                operation: "creator-influencers", statusCode: statusCode, payload: payload
            )
        }
    }

    /// `POST /api/v1/influencers/create` — the AI account's backend
    /// record. The backend derives the owner from the auth token. The
    /// avatar URL passed here is the DURABLE hosted one (from the
    /// profile-image upload step), not the short-lived generated one.
    func createInfluencer(
        profile: AIProfileDetails,
        aiSubjectID: String,
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
                            bot_principal_id: aiSubjectID,
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
        case .undocumented(let statusCode, let payload):
            throw await Self.undocumented(
                operation: "create-influencer", statusCode: statusCode, payload: payload
            )
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
        case .undocumented(let statusCode, let payload):
            throw await Self.undocumented(
                operation: "profile-image", statusCode: statusCode, payload: payload
            )
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

    /// Undeclared response status — surface the status AND the upstream
    /// body verbatim (Hard Rule: never hide upstream API errors behind
    /// our own generic text). The body is FastAPI's error JSON (e.g.
    /// {"detail":"Failed to validate and generate metadata"}); when it
    /// can't be read, fall back to the bare status. 1 MB cap per the
    /// HTTPBody collecting-initializer contract (memory safety).
    private static func undocumented(
        operation: String,
        statusCode: Int,
        payload: OpenAPIRuntime.UndocumentedPayload
    ) async -> NetworkError {
        guard let payloadBody = payload.body else {
            return .http(statusCode: statusCode, body: "\(operation): HTTP \(statusCode)")
        }
        let upstreamBody = try? await String(
            collecting: payloadBody,
            upTo: 1 << 20
        )
        if let upstreamBody, !upstreamBody.isEmpty {
            return .http(statusCode: statusCode, body: upstreamBody)
        }
        return .http(statusCode: statusCode, body: "\(operation): HTTP \(statusCode)")
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
