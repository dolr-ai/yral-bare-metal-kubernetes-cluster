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
/// `URLProtocol` seam (`RecordingURLProtocol` — same pattern as
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
@Suite(.serialized)  // RecordingURLProtocol's static handler is per-test state
@MainActor
struct AccountSwitcherAvatarIntegrationTests {

    // MARK: - Fixtures

    static let mainPrincipal = "110822651133748857609"
    static let firstBotPrincipal = "f0a50e9a-2565-4e76-894a-27edf2e833fc"
    static let secondBotPrincipal = "b3581395-5c53-4dfe-a978-2359b13d13e2"

    static let firstBotHostedAvatar =
        "https://link.storjshare.io/raw/bucket/owner/930816ac-22ea-4a31-8d3a-ed399e553169.jpg"
    static let secondBotHostedAvatar =
        "https://link.storjshare.io/raw/bucket/owner/a035d85e-4d23-4679-b080-c623412fd5df.jpg"

    func freshDefaults() -> UserDefaults {
        let name = "account-switcher-avatar-integration-\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: name)!
        defaults.removePersistentDomain(forName: name)
        return defaults
    }

    /// A client with two bots in the local identity store (as the JWT
    /// merge seeds them — principals + usernames only, NO avatar URLs).
    func makeClient(
        defaults: UserDefaults,
        keychain: KeychainStore
    ) -> (client: AuthClient, sessionStore: SessionStore) {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [RecordingURLProtocol.self]
        let dataSource = SpacetimeDBRemoteDataSource(
            idTokenProvider: { "test-id-token" },
            session: URLSession(configuration: configuration)
        )
        let sessionStore = SessionStore()
        let client = AuthClient(
            authDataSource: AuthDataSource(
                session: URLSession(configuration: configuration)
            ),
            redirectScheme: "com.yral.iosApp",
            keychain: keychain,
            defaults: defaults,
            spacetimeDataSource: dataSource,
            sessionStore: sessionStore
        )
        keychain.setString(Self.mainPrincipal, forKey: .mainPrincipal)
        AIIdentitiesStore.put(
            [
                AIIdentityEntry(principal: Self.firstBotPrincipal, username: "dekuizuku"),
                AIIdentityEntry(principal: Self.secondBotPrincipal, username: "uraraka")
            ],
            defaults: defaults
        )
        return (client, sessionStore)
    }

