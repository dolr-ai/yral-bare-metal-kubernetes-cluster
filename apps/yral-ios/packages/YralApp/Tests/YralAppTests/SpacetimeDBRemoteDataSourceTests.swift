import Foundation
import Testing
@testable import YralApp

/// Wire-format pins for the SpacetimeDB reducer/procedure calls —
/// encoded by the TYPED models (SpacetimeWireModels) via JSONEncoder.
///
/// Each test asserts the EXACT request body we POST to
/// `/v1/database/{db}/call/{name}` against the LIVE reducer signature
/// (generated bindings in apps/yral-database-spacetime — the source of
/// truth). These pins catch wire drift — the failure mode of both
/// "invalid arguments for reducer" bugs:
///   1. arity/shape drift (the 3-arg pre-migration wire shape against
///      the live 4-arg reducer), and
///   2. invalid JSON from the old hand-rolled encoder (no escaping —
///      an LLM bio containing a quote/newline broke the body).
///
/// Wire rules (SpacetimeDB serde bridge): plain `null` = `None`;
/// `Some(v)` = `[0, payload]` (ARG form, payload inlined); structs =
/// positional arrays.
struct SpacetimeDBRemoteDataSourceTests {

    /// Encodes arguments exactly as the transport does (JSONEncoder with
    /// `.withoutEscapingSlashes` — mirroring the transport encoder).
    private func wireBody(of arguments: some Encodable) throws -> String {
        let wireEncoder = JSONEncoder()
        wireEncoder.outputFormatting = .withoutEscapingSlashes
        let data = try wireEncoder.encode(arguments)
        return String(data: data, encoding: .utf8) ?? ""
    }

    // MARK: - update_profile_details (4 args — live signature)

    @Test func updateProfileDetailsSendsFourArgumentsMatchingLiveReducer() throws {
        let body = try wireBody(
            of: UpdateProfileDetailsArguments(
                bio: "A witty travel photographer",
                websiteURL: nil,
                profilePicture: SpacetimeWireProfilePictureData(
                    url: "https://images.yral.com/avatar.png",
                    nsfwInfo: SpacetimeWireNSFWInfo(isNSFW: false, nsfwEC: "", nsfwGore: "", csamDetected: false)
                ),
                updateAsAIAccountID: "ai-principal-1"
            )
        )
        // Exact wire body for the live 4-arg reducer. Option<String>
        // args encode as Some([0, value]) — the ARG-form sum encoding
        // SpacetimeDB's serde bridge documents (and accepts) for
        // `Some`; `None` is bare null.
        let expected = "["
            + "[0,\"A witty travel photographer\"],"
            + "null,"
            + "[0,[\"https://images.yral.com/avatar.png\",[false,\"\",\"\",false]]],"
            + "[0,\"ai-principal-1\"]"
            + "]"
        #expect(body == expected)
    }

    /// The regression that bit twice: an LLM-generated bio containing
    /// quotes and newlines MUST produce valid escaped JSON. The old
    /// hand-rolled encoder emitted a raw interpolated string — one
    /// quote in the bio made the whole body unparseable →
    /// "invalid arguments for reducer" at argument validation.
    @Test func updateProfileDetailsEscapesLLMTextInBio() throws {
        let bioWithQuotesAndNewlines = "Says \"hello\" loudly,\nloves O'Brien — backslash \\ too"
        let body = try wireBody(
            of: UpdateProfileDetailsArguments(
                bio: bioWithQuotesAndNewlines,
                websiteURL: nil,
                profilePicture: nil,
                updateAsAIAccountID: "ai-principal-1"
            )
        )
        // The body must parse as valid JSON (JSONSerialization accepts
        // only well-formed documents) AND the bio must survive the
        // escape round-trip intact — element 0 is the Some wrapper
        // [0, "bio…"].
        let parsed = try JSONSerialization.jsonObject(with: Data(body.utf8)) as? [Any]
        let bioWrapper = parsed?.first as? [Any]
        #expect(bioWrapper?.last as? String == bioWithQuotesAndNewlines)
    }

