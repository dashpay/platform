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
    case errorArithmeticOverflow = 13
    case errorNoSelectableInputs = 14
    case errorWalletAlreadyExists = 15
    /// Definitive shielded-broadcast failure: the shielded transition
    /// (identity-create, unshield, transfer, or withdrawal) was not executed
    /// (relay/CheckTx rejection or a platform-reported execution error), the
    /// spent notes were released, and the caller may retry.
    case errorShieldedBroadcastFailed = 16
    /// Shielded broadcast accepted but its execution result could not be
    /// confirmed; the identity may already exist on chain. The FFI also fills
    /// the derived id into `outIdentityId` on this code, so the caller can
    /// hold the slot rather than treat the registration as failed.
    case errorShieldedBroadcastUnconfirmed = 17
    /// A shielded operation (shield / unshield / transfer / withdrawal) was
    /// accepted by the relay but its execution result could not be
    /// confirmed. It may have executed on chain; for the spend-based
    /// operations the wallet keeps the notes reserved (a shield reserves
    /// nothing) until the next sync (or app restart) reconciles the
    /// outcome. Do NOT auto-retry — a retry would rebuild the bundle and
    /// could double-execute if the original landed.
    case errorShieldedSpendUnconfirmed = 18
    /// A shielded spend could not be built against a Platform-recorded anchor:
    /// the wallet's commitment tree isn't synced to a checkpoint Platform has
    /// recorded (an in-progress / interrupted sync leaves it mid-block). Nothing
    /// was broadcast and the notes were released. This is retryable — let the
    /// shielded sync reach a confirmed state and try again. Distinct from
    /// `errorShieldedSpendUnconfirmed`, which must NOT be retried.
    case errorShieldedNoRecordedAnchor = 19
    /// A core transaction broadcast (send, DashPay payment, or asset-lock
    /// funding) failed with an ambiguous outcome — the transaction may
    /// already be on the network. The wallet keeps the spent inputs' UTXO
    /// reservation, so an immediate retry fails at input selection instead
    /// of double-spending; the reservation TTL or a sync reconciles the
    /// outcome. Do NOT auto-retry.
    case errorTransactionBroadcastUnconfirmed = 20
    /// Definitively-failed address-nonce race: Platform rejected an
    /// address-funds transition (shield, or identity top-up-from-addresses)
    /// because the submitted address nonce raced Platform's expected value
    /// (a lagging DAPI replica read). The transition did NOT execute and any
    /// notes were released (a shield reserves none) — safe to retry; the retry
    /// re-fetches the nonce and self-heals. The submitted/expected nonce values
    /// travel in the message string, not as structured fields.
    case errorAddressNonceMismatch = 21
    /// Atomic Core selection found no or insufficient unreserved UTXOs.
    case errorCoreInsufficientFunds = 22
    case errorAssetLockNotTracked = 23
    case errorAssetLockAlreadyConsumed = 24
    case errorAssetLockFundingMismatch = 25
    /// Core definitively rejected the transaction. Its reserved inputs were
    /// released and a corrected transaction may be submitted again.
    case errorTransactionBroadcastRejected = 26
    /// A quiesce/drain barrier did not complete within its budget: an
    /// in-flight sync pass was still running when a Clear / reset /
    /// sync-stop needed it provably drained. The operation failed closed —
    /// no state was wiped — and the caller should retry once sync is idle.
    /// (Not returned by `destroy`: Rust owns the callback contexts, so a
    /// straggling worker is memory-safe and merely logged there.)
    case errorShutdownIncomplete = 27
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
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ARITHMETIC_OVERFLOW:
            self = .errorArithmeticOverflow
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_NO_SELECTABLE_INPUTS:
            self = .errorNoSelectableInputs
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_WALLET_ALREADY_EXISTS:
            self = .errorWalletAlreadyExists
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_BROADCAST_FAILED:
            self = .errorShieldedBroadcastFailed
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_BROADCAST_UNCONFIRMED:
            self = .errorShieldedBroadcastUnconfirmed
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_SPEND_UNCONFIRMED:
            self = .errorShieldedSpendUnconfirmed
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_NO_RECORDED_ANCHOR:
            self = .errorShieldedNoRecordedAnchor
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_TRANSACTION_BROADCAST_UNCONFIRMED:
            self = .errorTransactionBroadcastUnconfirmed
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ADDRESS_NONCE_MISMATCH:
            self = .errorAddressNonceMismatch
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_CORE_INSUFFICIENT_FUNDS:
            self = .errorCoreInsufficientFunds
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_NOT_TRACKED:
            self = .errorAssetLockNotTracked
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_ALREADY_CONSUMED:
            self = .errorAssetLockAlreadyConsumed
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_FUNDING_MISMATCH:
            self = .errorAssetLockFundingMismatch
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_TRANSACTION_BROADCAST_REJECTED:
            self = .errorTransactionBroadcastRejected
        case PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHUTDOWN_INCOMPLETE:
            self = .errorShutdownIncomplete
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
    case arithmeticOverflow(String)
    case noSelectableInputs(String)
    case coreInsufficientFunds(String)
    case assetLockNotTracked(String)
    case assetLockAlreadyConsumed(String)
    case assetLockFundingMismatch(String)
    case walletAlreadyExists(String)
    /// Definitive shielded-broadcast failure: the shielded transition
    /// (identity-create or a spend — unshield / transfer / withdrawal) was
    /// not executed and the spent notes were released; safe to retry.
    case shieldedBroadcastFailed(String)
    /// Shielded broadcast accepted but its execution result could not be
    /// confirmed; the identity may already exist on chain. Callers that need
    /// the derived id should special-case the
    /// `.errorShieldedBroadcastUnconfirmed` result code (which carries
    /// `outIdentityId`) before falling back to this error — see
    /// `ShieldedIdentityCreateUnconfirmedError`.
    case shieldedBroadcastUnconfirmed(String)
    /// A shielded operation (shield / unshield / transfer / withdrawal)
    /// was accepted by the relay but its execution result could not be
    /// confirmed. It may have executed; spend-based operations keep their
    /// notes reserved wallet-side (a shield reserves nothing) until the
    /// next sync reconciles the outcome. Do NOT auto-retry.
    case shieldedSpendUnconfirmed(String)
    /// A shielded spend could not be built against a Platform-recorded anchor —
    /// the wallet's commitment tree isn't synced to a checkpoint Platform has
    /// recorded (an in-progress / interrupted sync leaves it mid-block). Nothing
    /// was broadcast and the notes were released; retryable once the shielded
    /// sync reaches a confirmed state. Distinct from `shieldedSpendUnconfirmed`,
    /// which must NOT be retried.
    case shieldedNoRecordedAnchor(String)
    /// A core transaction broadcast was submitted but its outcome is
    /// unknown — the transaction may already be on the network. The wallet
    /// keeps the spent inputs reserved so a retry cannot double-spend; the
    /// reservation TTL or a later sync reconciles the outcome. Do NOT
    /// auto-retry. Core sibling of `shieldedSpendUnconfirmed`.
    case transactionBroadcastUnconfirmed(String)
    /// Core definitively rejected the transaction and its input reservation
    /// was released. Unlike `transactionBroadcastUnconfirmed`, retry is safe.
    case transactionBroadcastRejected(String)
    /// Definitively-failed address-nonce race (shield, or identity
    /// top-up-from-addresses): Platform rejected the transition because the
    /// submitted address nonce raced its expected value. The transition did
    /// NOT execute and any notes were released (a shield reserves none) — safe
    /// to retry, and the retry re-fetches the address nonce so the mismatch
    /// self-heals. The submitted/expected nonce values are in the message.
    case addressNonceMismatch(String)
    /// A quiesce/drain barrier (Clear / reset / sync-stop) timed out with a
    /// sync pass still in flight. The operation failed closed — retry once
    /// sync is idle.
    case shutdownIncomplete(String)
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
             .arithmeticOverflow(let m), .noSelectableInputs(let m),
             .coreInsufficientFunds(let m),
             .assetLockNotTracked(let m), .assetLockAlreadyConsumed(let m),
             .assetLockFundingMismatch(let m),
             .walletAlreadyExists(let m), .shieldedBroadcastFailed(let m),
             .shieldedBroadcastUnconfirmed(let m), .shieldedSpendUnconfirmed(let m),
             .shieldedNoRecordedAnchor(let m),
             .transactionBroadcastUnconfirmed(let m),
             .transactionBroadcastRejected(let m),
             .addressNonceMismatch(let m),
             .shutdownIncomplete(let m),
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
        case .errorArithmeticOverflow: self = .arithmeticOverflow(detail)
        case .errorNoSelectableInputs: self = .noSelectableInputs(detail)
        case .errorCoreInsufficientFunds: self = .coreInsufficientFunds(detail)
        case .errorAssetLockNotTracked: self = .assetLockNotTracked(detail)
        case .errorAssetLockAlreadyConsumed: self = .assetLockAlreadyConsumed(detail)
        case .errorAssetLockFundingMismatch: self = .assetLockFundingMismatch(detail)
        case .errorWalletAlreadyExists: self = .walletAlreadyExists(detail)
        case .errorShieldedBroadcastFailed: self = .shieldedBroadcastFailed(detail)
        case .errorShieldedBroadcastUnconfirmed: self = .shieldedBroadcastUnconfirmed(detail)
        case .errorShieldedSpendUnconfirmed: self = .shieldedSpendUnconfirmed(detail)
        case .errorShieldedNoRecordedAnchor: self = .shieldedNoRecordedAnchor(detail)
        case .errorTransactionBroadcastUnconfirmed:
            self = .transactionBroadcastUnconfirmed(detail)
        case .errorTransactionBroadcastRejected:
            self = .transactionBroadcastRejected(detail)
        case .errorAddressNonceMismatch:
            self = .addressNonceMismatch(detail)
        case .errorShutdownIncomplete:
            self = .shutdownIncomplete(detail)
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
