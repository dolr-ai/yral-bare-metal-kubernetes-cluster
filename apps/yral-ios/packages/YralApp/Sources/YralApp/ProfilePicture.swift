import Foundation
import CryptoKit

/// Deterministic profile-picture URL from a principal — 1:1 port of Kotlin
/// `PropicUtils.propicFromPrincipal`. The GobGob avatar index is stable per
/// principal: CRC32 (IEEE 802.3) of the UTF-8 bytes, then the Kotlin
/// remainder quirk reproduced exactly (see `avatarIndex`).
///
/// NOTE on the sign trap (verified against the Kotlin source): Kotlin's `%`
/// on a signed `Int` is remainder-with-sign-of-dividend, and a raw CRC32 as
/// `Int32` is frequently negative — so the shipped production behavior can
/// produce index ≤ 0. We reproduce that deliberately: byte-identical URLs
/// with production avatars matter more than "fixing" the hash here.
enum ProfilePicture {

    /// Total GobGob avatar count (server asset pool).
    static let gobgobTotalCount: Int = 18_557

    /// Hetzner object-storage prefix for GobGob avatars.
    static let gobgobURLPrefix =
        "https://prakash-yral.hel1.your-objectstorage.com/gobgob/gob."

    /// `https://…/gobgob/gob.<index>.png` for the given principal.
    static func url(fromPrincipal principal: String) -> String {
        "\(gobgobURLPrefix)\(avatarIndex(principal)).png"
    }

    /// `(crc32 % 18557) + 1` — with Kotlin's signed-remainder semantics.
    static func avatarIndex(_ principal: String) -> Int {
        let hash = crc32IEEE(Data(principal.utf8))
        // Bit-pattern reinterpretation: UInt32 → signed Int32 (Kotlin Int).
        let signedHash = Int32(bitPattern: hash)
        // Swift `%` on Int32 matches Kotlin's remainder semantics exactly.
        return Int(signedHash % Int32(gobgobTotalCount)) + 1
    }

    /// CRC32 (IEEE 802.3 / zlib polynomial 0xEDB88320) — table-driven,
    /// identical to the Kotlin implementation.
    static func crc32IEEE(_ data: Data) -> UInt32 {
        var table = [UInt32](repeating: 0, count: 256)
        for tableIndex in 0..<256 {
            var crc = UInt32(tableIndex)
            for _ in 0..<8 {
                crc = (crc & 1 != 0) ? (crc >> 1) ^ 0xEDB8_8320 : crc >> 1
            }
            table[tableIndex] = crc
        }
        var crc: UInt32 = 0xFFFF_FFFF
        for byte in data {
            let index = UInt8((crc ^ UInt32(byte)) & 0xFF)
            crc = (crc >> 8) ^ table[Int(index)]
        }
        return crc ^ 0xFFFF_FFFF
    }
}

/// Deterministic display-name fallback — 1:1 port of Kotlin
/// `generateUsernameFromPrincipal` (UsernameUtils.kt). SHA-256(principal)
/// seeds a byte-stream PRNG that picks two distinct modifiers + one noun;
/// retries up to 128 times for a ≤ 15-char result, falling back to
/// "cutekindpanda". Used when no server username exists.
enum UsernameGenerator {

    /// Maximum allowed username length (USERNAME_MAX_LENGTH).
    static let maximumUsernameLength = 15

    /// Generation attempts before falling back (USERNAME_GENERATION_ATTEMPTS).
    static let generationAttempts = 128

    /// The shipped fallback when all attempts exceed the length limit.
    static let fallbackUsername = "cutekindpanda"

    /// Kotlin `resolveUsername`: preferred (trimmed, non-empty) wins, else
    /// the generated name; nil principal with no preferred → nil.
    static func resolveUsername(
        preferred: String?,
        principal: String?
    ) -> String? {
        if let preferred, !preferred.trimmingCharacters(in: .whitespaces).isEmpty {
            return preferred
        }
        return principal.map { username(fromPrincipal: $0) }
    }