    @Test func updateProfileDetailsAIClaimEncodesSomeForm() throws {
        let body = try wireBody(
            of: UpdateProfileDetailsArguments(
                bio: nil,
                websiteURL: nil,
                profilePicture: nil,
                updateAsAIAccountID: "auth0|ai-account-1"
            )
        )
        // The 4th arg MUST be [0, id] (Some) — never a bare string: the
        // live reducer takes Option<String>.
        #expect(body == #"[null,null,null,[0,"auth0|ai-account-1"]]"#)
    }

    @Test func updateProfilePictureArgIsStructNotBareString() throws {
        let body = try wireBody(
            of: UpdateProfileDetailsArguments(
                bio: nil,
                websiteURL: nil,
                profilePicture: SpacetimeWireProfilePictureData(
                    url: "https://images.yral.com/pic.png",
                    nsfwInfo: SpacetimeWireNSFWInfo(isNSFW: false, nsfwEC: "", nsfwGore: "", csamDetected: false)
                ),
                updateAsAIAccountID: nil
            )
        )
        // The 3rd arg MUST be [0, [url, NSFWInfo]] (Some of the struct) —
        // the old wire shape sent a BARE STRING and failed arg validation.
        #expect(body.contains(#"[0,["https://images.yral.com/pic.png",[false,"","",false]]]"#))
    }

    @Test func updateProfileDetailsAllNullsEncodeAsBareNulls() throws {
        let body = try wireBody(
            of: UpdateProfileDetailsArguments(
                bio: nil,
                websiteURL: nil,
                profilePicture: nil,
                updateAsAIAccountID: nil
            )
        )
        #expect(body == "[null,null,null,null]")
    }

    // MARK: - accept_new_user_registration (3 args — live signature)

    @Test func ownerRegistrationEncodesNoneAsBareNull() throws {
        let body = try wireBody(
            of: AcceptNewUserRegistrationArguments(
                newPrincipalText: "owner-principal",
                authenticated: true,
                mainAccountText: nil
            )
        )
        #expect(body == #"["owner-principal",true,null]"#)
    }

    @Test func botAttachmentEncodesSomeWithInlinedPayload() throws {
        let body = try wireBody(
            of: AcceptNewUserRegistrationArguments(
                newPrincipalText: "ai-principal-1",
                authenticated: true,
                mainAccountText: "owner-principal"
            )
        )
        #expect(body == #"["ai-principal-1",true,[0,"owner-principal"]]"#)
    }

    // MARK: - Reads (procedures)

    @Test func followersCursorEncodesAsBareNullThenSome() throws {
        let firstPage = try wireBody(
            of: GetFollowersArguments(oauthSubject: "owner-principal", limit: 20, cursor: nil)
        )
        #expect(firstPage == "[\"owner-principal\",20,null]")

        let nextPage = try wireBody(
            of: GetFollowersArguments(oauthSubject: "owner-principal", limit: 20, cursor: "cursor-1")
        )
        #expect(nextPage == "[\"owner-principal\",20,[0,\"cursor-1\"]]")
    }

    @Test func batchProfileReadNestsTheSubjectList() throws {
        let body = try wireBody(
            of: GetUsersProfileDetailsArguments(oauthSubjects: ["subject-a", "subject-b"])
        )
        #expect(body == "[[\"subject-a\",\"subject-b\"]]")
    }

    @Test func postsOfUserSendsOffsetThenLimit() throws {
        let body = try wireBody(
            of: GetPostsOfUserByPrincipalArguments(
                creatorOauthSubject: "owner-principal",
                offset: 40,
                limit: 20
            )
        )
        #expect(body == "[\"owner-principal\",40,20]")
    }

    @Test func noArgumentCallsEncodeAsEmptyArray() throws {
        #expect(try wireBody(of: SpacetimeNoArguments()) == "[]")
    }
}
