package org.dashfoundation.dashsdk.errors

import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.json.JSONObject

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

        /**
         * `ErrorShieldedInsufficientBalance` (native code 41; historical FFI
         * spelling). A Platform Payment account's deterministic shield input
         * set cannot cover the requested amount plus input 0's retained fee
         * reserve. Nothing was built or broadcast. Refresh preflight and ask
         * the user to confirm a smaller amount rather than retrying unchanged.
         * This is distinct from insufficient private shielded-note balance.
         */
        class PlatformShieldCapacityExceeded(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause)

        class AssetLockNotTracked(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause)

        /**
         * The one-shot output cannot be reused. This may be a retained local
         * tombstone or an unauthenticated Platform report; operation completion
         * must not be inferred from this signal.
         */
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
         * A state transition could not be signed because the signer has no
         * usable private key for the requested public key — the stored blob
         * is missing (never derived, wiped, or written under a different
         * Keystore alias/policy) rather than the operation itself failing.
         *
         * Primary path (dashpay/platform#4060 finding 7): `KeystoreSigner`
         * completes with the STRUCTURED
         * `SignerNative.SIGNER_ERROR_CODE_KEY_UNAVAILABLE`, which travels
         * typed across the Rust boundary and returns as
         * `PlatformWalletFFIResultCode::ErrorSigningKeyUnavailable` (31) —
         * mapped directly here, no message inspection involved. Hosts route
         * users to key repair (e.g.
         * `PlatformWalletManager.repairIdentityKey`) instead of treating it
         * as an opaque [Generic] failure. Not retryable as-is — the key must
         * be (re-)derived first.
         *
         * Also raised WITHOUT any signer round-trip by
         * [org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.signMessage],
         * for a message-signing address this wallet does not own — or owns only
         * watch-only, since a DashPay *external* account holds a contact's
         * addresses whose private keys we never had. Same conclusion, same host
         * response: correct the address or repair the keys, do not retry.
         */
        class SigningKeyUnavailable(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause) {
            companion object {
                /**
                 * Stable prefix of the `KeystoreSigner` "missing key"
                 * completion error message. `KeystoreSigner` builds its
                 * message from this constant, so the emitter and the matcher
                 * cannot drift.
                 *
                 * DEPRECATED as a discriminator: superseded by the typed
                 * code-31 mapping above. The marker match is retained for
                 * the #4191 merge-order transition — a consumer pinned at
                 * #4191's revision (marker classification, no native code
                 * 31) or any Rust conversion path that flattens the signer
                 * failure to text without the machine prefix still
                 * classifies via the marker. It is NOT a mixed-artifact
                 * escape hatch: an old native library paired with new
                 * Kotlin is unsupported outright (the sign-completion JNI
                 * arity changed from 3 to 4 args — every completion would
                 * be a type-confused native call). Remove the fallback (and
                 * this constant's matcher role) in the next minor release.
                 */
                const val MESSAGE_MARKER = "no private key stored for"
            }
        }

        /**
         * `PlatformWalletFFIResultCode::NotFound` (native code 98,
         * [PLATFORM_WALLET_NOT_FOUND_CODE]) — the code the FFI's blanket
         * `Option → result` conversion emits for every "requested <thing>
         * not found" miss (an unknown wallet id, an identity the wallet
         * does not manage, …). Typed inside the wallet-error family —
         * parity with Swift's `PlatformWalletError.notFound`, which also
         * keeps 98 in the wallet family — so callers can match a
         * wallet-level absence without sniffing [Generic] codes, while
         * staying distinct from the rs-sdk-ffi top-level
         * [DashSdkError.NotFound] that codes 7/8 map to.
         *
         * Dashpay's managed-identity local reads never see this type:
         * `translateManagedIdentityNotFoundToZero` intercepts the RAW
         * code (offset + 98) on the [org.dashfoundation.dashsdk.ffi.DashSDKException]
         * before [fromNative] runs and turns the miss into an absence.
         */
        class NotFound(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause)

        /**
         * `ErrorTransactionBroadcastRejected` (native code 26). Core
         * DEFINITIVELY rejected the core transaction: it is not on the network
         * and will not get there. The build's UTXO reservation was released and,
         * on the deferred (BIP70/BIP270) path, the token was consumed at the
         * same time — so the inputs are spendable again and the token is gone.
         *
         * The definitive counterpart to [TransactionBroadcastUnconfirmed] (20),
         * whose outcome is AMBIGUOUS and which therefore keeps its inputs
         * reserved. Because the reservation and token are already gone, this is
         * NOT retryable in place: address the rejection reason carried in the
         * message, then rebuild with
         * [buildSignedPayment][org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.buildSignedPayment]
         * (deferred) or re-issue the send.
         */
        class TransactionBroadcastRejected(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause)

        /**
         * `ErrorStaleReservationToken` (native code 34). A deferred
         * (BIP70/BIP270) [broadcastSigned][org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.broadcastSigned]
         * token has outlived its funding reservation's lifetime: key-wallet's
         * TTL may already have swept and re-selected the inputs, so acting on it
         * could touch a newer, unrelated reservation. The call did NOT touch the
         * network. NOT retryable in place — rebuild the payment with
         * [buildSignedPayment][org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.buildSignedPayment].
         *
         * Sibling of the other two deferred-token failures this code used to
         * conflate: [ReservationTokenConsumed] (unknown / already broadcast /
         * already released) and [ReservationWalletMismatch] (minted against a
         * different wallet generation).
         */
        class StaleReservationToken(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause)

        /**
         * `ErrorReservationTokenConsumed` (native code 35). A deferred
         * (BIP70/BIP270) [broadcastSigned][org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.broadcastSigned]
         * token is unknown, already broadcast, or already released — the guard
         * that turns a double-broadcast (or a broadcast after release) into a
         * typed error instead of a second send. The call did NOT touch the
         * network. NOT retryable: rebuild the payment with
         * [buildSignedPayment][org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.buildSignedPayment].
         * (Release is idempotent and never raises this.)
         */
        class ReservationTokenConsumed(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause)

        /**
         * `ErrorReservationWalletMismatch` (native code 36). A deferred
         * (BIP70/BIP270) [broadcastSigned][org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.broadcastSigned]
         * token was minted against a different wallet *generation* than the one
         * broadcasting it (e.g. a wallet re-created under the same id); its
         * reservation lives in that other generation's reservation set. The call
         * did NOT touch the network and did NOT consume the rightful owner's
         * token. NOT retryable through this handle: rebuild the payment with
         * [buildSignedPayment][org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.buildSignedPayment].
         */
        class ReservationWalletMismatch(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause)

        /** The DPNS name is not currently listed for sale. */
        class DocumentNotForSale(message: String, cause: Throwable? = null) :
            PlatformWallet(message, cause)

        /** The listing changed after the user confirmed [expectedCredits]. */
        class DocumentPriceChanged(
            val documentId: String,
            val expectedCredits: ULong,
            val actualCredits: ULong,
            cause: Throwable? = null,
        ) : PlatformWallet(
            "The DPNS price changed from $expectedCredits to $actualCredits credits. " +
                "Nothing was purchased.",
            cause,
        )

        /** The purchasing identity cannot cover price plus the fee reserve. */
        class InsufficientIdentityCredits(
            val identityId: String,
            val requiredCredits: ULong,
            val availableCredits: ULong,
            cause: Throwable? = null,
        ) : PlatformWallet(
            "Identity $identityId has $availableCredits credits but $requiredCredits are required.",
            cause,
        )

        /** The name is still in an active contested-name vote. */
        class ContestedNameNotTradable(
            val label: String,
            val endsAtMs: Long,
            cause: Throwable? = null,
        ) : PlatformWallet(
            if (endsAtMs == 0L) {
                "\"$label\" cannot be traded until its contested-name vote resolves."
            } else {
                "\"$label\" cannot be traded until its contested-name vote ends at $endsAtMs ms."
            },
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
         * by the wallet). Mapped to the typed [PlatformWallet.NotFound].
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
         * [PlatformWallet.Generic]. The `KeystoreSigner` "missing key"
         * completion arrives TYPED as code 31
         * (`ErrorSigningKeyUnavailable`, dashpay/platform#4060 finding 7);
         * the legacy message-marker sniff on the catch-all codes is a
         * deprecated fallback for the #4191 merge-order transition (never
         * applied to the dedicated retry-semantics types, so those are
         * never overridden; mixed old-native/new-Kotlin artifacts are
         * unsupported — see [PlatformWallet.SigningKeyUnavailable.MESSAGE_MARKER]).
         */
        private fun fromPlatformWalletNative(
            code: Int,
            message: String,
            cause: Throwable?,
        ): DashSdkError = when (code) {
            // PlatformWalletFFIResultCode variants (platform-wallet-ffi/src/error.rs)
            1 -> PlatformWallet.InvalidHandle(message, cause) // ErrorInvalidHandle
            6 -> // ErrorWalletOperation
                // @Deprecated fallback: the marker sniff survives for the
                // #4191 merge-order transition (and any conversion path
                // that lost the machine prefix); the typed code 31 below is
                // the real discriminator (#4060 finding 7). NOT for mixed
                // old-native/new-Kotlin builds — those are unsupported (the
                // completion JNI arity changed). Remove with
                // MESSAGE_MARKER's matcher role next minor release.
                if (isSigningKeyUnavailable(message)) {
                    PlatformWallet.SigningKeyUnavailable(message, cause)
                } else {
                    PlatformWallet.WalletOperation(message, cause)
                }
            7, // ErrorIdentityNotFound
            8, // ErrorContactNotFound
            -> NotFound(message, cause)
            // 98 (PlatformWalletFFIResultCode::NotFound, the blanket Option →
            // result miss) stays inside the wallet-error family as the typed
            // PlatformWallet.NotFound — exact Swift parity
            // (PlatformWalletError.notFound) — rather than collapsing into the
            // top-level NotFound that rs-sdk-ffi codes 7/8 map to. Dashpay's
            // managed-identity local reads are unaffected: they intercept the
            // RAW code via translateManagedIdentityNotFoundToZero (#4051)
            // before this mapping ever runs. BREAKING for Kotlin hosts that
            // caught DashSdkError.NotFound from platform-wallet operations.
            //
            // 98 is also what the wallet-was-REMOVED case returns on BOTH
            // deferred-send paths:
            //  * deferred (BIP70/BIP270) TOKEN path — a signed-payment broadcast
            //    whose wallet is no longer registered in the manager, or a
            //    signed-payment finalize whose wallet was removed while it was
            //    being signed;
            //  * finalized-transaction HANDLE path — a tx-builder finalize
            //    whose wallet was removed or re-created during signing (no handle
            //    is published), or a finalized-handle broadcast whose generation is gone.
            // Every one reconciles the build's UTXO reservation before returning.
            // Nothing was broadcast, and unlike ReservationWalletMismatch (36)
            // no other live generation holds the payment either — so it is not
            // retryable. See dashpay/platform#4185.
            PLATFORM_WALLET_NOT_FOUND_CODE ->
                PlatformWallet.NotFound(message, cause)
            16 -> PlatformWallet.ShieldedBroadcastFailed(message, cause) // ErrorShieldedBroadcastFailed
            18 -> PlatformWallet.ShieldedSpendUnconfirmed(message, cause) // ErrorShieldedSpendUnconfirmed
            19 -> PlatformWallet.ShieldedNoRecordedAnchor(message, cause) // ErrorShieldedNoRecordedAnchor
            20 -> PlatformWallet.TransactionBroadcastUnconfirmed(message, cause) // ErrorTransactionBroadcastUnconfirmed
            22 -> PlatformWallet.CoreInsufficientFunds(message, cause) // ErrorCoreInsufficientFunds
            23 -> PlatformWallet.AssetLockNotTracked(message, cause) // ErrorAssetLockNotTracked
            24 -> PlatformWallet.AssetLockAlreadyConsumed(message, cause) // ErrorAssetLockAlreadyConsumed
            25 -> PlatformWallet.AssetLockFundingMismatch(message, cause) // ErrorAssetLockFundingMismatch
            26 -> PlatformWallet.TransactionBroadcastRejected(message, cause) // ErrorTransactionBroadcastRejected
            // The deferred-token trio sits at the contiguous block 34-36 because
            // 27-33 are claimed elsewhere: 27 ErrorShutdownIncomplete
            // (dashpay/platform#4268, merged), 29 ErrorAssetLockInsufficientFunds
            // (#4184), 31 ErrorSigningKeyUnavailable (#4183/#4259), 32
            // ErrorTransactionBuild (#4247/#4256), 33 ErrorTransactionSigning
            // (#4256). See packages/rs-platform-wallet-ffi/ERROR_CODE_REGISTRY.md.
            34 -> PlatformWallet.StaleReservationToken(message, cause) // ErrorStaleReservationToken
            35 -> PlatformWallet.ReservationTokenConsumed(message, cause) // ErrorReservationTokenConsumed
            36 -> PlatformWallet.ReservationWalletMismatch(message, cause) // ErrorReservationWalletMismatch
            37 -> PlatformWallet.DocumentNotForSale(message, cause)
            38 -> parseMarketplaceDetail(message)?.let { detail ->
                runCatching {
                    PlatformWallet.DocumentPriceChanged(
                        documentId = detail.getString("documentId"),
                        expectedCredits = detail.requiredULong("expected"),
                        actualCredits = detail.requiredULong("actual"),
                        cause = cause,
                    )
                }.getOrNull()
            } ?: PlatformWallet.Generic(code, message, cause)
            39 -> parseMarketplaceDetail(message)?.let { detail ->
                runCatching {
                    PlatformWallet.InsufficientIdentityCredits(
                        identityId = detail.getString("identityId"),
                        requiredCredits = detail.requiredULong("required"),
                        availableCredits = detail.requiredULong("available"),
                        cause = cause,
                    )
                }.getOrNull()
            } ?: PlatformWallet.Generic(code, message, cause)
            40 -> parseMarketplaceDetail(message)?.let { detail ->
                runCatching {
                    PlatformWallet.ContestedNameNotTradable(
                        label = detail.getString("label"),
                        endsAtMs = detail.getLong("endsAtMs"),
                        cause = cause,
                    )
                }.getOrNull()
            } ?: PlatformWallet.Generic(code, message, cause)
            41 -> PlatformWallet.PlatformShieldCapacityExceeded(message, cause)
            // ErrorSigningKeyUnavailable — the STRUCTURED signer
            // discriminator (dashpay/platform#4060 finding 7): the typed
            // completion code rides the whole Rust round-trip, no message
            // sniffing involved. (Codes 26-33 are claimed by merged/sibling
            // PRs — 26 broadcast-rejected, 27 shutdown-incomplete, 29
            // asset-lock insufficient funds, 32/33 transaction build/signing;
            // the deferred-token trio sits at 34-36 above. See
            // PlatformWalletFFIResultCode for the authoritative map.)
            31 -> PlatformWallet.SigningKeyUnavailable(message, cause)
            else ->
                // @Deprecated fallback — see the code-6 arm; code 31 is the
                // real discriminator.
                if (isSigningKeyUnavailable(message)) {
                    PlatformWallet.SigningKeyUnavailable(message, cause)
                } else {
                    PlatformWallet.Generic(code, message, cause)
                }
        }

        private fun isSigningKeyUnavailable(message: String): Boolean =
            message.contains(PlatformWallet.SigningKeyUnavailable.MESSAGE_MARKER)

        private fun parseMarketplaceDetail(message: String): JSONObject? =
            runCatching { JSONObject(message) }.getOrNull()

        private fun JSONObject.requiredULong(name: String): ULong =
            get(name).toString().toULong()
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
