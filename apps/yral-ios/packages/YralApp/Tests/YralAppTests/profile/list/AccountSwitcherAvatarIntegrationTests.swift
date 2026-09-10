import Foundation
import Testing

@testable import YralApp

/// Integration tests for the account switcher's avatar display — the
/// device-reported bug: "images for the AI influencer profiles I created
/// are not loading."
///
/// The contract under test: the switcher rows must carry the bot's
/// DURABLE hosted avatar URL (the Storj link written to the SpacetimeDB
/// profile table at creation), not the GobGob deterministic fallback.
///
/// Level: the real `AuthClient` + real `AIIdentitiesStore` + real
/// `SpacetimeDBRemoteDataSource`, with HTTP stubbed at the Apple-native
/// `URLProtocol` seam (`ChannelURLProtocol` — same pattern as
/// `AuthClientTests`). The stub serves the EXACT live wire shape of
/// `get_users_profile_details` (positional arrays — see
/// `SpacetimePositionalDecoderTests` for the field order).
///
/// Why not XCUITest: SwiftUI has no native headless view-render testing
/// mechanism; XCUITest launches the whole app in a simulator and can
/// assert only that an element EXISTS — `AsyncImage` renders its
/// placeholder on load failure while the element still exists, so it
/// cannot see this bug at all. The bug is which URL STRING reaches the
/// row — a data-flow assertion, testable deterministically here.
@MainActor
struct AccountSwitcherAvatarIntegrationTests {

    /// Short alias — the fixture namespace name is long.
    typealias Fixtures = AccountSwitcherFixtures

    // MARK: - The contract: table URL reaches the switcher row

    @Test("switcher rows carry the hosted avatar URLs from the profile table")
    func switcherRowsShowHostedAvatars() async throws {
        let keychain = KeychainStore(service: "switcher-avatar-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = Fixtures.freshDefaults()
        let channel = UUID().uuidString
        defer { ChannelURLProtocol.unregister(channel: channel) }
        let (client, _) = Fixtures.makeClient(
                defaults: defaults,
                keychain: keychain,
                channel: channel
            )

        let recorder = RequestRecorder()
        ChannelURLProtocol.register(
            Fixtures.serveProfilesAndNames(
                profilesBody: Fixtures.profilesResponseBody(rows: [
                    Fixtures.profileWireRow(
                        subject: Fixtures.firstBotSubject,
                        avatarURL: Fixtures.firstBotHostedAvatar
                    ),
                    Fixtures.profileWireRow(
                        subject: Fixtures.secondBotSubject,
                        avatarURL: Fixtures.secondBotHostedAvatar
                    )
                ]),
                namesBody: Fixtures.creatorNamesResponseBody(entries: [
                    (subject: Fixtures.firstBotSubject, name: "dekuizuku"),
                    (subject: Fixtures.secondBotSubject, name: "uraraka")
                ]),
                recorder: recorder
            ),
            forChannel: channel
        )

        let entries = await client.refreshedAccountSwitcherEntries()

        // THE assertion — the device-reported bug: rows must show the
        // durable URLs written at creation, not the GobGob fallback.
        let avatarsBySubject = Dictionary(
            uniqueKeysWithValues: (entries?.aiAccounts ?? []).map { ($0.subject, $0.avatarURL) }
        )
        #expect(avatarsBySubject[Fixtures.firstBotSubject] == Fixtures.firstBotHostedAvatar)
        #expect(avatarsBySubject[Fixtures.secondBotSubject] == Fixtures.secondBotHostedAvatar)

        // Exactly TWO calls: the profiles batch + the creator listing.
        #expect(recorder.requests.count == 2)
        guard let profileCall = recorder.requests.first(where: {
            $0.url?.path.contains("/call/get_users_profile_details") == true
        }) else {
            Issue.record("expected a profiles batch call")
            return
        }
        let profileBody = String(
            data: profileCall.httpBody ?? profileCall.bodyStreamData ?? Data(),
            encoding: .utf8
        ) ?? ""
        #expect(profileBody.contains(Fixtures.firstBotSubject))
        #expect(profileBody.contains(Fixtures.secondBotSubject))
    }

    @Test("blank profile rows keep the GobGob fallback (bot whose write never landed)")
    func blankProfileRowKeepsFallback() async throws {
        let keychain = KeychainStore(service: "switcher-avatar-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = Fixtures.freshDefaults()
        let channel = UUID().uuidString
        defer { ChannelURLProtocol.unregister(channel: channel) }
        let (client, _) = Fixtures.makeClient(
                defaults: defaults,
                keychain: keychain,
                channel: channel
            )

        let recorder = RequestRecorder()
        // One bot with its real picture, one with a None picture (the
        // pre-wire-fix creations left the table row blank).
        ChannelURLProtocol.register(
            Fixtures.serveProfilesAndNames(
                profilesBody: Fixtures.profilesResponseBody(rows: [
                    Fixtures.profileWireRow(
                        subject: Fixtures.firstBotSubject,
                        avatarURL: Fixtures.firstBotHostedAvatar
                    ),
                    Fixtures.profileWireRow(subject: Fixtures.secondBotSubject, avatarURL: nil)
                ]),
                namesBody: Fixtures.creatorNamesResponseBody(entries: []),
                recorder: recorder
            ),
            forChannel: channel
        )

        let entries = await client.refreshedAccountSwitcherEntries()

        let avatarsBySubject = Dictionary(
            uniqueKeysWithValues: (entries?.aiAccounts ?? []).map { ($0.subject, $0.avatarURL) }
        )
        #expect(avatarsBySubject[Fixtures.firstBotSubject] == Fixtures.firstBotHostedAvatar)
        // The GobGob fallback — deterministic per-subject, and (after
        // the sign-trap fix) always a real avatar index.
        #expect(
            avatarsBySubject[Fixtures.secondBotSubject]
                == ProfilePicture.url(fromSubject: Fixtures.secondBotSubject)
        )
        #expect(
            avatarsBySubject[Fixtures.secondBotSubject]
                != Fixtures.secondBotHostedAvatar
        )
    }

    @Test("network failure keeps the GobGob fallback rows (offline-safe)")
    func networkFailureKeepsFallbacks() async throws {
        let keychain = KeychainStore(service: "switcher-avatar-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = Fixtures.freshDefaults()
        let channel = UUID().uuidString
        defer { ChannelURLProtocol.unregister(channel: channel) }
        let (client, _) = Fixtures.makeClient(
                defaults: defaults,
                keychain: keychain,
                channel: channel
            )

        // The handler throws — the profile read fails entirely.
        ChannelURLProtocol.register(
            { _ in throw URLError(.notConnectedToInternet) },
            forChannel: channel
        )

        let entries = await client.refreshedAccountSwitcherEntries()

        let avatarsBySubject = Dictionary(
            uniqueKeysWithValues: (entries?.aiAccounts ?? []).map { ($0.subject, $0.avatarURL) }
        )
        #expect(
            avatarsBySubject[Fixtures.firstBotSubject]
                == ProfilePicture.url(fromSubject: Fixtures.firstBotSubject)
        )
        #expect(
            avatarsBySubject[Fixtures.secondBotSubject]
                == ProfilePicture.url(fromSubject: Fixtures.secondBotSubject)
        )
    }

}
