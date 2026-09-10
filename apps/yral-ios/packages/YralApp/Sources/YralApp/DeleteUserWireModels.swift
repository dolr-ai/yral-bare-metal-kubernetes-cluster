// Wire models for `delete_user` — the module's mutation entry point
// (the old `delete_user_info` reducer is removed).

/// Positional arguments. The id_token is forwarded as the agent-service
/// bearer (the module's parsed-claims API cannot reconstruct the signed
/// token).
struct DeleteUserArguments: Encodable {
    let subjectToDelete: String
    let idToken: String

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(subjectToDelete)
        try container.encode(idToken)
    }
}

/// `DeleteUserResult` positional shape: [error, deleted_subjects,
/// backend_deletions] — module struct-declaration field order.
public struct DeleteUserResult: Equatable, Sendable {
    /// The cascade's error ("" on success) — the module's own words.
    public let error: String
    public let deletedSubjects: [String]
    public let backendDeletions: [BackendDeletionStatus]

    static func fromPositionalResponse(_ array: [Any]) throws -> DeleteUserResult {
        let backendArrays = try SpacetimePositionalDecoder.array(array, at: 2)
        let backendDeletions = try backendArrays.map { entry in
            guard let entry = entry as? [Any] else {
                throw SpacetimeDecodingError.typeMismatch(
                    expected: "backend deletion status", index: 2
                )
            }
            return try BackendDeletionStatus.fromPositionalResponse(entry)
        }
        return DeleteUserResult(
            error: try SpacetimePositionalDecoder.string(array, at: 0),
            deletedSubjects: try SpacetimePositionalDecoder.stringVector(array, at: 1),
            backendDeletions: backendDeletions
        )
    }
}

/// One bot's agent-service soft-delete outcome — upstream status and
/// body surfaced verbatim.
public struct BackendDeletionStatus: Equatable, Sendable {
    public let botSubject: String
    public let succeeded: Bool
    public let httpStatus: Int
    /// The agent's response body (or transport error text).
    public let error: String

    static func fromPositionalResponse(_ array: [Any]) throws -> BackendDeletionStatus {
        BackendDeletionStatus(
            botSubject: try SpacetimePositionalDecoder.string(array, at: 0),
            succeeded: try SpacetimePositionalDecoder.boolean(array, at: 1),
            httpStatus: Int(try SpacetimePositionalDecoder.unsigned32(array, at: 2)),
            error: try SpacetimePositionalDecoder.string(array, at: 3)
        )
    }
}
