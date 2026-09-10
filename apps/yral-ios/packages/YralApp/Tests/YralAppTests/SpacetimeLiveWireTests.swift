import Foundation
import Testing

@testable import YralApp

/// LIVE WIRE pins — responses captured byte-for-byte from Maincloud
/// (`yral-database-spacetime-4lbo7`, Sep 9 2026). These pin the exact
/// SATS JSON serialization SpacetimeDB actually emits, not what we
/// assumed it emits — the distinction behind the "GobGob avatars on
/// device" bug: the hand-written fixtures used wrapped single-field
/// variant payloads (`Some(false)` as `[0,[false]]`) while the live
/// wire INLINES them (`[0,false]`), so the decoder threw on every real
/// profile and every caller silently fell back.
///
/// The serialization rules these pins enforce:
///   - ONE-field variants inline the payload:  `Some(bool)` → `[0,false]`,
///     `BotAccount(String)` → `[1,"owner"]`, `MainAccount(Vec<String>)` →
///     `[0,["id","id"]]` (the Vec IS the payload).
///   - Multi-field variants wrap it: `Pro(u32,u32)` → `[1,[3,10]]`.
///   - Struct payloads of Some stay wrapped: `Some(ProfilePictureData)` →
///     `[0,[url,[nsfw…]]]` (the struct itself is the payload, an array).
///   - Unit variants use empty arrays: `Free` → `[0,[]]`, `None` → `[1,[]]`.
struct SpacetimeLiveWireTests {

    // MARK: - get_users_profile_details (bot batch read)

    static let firstBotAvatarURL =
        "https://link.storjshare.io/raw/juojrspbmsy7dtukovdepimmpnma"
        + "/yral-profile-pictures/110822651133748857609"
        + "/930816ac-22ea-4a31-8d3a-ed399e553169.jpg"

    static let secondBotAvatarURL =
        "https://link.storjshare.io/raw/juojrspbmsy7dtukovdepimmpnma"
        + "/yral-profile-pictures/110822651133748857609"
        + "/a035d85e-4d23-4679-b080-c623412fd5df.jpg"

    /// The exact response for the owner's two bots (raw capture; the long
    /// Storj URLs are split inside the JSON string via concatenation —
    /// the byte stream is unchanged). SpacetimeDB's live serialization:
    /// follow flags and BotAccount owner are INLINED single-field
    /// payloads; the Some(ProfilePictureData) struct payload stays
    /// wrapped; Free and None are empty arrays.
    static let storjBase = "https://link.storjshare.io/raw/juojrspbmsy7dtukovdepimmpnma"
    static let storjPrefix = "/yral-profile-pictures/110822651133748857609/"
    static let firstBotBio =
        "A determined hero-in-training from U.A. High School, Izuku Midoriya, known as Deku,"
        + " strives to master One For All and become the greatest hero."
    static let secondBotBio =
        "A cheerful and determined U.A. High student with the 'Zero Gravity' Quirk,"
        + " striving to become a Pro Hero to support her family."

    static let liveBotProfileBatch = #"""
    [
      [
        "f0a50e9a-2565-4e76-894a-27edf2e833fc",
        [0,["\#(storjBase)\#(storjPrefix)930816ac-22ea-4a31-8d3a-ed399e553169.jpg",[false,"","",false]]],
        "\#(firstBotBio)",
        "",
        0,
        0,
        [0,false],
        [0,false],
        [0,[]],
        false,
        [1,"110822651133748857609"]
      ],
      [
        "b3581395-5c53-4dfe-a978-2359b13d13e2",
        [0,["\#(storjBase)\#(storjPrefix)a035d85e-4d23-4679-b080-c623412fd5df.jpg",[false,"","",false]]],
        "\#(secondBotBio)",
        "",
        0,
        0,
        [0,false],
        [0,false],
        [0,[]],
        false,
        [1,"110822651133748857609"]
      ]
    ]
    """#

    @Test("live-captured bot profile batch decodes — the device regression")
    func liveBotProfileBatchDecodes() throws {
        let batch = try SpacetimePositionalDecoder.parseArray(Self.liveBotProfileBatch)
        #expect(batch.count == 2)

        let first = try SpacetimeUserProfile.fromPositionalArray(
            try SpacetimePositionalDecoder.array(batch, at: 0)
        )
        #expect(first.oauthSubject == "f0a50e9a-2565-4e76-894a-27edf2e833fc")
        #expect(first.profilePicture?.url == Self.firstBotAvatarURL)
        #expect(first.callerFollowsUser == false)
        #expect(first.userFollowsCaller == false)
        #expect(first.subscriptionPlan == .free)
        #expect(first.isAiInfluencer == false)
        guard case let .botAccount(owner) = first.accountType else {
            Issue.record("expected botAccount with the inlined owner payload")
            return
        }
        #expect(owner == "110822651133748857609")

        let second = try SpacetimeUserProfile.fromPositionalArray(
            try SpacetimePositionalDecoder.array(batch, at: 1)
        )
        #expect(second.profilePicture?.url == Self.secondBotAvatarURL)
        guard case let .botAccount(secondOwner) = second.accountType else {
            Issue.record("expected botAccount")
            return
        }
        #expect(secondOwner == "110822651133748857609")
    }

    // MARK: - get_users_profile_details (main-account read)

    /// The owner's own profile — the MainAccount Vec-payload inline form
    /// (`[0,["id","id"]]`), captured from Maincloud.
    @Test("live-captured main-account profile decodes — MainAccount Vec inline")
    func liveMainAccountProfileDecodes() throws {
        let body = #"""
        [
          "110822651133748857609",
          [1,[]],
          "",
          "",
          0,
          0,
          [0,false],
          [0,false],
          [0,[]],
          false,
          [0,["f0a50e9a-2565-4e76-894a-27edf2e833fc","b3581395-5c53-4dfe-a978-2359b13d13e2"]]
        ]
        """#
        let profile = try SpacetimeUserProfile.fromPositionalArray(
            try SpacetimePositionalDecoder.parseArray(body)
        )
        #expect(profile.oauthSubject == "110822651133748857609")
        #expect(profile.profilePicture == nil)
        #expect(profile.callerFollowsUser == false)
        guard case let .mainAccount(aiAccounts) = profile.accountType else {
            Issue.record("expected mainAccount with the inlined Vec payload")
            return
        }
        #expect(aiAccounts.count == 2)
        #expect(aiAccounts.contains("f0a50e9a-2565-4e76-894a-27edf2e833fc"))
        #expect(aiAccounts.contains("b3581395-5c53-4dfe-a978-2359b13d13e2"))
    }

    // MARK: - get_followers (cursor pagination)

    /// Captured empty first page: `[[],0,[1,[]]]` — None cursor is the
    /// unit form. (A Some cursor was captured as `[0,"subject"]` from the
    /// follow-flags pattern; the decoder test suite pins both forms.)
    @Test("live-captured empty followers page decodes — None cursor")
    func liveEmptyFollowersPageDecodes() throws {
        let page = try SpacetimeFollowersPage.fromPositionalArray(
            try SpacetimePositionalDecoder.parseArray("[[],0,[1,[]]]")
        )
        #expect(page.followers.isEmpty)
        #expect(page.totalCount == 0)
        #expect(page.nextCursor == nil)
    }
}
