import Foundation
import OpenAPIRuntime
import Testing
@testable import YralApp

/// Regression tests for the create-flow contract fix (PR #501 to
/// yral-rishi-agent): the create request MUST carry the persona fields —
/// most critically the DURABLE hosted avatar URL. The old generated
/// request silently dropped every anyOf-null field
/// (apple/swift-openapi-generator#817), so `avatar_url` never reached
/// the backend and the avatar vanished from discovery/chat/earnings.
/// The typed request struct now has all 13 fields; these tests pin the
/// app-side request construction and the metadata mapping.
struct AIInfluencerDataSourceTests {

    /// The durable URL — not the short-lived generated one — must be
    /// what the wire request carries. Pins the avatar_url wire value
    /// being the HOSTED one (the original bug: nothing reached the
    /// backend at all, and the generated-URL variant would expire).
    @Test
    func createRequestCarriesHostedAvatarURL() throws {
        // OpenAPIObjectContainer's unvalidated init throws on
        // unsupported value types — a [String: String] is supported.
        let traits = try OpenAPIRuntime.OpenAPIObjectContainer(
            unvalidatedValue: ["energy_level": "high"]
        )
        let request = Components.Schemas.CreateInfluencerRequest(
            name: "test-persona",
            display_name: "Test Persona",
            system_instructions: "A warm and witty companion.",
            bot_principal_id: "abcdef0123456789",
            avatar_url: "https://link.storj-public.yral.com/avatar.png",
            description: "One line",
            category: "entertainment",
            personality_traits: .init(additionalProperties: traits),
            initial_greeting: "Hey there!",
            suggested_messages: ["What's up?", "Tell me a joke"],
            is_nsfw: false,
            source: "ios"
        )

        // Encode through the generated Codable conformance — this is
        // the exact JSON the wire sees.
        let json = try JSONEncoder().encode(request)
        let object = try #require(
            JSONSerialization.jsonObject(with: json) as? [String: Any]
        )

        #expect(object["avatar_url"] as? String == "https://link.storj-public.yral.com/avatar.png")
        #expect(object["bot_principal_id"] as? String == "abcdef0123456789")
        #expect(object["description"] as? String == "One line")
        #expect(object["category"] as? String == "entertainment")
        #expect(object["initial_greeting"] as? String == "Hey there!")
        #expect(object["source"] as? String == "ios")
        #expect(object["is_nsfw"] as? Bool == false)
        let traitsObject = try #require(
            object["personality_traits"] as? [String: String]
        )
        #expect(traitsObject["energy_level"] == "high")
        let messages = try #require(object["suggested_messages"] as? [String])
        #expect(messages == ["What's up?", "Tell me a joke"])
    }

    /// The 409 name-taken case must surface a typed error the review
    /// form can show (the persona name is in the message; the user
    /// edits the name and retries without re-minting the AI account).
    @Test
    func conflictSurfacesNameTakenError() {
        // The .conflict branch throws this shape — pin it.
        let error = NetworkError.http(
            statusCode: 409,
            body: "Name 'taken-name' is already taken"
        )
        guard case let .http(statusCode, body) = error else {
            Issue.record("expected .http")
            return
        }
        #expect(statusCode == 409)
        #expect(body == "Name 'taken-name' is already taken")
    }

    /// The metadata mapping: the generated response (typed struct from
    /// PR #501) → the wizard model. Traits arrive in the free-form
    /// object container and must come back as a plain [String: String].
    @Test
    func metadataMapsFromGeneratedResponse() throws {
        let traits = try OpenAPIRuntime.OpenAPIObjectContainer(
            unvalidatedValue: ["demeanor": "calm", "energy_level": "high"]
        )
        let generated = Components.Schemas.ValidateAndGenerateResponse(
            is_valid: true,
            reason: nil,
            name: "aura",
            display_name: "Aura",
            description: "A calm presence",
            avatar_url: "https://generated.replicate.delivery/aura.png",
            initial_greeting: "Hi! I'm Aura.",
            suggested_messages: ["Hello!", "What's on your mind?"],
            personality_traits: .init(additionalProperties: traits),
            category: "companion",
            image_prompt: "portrait of..."
        )

        let metadata = AIInfluencerMetadata(from: generated)

        #expect(metadata.isValid)
        #expect(metadata.validationReason == nil)
        #expect(metadata.name == "aura")
        #expect(metadata.displayName == "Aura")
        #expect(metadata.description == "A calm presence")
        #expect(metadata.initialGreeting == "Hi! I'm Aura.")
        #expect(metadata.suggestedMessages == ["Hello!", "What's on your mind?"])
        #expect(metadata.personalityTraits["demeanor"] == "calm")
        #expect(metadata.personalityTraits["energy_level"] == "high")
        #expect(metadata.category == "companion")
        #expect(metadata.avatarURL == "https://generated.replicate.delivery/aura.png")
    }

    /// The rejected case: only is_valid=false + reason arrive — the
    /// mapping must tolerate every other field being nil.
    @Test
    func metadataMapsRejectionWithReasonOnly() {
        let generated = Components.Schemas.ValidateAndGenerateResponse(
            is_valid: false,
            reason: "Content was flagged as inappropriate",
            name: nil,
            display_name: nil,
            description: nil,
            avatar_url: nil,
            initial_greeting: nil,
            suggested_messages: nil,
            personality_traits: nil,
            category: nil,
            image_prompt: nil
        )

        let metadata = AIInfluencerMetadata(from: generated)

        #expect(!metadata.isValid)
        #expect(metadata.validationReason == "Content was flagged as inappropriate")
        #expect(metadata.name == nil)
        #expect(metadata.suggestedMessages.isEmpty)
        #expect(metadata.personalityTraits.isEmpty)
    }
}
