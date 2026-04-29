import Foundation
import DashSDKFFI

let NULL_HANDLE: Handle = 0

extension Identifier {
    /// Run `body` with a `*const u8` to this identifier's 32-byte
    /// payload.
    ///
    /// Preconditions: `count == 32`. Identifiers on Platform are
    /// always exactly 32 bytes; a precondition here surfaces the
    /// drift loudly rather than letting Rust read garbage on
    /// dereference.
    @inline(__always)
    func withFFIBytes<R>(_ body: (UnsafePointer<UInt8>) throws -> R) rethrows -> R {
        precondition(count == 32, "identifier must be 32 bytes, got \(count)")
        return try withUnsafeBytes { raw in
            try body(raw.bindMemory(to: UInt8.self).baseAddress!)
        }
    }
}

typealias NetworkType = UInt32

/// Platform Wallet error types
public enum PlatformWalletError: LocalizedError {
    case nullPointer
    case invalidHandle
    case invalidParameter
    case invalidIdentifier
    case invalidNetwork
    case walletOperation(String)
    case identityNotFound
    case contactNotFound
    case utf8Conversion
    case serialization(String)
    case deserialization(String)
    case memoryAllocation
    case unknown(String)

    public var errorDescription: String? {
        switch self {
        case .nullPointer: return "Null pointer"
        case .invalidHandle: return "Invalid handle"
        case .invalidParameter: return "Invalid parameter"
        case .invalidIdentifier: return "Invalid identifier"
        case .invalidNetwork: return "Invalid network"
        case .walletOperation(let msg): return "Wallet operation: \(msg)"
        case .identityNotFound: return "Identity not found"
        case .contactNotFound: return "Contact not found"
        case .utf8Conversion: return "UTF-8 conversion error"
        case .serialization(let msg): return "Serialization: \(msg)"
        case .deserialization(let msg): return "Deserialization: \(msg)"
        case .memoryAllocation: return "Memory allocation error"
        case .unknown(let msg): return msg
        }
    }

    init(result: PlatformWalletFFIResult, error: PlatformWalletFFIError) {
        // Prefer the Rust-side detail message when one was supplied —
        // the payload-less enum cases below otherwise drop it on the
        // floor, which makes alerts like "Null pointer" impossible to
        // diagnose. When Rust gave us no message we fall back to the
        // typed bare case (keeps existing behavior for callers that
        // only compare on the case label).
        let rustMessage: String? = error.message.map { String(cString: $0) }

        /// Promote a payload-less enum case to `.unknown(detail)`
        /// when Rust supplied a message; otherwise keep the typed case.
        func withDetail(_ bare: PlatformWalletError, prefix: String) -> PlatformWalletError {
            guard let msg = rustMessage, !msg.isEmpty else { return bare }
            return .unknown("\(prefix): \(msg)")
        }

        switch result {
        case PLATFORM_WALLET_FFI_RESULT_ERROR_INVALID_HANDLE:
            self = withDetail(.invalidHandle, prefix: "Invalid handle")
        case PLATFORM_WALLET_FFI_RESULT_ERROR_INVALID_PARAMETER:
            self = withDetail(.invalidParameter, prefix: "Invalid parameter")
        case PLATFORM_WALLET_FFI_RESULT_ERROR_NULL_POINTER:
            self = withDetail(.nullPointer, prefix: "Null pointer")
        case PLATFORM_WALLET_FFI_RESULT_ERROR_SERIALIZATION:
            self = .serialization(rustMessage ?? "Unknown error")
        case PLATFORM_WALLET_FFI_RESULT_ERROR_DESERIALIZATION:
            self = .deserialization(rustMessage ?? "Unknown error")
        case PLATFORM_WALLET_FFI_RESULT_ERROR_WALLET_OPERATION:
            self = .walletOperation(rustMessage ?? "Unknown error")
        case PLATFORM_WALLET_FFI_RESULT_ERROR_IDENTITY_NOT_FOUND:
            self = withDetail(.identityNotFound, prefix: "Identity not found")
        case PLATFORM_WALLET_FFI_RESULT_ERROR_CONTACT_NOT_FOUND:
            self = withDetail(.contactNotFound, prefix: "Contact not found")
        case PLATFORM_WALLET_FFI_RESULT_ERROR_INVALID_NETWORK:
            self = withDetail(.invalidNetwork, prefix: "Invalid network")
        case PLATFORM_WALLET_FFI_RESULT_ERROR_INVALID_IDENTIFIER:
            self = withDetail(.invalidIdentifier, prefix: "Invalid identifier")
        case PLATFORM_WALLET_FFI_RESULT_ERROR_MEMORY_ALLOCATION:
            self = withDetail(.memoryAllocation, prefix: "Memory allocation")
        case PLATFORM_WALLET_FFI_RESULT_ERROR_UTF8_CONVERSION:
            self = withDetail(.utf8Conversion, prefix: "UTF-8 conversion")
        default:
            self = .unknown(rustMessage ?? "Unknown error")
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

/// Identity lifecycle status as carried on the Rust-side
/// `ManagedIdentity.status`. Mirrors `IdentityStatusFFI` /
/// `platform_wallet::wallet::identity::state::managed_identity::IdentityStatus`.
public enum IdentityStatus: UInt8, Sendable {
    case unknown = 0
    case pendingCreation = 1
    case active = 2
    case failedCreation = 3
    case notFound = 4

    /// Short human-readable label for the explorer UI.
    public var displayName: String {
        switch self {
        case .unknown: return "Unknown"
        case .pendingCreation: return "Pending Creation"
        case .active: return "Active"
        case .failedCreation: return "Failed Creation"
        case .notFound: return "Not Found"
        }
    }
}

// MARK: - Identifier FFI Conversion Helpers

/// Read 32 bytes from `ptr` into an `Identifier` (a `Data` of length 32).
///
/// Replaces the old `identifierFromFFI(_: IdentifierBytes)` helper.
/// `ptr` must point at exactly 32 readable bytes.
@inline(__always)
func identifierFromFFI(_ ptr: UnsafePointer<UInt8>) -> Identifier {
    Data(bytes: ptr, count: 32)
}

/// Read a row from an `IdentifierArray` returned by Rust.
///
/// `array.items` points at a contiguous `[[u8; 32]; count]` buffer;
/// take a `Data` snapshot of the i-th 32-byte row. Swift imports
/// `uint8_t (*)[32]` as a pointer to a 32-tuple, so we rebind to a
/// flat `UInt8` pointer before indexing.
@inline(__always)
func identifierFromFFIArray(_ array: IdentifierArray, at index: Int) -> Identifier {
    precondition(index >= 0 && index < Int(array.count), "index out of range")
    let raw = UnsafeRawPointer(array.items!)
    let base = raw.assumingMemoryBound(to: UInt8.self)
    let row = base.advanced(by: index * 32)
    return Data(bytes: row, count: 32)
}

/// Generate a random identifier via the FFI.
public func generateRandomIdentifier() throws -> Identifier {
    var buf = [UInt8](repeating: 0, count: 32)
    var error = PlatformWalletFFIError()

    let result = buf.withUnsafeMutableBufferPointer { bp -> PlatformWalletFFIResult in
        platform_wallet_generate_random_identifier(bp.baseAddress!, &error)
    }
    guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
        throw PlatformWalletError(result: result, error: error)
    }

    return Data(buf)
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
