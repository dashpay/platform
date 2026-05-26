import Foundation
import DashSDKFFI

// MARK: - Typed result code

/// Swift mirror of the Rust `PlatformWalletFFIResultCode` enum.
public enum PlatformWalletResultCode: Int32, Sendable {
    case success = 0
    case errorInvalidHandle = 1
    case errorInvalidParameter = 2
    case errorNullPointer = 3
    case errorSerialization = 4
    case errorDeserialization = 5
    case errorWalletOperation = 6
    case errorIdentityNotFound = 7
    case errorContactNotFound = 8
    case errorInvalidNetwork = 9
    case errorInvalidIdentifier = 10
    case errorMemoryAllocation = 11
    case errorUtf8Conversion = 12
    case errorNoSpendableInputs = 30
    case errorConcurrentSpendConflict = 31
    case notFound = 98
    case errorUnknown = 99

    init(ffi: PlatformWalletFFIResultCode) {
        switch ffi {
        case PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS:
            self = .success
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_INVALID_HANDLE:
            self = .errorInvalidHandle
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_INVALID_PARAMETER:
            self = .errorInvalidParameter
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_NULL_POINTER:
            self = .errorNullPointer
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SERIALIZATION:
            self = .errorSerialization
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_DESERIALIZATION:
            self = .errorDeserialization
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_WALLET_OPERATION:
            self = .errorWalletOperation
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_IDENTITY_NOT_FOUND:
            self = .errorIdentityNotFound
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_CONTACT_NOT_FOUND:
            self = .errorContactNotFound
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_INVALID_NETWORK:
            self = .errorInvalidNetwork
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_INVALID_IDENTIFIER:
            self = .errorInvalidIdentifier
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_MEMORY_ALLOCATION:
            self = .errorMemoryAllocation
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_UTF8_CONVERSION:
            self = .errorUtf8Conversion
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_NO_SPENDABLE_INPUTS:
            self = .errorNoSpendableInputs
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_CONCURRENT_SPEND_CONFLICT:
            self = .errorConcurrentSpendConflict
        case PLATFORM_WALLET_FFI_RESULT_CODE_NOT_FOUND:
            self = .notFound
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_UNKNOWN:
            self = .errorUnknown
        default:
            self = .errorUnknown
        }
    }
}

// MARK: - Class wrapper

/// Reference-counted wrapper around a `PlatformWalletFFIResult`
/// returned by Rust. Owns the heap-allocated `message` C string and
/// frees it through `platform_wallet_ffi_result_free` in `deinit`,
/// regardless of whether the caller threw, returned early, or simply
/// dropped the wrapper.
///
/// Use the `try ffiResult.check()` extension below for the common
/// "throw on non-success" shape; reach for an explicit instance when
/// you want to inspect the `message` without throwing (e.g. logging
/// a warning on a soft-failure path).
final class PlatformWalletResult {
    private var inner: PlatformWalletFFIResult

    init(_ ffi: PlatformWalletFFIResult) {
        self.inner = ffi
    }

    deinit {
        platform_wallet_ffi_result_free(&inner)
    }

    /// Typed result code; unknown raw values fall back to `.errorUnknown`.
    var code: PlatformWalletResultCode {
        PlatformWalletResultCode(ffi: inner.code)
    }

    var message: String? {
        inner.message.map { String(cString: $0) }
    }

    var isSuccess: Bool {
        code == .success
    }

    func throwIfError() throws {
        guard !isSuccess else { return }
        throw PlatformWalletError(result: self)
    }
}

// MARK: - PlatformWalletError

/// Platform Wallet error type.
///
/// Built directly from a `PlatformWalletResult` via
/// `PlatformWalletError(result:)`. The wrapper owns both the typed
/// code and the Rust-supplied message, so taking it as a single
/// input avoids mismatched (code, message) pairs at the call site.
/// Most call sites just write `try ffi(...).check()` and let the
/// FFI extension do the construction.
public enum PlatformWalletError: LocalizedError {
    case nullPointer(String)
    case invalidHandle(String)
    case invalidParameter(String)
    case invalidIdentifier(String)
    case invalidNetwork(String)
    case walletOperation(String)
    case identityNotFound(String)
    case contactNotFound(String)
    case utf8Conversion(String)
    case serialization(String)
    case deserialization(String)
    case memoryAllocation(String)
    /// No spendable inputs on the requested account — retryable after
    /// sync, or surface a depleted-wallet message. Mirrors the Rust
    /// `PlatformWalletError::NoSpendableInputs` variant.
    case noSpendableInputs(String)
    /// Transaction builder picked an outpoint another concurrent
    /// build had already selected. Retry — the underlying reservation
    /// will have cleared. Mirrors the Rust
    /// `PlatformWalletError::ConcurrentSpendConflict` variant.
    case concurrentSpendConflict(String)
    case notFound(String)
    case unknown(String)

    /// Diagnostic detail Rust attached to the originating
    /// `PlatformWalletFFIResult`, or the context string a Swift-side
    /// guard chose when constructing the error inline.
    public var errorDescription: String? {
        switch self {
        case .nullPointer(let m), .invalidHandle(let m), .invalidParameter(let m),
             .invalidIdentifier(let m), .invalidNetwork(let m), .walletOperation(let m),
             .identityNotFound(let m), .contactNotFound(let m), .utf8Conversion(let m),
             .serialization(let m), .deserialization(let m), .memoryAllocation(let m),
             .noSpendableInputs(let m), .concurrentSpendConflict(let m),
             .notFound(let m), .unknown(let m):
            return m
        }
    }

    init(result: PlatformWalletResult) {
        let detail = result.message ?? "<no detail from Rust>"
        switch result.code {
        case .success:
            // Constructing an error from a success result is a caller bug
            self = .unknown(result.message
                ?? "PlatformWalletError built from a success result")
        case .errorInvalidHandle:     self = .invalidHandle(detail)
        case .errorInvalidParameter:  self = .invalidParameter(detail)
        case .errorNullPointer:       self = .nullPointer(detail)
        case .errorSerialization:     self = .serialization(detail)
        case .errorDeserialization:   self = .deserialization(detail)
        case .errorWalletOperation:   self = .walletOperation(detail)
        case .errorIdentityNotFound:  self = .identityNotFound(detail)
        case .errorContactNotFound:   self = .contactNotFound(detail)
        case .errorInvalidNetwork:    self = .invalidNetwork(detail)
        case .errorInvalidIdentifier: self = .invalidIdentifier(detail)
        case .errorMemoryAllocation:  self = .memoryAllocation(detail)
        case .errorUtf8Conversion:    self = .utf8Conversion(detail)
        case .errorNoSpendableInputs: self = .noSpendableInputs(detail)
        case .errorConcurrentSpendConflict:
            self = .concurrentSpendConflict(detail)
        case .notFound:               self = .notFound(detail)
        case .errorUnknown:           self = .unknown(detail)
        }
    }
}

// MARK: - Convenience extensions

extension PlatformWalletFFIResult {
    @inline(__always)
    func check() throws {
        try PlatformWalletResult(self).throwIfError()
    }

    @inline(__always)
    func discard() {
        _ = PlatformWalletResult(self)
    }
}