    /// One bot profile row in the LIVE positional wire shape of
    /// `UserProfileDetails` (11 fields; verified against Maincloud):
    /// [oauthSubject, profilePicture?, bio, websiteURL, followersCount,
    ///  followingCount, callerFollowsUser?, userFollowsCaller?,
    ///  subscriptionPlan, isAiInfluencer, accountType]
    /// Single-field variants INLINE their payloads (Some(bool) = [0,false],
    /// BotAccount = [1,"owner"]); struct payloads stay wrapped
    /// (Some(ProfilePictureData) = [0,[url,[nsfw…]]]).
    static func profileWireRow(
        principal: String,
        avatarURL: String?
    ) -> String {
        let pictureField: String
        if let avatarURL {
            // Some(ProfilePictureData) — struct payload stays wrapped:
            // [0, [url, [isNsfw, nsfwEc, nsfwGore, csamDetected]]]
            pictureField = #"[0, ["\#(avatarURL)", [false, "0.0", "0.0", false]]]"#
        } else {
            // None: [1, []]
            pictureField = "[1, []]"
        }
        return #"""
        [
          "\#(principal)",
          \#(pictureField),
          "bot bio",
          "",
          100,
          50,
          [0, false],
          [0, false],
          [0, []],
          true,
          [1, "\#(mainPrincipal)"]
        ]
        """#
    }

    /// The full response body of `get_users_profile_details` — a JSON
    /// array of profile positional arrays.
    static func profilesResponseBody(rows: [String]) -> String {
        "[" + rows.joined(separator: ",") + "]"
    }

    @discardableResult
    func serveProfiles(
        body: String,
        recorder: RequestRecorder
    ) -> @Sendable (URLRequest) throws -> (HTTPURLResponse, Data) {
        return { request in
            recorder.record(request)
            guard let response = HTTPURLResponse(
                url: request.url!,
                statusCode: 200,
                httpVersion: "HTTP/1.1",
                headerFields: ["Content-Type": "application/json"]
            ) else {
                throw URLError(.badServerResponse)
            }
            return (response, Data(body.utf8))
        }
    }

    // MARK: - The contract: table URL reaches the switcher row

    @Test("switcher rows carry the hosted avatar URLs from the profile table")
    func switcherRowsShowHostedAvatars() async throws {
        let keychain = KeychainStore(service: "switcher-avatar-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = freshDefaults()
        let (client, _) = makeClient(defaults: defaults, keychain: keychain)

        let recorder = RequestRecorder()
        RecordingURLProtocol.handler = serveProfiles(
            body: Self.profilesResponseBody(rows: [
                Self.profileWireRow(
                    principal: Self.firstBotPrincipal,
                    avatarURL: Self.firstBotHostedAvatar
                ),
                Self.profileWireRow(
                    principal: Self.secondBotPrincipal,
                    avatarURL: Self.secondBotHostedAvatar
                )
            ]),
            recorder: recorder
        )

        let entries = await client.refreshedAccountSwitcherEntries()

        // THE assertion — the device-reported bug: rows must show the
        // durable URLs written at creation, not the GobGob fallback.
        let avatarsByPrincipal = Dictionary(
            uniqueKeysWithValues: (entries?.aiAccounts ?? []).map { ($0.principal, $0.avatarURL) }
        )
        #expect(avatarsByPrincipal[Self.firstBotPrincipal] == Self.firstBotHostedAvatar)
        #expect(avatarsByPrincipal[Self.secondBotPrincipal] == Self.secondBotHostedAvatar)

        // The read is ONE batch call carrying both bot principals.
        #expect(recorder.requests.count == 1)
        guard let body = recorder.requestBodies.first else {
            Issue.record("expected a request body")
            return
        }
        #expect(body.contains(Self.firstBotPrincipal))
        #expect(body.contains(Self.secondBotPrincipal))
    }

    @Test("blank profile rows keep the GobGob fallback (bot whose write never landed)")
    func blankProfileRowKeepsFallback() async throws {
        let keychain = KeychainStore(service: "switcher-avatar-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = freshDefaults()
        let (client, _) = makeClient(defaults: defaults, keychain: keychain)

        let recorder = RequestRecorder()
        // One bot with its real picture, one with a None picture (the
        // pre-wire-fix creations left the table row blank).
        RecordingURLProtocol.handler = serveProfiles(
            body: Self.profilesResponseBody(rows: [
                Self.profileWireRow(
                    principal: Self.firstBotPrincipal,
                    avatarURL: Self.firstBotHostedAvatar
                ),
                Self.profileWireRow(principal: Self.secondBotPrincipal, avatarURL: nil)
            ]),
            recorder: recorder
        )

        let entries = await client.refreshedAccountSwitcherEntries()

        let avatarsByPrincipal = Dictionary(
            uniqueKeysWithValues: (entries?.aiAccounts ?? []).map { ($0.principal, $0.avatarURL) }
        )
        #expect(avatarsByPrincipal[Self.firstBotPrincipal] == Self.firstBotHostedAvatar)
        // The GobGob fallback — deterministic per-principal, and (after
        // the sign-trap fix) always a real avatar index.
        #expect(
            avatarsByPrincipal[Self.secondBotPrincipal]
                == ProfilePicture.url(fromPrincipal: Self.secondBotPrincipal)
        )
        #expect(
            avatarsByPrincipal[Self.secondBotPrincipal]
                != Self.secondBotHostedAvatar
        )
    }

    @Test("network failure keeps the GobGob fallback rows (offline-safe)")
    func networkFailureKeepsFallbacks() async throws {
        let keychain = KeychainStore(service: "switcher-avatar-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = freshDefaults()
        let (client, _) = makeClient(defaults: defaults, keychain: keychain)

        // The handler throws — the profile read fails entirely.
        RecordingURLProtocol.handler = { _ in
            throw URLError(.notConnectedToInternet)
        }

        let entries = await client.refreshedAccountSwitcherEntries()

        let avatarsByPrincipal = Dictionary(
            uniqueKeysWithValues: (entries?.aiAccounts ?? []).map { ($0.principal, $0.avatarURL) }
        )
        #expect(
            avatarsByPrincipal[Self.firstBotPrincipal]
                == ProfilePicture.url(fromPrincipal: Self.firstBotPrincipal)
        )
        #expect(
            avatarsByPrincipal[Self.secondBotPrincipal]
                == ProfilePicture.url(fromPrincipal: Self.secondBotPrincipal)
        )
    }

    @Test("switchToAccount persists the tapped row's hosted URL into the session cache")
    func switchPersistsRowAvatarIntoSession() async throws {
        let keychain = KeychainStore(service: "switcher-avatar-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = freshDefaults()
        let (client, sessionStore) = makeClient(defaults: defaults, keychain: keychain)

        // The user taps the row AFTER the live refresh — the row carries
        // the hosted URL; the session + PROFILE_PIC cache must show it.
        client.switchToAccount(
            principal: Self.firstBotPrincipal,
            avatarURL: Self.firstBotHostedAvatar
        )

        #expect(sessionStore.profilePic == Self.firstBotHostedAvatar)
        #expect(
            defaults.string(forKey: "PROFILE_PIC") == Self.firstBotHostedAvatar
        )
        // The bot becomes the last-active principal (cold-start continuity).
        #expect(keychain.string(forKey: .lastActivePrincipal) == Self.firstBotPrincipal)
    }

    @Test("switchToAccount without a URL falls back to GobGob (offline switch)")
    func switchWithoutURLFallsBack() {
        let keychain = KeychainStore(service: "switcher-avatar-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = freshDefaults()
        let (client, sessionStore) = makeClient(defaults: defaults, keychain: keychain)

        client.switchToAccount(principal: Self.firstBotPrincipal, avatarURL: nil)

        #expect(
            sessionStore.profilePic
                == ProfilePicture.url(fromPrincipal: Self.firstBotPrincipal)
        )
    }
}

/// Request recorder — reference-boxed so the @Sendable URLProtocol
/// handler (running off the MainActor) can record into it. Same shape
/// as `RefreshRecorder` in `AuthClientTests` (file-scope for the same
/// isolation reasons).
final class RequestRecorder: @unchecked Sendable {
    private let lock = NSLock()
    private var storedRequests: [URLRequest] = []
    private var storedBodies: [String] = []

    var requests: [URLRequest] {
        lock.lock()
        defer { lock.unlock() }
        return storedRequests
    }

    var requestBodies: [String] {
        lock.lock()
        defer { lock.unlock() }
        return storedBodies
    }

    func record(_ request: URLRequest) {
        lock.lock()
        defer { lock.unlock() }
        storedRequests.append(request)
        if let body = request.httpBody ?? request.bodyStreamData {
            storedBodies.append(String(data: body, encoding: .utf8) ?? "")
        }
    }
}
