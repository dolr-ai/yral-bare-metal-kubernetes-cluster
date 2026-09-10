import Foundation

/// Mutations (reducer/procedure calls) — extension of
/// `SpacetimeDBRemoteDataSource` split from the main file (type-body
/// lint). Every write goes through the same transport helper.
extension SpacetimeDBRemoteDataSource {

    /// `follow_user` — JWT required.
    public func followUser(followeeSubject: String) async throws {
        try await callReducer(name: "follow_user", arguments: FollowUserArguments(followeeSubject: followeeSubject))
    }

    /// `unfollow_user` — JWT required.
    public func unfollowUser(followeeSubject: String) async throws {
        try await callReducer(name: "unfollow_user", arguments: UnfollowUserArguments(followeeSubject: followeeSubject))
    }

    /// `register_new_user` — JWT required.
    public func registerNewUser() async throws {
        try await callReducer(name: "register_new_user", arguments: SpacetimeNoArguments())
    }

    /// `update_profile_details` — `update_as_ai_account_id` is REQUIRED
    /// when editing an AI account's profile — without it the details land
    /// on the OWNER's profile (see the reducer's docs in user_info.rs).
    public func updateProfileDetails(
        bio: String?,
        websiteURL: String?,
        profilePictureURL: String?,
        updateAsAIAccountID: String?
    ) async throws {
        let profilePicture = profilePictureURL.map {
            SpacetimeWireProfilePictureData(
                url: $0,
                nsfwInfo: SpacetimeWireNSFWInfo(
                    isNSFW: false,
                    nsfwEC: "",
                    nsfwGore: "",
                    csamDetected: false
                )
            )
        }
        try await callReducer(
            name: "update_profile_details",
            arguments: UpdateProfileDetailsArguments(
                bio: bio,
                websiteURL: websiteURL,
                profilePicture: profilePicture,
                updateAsAIAccountID: updateAsAIAccountID
            )
        )
    }

    /// `accept_new_user_registration` — used for both owner registration
    /// and AI account attachment.
    public func acceptNewUserRegistration(
        newSubjectText: String,
        authenticated: Bool,
        mainAccountText: String?
    ) async throws {
        try await callReducer(
            name: "accept_new_user_registration",
            arguments: AcceptNewUserRegistrationArguments(
                newSubjectText: newSubjectText,
                authenticated: authenticated,
                mainAccountText: mainAccountText
            )
        )
    }

    /// Transactional cascade + the bots' backend soft-deletes; the result
    /// carries the cascade's verbatim error and each backend outcome.
    public func deleteUser(subjectToDelete: String, idToken: String) async throws -> DeleteUserResult {
        let responseBody = try await callProcedure(
            name: "delete_user",
            arguments: DeleteUserArguments(
                subjectToDelete: subjectToDelete,
                idToken: idToken
            ),
            requiresToken: true
        )
        let responseArray = try SpacetimePositionalDecoder.parseArray(responseBody)
        return try DeleteUserResult.fromPositionalResponse(responseArray)
    }

    /// `register_notification_token` — JWT required (Phase 2 push wiring).
    public func registerNotificationToken(_ token: String) async throws {
        try await callReducer(
            name: "register_notification_token",
            arguments: RegisterNotificationTokenArguments(token: token)
        )
    }

    /// `unregister_notification_token` — JWT required.
    public func unregisterNotificationToken(_ token: String) async throws {
        try await callReducer(
            name: "unregister_notification_token",
            arguments: UnregisterNotificationTokenArguments(token: token)
        )
    }

    /// `update_user_last_access_time` — JWT required.
    public func updateUserLastAccessTime() async throws {
        try await callReducer(name: "update_user_last_access_time", arguments: SpacetimeNoArguments())
    }
}
