import Foundation

/// Positional-argument encoding for the SpacetimeDB REST API — extracted
/// from SpacetimeDBRemoteDataSource (root-level: cross-feature wire infra).
///
/// Wire rules (SpacetimeDB's serde bridge, verified against
/// spacetimedb's de/serde.rs):
///   - plain `null` encodes a `None` argument;
///   - `Some(v)` encodes as `[0, payload]` (ARG form — the payload is
///     INLINED, unlike the RESPONSE form `[0, [v]]`);
///   - structs encode as positional arrays `[field, field, …]`.
/// These rules are pinned by SpacetimeDBRemoteDataSourceTests.
public enum SpacetimeArgument {
    case string(String)
    case unsignedInteger(UInt64)
    case boolean(Bool)
    case jsonAny(SpacetimeArgumentJSON)

    /// Encoded JSON form.
    var encodingDescription: String {
        switch self {
        case let .string(value): return "\"\(value)\""
        case let .unsignedInteger(value): return "\(value)"
        case let .boolean(value): return value ? "true" : "false"
        case let .jsonAny(value): return value.encodingDescription
        }
    }
}

/// JSON fragment for nested args (Vec args, sum-type arg encodings).
public enum SpacetimeArgumentJSON: Sendable {
    case string(String)
    case number(String)
    case bool(Bool)
    case null
    indirect case array([SpacetimeArgumentJSON])

    var encodingDescription: String {
        switch self {
        case let .string(value): return "\"\(value)\""
        case let .number(value): return value
        case let .bool(value): return value ? "true" : "false"
        case .null: return "null"
        case let .array(elements):
            return "[\(elements.map(\.encodingDescription).joined(separator: ","))]"
        }
    }
}

/// Builds the request body: a JSON array of positional arguments
/// (compact — matches Kotlin's kotlinx serialization output).
/// Internal (not private) so the wire tests can pin exact bodies
/// against the live reducer signatures.
func encodeSpacetimeArguments(_ arguments: [SpacetimeArgument]) -> String {
    "[\(arguments.map(\.encodingDescription).joined(separator: ","))]"
}

/// Pure argument construction for `update_profile_details` (tested —
/// see SpacetimeDBRemoteDataSourceTests). Live signature (4 params, per
/// the generated bindings in apps/yral-database-spacetime):
///   (bio: Option<String>, website_url: Option<String>,
///    profile_picture: Option<ProfilePictureData>,
///    update_as_ai_account_id: Option<String>)
/// `ProfilePictureData` is a STRUCT {url, nsfw_info} — `nsfw_info` is
/// `[is_nsfw, nsfw_ec, nsfw_gore, csam_detected]`, all-false here
/// (moderation happens server-side via update_profile_picture_nsfw_info).
func updateProfileDetailsArguments(
    bio: String?,
    websiteURL: String?,
    profilePictureURL: String?,
    updateAsAIAccountID: String?
) -> [SpacetimeArgument] {
    let profilePicture: SpacetimeArgumentJSON
    if let profilePictureURL {
        profilePicture = .array([
            .number("0"),
            .array([
                .string(profilePictureURL),
                .array([.bool(false), .string(""), .string(""), .bool(false)])
            ])
        ])
    } else {
        profilePicture = .null
    }
    return [
        bio.map { .string($0) } ?? .jsonAny(.null),
        websiteURL.map { .string($0) } ?? .jsonAny(.null),
        .jsonAny(profilePicture),
        updateAsAIAccountID.map {
            .jsonAny(.array([.number("0"), .string($0)]))
        } ?? .jsonAny(.null)
    ]
}

/// Pure argument construction for `accept_new_user_registration`
/// (tested — see SpacetimeDBRemoteDataSourceTests).
func acceptNewUserRegistrationArguments(
    newPrincipalText: String,
    authenticated: Bool,
    mainAccountText: String?
) -> [SpacetimeArgument] {
    let mainAccount: SpacetimeArgumentJSON
    if let mainAccountText {
        mainAccount = .array([.number("0"), .string(mainAccountText)])
    } else {
        mainAccount = .array([.number("1"), .array([])])
    }
    return [.string(newPrincipalText), .boolean(authenticated), .jsonAny(mainAccount)]
}
