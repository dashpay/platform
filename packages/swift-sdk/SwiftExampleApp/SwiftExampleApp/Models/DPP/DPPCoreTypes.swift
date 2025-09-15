import Foundation

// MARK: - Core Types based on DPP

/// 32-byte identifier used throughout the platform
public typealias Identifier = Data

/// Revision number for versioning
public typealias Revision = UInt64

/// Timestamp in milliseconds since Unix epoch
public typealias TimestampMillis = UInt64

/// Credits amount
public typealias Credits = UInt64

/// Key ID for identity public keys
public typealias KeyID = UInt32

/// Key count
typealias KeyCount = KeyID

/// Block height on the platform chain
public typealias BlockHeight = UInt64

/// Block height on the core chain
public typealias CoreBlockHeight = UInt32

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
    /// Create an Identifier from a hex string
    static func identifier(fromHex hexString: String) -> Identifier? {
        return Data(hexString: hexString)
    }
    
    /// Create an Identifier from a base58 string
    static func identifier(fromBase58 base58String: String) -> Identifier? {
        let alphabet = Array("123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz")
        let base = alphabet.count
        
        var bytes = [UInt8]()
        var num = [UInt8](repeating: 0, count: 1)
        
        for char in base58String {
            guard let index = alphabet.firstIndex(of: char) else {
                return nil
            }
            
            // Multiply num by base
            var carry = 0
            for i in 0..<num.count {
                carry = Int(num[i]) * base + carry
                num[i] = UInt8(carry % 256)
                carry /= 256
            }
            while carry > 0 {
                num.append(UInt8(carry % 256))
                carry /= 256
            }
            
            // Add index
            carry = index
            for i in 0..<num.count {
                carry = Int(num[i]) + carry
                num[i] = UInt8(carry % 256)
                carry /= 256
            }
            while carry > 0 {
                num.append(UInt8(carry % 256))
                carry /= 256
            }
        }
        
        // Handle leading zeros (1s in base58)
        for char in base58String {
            if char == "1" {
                bytes.append(0)
            } else {
                break
            }
        }
        
        // Append the rest in reverse order
        bytes.append(contentsOf: num.reversed())
        
        return Data(bytes)
    }
    
    /// Convert to base58 string
    func toBase58String() -> String {
        let alphabet = Array("123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz")
        
        if self.isEmpty {
            return ""
        }
        
        var bytes = Array(self)
        var encoded = ""
        
        // Count leading zero bytes
        let zeroCount = bytes.prefix(while: { $0 == 0 }).count
        
        // Skip leading zeros for conversion
        bytes = Array(bytes.dropFirst(zeroCount))
        
        if bytes.isEmpty {
            return String(repeating: "1", count: zeroCount)
        }
        
        // Convert bytes to base58
        while !bytes.isEmpty && !bytes.allSatisfy({ $0 == 0 }) {
            var remainder = 0
            var newBytes = [UInt8]()
            
            for byte in bytes {
                let temp = remainder * 256 + Int(byte)
                remainder = temp % 58
                let quotient = temp / 58
                if !newBytes.isEmpty || quotient > 0 {
                    newBytes.append(UInt8(quotient))
                }
            }
            
            bytes = newBytes
            encoded = String(alphabet[remainder]) + encoded
        }
        
        // Add '1' for each leading zero byte
        encoded = String(repeating: "1", count: zeroCount) + encoded
        
        return encoded
    }
    
    /// Convert to hex string
    func toHexString() -> String {
        return self.map { String(format: "%02x", $0) }.joined()
    }
    
    /// Initialize Data from hex string
    init?(hexString: String) {
        let hex = hexString.trimmingCharacters(in: .whitespacesAndNewlines)
        guard hex.count % 2 == 0 else { return nil }
        
        var data = Data()
        var index = hex.startIndex
        
        while index < hex.endIndex {
            let nextIndex = hex.index(index, offsetBy: 2)
            let byteString = hex[index..<nextIndex]
            
            if let byte = UInt8(byteString, radix: 16) {
                data.append(byte)
            } else {
                return nil
            }
            
            index = nextIndex
        }
        
        self = data
    }
}

// MARK: - Platform Value Type
/// Represents a value that can be stored in documents or contracts
public enum PlatformValue: Codable, Equatable {
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

// In Swift 6 strict concurrency, PlatformValue is frequently passed across
// actor boundaries as part of dictionaries. Its cases contain only Sendable
// payloads (Bool, integers, Double, String, Data, arrays/maps of PlatformValue).
// We mark it as @unchecked Sendable to permit usage in cross-actor contexts.
extension PlatformValue: @unchecked Sendable {}
