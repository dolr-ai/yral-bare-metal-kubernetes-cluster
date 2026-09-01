import Foundation

/// REST client for the SpacetimeDB database — the app's primary data plane.
///
/// Port of Kotlin `SpacetimeDBRemoteDataSource`. Every call is
/// `POST https://maincloud.spacetimedb.com/v1/database/{db}/call/{procedure}`
/// with a JSON array of positional arguments (no field names on the wire).
/// Server function names are used VERBATIM (CaseConversionPolicy::None).
///
/// Auth: `Authorization: Bearer <yral-auth id_token>` — reads work without a
/// token (sender = Identity::ZERO); writes throw when no token is present.
///
/// Arguments are TYPED wire models (SpacetimeWireModels.swift — Swift's
/// answer to Serde): each struct mirrors one live procedure/reducer
/// signature from the generated bindings, encoded by JSONEncoder as
/// positional arrays. Escaping is framework-handled.
public struct SpacetimeDBRemoteDataSource: Sendable {

    // TODO(endpoint-input-sanitization): audit ALL endpoints we call —
    // here (SpacetimeDB procedures/reducers), AIInfluencerDataSource
    // (agent.rishi.yral.com persona/metadata/influencer/avatar), and
    // AuthDataSource (yral-auth) — and confirm each one SANITIZES the
    // inputs it accepts (length caps, character-set/format validation,
    // injection-safe persistence). The iOS side pre-sanitizes some
    // (the username field), but client-side checks are UX only — the
    // server must enforce. Known probe points: the bio (LLM free text),
    // the username (regex-validity + uniqueness), instructions text
    // (400+ chars of free text). Verify each backend rejects malformed
    // input with a specific error BEFORE persisting, and add
    // server-side tests for the rejections.

    /// Default URLSession — no custom client (30s default timeout matches
    /// the Kotlin policy; per-endpoint overrides are inline
    /// `request.timeoutInterval` assignments).
    private let session: URLSession

    /// Token provider — nil → anonymous (reads only).
    private let idTokenProvider: @Sendable () -> String?

    public init(idTokenProvider: @escaping @Sendable () -> String?) {
        self.session = .shared
        self.idTokenProvider = idTokenProvider
    }

    // MARK: - Reads (procedures)

    /// `get_post_by_id` → `Option<PostDetails>`.
    public func getPostByID(_ postID: String) async throws -> SpacetimePostDetails? {
        try await callReturningOptionPost("get_post_by_id", arguments: GetPostByIDArguments(postID: postID))
    }

    /// `get_individual_post_details_by_id` → `Option<PostDetails>`.
    public func getIndividualPostDetailsByID(_ postID: String) async throws -> SpacetimePostDetails? {
        try await callReturningOptionPost(
            "get_individual_post_details_by_id",
            arguments: GetIndividualPostDetailsByIDArguments(postID: postID)
        )
    }

    /// `get_posts_of_user_by_principal` → `PostListOffset` (offset-paginated).
    public func getPostsOfUser(
        creatorOauthSubject: String,
        offset: UInt64,
        limit: UInt64
    ) async throws -> SpacetimePostListOffset {
        try await callReturningPostList(
            "get_posts_of_user_by_principal",
            arguments: GetPostsOfUserByPrincipalArguments(
                creatorOauthSubject: creatorOauthSubject,
                offset: offset,
                limit: limit
            )
        )
    }

    /// `get_draft_posts_of_user_by_principal` → `PostListOffset`.
    public func getDraftPostsOfUser(
        creatorOauthSubject: String,
        offset: UInt64,
        limit: UInt64
    ) async throws -> SpacetimePostListOffset {
        try await callReturningPostList(
            "get_draft_posts_of_user_by_principal",
            arguments: GetDraftPostsOfUserByPrincipalArguments(
                creatorOauthSubject: creatorOauthSubject,
                offset: offset,
                limit: limit
            )
        )
    }

    /// `get_user_profile_details` → `Option<UserProfileDetails>`.
    public func getUserProfileDetails(oauthSubject: String) async throws -> SpacetimeUserProfile? {
        try await callReturningOptionProfile(
            "get_user_profile_details",
            arguments: GetUserProfileDetailsArguments(oauthSubject: oauthSubject)
        )
    }

    /// `get_users_profile_details` — batch read. NOTE: the subject list is ONE
    /// positional arg (a nested JSON array), not spread args.
    public func getUsersProfileDetails(oauthSubjects: [String]) async throws -> [SpacetimeUserProfile] {
        let responseBody = try await callProcedure(
            name: "get_users_profile_details",
            arguments: GetUsersProfileDetailsArguments(oauthSubjects: oauthSubjects),
            requiresToken: false
        )
        let bodyArray = try SpacetimePositionalDecoder.parseArray(responseBody)
        var profiles: [SpacetimeUserProfile] = []
        for profileArray in bodyArray {
            guard let profileArray = profileArray as? [Any] else {
                throw SpacetimeDecodingError.typeMismatch(expected: "user profile item", index: 0)
            }
            profiles.append(try SpacetimeUserProfile.fromPositionalArray(profileArray))
        }
        return profiles
    }

    /// `get_followers` → `FollowersPage` (cursor-paginated; pass nil cursor
    /// for the first page).
    public func getFollowers(
        oauthSubject: String,
        limit: UInt64,
        cursor: String?
    ) async throws -> SpacetimeFollowersPage {
        let responseBody = try await callProcedure(
            name: "get_followers",
            arguments: GetFollowersArguments(
                oauthSubject: oauthSubject,
                limit: limit,
                cursor: cursor
            ),
            requiresToken: false
        )
        return try SpacetimeFollowersPage.fromPositionalArray(
            SpacetimePositionalDecoder.parseArray(responseBody)
        )
    }

