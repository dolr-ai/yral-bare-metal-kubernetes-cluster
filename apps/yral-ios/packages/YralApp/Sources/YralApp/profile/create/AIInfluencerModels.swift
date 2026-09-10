// Models for the AI-influencer creation flow — colocated beside
// `AIInfluencerDataSource` (feature-local; see apps/yral-ios/AGENTS.md).
// Split from the data-source file to stay within the file-length lint.

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
        personalityTraits =
            (generated.personality_traits?.additionalProperties.value
                .mapValues { "\($0 ?? "")" }) ?? [:]
        category = generated.category
        avatarURL = generated.avatar_url
    }
}

// MARK: - Creator's influencer listing (the bots' real names)

/// One row of `GET /api/v1/creator/influencers` — the bot's real name
/// (`name`, the creation-time handle), optional display name, and its
/// SpacetimeDB subject (`bot_principal_id` is NOT in this response —
/// the `id` field IS the bot's subject, verified live: `id ==
/// `user_profiles_2.oauth_subject`).
public struct CreatorInfluencer: Equatable, Sendable {
    public let subject: String
    public let name: String
    public let displayName: String?
    public let avatarURL: String?
}

/// Decodes the untyped response container (spec defect — empty 200
/// schema) into typed rows. Pure: takes the runtime JSON value. The
/// runtime decodes objects to `[String: (any Sendable)?]` and arrays to
/// `[(any Sendable)?]` — the casts must match those EXACTLY (a plain
/// `as? [[String: (any Sendable)?]]` fails: array elements are
/// optionals of dictionaries, not dictionaries).
struct CreatorInfluencerList: Equatable {
    let influencers: [CreatorInfluencer]

    init(unvalidated: (any Sendable)?) throws {
        guard let dictionary = unvalidated as? [String: (any Sendable)?],
            let rows = dictionary["influencers"] as? [(any Sendable)?]
        else {
            // Local DECODE failure (not the server's words — the body
            // arrived but didn't match the known shape). Describe what
            // we received so diagnosis starts at the right layer.
            let received = String(describing: unvalidated)
            throw NetworkError.transport(
                underlying:
                    "creator-influencers: response body did not match the expected shape: \(received)"
            )
        }
        influencers = rows.compactMap { row -> CreatorInfluencer? in
            guard let row = row as? [String: (any Sendable)?],
                let subject = row["id"] as? String,
                let name = row["name"] as? String
            else { return nil }
            return CreatorInfluencer(
                subject: subject,
                name: name,
                displayName: row["display_name"] as? String,
                avatarURL: row["avatar_url"] as? String
            )
        }
    }
}
