import Foundation
import Testing

@testable import YralApp

/// Real-bot-name overlay + account-switch session tests — split
/// from `AccountSwitcherAvatarIntegrationTests.swift` to stay
/// within the file/type lint bounds.
@Suite(.serialized)
@MainActor
struct AccountSwitcherNameAndSwitchTests {

    /// Short alias — the fixture namespace name is long.
    typealias Fixtures = AccountSwitcherFixtures
    // MARK: - Real bot names (the creator-name overlay)

    @Test("creator listing overlays the real bot names over pseudonyms")
    func creatorNamesOverlay() async throws {
        let keychain = KeychainStore(service: "switcher-avatar-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = Fixtures.freshDefaults()
        let (client, _) = Fixtures.makeClient(
                defaults: defaults,
                keychain: keychain,
                protocolClass: SwitcherNameURLProtocol.self
            )

        // The local store's usernames exist only on the creating device;
        // on a fresh install they're absent and rows show pseudonyms.
        AIIdentitiesStore.put(
            [
                AIIdentityEntry(principal: Fixtures.firstBotPrincipal, username: nil),
                AIIdentityEntry(principal: Fixtures.secondBotPrincipal, username: nil)
            ],
            defaults: defaults
        )
        SwitcherNameURLProtocol.handler = Fixtures.serveProfilesAndNames(
            profilesBody: Fixtures.profilesResponseBody(rows: [
                Fixtures.profileWireRow(
                    principal: Fixtures.firstBotPrincipal,
                    avatarURL: Fixtures.firstBotHostedAvatar
                ),
                Fixtures.profileWireRow(principal: Fixtures.secondBotPrincipal, avatarURL: nil)
            ]),
            namesBody: Fixtures.creatorNamesResponseBody(entries: [
                (principal: Fixtures.firstBotPrincipal, name: "dekuizuku"),
                (principal: Fixtures.secondBotPrincipal, name: "uraraka")
            ]),
            recorder: RequestRecorder()
        )

        let entries = await client.refreshedAccountSwitcherEntries()

        let namesByPrincipal = Dictionary(
            uniqueKeysWithValues: (entries?.aiAccounts ?? []).map { ($0.principal, $0.username) }
        )
        #expect(namesByPrincipal[Fixtures.firstBotPrincipal] == "dekuizuku")
        #expect(namesByPrincipal[Fixtures.secondBotPrincipal] == "uraraka")
    }

    @Test("bots without a creator record keep their pseudonym fallback")
    func missingCreatorRecordKeepsPseudonym() async throws {
        let keychain = KeychainStore(service: "switcher-avatar-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = Fixtures.freshDefaults()
        let (client, _) = Fixtures.makeClient(
                defaults: defaults,
                keychain: keychain,
                protocolClass: SwitcherNameURLProtocol.self
            )

        AIIdentitiesStore.put(
            [AIIdentityEntry(principal: Fixtures.firstBotPrincipal, username: nil)],
            defaults: defaults
        )
        SwitcherNameURLProtocol.handler = Fixtures.serveProfilesAndNames(
            profilesBody: Fixtures.profilesResponseBody(rows: [
                Fixtures.profileWireRow(
                    principal: Fixtures.firstBotPrincipal,
                    avatarURL: Fixtures.firstBotHostedAvatar
                )
            ]),
            // The creator listing knows only the first bot.
            namesBody: Fixtures.creatorNamesResponseBody(entries: [
                (principal: Fixtures.firstBotPrincipal, name: "dekuizuku")
            ]),
            recorder: RequestRecorder()
        )

        let entries = await client.refreshedAccountSwitcherEntries()

        guard let firstBot = entries?.aiAccounts.first else {
            Issue.record("expected the bot row")
            return
        }
        #expect(firstBot.principal == Fixtures.firstBotPrincipal)
        #expect(firstBot.username == "dekuizuku")
    }

    @Test("switchToAccount persists the tapped row's hosted URL into the session cache")
    func switchPersistsRowAvatarIntoSession() async throws {
        let keychain = KeychainStore(service: "switcher-avatar-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = Fixtures.freshDefaults()
        let (client, sessionStore) = Fixtures.makeClient(
                defaults: defaults,
                keychain: keychain,
                protocolClass: SwitcherNameURLProtocol.self
            )

        // The user taps the row AFTER the live refresh — the row carries
        // the hosted URL; the session + PROFILE_PIC cache must show it.
        client.switchToAccount(
            principal: Fixtures.firstBotPrincipal,
            avatarURL: Fixtures.firstBotHostedAvatar
        )

        #expect(sessionStore.profilePic == Fixtures.firstBotHostedAvatar)
        #expect(
            defaults.string(forKey: "PROFILE_PIC") == Fixtures.firstBotHostedAvatar
        )
        // The bot becomes the last-active principal (cold-start continuity).
        #expect(keychain.string(forKey: .lastActivePrincipal) == Fixtures.firstBotPrincipal)
    }

    @Test("switchToAccount without a URL falls back to GobGob (offline switch)")
    func switchWithoutURLFallsBack() {
        let keychain = KeychainStore(service: "switcher-avatar-tests-\(UUID().uuidString)")
        defer { keychain.removeAll() }
        let defaults = Fixtures.freshDefaults()
        let (client, sessionStore) = Fixtures.makeClient(
                defaults: defaults,
                keychain: keychain,
                protocolClass: SwitcherNameURLProtocol.self
            )

        client.switchToAccount(principal: Fixtures.firstBotPrincipal, avatarURL: nil)

        #expect(
            sessionStore.profilePic
                == ProfilePicture.url(fromPrincipal: Fixtures.firstBotPrincipal)
        )
    }
}

/// Request recorder — reference-boxed so the @Sendable URLProtocol
/// handler (running off the MainActor) can record into it. Same shape
/// as `RefreshRecorder` in `AuthClientTests` (file-scope for the same
/// isolation reasons).
