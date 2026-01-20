import Foundation
import DashSDKFFI

/// Information about a Platform address including its nonce and balance
/// Note: This is distinct from KeyWallet's AddressInfo which contains local wallet address info
public struct PlatformAddressInfo: Sendable, Equatable, Codable {
    /// Address bytes (21 bytes: 1 byte type + 20 bytes hash)
    public let addressBytes: Data
    
    /// Nonce associated with the address
    public let nonce: UInt32
    
    /// Balance in credits
    public let balance: UInt64
    
    /// Whether the address was found on Platform
    public var isFound: Bool {
        return nonce != UInt32.max && balance != UInt64.max
    }
    
    /// Initialize from address bytes, nonce, and balance
    public init(addressBytes: Data, nonce: UInt32, balance: UInt64) {
        self.addressBytes = addressBytes
        self.nonce = nonce
        self.balance = balance
    }
    
    /// Create a PlatformAddressInfo from FFI DashSDKAddressInfo
    internal init(from ffi: DashSDKAddressInfo) {
        if ffi.address != nil && ffi.address_len > 0 {
          self.addressBytes = Data(bytes: ffi.address!, count: Int(ffi.address_len))
        } else {
            self.addressBytes = Data()
        }
        self.nonce = ffi.nonce
        self.balance = ffi.balance
    }
    
    /// Convert address bytes to hex string
    public var addressHex: String {
        return addressBytes.map { String(format: "%02x", $0) }.joined()
    }
    
    /// Convert address bytes to bech32m string (requires network parameter)
    /// Format: [type_byte][20_byte_hash]
    public func toBech32m(network: DashSDKNetwork) -> String? {
        guard addressBytes.count == 21 else { return nil }
        
        // Get HRP based on network
        // DashSDKNetwork raw values: 0 = mainnet, 1 = testnet, 2 = regtest, 3 = devnet, 4 = local
        let hrp: String
        if network.rawValue == 0 {
            hrp = "dashevo"  // mainnet
        } else {
            hrp = "tdashevo"  // testnet, devnet, regtest, local
        }
        
        // Use bech32m encoding
        return Bech32m.encode(hrp: hrp, data: addressBytes)
    }
}

/// Result for fetching multiple Platform address infos
public struct PlatformAddressInfosResult: Sendable {
    /// Dictionary mapping address bytes to their info
    public let infos: [Data: PlatformAddressInfo]
    
    /// Get info for a specific address
    public func info(for addressBytes: Data) -> PlatformAddressInfo? {
        return infos[addressBytes]
    }
    
    /// Get all found addresses (with valid balance/nonce)
    public var foundAddresses: [PlatformAddressInfo] {
        return infos.values.filter { $0.isFound }
    }
    
    /// Get all not-found addresses
    public var notFoundAddresses: [PlatformAddressInfo] {
        return infos.values.filter { !$0.isFound }
    }
    
    /// Total balance across all found addresses
    public var totalBalance: UInt64 {
        return foundAddresses.reduce(0) { $0 + $1.balance }
    }
}

// MARK: - Trunk State Types

/// Element in trunk state - an address with balance/nonce found at trunk level
public struct TrunkStateElement: Sendable, Equatable {
    /// Address key bytes
    public let key: Data
    
    /// Nonce for the address
    public let nonce: UInt32
    
    /// Balance in credits
    public let balance: UInt64
    
    /// Convert key to hex string
    public var keyHex: String {
        return key.map { String(format: "%02x", $0) }.joined()
    }
}

/// Leaf boundary in trunk state - subtree that needs further branch queries
public struct LeafBoundary: Sendable, Equatable {
    /// Leaf key bytes
    public let key: Data
    
    /// Expected hash (32 bytes)
    public let hash: Data
    
    /// Estimated element count in this subtree (0 if unknown)
    public let estimatedCount: UInt64
    
    /// Convert key to hex string
    public var keyHex: String {
        return key.map { String(format: "%02x", $0) }.joined()
    }
    
    /// Convert hash to hex string
    public var hashHex: String {
        return hash.map { String(format: "%02x", $0) }.joined()
    }
}

/// Trunk state for address synchronization
/// Contains addresses found at top levels and leaf boundaries for subtrees needing further queries
public struct PlatformTrunkState: Sendable {
    /// Elements (addresses with balances) found at trunk level
    public let elements: [TrunkStateElement]
    
    /// Leaf boundaries (subtrees needing branch queries)
    public let leafBoundaries: [LeafBoundary]
    
    /// Checkpoint height for consistency
    public let checkpointHeight: UInt64
    
    /// Total balance across all elements
    public var totalBalance: UInt64 {
        return elements.reduce(0) { $0 + $1.balance }
    }
}

/// Branch state for address synchronization
/// Contains addresses found in a specific branch and deeper leaf boundaries
public struct PlatformBranchState: Sendable {
    /// Elements (addresses with balances) found in this branch
    public let elements: [TrunkStateElement]
    
    /// Leaf boundaries (deeper subtrees needing further queries)
    public let leafBoundaries: [LeafBoundary]
    
    /// Total balance across all elements in this branch
    public var totalBalance: UInt64 {
        return elements.reduce(0) { $0 + $1.balance }
    }
}

// MARK: - Bech32m Encoding/Decoding Helper

/// Bech32m encoding/decoding helper for Platform addresses
public enum Bech32m {
    private static let charset = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
    private static let charsetMap: [Character: UInt8] = {
        var map: [Character: UInt8] = [:]
        for (index, char) in charset.enumerated() {
            map[char] = UInt8(index)
        }
        return map
    }()
    
    /// Decode result containing HRP and data
    public struct DecodeResult {
        public let hrp: String
        public let data: Data
    }
    
