import Foundation

// MARK: - Core DPP Type Aliases
// Note: Some types (KeyID, BinaryData, TimestampMillis, Identifier) are in IdentityTypes.swift

/// Revision number for versioning
public typealias Revision = UInt64

/// Credits amount
public typealias Credits = UInt64

/// Block height on the platform chain
public typealias BlockHeight = UInt64

/// Block height on the core chain
public typealias CoreBlockHeight = UInt32

/// Key count
public typealias KeyCount = KeyID

/// Epoch index
public typealias EpochIndex = UInt16

/// 32-byte hash
public typealias Bytes32 = Data

/// Document name/type within a data contract
public typealias DocumentName = String

/// Definition name for schema definitions
public typealias DefinitionName = String

/// Group contract position
public typealias GroupContractPosition = UInt16

/// Token contract position
public typealias TokenContractPosition = UInt16

// MARK: - Platform Value Type

/// Represents a value that can be stored in documents or contracts
public enum PlatformValue: Codable, Equatable, Sendable {
    case null
    case bool(Bool)
    case integer(Int64)
    case unsignedInteger(UInt64)
    case float(Double)
    case string(String)
    case bytes(Data)
    case array([PlatformValue])
    case map([String: PlatformValue])

    // MARK: - Codable

    enum CodingKeys: String, CodingKey {
        case type, value
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)

        switch type {
        case "null":
            self = .null
        case "bool":
            self = .bool(try container.decode(Bool.self, forKey: .value))
        case "integer":
            self = .integer(try container.decode(Int64.self, forKey: .value))
        case "unsignedInteger":
            self = .unsignedInteger(try container.decode(UInt64.self, forKey: .value))
        case "float":
            self = .float(try container.decode(Double.self, forKey: .value))
        case "string":
            self = .string(try container.decode(String.self, forKey: .value))
        case "bytes":
            self = .bytes(try container.decode(Data.self, forKey: .value))
        case "array":
            self = .array(try container.decode([PlatformValue].self, forKey: .value))
        case "map":
            let mapValue: [Swift.String: PlatformValue] = try container.decode([Swift.String: PlatformValue].self, forKey: .value)
            self = .map(mapValue)
        default:
            throw DecodingError.dataCorruptedError(forKey: .type, in: container, debugDescription: "Unknown type: \(type)")
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)

        switch self {
        case .null:
            try container.encode("null", forKey: .type)
        case .bool(let value):
            try container.encode("bool", forKey: .type)
            try container.encode(value, forKey: .value)
        case .integer(let value):
            try container.encode("integer", forKey: .type)
            try container.encode(value, forKey: .value)
        case .unsignedInteger(let value):
            try container.encode("unsignedInteger", forKey: .type)
            try container.encode(value, forKey: .value)
        case .float(let value):
            try container.encode("float", forKey: .type)
            try container.encode(value, forKey: .value)
        case .string(let value):
            try container.encode("string", forKey: .type)
            try container.encode(value, forKey: .value)
        case .bytes(let value):
            try container.encode("bytes", forKey: .type)
            try container.encode(value, forKey: .value)
        case .array(let value):
            try container.encode("array", forKey: .type)
            try container.encode(value, forKey: .value)
        case .map(let value):
            try container.encode("map", forKey: .type)
            try container.encode(value, forKey: .value)
        }
    }

    // MARK: - Convenience Initializers

    /// Create from Any value (for JSON parsing)
    public init?(from anyValue: Any) {
        if anyValue is NSNull {
            self = .null
        } else if let bool = anyValue as? Bool {
            self = .bool(bool)
        } else if let int = anyValue as? Int64 {
            self = .integer(int)
        } else if let int = anyValue as? Int {
            self = .integer(Int64(int))
        } else if let uint = anyValue as? UInt64 {
            self = .unsignedInteger(uint)
        } else if let double = anyValue as? Double {
            self = .float(double)
        } else if let string = anyValue as? String {
            self = .string(string)
        } else if let data = anyValue as? Data {
            self = .bytes(data)
        } else if let array = anyValue as? [Any] {
            let converted = array.compactMap { PlatformValue(from: $0) }
            self = .array(converted)
        } else if let dict = anyValue as? [String: Any] {
            var converted: [String: PlatformValue] = [:]
            for (key, value) in dict {
                if let pv = PlatformValue(from: value) {
                    converted[key] = pv
                }
            }
            self = .map(converted)
        } else {
            return nil
        }
    }

    /// Convert to Any value
    public func toAny() -> Any {
        switch self {
        case .null:
            return NSNull()
        case .bool(let value):
            return value
        case .integer(let value):
            return value
        case .unsignedInteger(let value):
            return value
        case .float(let value):
            return value
        case .string(let value):
            return value
        case .bytes(let value):
            return value
        case .array(let value):
            return value.map { $0.toAny() }
        case .map(let value):
            return value.mapValues { $0.toAny() }
        }
    }
}

// MARK: - Data Extensions

extension Data {
    /// Pad or truncate data to specified length
    public func paddedToLength(_ length: Int) -> Data {
        if self.count >= length {
            return self.prefix(length)
        } else {
            var padded = self
            padded.append(Data(repeating: 0, count: length - self.count))
            return padded
        }
    }

    /// Create an Identifier from a hex string
    public static func identifier(fromHex hexString: String) -> Identifier? {
        return Data(hexString: hexString)
    }
}
