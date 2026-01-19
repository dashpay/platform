import Foundation
import DashSDKFFI

// FFI types from platform-wallet-ffi (not in C header, so we define them here)
// These match the Rust definitions in rs-platform-wallet-ffi

typealias Handle = UInt64
let NULL_HANDLE: Handle = 0

struct IdentifierBytes {
    var bytes: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)
}

struct IdentifierArray {
    var items: UnsafeMutablePointer<IdentifierBytes>?
    var count: Int
}

typealias NetworkType = UInt32

typealias PlatformWalletFFIResult = Int32

struct PlatformWalletFFIError {
    var message: UnsafePointer<CChar>?
}

struct FFIBlockTime {
    var height: UInt32
    var core_height: UInt32
    var timestamp: UInt64
}

/// FFI-compatible platform sync state
struct PlatformSyncStateFFI {
    var last_full_sync_timestamp: UInt64
    var checkpoint_height: UInt64
    var last_terminal_block: UInt64
    var highest_found_index: UInt32
}

/// FFI-compatible platform sync result
struct PlatformSyncResultFFI {
    var full_sync_performed: Bool
    var checkpoint_height: UInt64
    var highest_terminal_block: UInt64
    var total_balance: UInt64
    var funded_address_count: UInt32
    var highest_found_index: UInt32
}

// Error result codes (must match Rust enum values)
let Success: PlatformWalletFFIResult = 0
let ErrorNullPointer: PlatformWalletFFIResult = 1
let ErrorInvalidHandle: PlatformWalletFFIResult = 2
let ErrorInvalidParameter: PlatformWalletFFIResult = 3
let ErrorInvalidIdentifier: PlatformWalletFFIResult = 4
let ErrorInvalidNetwork: PlatformWalletFFIResult = 5
let ErrorWalletOperation: PlatformWalletFFIResult = 6
let ErrorIdentityNotFound: PlatformWalletFFIResult = 7
let ErrorContactNotFound: PlatformWalletFFIResult = 8
let ErrorUtf8Conversion: PlatformWalletFFIResult = 9
let ErrorSerialization: PlatformWalletFFIResult = 10
let ErrorDeserialization: PlatformWalletFFIResult = 11

/// Platform Wallet error types
public enum PlatformWalletError: Error {
    case nullPointer
    case invalidHandle
    case invalidParameter
    case invalidIdentifier
    case invalidNetwork
    case walletOperation(String)
    case identityNotFound
    case contactNotFound
    case utf8Conversion
    case serialization
    case deserialization
    case unknown(String)

    init(result: PlatformWalletFFIResult, error: PlatformWalletFFIError) {
        let message = error.message != nil ? String(cString: error.message!) : "Unknown error"

        switch result {
        case ErrorNullPointer:
            self = .nullPointer
        case ErrorInvalidHandle:
            self = .invalidHandle
        case ErrorInvalidParameter:
            self = .invalidParameter
        case ErrorInvalidIdentifier:
            self = .invalidIdentifier
        case ErrorInvalidNetwork:
            self = .invalidNetwork
        case ErrorWalletOperation:
            self = .walletOperation(message)
        case ErrorIdentityNotFound:
            self = .identityNotFound
        case ErrorContactNotFound:
            self = .contactNotFound
        case ErrorUtf8Conversion:
            self = .utf8Conversion
        case ErrorSerialization:
            self = .serialization
        case ErrorDeserialization:
            self = .deserialization
        default:
            self = .unknown(message)
        }
    }
}

/// Network type for Platform wallet
public enum PlatformNetwork: UInt32 {
    case mainnet = 0
    case testnet = 1
    case devnet = 2
    case local = 3

    var ffiValue: NetworkType {
        NetworkType(self.rawValue)
    }
}

/// Block time information
public struct BlockTime {
    public let height: UInt32
    public let coreHeight: UInt32
    public let timestamp: UInt64

    public init(height: UInt32, coreHeight: UInt32, timestamp: UInt64) {
        self.height = height
        self.coreHeight = coreHeight
        self.timestamp = timestamp
    }

    init(ffiBlockTime: FFIBlockTime) {
        self.height = ffiBlockTime.height
        self.coreHeight = ffiBlockTime.core_height
        self.timestamp = ffiBlockTime.timestamp
    }

    var ffiValue: FFIBlockTime {
        FFIBlockTime(
            height: self.height,
            core_height: self.coreHeight,
            timestamp: self.timestamp
        )
    }
}

// MARK: - Identifier FFI Conversion Helpers

