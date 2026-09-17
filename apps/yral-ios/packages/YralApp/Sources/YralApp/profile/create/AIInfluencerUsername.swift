import Foundation

/// The influencer handle alphabet — one source of truth for the client,
/// mirroring the server's settled alphabet (Rishi's `_settle_unused_slug`
/// in `app/routes/influencers.py`).
///
/// Why mirror it at all: `validate-and-generate-metadata` settles a free
/// handle server-side and returns it as `name`, then `/influencers/create`
/// re-checks uniqueness on the SAME string. If the client re-normalised
/// differently (it used to allow `_` and applied no length cap), the name
/// validated ≠ the name created — the drift Rishi fixed for Kotlin in
/// dolr-ai/yral-rishi-agent#522. Matching here keeps "what we send" equal
/// to "what validate checked".
enum AIInfluencerUsername {

  /// Kotlin `MAX_USERNAME_LENGTH` — also the server's settle cap.
  static let maximumLength = 15

  /// `CreateInfluencerRequest.name` requires `min_length=3`; the server
  /// pads shorter bases with `bot` during settling, so an edited handle
  /// must clear this for `/create` to accept it.
  static let minimumLength = 3

  /// Strips everything outside `[a-z0-9]` and caps at `maximumLength`.
  /// Pure — the form's live field sanitizer and its tests share it.
  static func sanitized(_ rawName: String) -> String {
    let lowercaseASCIIAlphanumerics = rawName.lowercased().filter { character in
      character.isASCII && (character.isLetter || character.isNumber)
    }
    return String(lowercaseASCIIAlphanumerics.prefix(maximumLength))
  }

  /// Whether `/influencers/create` will accept the handle as-is.
  static func isValid(_ name: String) -> Bool {
    name.count >= minimumLength && name.count <= maximumLength
      && name.allSatisfy { $0.isASCII && ($0.isLetter || $0.isNumber) }
  }
}
