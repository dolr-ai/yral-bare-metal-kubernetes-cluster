import Testing
import Foundation
@testable import YralApp

/// Wire fixtures for the SpacetimeDB positional decoder — built from the
/// documented wire shapes (the Kotlin module had NO decoder tests; these
/// encode the server contract so regressions surface immediately).
struct SpacetimePositionalDecoderTests {

    // MARK: - Option<T> response decoding

    @Test("Option Some response wraps payload in a single-element array")
    func optionSomeResponse() throws {
        // Response form: Some("abc") = [0, ["abc"]] — payload is a WRAPPER
        // array, unlike the arg form [0, "abc"].
        let payload = try SpacetimePositionalDecoder.optionPayload(
            SpacetimePositionalDecoder.parseArray(#"[0,["abc"]]"#)
        )
        #expect(payload?.first as? String == "abc")
    }

    @Test("Option None response is [1, []]")
    func optionNoneResponse() throws {
        let payload = try SpacetimePositionalDecoder.optionPayload(
            SpacetimePositionalDecoder.parseArray("[1,[]]")
        )
        #expect(payload == nil)
    }

    @Test("Option<String> field decode")
    func optionStringField() throws {
        let array = try SpacetimePositionalDecoder.parseArray(#"[[0,["cursor-123"]]]"#)
        #expect(try SpacetimePositionalDecoder.optionString(array, at: 0) == "cursor-123")
    }

    @Test("Option<Bool> field decode")
    func optionBoolField() throws {
        #expect(try SpacetimePositionalDecoder.optionBool(
            SpacetimePositionalDecoder.parseArray("[[0,[true]]]"), at: 0
        ) == true)
        #expect(try SpacetimePositionalDecoder.optionBool(
            SpacetimePositionalDecoder.parseArray("[[0,[false]]]"), at: 0
        ) == false)
    }

    // MARK: - PostDetails (11 positional fields)

    /// A full Option<PostDetails> Some response with every field populated.
    /// Wire shape: `[0, [<post array>]]` — the Some payload IS the post's
    /// positional array (not an extra wrapping level).
    static let postOptionSome = #"""
    [0, [
      "post-42",
      "A sunset timelapse",
      ["sunset","timelapse"],
      "video-uid-9",
      ["0xdeadbeef"],
      "auth0|user-77",
      [1730000000000000],
      18446744073709551615,
      0,
      false,
      [4, []]
    ]]
    """#

    @Test("PostDetails decodes all fields incl. u64 max, Identity skip, Timestamp unwrap")
    func postDetailsDecode() throws {
        let payload = try SpacetimePositionalDecoder.optionPayload(
            SpacetimePositionalDecoder.parseArray(Self.postOptionSome)
        )
        let post = try SpacetimePostDetails.fromPositionalArray(payload!)

        #expect(post.id == "post-42")
        #expect(post.description == "A sunset timelapse")
        #expect(post.hashtags == ["sunset", "timelapse"])
        #expect(post.videoUID == "video-uid-9")
        #expect(post.creatorOauthSubject == "auth0|user-77")
        #expect(post.createdAtMicros == 1_730_000_000_000_000)
        // u64 max (2^64-1) — must NOT route through Double (2^53 loss).
        #expect(post.totalViewCount == UInt64.max)
        #expect(post.likeCount == 0)
        #expect(post.likedByMe == false)
        #expect(post.status == .readyToView)
    }

    @Test("PostDetails None round-trip")
    func postDetailsNone() throws {
        let payload = try SpacetimePositionalDecoder.optionPayload(
            SpacetimePositionalDecoder.parseArray("[1,[]]")
        )
        #expect(payload == nil)
    }

    @Test("all 8 PostStatus tags decode in declaration order")
    func postStatusTags() throws {
        let expected: [SpacetimePostStatus] = [
            .uploaded, .transcoding, .checkingExplicitness, .bannedForExplicitness,
            .readyToView, .bannedDueToUserReporting, .deleted, .draft
        ]
        for (tag, expectedStatus) in expected.enumerated() {
            let status = try parsePostStatus(
                SpacetimePositionalDecoder.parseArray("[[\(tag),[]]]"), at: 0
            )
            #expect(status == expectedStatus)
        }
    }

    @Test("unknown PostStatus tag throws (typed error, not a crash)")
    func unknownPostStatusTag() {
        #expect(throws: SpacetimeDecodingError.unknownVariantTag(type: "PostStatus", tag: 8)) {
            try SpacetimePostStatus.fromTag(8)
        }
    }

    // MARK: - UserProfile (11 positional fields)

    /// Self-view profile: profilePicture None, follow flags None, Free plan,
    /// MainAccount with bots.
    @Test("UserProfile self-view decodes: None options, Free plan, MainAccount bots")
    func userProfileSelfView() throws {
        let body = #"""
        [
          "auth0|user-77",
          [1, []],
          "bio text",
          "https://example.com",
          12,
          8,
          [1, []],
          [1, []],
          [0, []],
          false,
          [0, [["auth0|bot-1","auth0|bot-2"]]]
        ]
        """#
        let profile = try SpacetimeUserProfile.fromPositionalArray(
            SpacetimePositionalDecoder.parseArray(body)
        )

        #expect(profile.oauthSubject == "auth0|user-77")
        #expect(profile.profilePicture == nil)
        #expect(profile.bio == "bio text")
        #expect(profile.websiteURL == "https://example.com")
        #expect(profile.followersCount == 12)
        #expect(profile.followingCount == 8)
        // Self-view: both follow flags are None.
        #expect(profile.callerFollowsUser == nil)
        #expect(profile.userFollowsCaller == nil)
        #expect(profile.subscriptionPlan == .free)
        #expect(profile.isAiInfluencer == false)
        guard case let .mainAccount(bots) = profile.accountType else {
            Issue.record("expected mainAccount")
            return
        }
        #expect(bots == ["auth0|bot-1", "auth0|bot-2"])
    }

    @Test("UserProfile other-view: Pro plan, BotAccount, follow flags present")
    func userProfileOtherView() throws {
        let body = #"""
        [
          "auth0|bot-1",
          [0, ["https://cdn.example.com/pic.png", [false, "0.1", "0.2", false]]],
          "bot bio",
          "",
          100,
          50,
          [0, [true]],
          [0, [false]],
          [1, [3, 10]],
          true,
          [1, ["auth0|user-77"]]
        ]
        """#
        let profile = try SpacetimeUserProfile.fromPositionalArray(
            SpacetimePositionalDecoder.parseArray(body)
        )

        #expect(profile.callerFollowsUser == true)
        #expect(profile.userFollowsCaller == false)
        guard case let .pro(freeVideoCreditsLeft, totalVideoCreditsAlloted) = profile.subscriptionPlan else {
            Issue.record("expected pro")
            return
        }
        #expect(freeVideoCreditsLeft == 3)
        #expect(totalVideoCreditsAlloted == 10)
        guard case let .botAccount(owner) = profile.accountType else {
            Issue.record("expected botAccount")
            return
        }
        #expect(owner == "auth0|user-77")
        guard let picture = profile.profilePicture else {
            Issue.record("expected profile picture")
            return
        }
        #expect(picture.url == "https://cdn.example.com/pic.png")
        #expect(picture.nsfwInfo.csamDetected == false)
    }

    // MARK: - Followers/Following pages (cursor pagination)

    @Test("FollowersPage decodes items, total, and Some cursor")
    func followersPage() throws {
        let body = #"""
        [[
          ["auth0|f1", true, "https://cdn.example.com/f1.png"],
          ["auth0|f2", false, "https://cdn.example.com/f2.png"]
        ], 500, [0, ["auth0|f2"]]]
        """#
        let page = try SpacetimeFollowersPage.fromPositionalArray(
            SpacetimePositionalDecoder.parseArray(body)
        )
        #expect(page.followers.count == 2)
        #expect(page.followers[0].oauthSubject == "auth0|f1")
        #expect(page.followers[0].callerFollows == true)
        #expect(page.totalCount == 500)
        #expect(page.nextCursor == "auth0|f2")
    }

    @Test("FollowersPage None cursor means no more pages")
    func followersPageEnd() throws {
        let page = try SpacetimeFollowersPage.fromPositionalArray(
            SpacetimePositionalDecoder.parseArray("[[],0,[1,[]]]")
        )
        #expect(page.followers.isEmpty)
        #expect(page.nextCursor == nil)
    }

    // MARK: - PostListOffset (extra nesting level)

    @Test("PostListOffset response has one extra array nesting: [[post, ...]]")
    func postListOffset() throws {
        let body = #"""
        [[
          ["p1","d1",[],"v1",["0xa1"],"auth0|u1",[1730000000000000],5,0,false,[4,[]]],
          ["p2","d2",[],"v2",["0xa2"],"auth0|u2",[1730000000000001],6,0,false,[7,[]]]
        ]]
        """#
        let list = try SpacetimePostListOffset.fromPositionalArray(
            SpacetimePositionalDecoder.parseArray(body)
        )
        #expect(list.posts.count == 2)
        #expect(list.posts[0].id == "p1")
        #expect(list.posts[1].status == .draft)
    }

    // MARK: - Error paths

    @Test("non-array response body throws responseBodyNotArray")
    func nonArrayBody() {
        #expect(throws: SpacetimeDecodingError.self) {
            _ = try SpacetimePositionalDecoder.parseArray(#"{"error":"x"}"#)
        }
    }

    @Test("index out of bounds throws typed error")
    func indexOutOfBounds() throws {
        let array = try SpacetimePositionalDecoder.parseArray(#"[["only-one"]]"#)
        #expect(throws: SpacetimeDecodingError.indexOutOfBounds(index: 1, count: 1)) {
            _ = try SpacetimePositionalDecoder.array(array, at: 1)
        }
    }

    @Test("malformed sum variant throws")
    func malformedSumVariant() {
        #expect(throws: SpacetimeDecodingError.malformedSumVariant) {
            _ = try SpacetimePositionalDecoder.sumVariant(
                SpacetimePositionalDecoder.parseArray(#"["oops"]"#)
            )
        }
    }

    // MARK: - Argument encoding (transport)

    @Test("accept_new_user_registration arg encoding: Some inlines payload, None is [1,[]]")
    func registrationArgEncoding() {
        let some = SpacetimeDBRemoteDataSource.SpacetimeArgumentJSON
            .array([.number("0"), .string("auth0|owner")])
        #expect(some.encodingDescription == #"[0,"auth0|owner"]"#)

        let none = SpacetimeDBRemoteDataSource.SpacetimeArgumentJSON
            .array([.number("1"), .array([])])
        #expect(none.encodingDescription == "[1,[]]")
    }

    @Test("positional args body is a compact JSON array")
    func argumentsBodyEncoding() {
        let arguments: [SpacetimeDBRemoteDataSource.Argument] = [
            .string("auth0|user-77"),
            .unsignedInteger(25),
            .boolean(true)
        ]
        let encoded = "[\(arguments.map(\.encodingDescription).joined(separator: ","))]"
        #expect(encoded == #"["auth0|user-77",25,true]"#)
    }
}
