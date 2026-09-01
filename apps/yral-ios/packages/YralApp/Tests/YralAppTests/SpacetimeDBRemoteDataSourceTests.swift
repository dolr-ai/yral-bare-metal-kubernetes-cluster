import Testing
@testable import YralApp

/// Wire-format pins for the reducer calls our creation pipeline makes.
///
/// Each test asserts the EXACT request body we POST to
/// `/v1/database/{db}/call/{reducer}` against the LIVE reducer signature
/// (generated bindings in apps/yral-database-spacetime). These pins catch
/// wire drift — the exact failure mode of the 3-arg
/// `update_profile_details` call against the live 4-arg reducer
/// ("invalid arguments for reducer", caught at arg validation BEFORE the
/// reducer body runs, so no data was ever at risk).
///
/// Wire rules (SpacetimeDB serde bridge — verified against spacetimedb's
/// de/serde.rs): plain `null` = `None`; `Some(v)` = `[0, payload]` (ARG
/// form, payload inlined); structs = positional arrays.
struct SpacetimeDBRemoteDataSourceTests {

    // MARK: - update_profile_details (4 args — live signature)

    @Test func updateProfileDetailsSendsFourArgumentsMatchingLiveReducer() {
        let arguments = updateProfileDetailsArguments(
            bio: "A witty travel photographer",
            websiteURL: nil,
            profilePictureURL: "https://images.yral.com/avatar.png",
            updateAsAIAccountID: "ai-principal-1"
        )
        let body = encodeSpacetimeArguments(arguments)
        // Exact wire body for the live 4-arg reducer:
        //   [bio, null, [0, [url, [false,"","",false]]], [0, ai-principal]]
        let expected = "[\"A witty travel photographer\",null,"
            + "[0,[\"https://images.yral.com/avatar.png\",[false,\"\",\"\",false]]],"
            + "[0,\"ai-principal-1\"]]"
        #expect(body == expected)
    }

    @Test func updateProfileDetailsAIClaimEncodesSomeForm() {
        let arguments = updateProfileDetailsArguments(
            bio: nil,
            websiteURL: nil,
            profilePictureURL: nil,
            updateAsAIAccountID: "auth0|ai-account-1"
        )
        let body = encodeSpacetimeArguments(arguments)
        // The 4th arg MUST be [0, id] (Some) — never a bare string: the
        // live reducer takes Option<String>.
        let expected = #"[null,null,null,[0,"auth0|ai-account-1"]]"#
        #expect(body == expected)
    }

    @Test func updateProfilePictureArgIsStructNotBareString() {
        let arguments = updateProfileDetailsArguments(
            bio: nil,
            websiteURL: nil,
            profilePictureURL: "https://images.yral.com/pic.png",
            updateAsAIAccountID: nil
        )
        let body = encodeSpacetimeArguments(arguments)
        // The 3rd arg MUST be [0, [url, NSFWInfo]] (Some of the struct) —
        // the old wire shape sent a BARE STRING and failed arg validation.
        #expect(body.contains(#"[0,["https://images.yral.com/pic.png",[false,"","",false]]]"#))
    }

    @Test func updateProfileDetailsAllNullsEncodeAsBareNulls() {
        let arguments = updateProfileDetailsArguments(
            bio: nil,
            websiteURL: nil,
            profilePictureURL: nil,
            updateAsAIAccountID: nil
        )
        let body = encodeSpacetimeArguments(arguments)
        #expect(body == "[null,null,null,null]")
    }

    // MARK: - accept_new_user_registration (3 args — live signature)

    @Test func ownerRegistrationEncodesNoneAsEmptyUnitVariant() {
        let arguments = acceptNewUserRegistrationArguments(
            newPrincipalText: "owner-principal",
            authenticated: true,
            mainAccountText: nil
        )
        let body = encodeSpacetimeArguments(arguments)
        #expect(body == #"["owner-principal",true,[1,[]]]"#)
    }

    @Test func botAttachmentEncodesSomeWithInlinedPayload() {
        let arguments = acceptNewUserRegistrationArguments(
            newPrincipalText: "ai-principal-1",
            authenticated: true,
            mainAccountText: "owner-principal"
        )
        let body = encodeSpacetimeArguments(arguments)
        #expect(body == #"["ai-principal-1",true,[0,"owner-principal"]]"#)
    }
}
