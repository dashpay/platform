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
    public func toBech32m(network: Network) -> String? {
        guard let payload = Bech32m.bech32mPayload(fromStorageBytes: addressBytes) else { return nil }
        let hrp = Bech32m.platformHrp(mainnet: network == .mainnet)
        return Bech32m.encode(hrp: hrp, data: payload)
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

// MARK: - Recent Balance Changes Types

/// Credit operation type
public enum CreditOperationType: Sendable, Equatable, Codable {
    case setCredits(credits: UInt64)
    case addToCredits(credits: UInt64)

    public var credits: UInt64 {
        switch self {
        case .setCredits(let credits), .addToCredits(let credits):
            return credits
        }
    }
}

/// A single balance change for an address
public struct AddressBalanceChange: Sendable, Equatable {
    /// Address bytes
    public let addressBytes: Data

    /// Credit operation type and amount
    public let operation: CreditOperationType

    /// Convert address bytes to hex string
    public var addressHex: String {
        return addressBytes.map { String(format: "%02x", $0) }.joined()
    }

    /// Convert address bytes to bech32m string
    public func toBech32m(network: Network) -> String? {
        guard let payload = Bech32m.bech32mPayload(fromStorageBytes: addressBytes) else { return nil }
        let hrp = Bech32m.platformHrp(mainnet: network == .mainnet)
        return Bech32m.encode(hrp: hrp, data: payload)
    }
}

/// Balance changes for a single block
public struct BlockBalanceChanges: Sendable, Equatable {
    /// Block height
    public let blockHeight: UInt64

    /// Balance changes in this block
    public let changes: [AddressBalanceChange]
}

/// Recent balance changes across multiple blocks
public struct RecentBalanceChanges: Sendable {
    /// Block-by-block balance changes
    public let blocks: [BlockBalanceChanges]

    /// Total number of balance changes across all blocks
    public var totalChangesCount: Int {
        return blocks.reduce(0) { $0 + $1.changes.count }
    }

    /// Height range (if any blocks present)
    public var heightRange: ClosedRange<UInt64>? {
        guard let first = blocks.first, let last = blocks.last else { return nil }
        return first.blockHeight...last.blockHeight
    }
}

// MARK: - Compacted Balance Changes Types

/// Block-aware credit operation - can be a final set value or individual adds by block height
public enum BlockAwareCreditOperation: Sendable, Equatable {
    /// Credits were set to a final value (overwrites previous)
    case setCredits(credits: UInt64)
    /// Individual add-to-credits operations with their block heights (preserved for partial sync)
    case addToCreditsOperations(entries: [(blockHeight: UInt64, credits: UInt64)])

    /// Get the total credits (final value for setCredits, sum of adds for addToCreditsOperations)
    public var totalCredits: UInt64 {
        switch self {
        case .setCredits(let credits):
            return credits
        case .addToCreditsOperations(let entries):
            return entries.reduce(0) { $0 + $1.credits }
        }
    }

    public static func == (lhs: BlockAwareCreditOperation, rhs: BlockAwareCreditOperation) -> Bool {
        switch (lhs, rhs) {
        case (.setCredits(let c1), .setCredits(let c2)):
            return c1 == c2
        case (.addToCreditsOperations(let e1), .addToCreditsOperations(let e2)):
            guard e1.count == e2.count else { return false }
            for (entry1, entry2) in zip(e1, e2) {
                if entry1.blockHeight != entry2.blockHeight || entry1.credits != entry2.credits {
                    return false
                }
            }
            return true
        default:
            return false
        }
    }
}

/// A compacted balance change for an address
public struct CompactedAddressChange: Sendable, Equatable {
    /// Address bytes
    public let addressBytes: Data

    /// Block-aware operation (SetCredits or AddToCreditsOperations)
    public let operation: BlockAwareCreditOperation

    /// Convert address bytes to hex string
    public var addressHex: String {
        return addressBytes.map { String(format: "%02x", $0) }.joined()
    }

    /// Convert address bytes to bech32m string
    public func toBech32m(network: Network) -> String? {
        guard let payload = Bech32m.bech32mPayload(fromStorageBytes: addressBytes) else { return nil }
        let hrp = Bech32m.platformHrp(mainnet: network == .mainnet)
        return Bech32m.encode(hrp: hrp, data: payload)
    }
}

/// Compacted balance changes for a range of blocks
public struct CompactedBlockRange: Sendable, Equatable {
    /// Start block height of the range
    public let startBlockHeight: UInt64

    /// End block height of the range
    public let endBlockHeight: UInt64

    /// Balance changes in this range
    public let changes: [CompactedAddressChange]

    /// Height range
    public var heightRange: ClosedRange<UInt64> {
        return startBlockHeight...endBlockHeight
    }
}

/// Recent compacted balance changes across multiple ranges
public struct CompactedBalanceChanges: Sendable {
    /// Compacted block ranges
    public let ranges: [CompactedBlockRange]

    /// Total number of address changes across all ranges
    public var totalChangesCount: Int {
        return ranges.reduce(0) { $0 + $1.changes.count }
    }

    /// Overall height range (if any ranges present)
    public var heightRange: ClosedRange<UInt64>? {
        guard let first = ranges.first, let last = ranges.last else { return nil }
        return first.startBlockHeight...last.endBlockHeight
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

    /// DIP-0018 human-readable part for mainnet Platform addresses.
    public static let platformHrpMainnet = "dash"
    /// DIP-0018 human-readable part for testnet/devnet/regtest Platform addresses.
    public static let platformHrpTestnet = "tdash"

    /// HRP for the given network per DIP-0018 (mainnet = "dash", all others = "tdash").
    public static func platformHrp(mainnet: Bool) -> String {
        mainnet ? platformHrpMainnet : platformHrpTestnet
    }

    /// Whether a string looks like a bech32m Platform address (starts with the
    /// mainnet or testnet HRP + separator). Detection only — `decode` still
    /// validates the checksum, HRP, and 21-byte length. A 42-char hex address
    /// can never match, since neither HRP prefix is valid hex.
    public static func looksLikePlatformAddress(_ address: String) -> Bool {
        let lower = address.lowercased()
        return lower.hasPrefix(platformHrpMainnet + "1")
            || lower.hasPrefix(platformHrpTestnet + "1")
    }

    // Platform addresses have two distinct 21-byte serializations that differ
    // only in the leading type byte (per DIP-0018, mirrored in rs-dpp
    // `PlatformAddress`):
    //   • bech32m payload  — user-facing: 0xb0 = P2PKH, 0x80 = P2SH
    //   • storage / FFI    — bincode `PlatformAddress`: 0x00 = P2PKH, 0x01 = P2SH
    // The FFI (`PlatformAddress::from_bytes`) consumes and (`address_fetch_info`)
    // returns the storage form; only bech32m strings carry the user-facing byte.
    static let p2pkhBech32mTypeByte: UInt8 = 0xb0
    static let p2shBech32mTypeByte: UInt8 = 0x80
    static let p2pkhStorageTypeByte: UInt8 = 0x00
    static let p2shStorageTypeByte: UInt8 = 0x01

    /// Convert a decoded bech32m payload (`[0xb0|0x80] || 20-byte hash`) to the
    /// storage/FFI form (`[0x00|0x01] || hash`) that the FFI expects. Returns
    /// nil for a bad length or an unknown type byte.
    public static func storageBytes(fromBech32mPayload data: Data) -> Data? {
        let bytes = [UInt8](data)
        guard bytes.count == 21 else { return nil }
        let mapped: UInt8
        switch bytes[0] {
        case p2pkhBech32mTypeByte: mapped = p2pkhStorageTypeByte
        case p2shBech32mTypeByte: mapped = p2shStorageTypeByte
        default: return nil
        }
        return Data([mapped] + bytes[1...])
    }

    /// Inverse of `storageBytes(fromBech32mPayload:)`: convert the storage/FFI
    /// form (`[0x00|0x01] || hash`) to a bech32m payload (`[0xb0|0x80] || hash`)
    /// for encoding. Returns nil for a bad length or an unknown type byte.
    public static func bech32mPayload(fromStorageBytes data: Data) -> Data? {
        let bytes = [UInt8](data)
        guard bytes.count == 21 else { return nil }
        let mapped: UInt8
        switch bytes[0] {
        case p2pkhStorageTypeByte: mapped = p2pkhBech32mTypeByte
        case p2shStorageTypeByte: mapped = p2shBech32mTypeByte
        default: return nil
        }
        return Data([mapped] + bytes[1...])
    }

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
        // Valid Platform addresses have the DIP-0018 dash/tdash HRP and 21 bytes of data
        let validHrp = result.hrp == platformHrpMainnet || result.hrp == platformHrpTestnet
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
