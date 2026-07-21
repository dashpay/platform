package org.dashfoundation.dashsdk.errors

import org.dashfoundation.dashsdk.ffi.DashSDKException

/**
 * Public error hierarchy of the Kotlin SDK — the Android analog of the
 * Swift SDK's `UserFacingError`/`SDKError` split, keyed off the native
 * `DashSDKErrorCode` values (`rs-sdk-ffi/src/error.rs`).
 *
 * The JNI layer throws the internal [DashSDKException]; public API entry
 * points convert it via [fromNative] so callers only ever see this type.
 */
sealed class DashSdkError(
    message: String,
    cause: Throwable? = null,
) : Exception(message, cause) {

    /** Whether retrying the same operation can plausibly succeed. */
    open val isRetryable: Boolean get() = false

    class InvalidParameter(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class InvalidState(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class NetworkError(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause) {
        override val isRetryable: Boolean get() = true
    }

    class SerializationError(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class ProtocolError(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class CryptoError(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class NotFound(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class Timeout(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause) {
        override val isRetryable: Boolean get() = true
    }

    class NotImplemented(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class DriveInternalError(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class InternalError(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    /**
     * Errors raised by the `platform-wallet-ffi` layer (its own
     * `PlatformWalletFFIResultCode` enum), surfaced through the JNI bridge's
     * shared `take_pwffi_error` with the native code shifted by
     * [PLATFORM_WALLET_CODE_OFFSET] so it never collides with the
     * rs-sdk-ffi `DashSDKErrorCode` range decoded above.
     *
     * The Android analog of Swift's `PlatformWalletError` enum
     * (`PlatformWalletResult.swift`). Only the retry-semantics-bearing codes
     * get dedicated types; everything else falls through to the
     * [PlatformWallet] catch-all which still carries the native code + Rust
     * message.
     */
    sealed class PlatformWallet(
        message: String,
        cause: Throwable? = null,
    ) : DashSdkError(message, cause) {

        /** `ErrorInvalidHandle` (native code 1). A stale/closed wallet handle. */
        class InvalidHandle(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause)

        /**
         * `ErrorWalletOperation` (native code 6). A generic wallet-operation
         * failure — the platform-wallet catch-all mapping, distinct from the
         * rs-sdk-ffi `CryptoError` that shares raw code 6.
         */
        class WalletOperation(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause)

        /** Atomic Core selection found no or insufficient unreserved UTXOs. */
        class CoreInsufficientFunds(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause)

        class AssetLockNotTracked(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause)

        class AssetLockAlreadyConsumed(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause)

        class AssetLockFundingMismatch(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause)

        /**
         * `ErrorShieldedNoRecordedAnchor` (native code 19). A shielded spend
         * could not be built against a Platform-recorded anchor because the
         * local commitment tree is mid-block. Nothing was broadcast and the
         * notes were released, so this **is** retryable once the next
         * shielded sync advances the tree onto a recorded boundary. Distinct
         * from [TransactionBroadcastUnconfirmed], which must NOT be retried.
         */
        class ShieldedNoRecordedAnchor(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause) {
            override val isRetryable: Boolean get() = true
        }

        /**
         * `ErrorShieldedSpendUnconfirmed` (native code 18). A shielded spend
         * (transfer / unshield / withdraw / shield) was broadcast and
         * accepted, but its execution result couldn't be confirmed — it may
         * already be on chain. Rust intentionally KEEPS the spent notes'
         * reservations, so the caller must NOT retry (a retry would rebuild
         * the bundle against other notes and could double-spend); the next
         * shielded sync reconciles the outcome. The Android analog of
         * Swift's `PlatformWalletError.shieldedSpendUnconfirmed`, which the
         * iOS send flow surfaces as a non-retryable "may have gone through"
         * outcome rather than an error.
         */
        class ShieldedSpendUnconfirmed(message: String, cause: Throwable? = null) :
            PlatformWallet(
                "$message (do NOT retry: the spend may already be on chain; " +
                    "the wallet keeps the spent notes reserved until the next " +
                    "shielded sync reconciles the outcome)",
                cause,
            )

        /**
         * `ErrorShieldedBroadcastFailed` (native code 16). A DEFINITIVE
         * non-execution outcome — relay/CheckTx rejected the transaction
         * before it entered the chain, and any note reservations were
         * released. Safe to retry (the opposite of the ambiguous
         * [ShieldedSpendUnconfirmed]).
         */
        class ShieldedBroadcastFailed(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause) {
            override val isRetryable: Boolean get() = true
        }

        /**
         * `ErrorShieldedBroadcastUnconfirmed` (native code 17) on the
         * shielded identity-create path. The broadcast outcome is AMBIGUOUS
         * — the identity may already be live on-chain — so [identityId]
         * (written by the C ABI even on this outcome) MUST be retained and
         * its derivation slot held (the coordinator's `Unconfirmed` phase).
         * Retrying would reuse keys for a possibly-live identity. Thrown by
         * `PlatformWalletManager.shieldedIdentityCreateFromPool` from the
         * tagged JNI payload (the id never rides in the message text).
         */
        class ShieldedCreateUnconfirmed(
            val identityId: ByteArray,
            message: String,
            cause: Throwable? = null,
        ) : PlatformWallet(
            "$message (identity may already exist on-chain; hold the slot — do NOT retry)",
            cause,
        )

        /**
         * `ErrorTransactionBroadcastUnconfirmed` (native code 20). A core
         * transaction broadcast had an AMBIGUOUS outcome — it may already be
         * on the network. The wallet keeps the spent inputs reserved so a
         * retry can't double-spend; the reservation TTL or a later sync
         * reconciles the outcome. Do **NOT** auto-retry.
         */
        class TransactionBroadcastUnconfirmed(message: String, cause: Throwable? = null) :
            PlatformWallet(
                "$message (do NOT retry: the transaction may already be on the network; " +
                    "the wallet keeps its inputs reserved until a sync reconciles the outcome)",
                cause,
            )

        /**
         * Any other `PlatformWalletFFIResultCode` without a dedicated type.
         * Carries the platform-wallet [nativeCode] (already de-offset) and
         * the Rust-supplied message.
         */
        class Generic(
            val nativeCode: Int,
            message: String,
            cause: Throwable? = null,
        ) : PlatformWallet(message, cause)
    }

    companion object {
        /**
         * Offset the JNI bridge adds to a `PlatformWalletFFIResultCode`
         * before throwing it as a `DashSDKException`, keeping it clear of the
         * rs-sdk-ffi `DashSDKErrorCode` range (1–10). Must stay in lockstep
         * with `support::PWFFI_CODE_OFFSET` in `rs-unified-sdk-jni`.
         */
        const val PLATFORM_WALLET_CODE_OFFSET = 1000

        /**
         * `PlatformWalletFFIResultCode::NotFound` (98) — the code the FFI's
         * blanket `Option → result` conversion emits for every "requested
         * <thing> not found" miss (e.g. an identity id that is not managed
         * by the wallet). Mapped to [NotFound]; Dashpay's managed-identity
         * local reads intercept the RAW code (offset + 98) via
         * `translateManagedIdentityNotFoundToZero` before [fromNative] runs
         * and turn the miss into an absence (zero handle → null / empty).
         */
        const val PLATFORM_WALLET_NOT_FOUND_CODE = 98

        /** Map a native error code + message into the public hierarchy. */
        fun fromNative(e: DashSDKException): DashSdkError {
            val message = e.message ?: "Unknown SDK error"
            if (e.code >= PLATFORM_WALLET_CODE_OFFSET) {
                return fromPlatformWalletNative(e.code - PLATFORM_WALLET_CODE_OFFSET, message, e)
            }
            return when (e.code) {
                1 -> InvalidParameter(message, e)
                2 -> InvalidState(message, e)
                3 -> NetworkError(message, e)
                4 -> SerializationError(message, e)
                5 -> ProtocolError(message, e)
                6 -> CryptoError(message, e)
                7 -> NotFound(message, e)
                8 -> Timeout(message, e)
                9 -> NotImplemented(message, e)
                10 -> DriveInternalError(message, e)
                else -> InternalError(message, e)
            }
        }

        /**
         * Map a de-offset `PlatformWalletFFIResultCode` value into the
         * [PlatformWallet] subtree — mirror of Swift's
         * `PlatformWalletError(result:)` construction. Retry-semantics-bearing
         * codes get dedicated types; the rest fall through to
         * [PlatformWallet.Generic].
         */
        private fun fromPlatformWalletNative(
            code: Int,
            message: String,
            cause: Throwable?,
        ): DashSdkError = when (code) {
            // PlatformWalletFFIResultCode variants (platform-wallet-ffi/src/error.rs)
            1 -> PlatformWallet.InvalidHandle(message, cause) // ErrorInvalidHandle
            6 -> PlatformWallet.WalletOperation(message, cause) // ErrorWalletOperation
            7, // ErrorIdentityNotFound
            8, // ErrorContactNotFound
            PLATFORM_WALLET_NOT_FOUND_CODE, // NotFound (Option returned as an error)
            -> NotFound(message, cause)
            16 -> PlatformWallet.ShieldedBroadcastFailed(message, cause) // ErrorShieldedBroadcastFailed
            18 -> PlatformWallet.ShieldedSpendUnconfirmed(message, cause) // ErrorShieldedSpendUnconfirmed
            19 -> PlatformWallet.ShieldedNoRecordedAnchor(message, cause) // ErrorShieldedNoRecordedAnchor
            20 -> PlatformWallet.TransactionBroadcastUnconfirmed(message, cause) // ErrorTransactionBroadcastUnconfirmed
            22 -> PlatformWallet.CoreInsufficientFunds(message, cause) // ErrorCoreInsufficientFunds
            23 -> PlatformWallet.AssetLockNotTracked(message, cause) // ErrorAssetLockNotTracked
            24 -> PlatformWallet.AssetLockAlreadyConsumed(message, cause) // ErrorAssetLockAlreadyConsumed
            25 -> PlatformWallet.AssetLockFundingMismatch(message, cause) // ErrorAssetLockFundingMismatch
            else -> PlatformWallet.Generic(code, message, cause)
        }
    }
}

/**
 * Run [block], converting any [DashSDKException] escaping the native layer
 * into the public [DashSdkError] hierarchy. Every public SDK entry point
 * that calls an `external fun` goes through this.
 */
inline fun <T> mapNativeErrors(block: () -> T): T =
    try {
        block()
    } catch (e: DashSDKException) {
        throw DashSdkError.fromNative(e)
    }
