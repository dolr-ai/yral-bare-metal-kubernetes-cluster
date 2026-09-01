import Foundation

/// Typed wire models for the SpacetimeDB REST API — Swift's answer to
/// Serde: strong types in code, Codable synthesizes the wire format, so
/// a schema mismatch is a COMPILE error, not a runtime
/// "invalid arguments for reducer". Replaces the hand-rolled
/// string-interpolating encoder (which produced invalid JSON for any
/// bio containing a quote/newline — LLM output does both — the cause
/// of the second "invalid arguments" failure).
///
/// Wire format (SpacetimeDB's REST `/call/` endpoint — verified against
/// the generated bindings, the source of truth for the live schema):
///   - the whole request body is a JSON ARRAY of positional arguments;
///   - `None` encodes as bare `null`; `Some(v)` encodes as `[0, payload]`
///     (ARG form — the payload is INLINED, unlike the RESPONSE form
///     `[0, [v]]`);
///   - structs encode as positional arrays `[field, field, …]`
///     (SpacetimeDB's positional JSON, NOT keyed objects — hence the
///     unkeyed containers below).
///
/// Each struct below mirrors ONE live procedure/reducer signature in
/// apps/yral-database-spacetime/bindings/src/generated/.

/// `Some(v)` → `[0, payload]`; `None` → `null` (ARG form).
struct SpacetimeOption<Value: Encodable>: Encodable {
    let value: Value?

    init(_ value: Value?) {
        self.value = value
    }

    func encode(to encoder: Encoder) throws {
        switch value {
        case let .some(inner):
            var container = encoder.unkeyedContainer()
            try container.encode(0)
            try container.encode(inner)
        case .none:
            var single = encoder.singleValueContainer()
            try single.encodeNil()
        }
    }
}

/// `ProfilePictureData` — the live struct: `{ url: String,
/// nsfw_info: NSFWInfo }`, positional on the wire. Request-side (the
/// response-side decoders live in SpacetimeModels.swift).
struct SpacetimeWireProfilePictureData: Encodable {
    let url: String
    let nsfwInfo: SpacetimeWireNSFWInfo

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(url)
        try container.encode(nsfwInfo)
    }
}

/// `NSFWInfo` — `{ is_nsfw: bool, nsfw_ec: String, nsfw_gore: String,
/// csam_detected: bool }`, positional on the wire. All-false here:
/// moderation runs server-side via `update_profile_picture_nsfw_info`.
/// Request-side (the response-side decoder is SpacetimeNsfwInfo in
/// SpacetimeModels.swift).
struct SpacetimeWireNSFWInfo: Encodable {
    let isNSFW: Bool
    let nsfwEC: String
    let nsfwGore: String
    let csamDetected: Bool

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(isNSFW)
        try container.encode(nsfwEC)
        try container.encode(nsfwGore)
        try container.encode(csamDetected)
    }
}

/// `update_profile_details(bio: Option<String>,
/// website_url: Option<String>, profile_picture:
/// Option<ProfilePictureData>, update_as_ai_account_id:
/// Option<String>)` — the LIVE reducer signature.
struct UpdateProfileDetailsArguments: Encodable {
    let bio: String?
    let websiteURL: String?
    let profilePicture: SpacetimeWireProfilePictureData?
    let updateAsAIAccountID: String?

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(SpacetimeOption(bio))
        try container.encode(SpacetimeOption(websiteURL))
        try container.encode(SpacetimeOption(profilePicture))
        try container.encode(SpacetimeOption(updateAsAIAccountID))
    }
}

/// `accept_new_user_registration(new_principal_text: String,
/// authenticated: bool, main_account_text: Option<String>)` — the LIVE
/// reducer signature.
struct AcceptNewUserRegistrationArguments: Encodable {
    let newPrincipalText: String
    let authenticated: Bool
    let mainAccountText: String?

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(newPrincipalText)
        try container.encode(authenticated)
        try container.encode(SpacetimeOption(mainAccountText))
    }
}

/// `get_post_by_id(post_id: String)`.
struct GetPostByIDArguments: Encodable {
    let postID: String

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(postID)
    }
}

/// `get_individual_post_details_by_id(post_id: String)`.
struct GetIndividualPostDetailsByIDArguments: Encodable {
    let postID: String

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(postID)
    }
}

/// `get_posts_of_user_by_principal(creator_oauth_subject: String,
/// offset: u64, limit: u64)`.
struct GetPostsOfUserByPrincipalArguments: Encodable {
    let creatorOauthSubject: String
    let offset: UInt64
    let limit: UInt64

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(creatorOauthSubject)
        try container.encode(offset)
        try container.encode(limit)
    }
}

/// `get_draft_posts_of_user_by_principal(creator_oauth_subject: String,
/// offset: u64, limit: u64)`.
struct GetDraftPostsOfUserByPrincipalArguments: Encodable {
    let creatorOauthSubject: String
    let offset: UInt64
    let limit: UInt64

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(creatorOauthSubject)
        try container.encode(offset)
        try container.encode(limit)
    }
}

/// `get_user_profile_details(oauth_subject: String)`.
struct GetUserProfileDetailsArguments: Encodable {
    let oauthSubject: String

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(oauthSubject)
    }
}

/// `get_users_profile_details(oauth_subjects: Vec<String>)` — the list
/// is ONE positional arg (a nested JSON array), not spread args.
struct GetUsersProfileDetailsArguments: Encodable {
    let oauthSubjects: [String]

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(oauthSubjects)
    }
}

/// `get_followers(oauth_subject: String, limit: u64,
/// cursor: Option<String>)`.
struct GetFollowersArguments: Encodable {
    let oauthSubject: String
    let limit: UInt64
    let cursor: String?

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(oauthSubject)
        try container.encode(limit)
        try container.encode(SpacetimeOption(cursor))
    }
}

/// `get_following(oauth_subject: String, limit: u64,
/// cursor: Option<String>)`.
struct GetFollowingArguments: Encodable {
    let oauthSubject: String
    let limit: UInt64
    let cursor: String?

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(oauthSubject)
        try container.encode(limit)
        try container.encode(SpacetimeOption(cursor))
    }
}

/// `follow_user(followee_subject: String)`.
struct FollowUserArguments: Encodable {
    let followeeSubject: String

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(followeeSubject)
    }
}

/// `unfollow_user(followee_subject: String)`.
struct UnfollowUserArguments: Encodable {
    let followeeSubject: String

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(followeeSubject)
    }
}

/// `delete_user_info(principal_to_delete_text: String)`.
struct DeleteUserInfoArguments: Encodable {
    let principalToDeleteText: String

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(principalToDeleteText)
    }
}

/// `register_notification_token(token: String)`.
struct RegisterNotificationTokenArguments: Encodable {
    let token: String

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(token)
    }
}

/// `unregister_notification_token(token: String)`.
struct UnregisterNotificationTokenArguments: Encodable {
    let token: String

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(token)
    }
}

/// No-argument calls (`register_new_user`,
/// `update_user_last_access_time`) — an empty positional-args array.
struct SpacetimeNoArguments: Encodable {
    func encode(to encoder: Encoder) throws {
        _ = encoder.unkeyedContainer()
    }
}