    /// `get_following` → `FollowingPage` (cursor-paginated).
    public func getFollowing(
        oauthSubject: String,
        limit: UInt64,
        cursor: String?
    ) async throws -> SpacetimeFollowingPage {
        let responseBody = try await callProcedure(
            name: "get_following",
            arguments: GetFollowingArguments(
                oauthSubject: oauthSubject,
                limit: limit,
                cursor: cursor
            ),
            requiresToken: false
        )
        return try SpacetimeFollowingPage.fromPositionalArray(
            SpacetimePositionalDecoder.parseArray(responseBody)
        )
    }

    // MARK: - Writes (reducers)

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

    /// `update_profile_details` — typed args (UpdateProfileDetailsArguments —
    /// mirrors the LIVE reducer signature; see SpacetimeWireModels).
    /// `update_as_ai_account_id` is REQUIRED when editing an AI account's
    /// profile — without it the details land on the OWNER's profile (see
    /// the reducer's doc comment in src/user_info.rs).
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

    /// `accept_new_user_registration` — typed args
    /// (AcceptNewUserRegistrationArguments — mirrors the LIVE reducer
    /// signature; see SpacetimeWireModels). Used for both owner
    /// registration and AI account attachment.
    public func acceptNewUserRegistration(
        newPrincipalText: String,
        authenticated: Bool,
        mainAccountText: String?
    ) async throws {
        try await callReducer(
            name: "accept_new_user_registration",
            arguments: AcceptNewUserRegistrationArguments(
                newPrincipalText: newPrincipalText,
                authenticated: authenticated,
                mainAccountText: mainAccountText
            )
        )
    }

    /// `delete_user_info` — JWT required.
    public func deleteUserInfo(principalToDeleteText: String) async throws {
        try await callReducer(
            name: "delete_user_info",
            arguments: DeleteUserInfoArguments(principalToDeleteText: principalToDeleteText)
        )
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

    // MARK: - Transport

    /// POSTs `{base}/v1/database/{db}/call/{name}` and returns the raw body.
    /// The response body IS the return value (no wrapper array).
    private func callProcedure(
        name: String,
        arguments: some Encodable,
        requiresToken: Bool
    ) async throws -> String {
        let token = idTokenProvider()
        if requiresToken && token == nil {
            throw NetworkError.notAuthenticated(
                description: "Not authenticated — no ID token available for \(name)"
            )
        }
        let url = URL(string: "https://\(AppConfiguration.spacetimeDBBaseURL)")!
            .appending(path: "v1/database/\(AppConfiguration.spacetimeDBDatabaseName)/call/\(name)")
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if let token {
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        // Typed wire models + JSONEncoder — escaping handled by the
        // framework (the hand-rolled string encoder broke on any bio
        // containing a quote/newline: LLM output does both).
        // `.withoutEscapingSlashes` → canonical compact JSON (no `\/`).
        do {
            let wireEncoder = JSONEncoder()
            wireEncoder.outputFormatting = .withoutEscapingSlashes
            request.httpBody = try wireEncoder.encode(arguments)
        } catch {
            throw NetworkError.transport(underlying: "Failed to encode \(name) arguments: \(error)")
        }
        let (data, response): (Data, URLResponse)
        do {
            (data, response) = try await session.data(for: request)
        } catch {
            throw NetworkError.transport(underlying: "\(error)")
        }
        guard let httpResponse = response as? HTTPURLResponse else {
            throw NetworkError.transport(underlying: "Non-HTTP response")
        }
        guard (200..<300).contains(httpResponse.statusCode) else {
            throw NetworkError.http(
                statusCode: httpResponse.statusCode,
                body: String(data: data, encoding: .utf8)
            )
        }
        return String(data: data, encoding: .utf8) ?? ""
    }

    /// Calls a reducer (write) — JWT required; the unit-return body is discarded.
    private func callReducer(name: String, arguments: some Encodable) async throws {
        _ = try await callProcedure(name: name, arguments: arguments, requiresToken: true)
    }

    // MARK: - Response parse helpers (mirroring the Kotlin parse* functions)

    /// The response BODY is the `Option<PostDetails>` sum variant itself:
    /// `[0, [postArray]]` for Some, `[1, []]` for None.
    private func callReturningOptionPost(
        _ name: String,
        arguments: some Encodable
    ) async throws -> SpacetimePostDetails? {
        let responseBody = try await callProcedure(name: name, arguments: arguments, requiresToken: false)
        guard let payload = try SpacetimePositionalDecoder.optionPayload(
            SpacetimePositionalDecoder.parseArray(responseBody)
        ) else { return nil }
        return try SpacetimePostDetails.fromPositionalArray(payload)
    }

    /// The response BODY is the `Option<UserProfileDetails>` sum variant.
    private func callReturningOptionProfile(
        _ name: String,
        arguments: some Encodable
    ) async throws -> SpacetimeUserProfile? {
        let responseBody = try await callProcedure(name: name, arguments: arguments, requiresToken: false)
        guard let payload = try SpacetimePositionalDecoder.optionPayload(
            SpacetimePositionalDecoder.parseArray(responseBody)
        ) else { return nil }
        return try SpacetimeUserProfile.fromPositionalArray(payload)
    }

    /// The response BODY is the struct directly: `[[post, post, …]]`.
    private func callReturningPostList(
        _ name: String,
        arguments: some Encodable
    ) async throws -> SpacetimePostListOffset {
        let responseBody = try await callProcedure(name: name, arguments: arguments, requiresToken: false)
        return try SpacetimePostListOffset.fromPositionalArray(
            SpacetimePositionalDecoder.parseArray(responseBody)
        )
    }
}
