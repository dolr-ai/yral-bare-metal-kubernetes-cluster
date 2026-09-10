import Foundation
import Testing

@testable import YralApp

/// Shared fixtures for the account-switcher integration suites — the
/// wire builders (LIVE shapes verified against Maincloud), the mock
/// client factory, and the request recorder. Namespace enum: no
/// instance state; @MainActor (the suites are MainActor-isolated too —
/// AuthClient/SessionStore construction requires it).
@MainActor
enum AccountSwitcherFixtures {

    // MARK: - Fixtures

    static let mainPrincipal = "110822651133748857609"
    static let firstBotPrincipal = "f0a50e9a-2565-4e76-894a-27edf2e833fc"
    static let secondBotPrincipal = "b3581395-5c53-4dfe-a978-2359b13d13e2"

    static let firstBotHostedAvatar =
        "https://link.storjshare.io/raw/bucket/owner/930816ac-22ea-4a31-8d3a-ed399e553169.jpg"
    static let secondBotHostedAvatar =
        "https://link.storjshare.io/raw/bucket/owner/a035d85e-4d23-4679-b080-c623412fd5df.jpg"

    static func freshDefaults() -> UserDefaults {
        let name = "account-switcher-avatar-integration-\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: name)!
        defaults.removePersistentDomain(forName: name)
        return defaults
    }

    /// A client with two bots in the local identity store (as the JWT
    /// merge seeds them — principals + usernames only, NO avatar URLs).
    static func makeClient(
        defaults: UserDefaults,
        keychain: KeychainStore,
        protocolClass: URLProtocol.Type
    ) -> (client: AuthClient, sessionStore: SessionStore) {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [protocolClass]
        let mockSession = URLSession(configuration: configuration)
        let dataSource = SpacetimeDBRemoteDataSource(
            idTokenProvider: { "test-id-token" },
            session: mockSession
        )
        let sessionStore = SessionStore()
        let client = AuthClient(
            authDataSource: AuthDataSource(session: mockSession),
            redirectScheme: "com.yral.iosApp",
            keychain: keychain,
            defaults: defaults,
            spacetimeDataSource: dataSource,
            influencerDataSource: AIInfluencerDataSource(session: mockSession),
            sessionStore: sessionStore
        )
        keychain.setString(Self.mainPrincipal, forKey: .mainPrincipal)
        // The name overlay requires an id token — the creator call is
        // Bearer-authenticated.
        keychain.setString("test-id-token", forKey: .idToken)
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

    /// The creator-names response body (`GET /api/v1/creator/influencers`,
    /// untyped per the spec defect) — shape per creator.py: {influencers: [...]}.
    static func creatorNamesResponseBody(
        entries: [(principal: String, name: String)]
    ) -> String {
        let rows = entries.map { entry in
            "{\"id\":\"\(entry.principal)\",\"name\":\"\(entry.name)\",\"display_name\":null,\"avatar_url\":null}"
        }
        return "{\"influencers\":[\(rows.joined(separator: ","))],\"total\":\(entries.count)}"
    }

    /// Serves BOTH endpoints the refresh flow hits: the SpacetimeDB
    /// profiles batch and the creator-names listing.
    @discardableResult
    static func serveProfilesAndNames(
        profilesBody: String,
        namesBody: String,
        recorder: RequestRecorder
    ) -> @Sendable (URLRequest) throws -> (HTTPURLResponse, Data) {
        return { request in
            recorder.record(request)
            let isCreatorCall = request.url?.path.contains("/api/v1/creator/influencers") == true
            let body = isCreatorCall ? namesBody : profilesBody
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

    /// Serves the profiles batch only (the creator listing fails — for
    /// asserting the name overlay's absence).
    @discardableResult
    static func serveProfilesOnly(
        profilesBody: String,
        recorder: RequestRecorder
    ) -> @Sendable (URLRequest) throws -> (HTTPURLResponse, Data) {
        return { request in
            recorder.record(request)
            if request.url?.path.contains("/api/v1/creator/influencers") == true {
                throw URLError(.notConnectedToInternet)
            }
            guard let response = HTTPURLResponse(
                url: request.url!,
                statusCode: 200,
                httpVersion: "HTTP/1.1",
                headerFields: ["Content-Type": "application/json"]
            ) else {
                throw URLError(.badServerResponse)
            }
            return (response, Data(profilesBody.utf8))
        }
    }
}

/// Per-suite protocols — Swift Testing runs independent suites in
/// PARALLEL, and a shared static handler would be overwritten by the
/// other suite mid-flight (observed: profiles hit twice, creator twice,
/// one suite serving the other's responses). Each suite gets its own
/// static handler; `makeClient` registers BOTH.
final class SwitcherAvatarURLProtocol: URLProtocol, @unchecked Sendable {
    nonisolated(unsafe) static var handler:
        (@Sendable (URLRequest) throws -> (HTTPURLResponse, Data))?

    static override func canInit(with request: URLRequest) -> Bool { true }
    static override func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        guard let handler = Self.handler else {
            client?.urlProtocol(
                self, didFailWithError: URLError(.unsupportedURL)
            )
            return
        }
        do {
            let (response, data) = try handler(request)
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: data)
            client?.urlProtocolDidFinishLoading(self)
        } catch {
            client?.urlProtocol(self, didFailWithError: error)
        }
    }

    override func stopLoading() {}
}

/// See `SwitcherAvatarURLProtocol` — this one backs the name/switch suite.
final class SwitcherNameURLProtocol: URLProtocol, @unchecked Sendable {
    nonisolated(unsafe) static var handler:
        (@Sendable (URLRequest) throws -> (HTTPURLResponse, Data))?

    static override func canInit(with request: URLRequest) -> Bool { true }
    static override func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        guard let handler = Self.handler else {
            client?.urlProtocol(
                self, didFailWithError: URLError(.unsupportedURL)
            )
            return
        }
        do {
            let (response, data) = try handler(request)
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: data)
            client?.urlProtocolDidFinishLoading(self)
        } catch {
            client?.urlProtocol(self, didFailWithError: error)
        }
    }

    override func stopLoading() {}
}

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