/// Convert Identifier (Data) to FFI IdentifierBytes
func identifierToFFI(_ identifier: Identifier) -> IdentifierBytes {
    var ffiBytes = IdentifierBytes(bytes: (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0))
    identifier.withUnsafeBytes { (ptr: UnsafeRawBufferPointer) in
        withUnsafeMutableBytes(of: &ffiBytes.bytes) { ffiPtr in
            for i in 0..<min(32, identifier.count) {
                ffiPtr[i] = ptr[i]
            }
        }
    }
    return ffiBytes
}

/// Convert FFI IdentifierBytes to Identifier (Data)
func identifierFromFFI(_ ffiIdentifier: IdentifierBytes) -> Identifier {
    var bytesArray = ffiIdentifier.bytes
    return withUnsafeBytes(of: &bytesArray) { Data($0) }
}

/// Generate a random identifier
public func generateRandomIdentifier() throws -> Identifier {
    var ffiId = IdentifierBytes(bytes: (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0))
    var error = PlatformWalletFFIError()

    let result = platform_wallet_generate_random_identifier(&ffiId, &error)
    guard result == Success else {
        throw PlatformWalletError(result: result, error: error)
    }

    return identifierFromFFI(ffiId)
}

extension Data {
    public init?(hexString: String) {
        let len = hexString.count / 2
        var data = Data(capacity: len)
        var index = hexString.startIndex
        for _ in 0..<len {
            let nextIndex = hexString.index(index, offsetBy: 2)
            if let b = UInt8(hexString[index..<nextIndex], radix: 16) {
                data.append(b)
            } else {
                return nil
            }
            index = nextIndex
        }
        self = data
    }
}

// MARK: - Platform Sync Types

/// Platform sync state for tracking address balance synchronization
public struct PlatformSyncState {
    /// Timestamp of the last full sync (seconds since Unix epoch)
    public let lastFullSyncTimestamp: UInt64
    /// Platform block height at last checkpoint
    public let checkpointHeight: UInt64
    /// Last terminal block processed
    public let lastTerminalBlock: UInt64
    /// Highest address index found with a balance (nil if none)
    public let highestFoundIndex: UInt32?

    public init(
        lastFullSyncTimestamp: UInt64 = 0,
        checkpointHeight: UInt64 = 0,
        lastTerminalBlock: UInt64 = 0,
        highestFoundIndex: UInt32? = nil
    ) {
        self.lastFullSyncTimestamp = lastFullSyncTimestamp
        self.checkpointHeight = checkpointHeight
        self.lastTerminalBlock = lastTerminalBlock
        self.highestFoundIndex = highestFoundIndex
    }

    init(ffiState: PlatformSyncStateFFI) {
        self.lastFullSyncTimestamp = ffiState.last_full_sync_timestamp
        self.checkpointHeight = ffiState.checkpoint_height
        self.lastTerminalBlock = ffiState.last_terminal_block
        self.highestFoundIndex = ffiState.highest_found_index == UInt32.max ? nil : ffiState.highest_found_index
    }

    var ffiValue: PlatformSyncStateFFI {
        PlatformSyncStateFFI(
            last_full_sync_timestamp: self.lastFullSyncTimestamp,
            checkpoint_height: self.checkpointHeight,
            last_terminal_block: self.lastTerminalBlock,
            highest_found_index: self.highestFoundIndex ?? UInt32.max
        )
    }
}

/// Result from a platform address sync operation
public struct PlatformSyncResult {
    /// Whether a full sync was performed
    public let fullSyncPerformed: Bool
    /// Checkpoint height from full sync (Platform block height)
    public let checkpointHeight: UInt64
    /// Highest terminal block processed
    public let highestTerminalBlock: UInt64
    /// Total credits found across all addresses
    public let totalBalance: UInt64
    /// Number of addresses with non-zero balance
    public let fundedAddressCount: UInt32
    /// Highest address index found with a balance (nil if none)
    public let highestFoundIndex: UInt32?

    init(ffiResult: PlatformSyncResultFFI) {
        self.fullSyncPerformed = ffiResult.full_sync_performed
        self.checkpointHeight = ffiResult.checkpoint_height
        self.highestTerminalBlock = ffiResult.highest_terminal_block
        self.totalBalance = ffiResult.total_balance
        self.fundedAddressCount = ffiResult.funded_address_count
        self.highestFoundIndex = ffiResult.highest_found_index == UInt32.max ? nil : ffiResult.highest_found_index
    }
}
