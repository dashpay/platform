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
    init?(hexString: String) {
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
