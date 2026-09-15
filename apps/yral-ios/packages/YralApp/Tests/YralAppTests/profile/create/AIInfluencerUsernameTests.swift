import Testing
@testable import YralApp

/// Pins the handle alphabet to the server's settled alphabet.
///
/// The drift these lock down (found 2026-09-15, dolr-ai/yral-rishi-agent#522):
/// the server settles a free handle in Kotlin's `[a-z0-9]`, ≤15 alphabet and
/// `/create` re-checks uniqueness on that SAME string. Our form previously
/// allowed `_` and applied no length cap, so the name validate checked could
/// differ from the name create received — the identical bug class Rishi fixed
/// for Kotlin. `CreateInfluencerRequest` accepts `^[a-z0-9_-]+$` with
/// `min_length=3, max_length=50`, so `_` is server-legal but NOT produced by
/// the server's settle step; mirroring the settle alphabet keeps the two
/// identical.
@Suite("Influencer username alphabet")
struct AIInfluencerUsernameTests {

    @Test("strips everything outside [a-z0-9]")
    func stripsNonAlphanumerics() {
        // Underscores and hyphens are server-legal but not in the settle
        // alphabet — the case our form used to let through.
        #expect(AIInfluencerUsername.sanitized("my_bot") == "mybot")
        #expect(AIInfluencerUsername.sanitized("meera-2") == "meera2")
        #expect(AIInfluencerUsername.sanitized("My Bot!") == "mybot")
        #expect(AIInfluencerUsername.sanitized("  spaced  ") == "spaced")
    }

    @Test("lowercases")
    func lowercases() {
        #expect(AIInfluencerUsername.sanitized("WanderLens") == "wanderlens")
    }

    @Test("caps at 15 characters")
    func capsAtMaximumLength() {
        let sixteenCharacters = "abcdefghijklmnop"
        #expect(AIInfluencerUsername.sanitized(sixteenCharacters).count == 15)
        #expect(AIInfluencerUsername.sanitized(sixteenCharacters) == "abcdefghijklmno")
    }

    @Test("empty and all-invalid input sanitize to empty")
    func emptyInputs() {
        #expect(AIInfluencerUsername.sanitized("") == "")
        #expect(AIInfluencerUsername.sanitized("___---") == "")
    }

    @Test("isValid requires 3-15 characters of letters/digits")
    func validityBounds() {
        // The server pads shorter bases with `bot` while settling; an
        // EDITOR's handle must clear min_length=3 to be accepted.
        #expect(AIInfluencerUsername.isValid("ab") == false)
        #expect(AIInfluencerUsername.isValid("abc"))
        #expect(AIInfluencerUsername.isValid("abcde12345abcde"))
        #expect(AIInfluencerUsername.isValid("abcde12345abcdef") == false)
    }

    @Test("isValid rejects characters the sanitizer would strip")
    func validityRejectsStrippables() {
        #expect(AIInfluencerUsername.isValid("my_bot") == false)
        #expect(AIInfluencerUsername.isValid("meera two") == false)
        #expect(AIInfluencerUsername.isValid("") == false)
        // Uppercase letters are inside the charset (the sanitizer is what
        // lowercases) — a valid handle only ever reaches this check already
        // sanitized, so this asserts the charset boundary, not casing.
        #expect(AIInfluencerUsername.isValid("Meera"))
    }

    @Test("sanitized output is always valid-or-empty")
    func sanitizedOutputIsAlwaysAcceptable() {
        // Guard against the two functions disagreeing: any non-empty
        // sanitizer result of sufficient length must pass isValid.
        for rawName in ["my_bot", "MEERA-2", "ab!!", "ab", "meera two", "___"] {
            let sanitized = AIInfluencerUsername.sanitized(rawName)
            if sanitized.count >= AIInfluencerUsername.minimumLength {
                #expect(AIInfluencerUsername.isValid(sanitized))
            }
            #expect(AIInfluencerUsername.sanitized(sanitized) == sanitized)
        }
    }
}