    static func username(fromPrincipal principal: String) -> String {
        var generator = SeededGenerator(seed: SHA256.hash(data: Data(principal.utf8)))
        for _ in 0..<generationAttempts {
            let firstModifier = yralUsernameModifiers.randomOrDefault(
                generator: &generator, fallback: "cute"
            )
            let secondModifier = yralUsernameModifiers.randomDistinctOrDefault(
                generator: &generator,
                excluded: firstModifier,
                fallback: "kind"
            )
            let noun = yralUsernameNouns.randomOrDefault(
                generator: &generator, fallback: "panda"
            )
            let username = firstModifier + secondModifier + noun
            if username.count <= maximumUsernameLength {
                return username
            }
        }
        return fallbackUsername
    }

    /// Kotlin `SeededGenerator`: a SHA-256-state PRNG. The seed initializes a
    /// 32-byte state; each `nextInt` consumes an 8-byte big-endian chunk and
    /// rehashes the state with SHA-256 when exhausted. `next % bound` masks
    /// off the sign bit first (Kotlin `and Long.MAX_VALUE`), so draws are
    /// non-negative.
    struct SeededGenerator {

        private var state: [UInt8]
        private var index = 0

        init(seed: some Sequence<UInt8>) {
            let seedBytes = Array(seed)
            if seedBytes.count >= 32 {
                state = Array(seedBytes.prefix(32))
            } else {
                state = Array(repeating: 0, count: 32)
                state.replaceSubrange(
                    0..<seedBytes.count, with: seedBytes
                )
            }
        }

        mutating func nextInt(bound: Int) -> Int {
            precondition(bound > 0)
            let next = nextChunk() & Int64.max
            return Int(next % Int64(bound))
        }

        private mutating func nextChunk() -> Int64 {
            if index + 8 > state.count {
                // Rehash the state with SHA-256 (Kotlin sha256(state)).
                let digest = SHA256.hash(data: Data(state))
                state = Array(digest)
                index = 0
            }
            var value: Int64 = 0
            for offset in 0..<8 {
                value = (value << 8) | Int64(state[index + offset])
            }
            index += 8
            return value
        }
    }
}

private extension [String] {
    /// Kotlin `randomOrDefault`: index by PRNG, or the fallback if empty.
    func randomOrDefault(
        generator: inout UsernameGenerator.SeededGenerator,
        fallback: String
    ) -> String {
        isEmpty ? fallback : self[generator.nextInt(bound: count)]
    }

    /// Kotlin `randomDistinctOrDefault`: pick an index ≠ the excluded item's
    /// index by drawing in `0..count-2` and shifting ≥ the excluded index up
    /// by one.
    func randomDistinctOrDefault(
        generator: inout UsernameGenerator.SeededGenerator,
        excluded: String,
        fallback: String
    ) -> String {
        if isEmpty { return fallback }
        if count == 1 { return first! }
        guard let excludedIndex = firstIndex(of: excluded) else {
            return randomOrDefault(generator: &generator, fallback: fallback)
        }
        let randomIndex = generator.nextInt(bound: count - 1)
        let adjustedIndex = randomIndex >= excludedIndex ? randomIndex + 1 : randomIndex
        return self[adjustedIndex]
    }
}

