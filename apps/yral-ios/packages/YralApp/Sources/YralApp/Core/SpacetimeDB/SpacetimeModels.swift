import Foundation

// MARK: - PostDetails

/// `PostDetailsForFrontend` (server: posts.rs) — positional field order.
public struct SpacetimePostDetails: Equatable, Sendable {
    public let id: String
    public let description: String
    public let hashtags: [String]
    public let videoUID: String
    /// OAuth subject of the creator (the app's user identifier).
    public let creatorOauthSubject: String
    /// SpacetimeDB Timestamp — micros since epoch.
    public let createdAtMicros: Int64
    public let totalViewCount: UInt64
    /// Legacy like count — always 0 (likes feature dropped).
    public let likeCount: UInt64
    /// Legacy like flag — always false.
    public let likedByMe: Bool
    public let status: SpacetimePostStatus

    /// Decodes the positional array. The raw Identity field (index 4) is
    /// skipped — the app identifies users by `creatorOauthSubject`. The
    /// Timestamp (index 6) arrives as a `[micros]` wrapper array.
    static func fromPositionalArray(_ array: [Any]) throws -> SpacetimePostDetails {
        let timestampWrapper = try SpacetimePositionalDecoder.array(array, at: 6)
        return SpacetimePostDetails(
            id: try SpacetimePositionalDecoder.string(array, at: 0),
            description: try SpacetimePositionalDecoder.string(array, at: 1),
            hashtags: try SpacetimePositionalDecoder.stringVector(array, at: 2),
            videoUID: try SpacetimePositionalDecoder.string(array, at: 3),
            // index 4 = creator Identity (["0x…"]) — unused, skipped.
            creatorOauthSubject: try SpacetimePositionalDecoder.string(array, at: 5),
            createdAtMicros: try SpacetimePositionalDecoder.long(timestampWrapper, at: 0),
            totalViewCount: try SpacetimePositionalDecoder.unsigned64(array, at: 7),
            likeCount: try SpacetimePositionalDecoder.unsigned64(array, at: 8),
            likedByMe: try SpacetimePositionalDecoder.boolean(array, at: 9),
            status: try parsePostStatus(array, at: 10)
        )
    }
}

/// `PostStatus` — unit variants; tag = declaration index. Swift throws on
/// unknown tags (the Kotlin version indexes unchecked and would crash).
public enum SpacetimePostStatus: Equatable, Sendable {
    case uploaded
    case transcoding
    case checkingExplicitness
    case bannedForExplicitness
    case readyToView
    case bannedDueToUserReporting
    case deleted
    case draft

    static func fromTag(_ tag: Int) throws -> SpacetimePostStatus {
        guard let status = allCases[safe: tag] else {
            throw SpacetimeDecodingError.unknownVariantTag(type: "PostStatus", tag: tag)
        }
        return status
    }

    /// Declaration order (must match the server enum exactly).
    static let allCases: [SpacetimePostStatus] = [
        .uploaded,
        .transcoding,
        .checkingExplicitness,
        .bannedForExplicitness,
        .readyToView,
        .bannedDueToUserReporting,
        .deleted,
        .draft
    ]
}

/// Decodes a `PostStatus` sum variant `[tag, []]` at a field index.
func parsePostStatus(_ array: [Any], at index: Int) throws -> SpacetimePostStatus {
    let variant = try SpacetimePositionalDecoder.sumVariant(
        try SpacetimePositionalDecoder.array(array, at: index)
    )
    return try SpacetimePostStatus.fromTag(variant.tag)
}

// MARK: - User profile

/// `UserProfileDetails` (server: user_info.rs) — positional field order.
public struct SpacetimeUserProfile: Equatable, Sendable {
    public let oauthSubject: String
    public let profilePicture: SpacetimeProfilePictureData?
    public let bio: String
    public let websiteURL: String
    public let followersCount: UInt64
    public let followingCount: UInt64
    /// `None` when viewing your own profile.
    public let callerFollowsUser: Bool?
    /// `None` when viewing your own profile.
    public let userFollowsCaller: Bool?
    public let subscriptionPlan: SpacetimeSubscriptionPlan
    public let isAiInfluencer: Bool
    public let accountType: SpacetimeUserAccountType