    /// Decode a bech32m string to HRP and data bytes
    /// - Parameter bech32m: The bech32m encoded string (e.g., "tdashevo1qqyfsqyzcn5hzu7echru54njypdq0v4d7gv8pkdf")
    /// - Returns: DecodeResult with hrp and data, or nil if invalid
    public static func decode(_ bech32m: String) -> DecodeResult? {
        let lowercased = bech32m.lowercased()
        
        // Find the separator '1'
        guard let separatorIndex = lowercased.lastIndex(of: "1") else {
            return nil
        }
        
        let hrp = String(lowercased[..<separatorIndex])
        let dataPartStart = lowercased.index(after: separatorIndex)
        let dataPart = String(lowercased[dataPartStart...])
        
        // HRP must be 1-83 characters, data part must be at least 6 characters (checksum)
        guard hrp.count >= 1 && hrp.count <= 83 && dataPart.count >= 6 else {
            return nil
        }
        
        // Decode data part characters to 5-bit values
        var values: [UInt8] = []
        for char in dataPart {
            guard let value = charsetMap[char] else {
                return nil // Invalid character
            }
            values.append(value)
        }
        
        // Verify checksum
        guard verifyChecksum(hrp: hrp, values: values) else {
            return nil
        }
        
        // Remove checksum (last 6 values)
        let dataValues = Array(values.dropLast(6))
        
        // Convert from 5-bit to 8-bit
        guard let data = convertFrom5Bit(dataValues) else {
            return nil
        }
        
        return DecodeResult(hrp: hrp, data: Data(data))
    }
    
    /// Check if a string is a valid bech32m Platform address
    public static func isValidPlatformAddress(_ address: String) -> Bool {
        guard let result = decode(address) else {
            return false
        }
        // Valid Platform addresses have dashevo or tdashevo HRP and 21 bytes of data
        let validHrp = result.hrp == "dashevo" || result.hrp == "tdashevo"
        let validLength = result.data.count == 21
        return validHrp && validLength
    }
    
    /// Debug: Decode a bech32m address and return details for troubleshooting
    public static func debugDecode(_ address: String) -> (hrp: String?, byteCount: Int?, hex: String?, error: String?) {
        guard let result = decode(address) else {
            return (nil, nil, nil, "Failed to decode bech32m address")
        }
        let hex = result.data.map { String(format: "%02x", $0) }.joined()
        return (result.hrp, result.data.count, hex, nil)
    }
    
    /// Encode data to bech32m string
    public static func encode(hrp: String, data: Data) -> String? {
        let values = convertTo5Bit(Array(data))
        guard !values.isEmpty else { return nil }
        
        let checksum = createChecksum(hrp: hrp, values: values)
        let combined = values + checksum
        
        var result = hrp + "1"
        for value in combined {
            let index = charset.index(charset.startIndex, offsetBy: Int(value))
            result.append(charset[index])
        }
        
        return result
    }
    
    private static func convertTo5Bit(_ data: [UInt8]) -> [UInt8] {
        var result: [UInt8] = []
        var acc: UInt32 = 0
        var bits: UInt32 = 0
        
        for byte in data {
            acc = (acc << 8) | UInt32(byte)
            bits += 8
            while bits >= 5 {
                bits -= 5
                result.append(UInt8((acc >> bits) & 0x1f))
            }
        }
        
        if bits > 0 {
            result.append(UInt8((acc << (5 - bits)) & 0x1f))
        }
        
        return result
    }
    
    private static func convertFrom5Bit(_ data: [UInt8]) -> [UInt8]? {
        var result: [UInt8] = []
        var acc: UInt32 = 0
        var bits: UInt32 = 0
        
        for value in data {
            guard value < 32 else { return nil }
            acc = (acc << 5) | UInt32(value)
            bits += 5
            while bits >= 8 {
                bits -= 8
                result.append(UInt8((acc >> bits) & 0xff))
            }
        }
        
        // Check for invalid padding - remaining bits must be zero and less than 5
        if bits > 4 {
            return nil
        }
        // The remaining padding bits (if any) must all be zero
        let paddingMask = (UInt32(1) << bits) - 1
        if (acc & paddingMask) != 0 {
            return nil
        }
        
        return result
    }
    
    private static func polymod(_ values: [UInt8]) -> UInt32 {
        let generator: [UInt32] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3]
        var chk: UInt32 = 1
        
        for value in values {
            let top = chk >> 25
            chk = ((chk & 0x1ffffff) << 5) ^ UInt32(value)
            for i in 0..<5 {
                if (top >> i) & 1 != 0 {
                    chk ^= generator[i]
                }
            }
        }
        
        return chk
    }
    
    private static func hrpExpand(_ hrp: String) -> [UInt8] {
        var result: [UInt8] = []
        for char in hrp {
            result.append(UInt8(char.asciiValue! >> 5))
        }
        result.append(0)
        for char in hrp {
            result.append(UInt8(char.asciiValue! & 31))
        }
        return result
    }
    
    private static func verifyChecksum(hrp: String, values: [UInt8]) -> Bool {
        let expanded = hrpExpand(hrp) + values
        return polymod(expanded) == 0x2bc830a3 // bech32m constant
    }
    
    private static func createChecksum(hrp: String, values: [UInt8]) -> [UInt8] {
        let enc = hrpExpand(hrp) + values + [0, 0, 0, 0, 0, 0]
        let mod = polymod(enc) ^ 0x2bc830a3 // bech32m constant
        
        var result: [UInt8] = []
        for i in 0..<6 {
            result.append(UInt8((mod >> (5 * (5 - i))) & 31))
        }
        return result
    }
}
