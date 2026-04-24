import Foundation
import DashSDKFFI

let NULL_HANDLE: Handle = 0


// Friendly aliases for the C enum cases that call sites compare against.
let Success: PlatformWalletFFIResult              = PLATFORM_WALLET_FFI_RESULT_SUCCESS
let ErrorInvalidHandle: PlatformWalletFFIResult   = PLATFORM_WALLET_FFI_RESULT_ERROR_INVALID_HANDLE
let ErrorInvalidParameter: PlatformWalletFFIResult = PLATFORM_WALLET_FFI_RESULT_ERROR_INVALID_PARAMETER
let ErrorNullPointer: PlatformWalletFFIResult     = PLATFORM_WALLET_FFI_RESULT_ERROR_NULL_POINTER
let ErrorSerialization: PlatformWalletFFIResult   = PLATFORM_WALLET_FFI_RESULT_ERROR_SERIALIZATION
let ErrorDeserialization: PlatformWalletFFIResult = PLATFORM_WALLET_FFI_RESULT_ERROR_DESERIALIZATION
let ErrorWalletOperation: PlatformWalletFFIResult = PLATFORM_WALLET_FFI_RESULT_ERROR_WALLET_OPERATION
let ErrorIdentityNotFound: PlatformWalletFFIResult = PLATFORM_WALLET_FFI_RESULT_ERROR_IDENTITY_NOT_FOUND
let ErrorContactNotFound: PlatformWalletFFIResult = PLATFORM_WALLET_FFI_RESULT_ERROR_CONTACT_NOT_FOUND
let ErrorInvalidNetwork: PlatformWalletFFIResult  = PLATFORM_WALLET_FFI_RESULT_ERROR_INVALID_NETWORK
let ErrorInvalidIdentifier: PlatformWalletFFIResult = PLATFORM_WALLET_FFI_RESULT_ERROR_INVALID_IDENTIFIER
let ErrorMemoryAllocation: PlatformWalletFFIResult = PLATFORM_WALLET_FFI_RESULT_ERROR_MEMORY_ALLOCATION
let ErrorUtf8Conversion: PlatformWalletFFIResult  = PLATFORM_WALLET_FFI_RESULT_ERROR_UTF8_CONVERSION
let ErrorUnknown: PlatformWalletFFIResult         = PLATFORM_WALLET_FFI_RESULT_ERROR_UNKNOWN

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
    case memoryAllocation
    case unknown(String)

    init(result: PlatformWalletFFIResult, error: PlatformWalletFFIError) {
        let message = error.message != nil ? String(cString: error.message!) : "Unknown error"

        switch result {
        case PLATFORM_WALLET_FFI_RESULT_ERROR_NULL_POINTER:
            self = .nullPointer
        case PLATFORM_WALLET_FFI_RESULT_ERROR_INVALID_HANDLE:
            self = .invalidHandle
        case PLATFORM_WALLET_FFI_RESULT_ERROR_INVALID_PARAMETER:
            self = .invalidParameter
        case PLATFORM_WALLET_FFI_RESULT_ERROR_INVALID_IDENTIFIER:
            self = .invalidIdentifier
        case PLATFORM_WALLET_FFI_RESULT_ERROR_INVALID_NETWORK:
            self = .invalidNetwork
        case PLATFORM_WALLET_FFI_RESULT_ERROR_WALLET_OPERATION:
            self = .walletOperation(message)
        case PLATFORM_WALLET_FFI_RESULT_ERROR_IDENTITY_NOT_FOUND:
            self = .identityNotFound
        case PLATFORM_WALLET_FFI_RESULT_ERROR_CONTACT_NOT_FOUND:
            self = .contactNotFound
        case PLATFORM_WALLET_FFI_RESULT_ERROR_UTF8_CONVERSION:
            self = .utf8Conversion
        case PLATFORM_WALLET_FFI_RESULT_ERROR_SERIALIZATION:
            self = .serialization
        case PLATFORM_WALLET_FFI_RESULT_ERROR_DESERIALIZATION:
            self = .deserialization
        case PLATFORM_WALLET_FFI_RESULT_ERROR_MEMORY_ALLOCATION:
            self = .memoryAllocation
        case PLATFORM_WALLET_FFI_RESULT_ERROR_UNKNOWN:
            self = .unknown(message)
        case PLATFORM_WALLET_FFI_RESULT_SUCCESS:
            assertionFailure("Error initialized from a success result")
            self = .unknown("Error initialized from a success result")
        default:
            self = .unknown(message)
        }
    }
}

/// Network type used by the Swift-facing Platform wallet API.
/// Raw values match the `FFINetwork` C enum from key-wallet-ffi.
public enum PlatformNetwork: UInt32 {
    case mainnet = 0
    case testnet = 1
    case regtest = 2
    case devnet = 3

    /// Value to pass to FFI calls that expect `FFINetwork`.
    var ffiValue: FFINetwork {
        FFINetwork(rawValue: self.rawValue)
    }
}

/// Swift-friendly block time. Wraps the C `BlockTime` struct (fields are
/// UInt64/UInt32/UInt64 there — we keep the same widths).
public struct PlatformBlockTime {
    public let height: UInt64
    public let coreHeight: UInt32
    public let timestamp: UInt64

    init(ffi: BlockTime) {
        self.height = ffi.height
        self.coreHeight = ffi.core_height
        self.timestamp = ffi.timestamp
    }

    var ffiValue: BlockTime {
        BlockTime(
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