    /// Decodes the 11-field positional array.
    static func fromPositionalArray(_ array: [Any]) throws -> SpacetimeUserProfile {
        SpacetimeUserProfile(
            oauthSubject: try SpacetimePositionalDecoder.string(array, at: 0),
            profilePicture: try SpacetimePositionalDecoder.optionPayload(
                try SpacetimePositionalDecoder.array(array, at: 1)
            ).map { try SpacetimeProfilePictureData.fromPositionalArray($0) },
            bio: try SpacetimePositionalDecoder.string(array, at: 2),
            websiteURL: try SpacetimePositionalDecoder.string(array, at: 3),
            followersCount: try SpacetimePositionalDecoder.unsigned64(array, at: 4),
            followingCount: try SpacetimePositionalDecoder.unsigned64(array, at: 5),
            callerFollowsUser: try SpacetimePositionalDecoder.optionBool(array, at: 6),
            userFollowsCaller: try SpacetimePositionalDecoder.optionBool(array, at: 7),
            subscriptionPlan: try parseSubscriptionPlan(array, at: 8),
            isAiInfluencer: try SpacetimePositionalDecoder.boolean(array, at: 9),
            accountType: try parseUserAccountType(array, at: 10)
        )
    }
}

/// `ProfilePictureData` — `[url, nsfwInfo]`.
public struct SpacetimeProfilePictureData: Equatable, Sendable {
    public let url: String
    public let nsfwInfo: SpacetimeNsfwInfo

    static func fromPositionalArray(_ array: [Any]) throws -> SpacetimeProfilePictureData {
        SpacetimeProfilePictureData(
            url: try SpacetimePositionalDecoder.string(array, at: 0),
            nsfwInfo: try SpacetimeNsfwInfo.fromPositionalArray(
                try SpacetimePositionalDecoder.array(array, at: 1)
            )
        )
    }
}

/// `NSFWInfo` — `[isNsfw, nsfwEc, nsfwGore, csamDetected]`.
public struct SpacetimeNsfwInfo: Equatable, Sendable {
    public let isNsfw: Bool
    public let nsfwEc: String
    public let nsfwGore: String
    public let csamDetected: Bool

    static func fromPositionalArray(_ array: [Any]) throws -> SpacetimeNsfwInfo {
        SpacetimeNsfwInfo(
            isNsfw: try SpacetimePositionalDecoder.boolean(array, at: 0),
            nsfwEc: try SpacetimePositionalDecoder.string(array, at: 1),
            nsfwGore: try SpacetimePositionalDecoder.string(array, at: 2),
            csamDetected: try SpacetimePositionalDecoder.boolean(array, at: 3)
        )
    }
}

/// `SubscriptionPlan` — tag 0 `Free` (unit), tag 1
/// `Pro(freeVideoCreditsLeft, totalVideoCreditsAlloted)` (both u32).
public enum SpacetimeSubscriptionPlan: Equatable, Sendable {
    case free
    case pro(freeVideoCreditsLeft: UInt32, totalVideoCreditsAlloted: UInt32)
}

func parseSubscriptionPlan(_ array: [Any], at index: Int) throws -> SpacetimeSubscriptionPlan {
    let variant = try SpacetimePositionalDecoder.sumVariant(
        try SpacetimePositionalDecoder.array(array, at: index)
    )
    switch variant.tag {
    case 0:
        return .free
    case 1:
        return .pro(
            freeVideoCreditsLeft: try SpacetimePositionalDecoder.unsigned32(variant.payload, at: 0),
            totalVideoCreditsAlloted: try SpacetimePositionalDecoder.unsigned32(variant.payload, at: 1)
        )
    default:
        throw SpacetimeDecodingError.unknownVariantTag(type: "SubscriptionPlan", tag: variant.tag)
    }
}

/// `UserAccountType` — tag 0 `MainAccount(bots: Vec<String>)`, tag 1
/// `BotAccount(owner: String)`.
public enum SpacetimeUserAccountType: Equatable, Sendable {
    case mainAccount(bots: [String])
    case botAccount(owner: String)
}

func parseUserAccountType(_ array: [Any], at index: Int) throws -> SpacetimeUserAccountType {
    let variant = try SpacetimePositionalDecoder.sumVariant(
        try SpacetimePositionalDecoder.array(array, at: index)
    )
    switch variant.tag {
    case 0:
        let bots = try SpacetimePositionalDecoder.array(variant.payload, at: 0)
        return .mainAccount(bots: bots.compactMap { $0 as? String })
    case 1:
        return .botAccount(owner: try SpacetimePositionalDecoder.string(variant.payload, at: 0))
    default:
        throw SpacetimeDecodingError.unknownVariantTag(type: "UserAccountType", tag: variant.tag)
    }
}