/// Deterministic username word pools — 1:1 port of Kotlin
/// `UsernameWordLists.kt`. Order IS the algorithm: PRNG draws are indices
/// into these arrays, so a reorder changes every user's fallback name.
let yralUsernameModifiers: [String] = [
    "able", "ace", "airy", "alert", "amazing", "ample", "apt", "avid",
    "awake", "best", "bliss", "bold", "bonny", "brave", "breezy",
    "bright", "brisk", "calm", "chic", "civil", "classy", "clever",
    "cozy", "crisp", "cute", "cuddly", "dandy", "dapper", "dear",
    "deluxe", "eager", "easy", "elated", "elite", "fair", "fancy",
    "fine", "fit", "fond", "fresh", "fun", "funny", "gentle", "gifted",
    "glad", "gleam", "good", "goofy", "grand", "great", "groovy",
    "happy", "hardy", "hearty", "hip", "honest", "humble", "ideal",
    "jolly", "joyful", "keen", "kind", "lively", "loyal", "lucky",
    "lush", "magic", "merry", "mighty", "minty", "neat", "nifty",
    "nimble", "noble", "peachy", "perky", "plucky", "polite", "posh",
    "primo", "proud", "quick", "radiant", "ready", "regal", "rosy",
    "safe", "sassy", "serene", "sharp", "shiny", "silky", "silly",
    "smart", "snappy", "soft", "solid", "sparkly", "speedy", "spry",
    "stable", "stellar", "sunny", "super", "sweet", "swift", "tidy",
    "top", "trusty", "upbeat", "vivid", "warm", "witty", "zesty",
    "adorable", "agile", "angelic", "artful", "awesome", "balanced",
    "beaming", "beloved", "blessed", "bouncy", "bubbly", "careful",
    "chipper", "cosmic", "crafty", "curious", "darling", "dreamy",
    "dulcet", "elegant", "fabled", "famous", "festive", "fluffy",
    "friendly", "glossy", "golden", "graceful", "gracious", "handsome",
    "helpful", "honored", "hopeful", "jovial", "joyous", "lovely",
    "loving", "lucid", "luminous", "mellow", "mirthful", "modern",
    "musical", "natural", "orderly", "patient", "peaceful", "playful",
    "pleasant", "plush", "pretty", "prime", "pure", "sincere",
    "skilled", "sleek", "smiling", "smooth", "snug", "soulful",
    "spryest", "sunlit", "talented", "tender", "thankful",
    "thoughtful", "thriving", "tough", "tranquil", "true", "useful",
    "valiant", "vibrant", "winsome", "wise", "worthy", "young",
    "yummy", "zippy", "active", "affable", "cheery", "comfy", "fizzy",
    "glowing", "peppy"
]

let yralUsernameNouns: [String] = [
    "alpaca", "badger", "bat", "bear", "beaver", "bee", "bird",
    "bison", "bunny", "calf", "camel", "cat", "chick", "chipmunk",
    "corgi", "crab", "deer", "dog", "dolphin", "donkey", "duck",
    "eagle", "fawn", "finch", "fish", "foal", "fox", "frog", "gecko",
    "goat", "goose", "guppy", "hamster", "hare", "hedgehog", "hen",
    "heron", "horse", "joey", "kitten", "kiwi", "koala", "lamb",
    "lemur", "llama", "loon", "macaw", "marmot", "meerkat", "mole",
    "monkey", "moose", "mouse", "newt", "otter", "owl", "panda",
    "parrot", "penguin", "pony", "poodle", "puffin", "quail",
    "rabbit", "raven", "robin", "seal", "sheep", "skunk", "sloth",
    "snail", "sparrow", "swan", "tamarin", "tapir", "toucan", "turtle",
    "whale", "wombat", "yak", "zebra", "akita", "angelfish", "antelope",
    "budgie", "butterfly", "capybara", "caribou", "chinchilla",
    "cicada", "collie", "cow", "cricket", "cuckoo", "dingo", "dove",
    "dragonfly", "duckling", "egret", "elk", "falcon", "ferret",
    "flamingo", "gazelle", "gerbil", "gibbon", "giraffe", "goldfinch",
    "goldfish", "gosling", "guinea", "honeybee", "husky", "ibis",
    "jay", "jellyfish", "kestrel", "kingfisher", "ladybug", "loris",
    "lynx", "mallard", "manatee", "manta", "marmoset", "minnow",
    "moth", "ocelot", "opossum", "oriole", "osprey", "parakeet",
    "peacock", "pelican", "pika", "platypus", "puffbird", "quokka",
    "reindeer", "seahorse", "shiba", "shrew", "starling", "stoat",
    "sugarbird", "sunbird", "swift", "tarsier", "wallaby", "weasel"
]
