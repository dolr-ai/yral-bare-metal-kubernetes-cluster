import Testing
import Foundation
@testable import YralApp

/// Tests for `ProfilePicture` (propic URL + CRC32 with the Kotlin
/// signed-remainder trap) and `UsernameGenerator` (deterministic
/// fallback names) — ported and expanded from Kotlin
/// `UsernameUtilsTest.kt` / propic expectations.
struct ProfilePictureTests {

    // MARK: - GobGob avatar URL

    @Test("avatar URL is prefix + index + .png")
    func avatarURL() {
        let url = ProfilePicture.url(fromPrincipal: "auth0|user-77")
        let index = ProfilePicture.avatarIndex("auth0|user-77")
        #expect(url == "https://prakash-yral.hel1.your-objectstorage.com/gobgob/gob.\(index).png")
        #expect(url.hasPrefix(ProfilePicture.gobgobURLPrefix))
        #expect(url.hasSuffix(".png"))
    }

    @Test("CRC32 is IEEE 802.3 — reference vector \"123456789\"")
    func crc32ReferenceVector() {
        // Canonical CRC-32/ISO-HDLC check value (IEEE 802.3).
        #expect(ProfilePicture.crc32IEEE(Data("123456789".utf8)) == 0xCBF4_3926)
    }

    @Test("CRC32 is deterministic and case-sensitive")
    func crc32Deterministic() {
        let firstHash = ProfilePicture.crc32IEEE(Data("auth0|user-77".utf8))
        #expect(firstHash == ProfilePicture.crc32IEEE(Data("auth0|user-77".utf8)))
        #expect(firstHash != ProfilePicture.crc32IEEE(Data("AUTH0|USER-77".utf8)))
    }

    @Test("avatar index reproduces Kotlin's signed-remainder trap")
    func avatarIndexSignTrap() {
        // Kotlin: (crc32 % 18557) + 1 where crc32 is a SIGNED Int and % is
        // remainder-with-sign-of-dividend. For a negative hash the result is
        // ≤ 0 — the shipped production behavior we reproduce verbatim.
        var foundNegativePath = false
        for index in 0..<1000 {
            let principal = "principal-\(index)"
            let hash = ProfilePicture.crc32IEEE(Data(principal.utf8))
            if Int32(bitPattern: hash) < 0 {
                let avatarIndex = ProfilePicture.avatarIndex(principal)
                let kotlinIndex = Int(Int32(bitPattern: hash) % Int32(18_557)) + 1
                #expect(avatarIndex == kotlinIndex)
                #expect(avatarIndex <= 0)
                foundNegativePath = true
            }
        }
        #expect(foundNegativePath, "expected at least one negative-hash principal in the sample")
    }

    @Test("avatar index is deterministic per principal")
    func avatarIndexDeterministic() {
        #expect(ProfilePicture.avatarIndex("p") == ProfilePicture.avatarIndex("p"))
        #expect(ProfilePicture.avatarIndex("p") != ProfilePicture.avatarIndex("q"))
    }

    // MARK: - Username generation

    @Test("word pools have the Kotlin-verified sizes and no duplicates")
    func wordPoolSizes() {
        #expect(yralUsernameModifiers.count == 200)
        #expect(Set(yralUsernameModifiers).count == 200)
        #expect(yralUsernameNouns.count == 150)
        #expect(Set(yralUsernameNouns).count == 150)
    }

    @Test("username generation is deterministic for the same principal")
    func usernameDeterminism() {
        #expect(
            UsernameGenerator.username(fromPrincipal: "test-principal")
                == UsernameGenerator.username(fromPrincipal: "test-principal")
        )
    }

    @Test("generated usernames are letters-only, 3–15 chars, two modifiers + noun")
    func usernameShape() {
        for index in 0..<200 {
            let username = UsernameGenerator.username(fromPrincipal: "principal-\(index)")
            #expect(username.count >= 3)
            #expect(username.count <= 15)
            #expect(username.allSatisfy { $0.isLetter })

            // Decompose: first word from modifiers, second from modifiers
            // (≠ first), remainder a noun — Kotlin's
            // "modifier modifier animal" contract.
            let composition = decompose(username)
            #expect(composition != nil, "username \(username) failed to decompose")
            guard let composition else { continue }
            #expect(composition.firstModifier != composition.secondModifier)
            #expect(yralUsernameNouns.contains(composition.noun))
        }
    }

    /// Kotlin `findUsernameComposition`'s `UsernameComposition`: two distinct
    /// modifiers + a noun.
    struct UsernameComposition {
        let firstModifier: String
        let secondModifier: String
        let noun: String
    }

    /// Kotlin `findUsernameComposition`: try each modifier prefix, then each
    /// distinct second modifier, remainder must be a noun.
    private func decompose(_ username: String) -> UsernameComposition? {
        for firstModifier in yralUsernameModifiers
        where username.hasPrefix(firstModifier) {
            let afterFirstModifier = String(username.dropFirst(firstModifier.count))
            for secondModifier in yralUsernameModifiers
            where secondModifier != firstModifier
                && afterFirstModifier.hasPrefix(secondModifier) {
                let noun = String(
                    afterFirstModifier.dropFirst(secondModifier.count)
                )
                if yralUsernameNouns.contains(noun) {
                    return UsernameComposition(
                        firstModifier: firstModifier,
                        secondModifier: secondModifier,
                        noun: noun
                    )
                }
            }
        }
        return nil
    }

    @Test("resolveUsername prefers trimmed non-empty preferred; falls back per principal")
    func resolveUsername() {
        #expect(UsernameGenerator.resolveUsername(preferred: "  saikat  ", principal: "p") == "  saikat  ")
        #expect(UsernameGenerator.resolveUsername(preferred: "", principal: "p") != nil)
        #expect(UsernameGenerator.resolveUsername(preferred: nil, principal: "p") != nil)
        #expect(UsernameGenerator.resolveUsername(preferred: nil, principal: nil) == nil)
    }

    @Test("unsafe words are absent from the username pools")
    func unsafeWordsAbsent() {
        let unsafeWords = [
            "sex", "sexy", "thong", "pimp", "racist", "machete",
            "nude", "naked", "violent", "terrorist", "obscene"
        ]
        let allWords = yralUsernameModifiers + yralUsernameNouns
        for unsafeWord in unsafeWords {
            #expect(!allWords.contains(unsafeWord))
        }
    }
}
