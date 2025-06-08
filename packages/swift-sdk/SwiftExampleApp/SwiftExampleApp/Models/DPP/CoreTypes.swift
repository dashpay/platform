import Foundation

// MARK: - Core Types based on DPP

/// 32-byte identifier used throughout the platform
typealias Identifier = Data

/// Revision number for versioning
typealias Revision = UInt64

/// Timestamp in milliseconds since Unix epoch
typealias TimestampMillis = UInt64

/// Credits amount
typealias Credits = UInt64

/// Key ID for identity public keys
typealias KeyID = UInt32

/// Key count
typealias KeyCount = KeyID

/// Block height on the platform chain
typealias BlockHeight = UInt64

/// Block height on the core chain
typealias CoreBlockHeight = UInt32

/// Epoch index
typealias EpochIndex = UInt16

/// Binary data
typealias BinaryData = Data

/// 32-byte hash
typealias Bytes32 = Data

/// Document name/type within a data contract
typealias DocumentName = String

/// Definition name for schema definitions
typealias DefinitionName = String

/// Group contract position
typealias GroupContractPosition = UInt16

/// Token contract position
typealias TokenContractPosition = UInt16

// MARK: - Helper Extensions

extension Data {
    /// Create an Identifier from a base58 string
    static func identifier(from base58String: String) -> Identifier? {
        // In a real implementation, this would decode base58
        // For now, return sample data
        return Data(repeating: 0, count: 32)
    }
    
    /// Convert to base58 string
    func toBase58String() -> String {
        // In a real implementation, this would encode to base58
        return self.base64EncodedString()
    }
    
    /// Convert to hex string
    func toHexString() -> String {
        return self.map { String(format: "%02x", $0) }.joined()
    }
}

// MARK: - Platform Value Type
/// Represents a value that can be stored in documents or contracts
enum PlatformValue: Codable, Equatable {
    case null
    case bool(Bool)
    case integer(Int64)
    case unsignedInteger(UInt64)
    case float(Double)
    case string(String)
    case bytes(Data)
    case array([PlatformValue])
    case map([String: PlatformValue])
    
    // Coding implementation would go here
}