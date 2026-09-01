import Foundation

/// Decodes SpacetimeDB's positional JSON encoding into plain Swift values.
///
/// SpacetimeDB serializes Rust types without field names (SATS wire format):
///   - Struct  → `[field0, field1, …]` (index = declaration order)
///   - Enum    → `[tag, payloadArray]` (tag = 0-based variant index)
///   - Option  → enum with Some=0, None=1: `[0, [v]]` / `[1, []]`
///   - Vec     → plain JSON array
/// No Swift SDK exists for iOS, so the app decodes REST responses by hand —
/// same as the Kotlin app's `SpacetimePositionalDecoder`.
///
/// Numbers parse via NSNumber's integer accessors (u64 precision preserved —
/// never routed through Double, which silently corrupts values above 2^53).
public enum SpacetimePositionalDecoder {

    /// Parses a response body into a positional array (throws when the body
    /// is not a top-level JSON array).
    public static func parseArray(_ body: String) throws -> [Any] {
        let data = Data(body.utf8)
        let object = try JSONSerialization.jsonObject(with: data, options: [])
        guard let array = object as? [Any] else {
            throw SpacetimeDecodingError.responseBodyNotArray(actual: body)
        }
        return array
    }

    // MARK: - Field access

    public static func element(_ array: [Any], at index: Int) throws -> Any {
        guard array.indices.contains(index) else {
            throw SpacetimeDecodingError.indexOutOfBounds(index: index, count: array.count)
        }
        return array[index]
    }

    // MARK: - Primitive decoders

    public static func string(_ positional: [Any], at index: Int) throws -> String {
        guard let value = try element(positional, at: index) as? String else {
            throw SpacetimeDecodingError.typeMismatch(expected: "string", index: index)
        }
        return value
    }

    public static func boolean(_ positional: [Any], at index: Int) throws -> Bool {
        let value = try element(positional, at: index)
        if let bool = value as? Bool { return bool }
        if let number = value as? NSNumber, number.isBoolean { return number.boolValue }
        throw SpacetimeDecodingError.typeMismatch(expected: "bool", index: index)
    }

    /// u64 field — parses via NSNumber.uint64Value (no Double round-trip).
    public static func unsigned64(_ positional: [Any], at index: Int) throws -> UInt64 {
        guard let number = try element(positional, at: index) as? NSNumber else {
            throw SpacetimeDecodingError.typeMismatch(expected: "u64", index: index)
        }
        return number.uint64Value
    }

    /// u32 field.
    public static func unsigned32(_ positional: [Any], at index: Int) throws -> UInt32 {
        guard let number = try element(positional, at: index) as? NSNumber else {
            throw SpacetimeDecodingError.typeMismatch(expected: "u32", index: index)
        }
        return number.uint32Value
    }

    /// i64 field (SpacetimeDB Timestamps arrive as `[micros]` wrappers).
    public static func long(_ positional: [Any], at index: Int) throws -> Int64 {
        guard let number = try element(positional, at: index) as? NSNumber else {
            throw SpacetimeDecodingError.typeMismatch(expected: "i64", index: index)
        }
        return number.int64Value
    }

    /// Nested positional array field.
    public static func array(_ positional: [Any], at index: Int) throws -> [Any] {
        guard let value = try element(positional, at: index) as? [Any] else {
            throw SpacetimeDecodingError.typeMismatch(expected: "array", index: index)
        }
        return value
    }

    /// Vec<String> field.
    public static func stringVector(_ positional: [Any], at index: Int) throws -> [String] {
        try array(positional, at: index).compactMap { $0 as? String }
    }

    // MARK: - Sum types

    /// Decodes `[tag, payloadArray]`.
    public static func sumVariant(_ array: [Any]) throws -> (tag: Int, payload: [Any]) {
        guard array.count == 2,
              let tag = (array[0] as? NSNumber)?.intValue,
              let payload = array[1] as? [Any]
        else {
            throw SpacetimeDecodingError.malformedSumVariant
        }
        return (tag, payload)
    }

    /// `Option<T>` response: `[0, [value]]` → payload, `[1, []]` → nil.
    public static func optionPayload(_ array: [Any]) throws -> [Any]? {
        let variant = try sumVariant(array)
        return variant.tag == 0 ? variant.payload : nil
    }

    /// `Option<String>` field.
    public static func optionString(_ positional: [Any], at index: Int) throws -> String? {
        let payload = try optionPayload(try array(positional, at: index))
        return (payload?.first as? String)
    }

    /// `Option<Bool>` field.
    public static func optionBool(_ positional: [Any], at index: Int) throws -> Bool? {
        let payload = try optionPayload(try array(positional, at: index))
        if let bool = payload?.first as? Bool { return bool }
        if let number = payload?.first as? NSNumber, number.isBoolean { return number.boolValue }
        return nil
    }
}

/// Typed decoding failures.
public enum SpacetimeDecodingError: Error, Equatable {
    case responseBodyNotArray(actual: String)
    case indexOutOfBounds(index: Int, count: Int)
    case typeMismatch(expected: String, index: Int)
    case malformedSumVariant
    case unknownVariantTag(type: String, tag: Int)
}

extension NSNumber {
    /// Distinguishes JSON booleans from numbers — JSONSerialization hands
    /// back NSNumber for both; objCType "c" is the boolean marker on Apple
    /// platforms.
    var isBoolean: Bool {
        objCType[0] == UInt8(0x63) /* 'c' */
    }
}
