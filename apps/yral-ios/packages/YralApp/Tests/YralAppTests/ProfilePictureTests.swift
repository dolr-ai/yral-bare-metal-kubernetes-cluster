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
        let url = ProfilePicture.url(fromSubject: "auth0|user-77")
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

    @Test("avatar index stays in the GobGob pool range for negative-hash subjects")
    func avatarIndexInRangeForNegativeHashes() {
        // Regression: the Kotlin port took the remainder on the SIGNED
        // CRC32, so any subject whose hash had the high bit set
        // (~50%) produced index <= 0 -> `gob.-12345.png` -> permanent
        // 404. The unsigned form must map EVERY subject into
        // 1...18557 — the range the GobGob pool actually serves.
        var foundNegativeHashSubject = false
        for index in 0..<1000 {
            let subject = "subject-\(index)"
            let hash = ProfilePicture.crc32IEEE(Data(subject.utf8))
            let avatarIndex = ProfilePicture.avatarIndex(subject)
            #expect(avatarIndex >= 1 && avatarIndex <= 18_557)
            if Int32(bitPattern: hash) < 0 {
                foundNegativeHashSubject = true
                // The bug: the signed remainder is <= 0 for these.
                let signedRemainder = Int(Int32(bitPattern: hash) % Int32(18_557)) + 1
                #expect(signedRemainder <= 0)
                #expect(avatarIndex != signedRemainder)
            }
        }
        #expect(foundNegativeHashSubject, "expected at least one negative-hash subject in the sample")
    }

    @Test("positive-hash subjects keep the exact Kotlin URL")
    func avatarIndexUnchangedForPositiveHashes() {
        // Subjects whose CRC32 has the high bit clear produced the
        // same index in Kotlin and here — the URL must not change for
        // them (production continuity).
        for index in 0..<1000 {
            let subject = "subject-\(index)"
            let hash = ProfilePicture.crc32IEEE(Data(subject.utf8))
            if Int32(bitPattern: hash) >= 0 {
                let kotlinIndex = Int(Int32(bitPattern: hash) % Int32(18_557)) + 1
                #expect(kotlinIndex >= 1 && kotlinIndex <= 18_557)
                #expect(ProfilePicture.avatarIndex(subject) == kotlinIndex)
            }
        }
    }

    @Test("avatar index is deterministic per subject")
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

    @Test("username generation is deterministic for the same subject")
    func usernameDeterminism() {
        #expect(
            UsernameGenerator.username(fromSubject: "test-subject")
                == UsernameGenerator.username(fromSubject: "test-subject")
        )
    }

    @Test("generated usernames are hyphenated: modifier-modifier-noun, 3–15 chars")
    func usernameShape() {
        for index in 0..<200 {
            let username = UsernameGenerator.username(fromSubject: "subject-\(index)")
            #expect(username.count >= 3)
            #expect(username.count <= 15)
            // Lowercase letters + hyphens only.
            #expect(username.allSatisfy { $0.isLetter || $0 == "-" })

            // Exactly three words: modifier, distinct modifier, noun.
            let words = username.split(separator: "-").map(String.init)
            #expect(words.count == 3, "username \(username) is not 3 words")
            guard words.count == 3 else { continue }
            #expect(yralUsernameModifiers.contains(words[0]))
            #expect(yralUsernameModifiers.contains(words[1]))
            #expect(words[0] != words[1])
            #expect(yralUsernameNouns.contains(words[2]))
        }
    }

    @Test("resolveUsername prefers trimmed non-empty preferred; falls back per subject")
    func resolveUsername() {
        #expect(UsernameGenerator.resolveUsername(preferred: "  saikat  ", subject: "p") == "  saikat  ")
        #expect(UsernameGenerator.resolveUsername(preferred: "", subject: "p") != nil)
        #expect(UsernameGenerator.resolveUsername(preferred: nil, subject: "p") != nil)
        #expect(UsernameGenerator.resolveUsername(preferred: nil, subject: nil) == nil)
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