// MARK: - Follower/following pages

/// `FollowersPage` — cursor-paginated (cursor = last item's oauth subject).
public struct SpacetimeFollowersPage: Equatable, Sendable {
    public let followers: [SpacetimeFollowerItem]
    public let totalCount: UInt64
    /// nil when there are no further pages.
    public let nextCursor: String?

    static func fromPositionalArray(_ array: [Any]) throws -> SpacetimeFollowersPage {
        let followerArrays = try SpacetimePositionalDecoder.array(array, at: 0)
        var followers: [SpacetimeFollowerItem] = []
        for followerArray in followerArrays {
            guard let followerArray = followerArray as? [Any] else {
                throw SpacetimeDecodingError.typeMismatch(expected: "follower item", index: 0)
            }
            followers.append(try SpacetimeFollowerItem.fromPositionalArray(followerArray))
        }
        return SpacetimeFollowersPage(
            followers: followers,
            totalCount: try SpacetimePositionalDecoder.unsigned64(array, at: 1),
            nextCursor: try SpacetimePositionalDecoder.optionString(array, at: 2)
        )
    }
}

/// `FollowersPage` item — `[oauthSubject, callerFollows, profilePictureUrl]`.
public struct SpacetimeFollowerItem: Equatable, Sendable {
    public let oauthSubject: String
    public let callerFollows: Bool
    public let profilePictureURL: String

    static func fromPositionalArray(_ array: [Any]) throws -> SpacetimeFollowerItem {
        SpacetimeFollowerItem(
            oauthSubject: try SpacetimePositionalDecoder.string(array, at: 0),
            callerFollows: try SpacetimePositionalDecoder.boolean(array, at: 1),
            profilePictureURL: try SpacetimePositionalDecoder.string(array, at: 2)
        )
    }
}

/// `FollowingPage` — same shape as `FollowersPage`.
public struct SpacetimeFollowingPage: Equatable, Sendable {
    public let following: [SpacetimeFollowingItem]
    public let totalCount: UInt64
    public let nextCursor: String?

    static func fromPositionalArray(_ array: [Any]) throws -> SpacetimeFollowingPage {
        let followingArrays = try SpacetimePositionalDecoder.array(array, at: 0)
        var following: [SpacetimeFollowingItem] = []
        for followingArray in followingArrays {
            guard let followingArray = followingArray as? [Any] else {
                throw SpacetimeDecodingError.typeMismatch(expected: "following item", index: 0)
            }
            following.append(try SpacetimeFollowingItem.fromPositionalArray(followingArray))
        }
        return SpacetimeFollowingPage(
            following: following,
            totalCount: try SpacetimePositionalDecoder.unsigned64(array, at: 1),
            nextCursor: try SpacetimePositionalDecoder.optionString(array, at: 2)
        )
    }
}

/// `FollowingPage` item — `[oauthSubject, callerFollows, profilePictureUrl]`.
public struct SpacetimeFollowingItem: Equatable, Sendable {
    public let oauthSubject: String
    public let callerFollows: Bool
    public let profilePictureURL: String

    static func fromPositionalArray(_ array: [Any]) throws -> SpacetimeFollowingItem {
        SpacetimeFollowingItem(
            oauthSubject: try SpacetimePositionalDecoder.string(array, at: 0),
            callerFollows: try SpacetimePositionalDecoder.boolean(array, at: 1),
            profilePictureURL: try SpacetimePositionalDecoder.string(array, at: 2)
        )
    }
}

// MARK: - Post list (offset-paginated)

/// `PostListOffset` — single-field struct; the response is `[[post, post, …]]`
/// (the posts array IS the whole struct — parse `array[0]` then iterate).
public struct SpacetimePostListOffset: Equatable, Sendable {
    public let posts: [SpacetimePostDetails]

    static func fromPositionalArray(_ array: [Any]) throws -> SpacetimePostListOffset {
        let postArrays = try SpacetimePositionalDecoder.array(array, at: 0)
        var posts: [SpacetimePostDetails] = []
        for postArray in postArrays {
            guard let postArray = postArray as? [Any] else {
                throw SpacetimeDecodingError.typeMismatch(expected: "post item", index: 0)
            }
            posts.append(try SpacetimePostDetails.fromPositionalArray(postArray))
        }
        return SpacetimePostListOffset(posts: posts)
    }
}

// MARK: - Safe subscript helper

extension Array {
    /// Bounds-checked access (nil instead of crash).
    subscript(safe index: Int) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}
