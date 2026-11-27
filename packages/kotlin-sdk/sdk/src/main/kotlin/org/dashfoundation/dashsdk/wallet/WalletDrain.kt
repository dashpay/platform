package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.WalletDrainNative
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.UUID

/**
 * A read-only estimate of a Core to shielded "Max" (a wallet drain). Port of
 * `WalletDrainPreview` in
 * `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PlatformWalletManagerWalletDrain.swift`.
 *
 * Candidate figures count inputs other operations hold, so this is never a
 * guaranteed minimum: show every amount as an estimate.
 */
data class WalletDrainPreview(
    val candidateInputCount: Long,
    val candidateBalanceDuffs: Long,
    val poolFeeCredits: Long,
    /** Null when the estimate could not be sized (see [Status.InputLimitMayBeExceeded]). */
    val estimatedL1FeeDuffs: Long?,
    /** Null when nothing would arrive, or the estimate could not be sized. */
    val estimatedRecipientCredits: Long?,
    val status: Status,
) {
    enum class Status(val raw: Int) {
        Ready(0),
        InsufficientFunds(1),

        /**
         * The candidate count exceeds what one asset-lock transaction takes.
         * The transfer may still fit once another operation's inputs are
         * left out.
         */
        InputLimitMayBeExceeded(2),
    }

    internal companion object {
        /** Number of longs [WalletDrainNative.preview] returns. */
        const val PACKED_SIZE = 8

        /** Decode [WalletDrainNative.preview]'s result. Throws on a malformed one. */
        fun decode(packed: LongArray): WalletDrainPreview {
            if (packed.size != PACKED_SIZE) {
                throw DashSdkError.SerializationError(
                    "wallet drain preview has ${packed.size} fields, expected $PACKED_SIZE",
                )
            }
            val status = Status.entries.firstOrNull { it.raw.toLong() == packed[0] }
                ?: throw DashSdkError.SerializationError(
                    "Unknown wallet drain preview status: ${packed[0]}",
                )
            // The native amounts are unsigned 64-bit; one that does not fit a
            // Long is not an amount this wallet can hold.
            fun amount(index: Int, name: String): Long = packed[index].also {
                if (it < 0) {
                    throw DashSdkError.SerializationError("wallet drain preview $name is out of range")
                }
            }
            return WalletDrainPreview(
                candidateInputCount = amount(1, "candidate input count"),
                candidateBalanceDuffs = amount(2, "candidate balance"),
                poolFeeCredits = amount(3, "pool fee"),
                estimatedL1FeeDuffs = if (packed[4] != 0L) amount(5, "L1 fee") else null,
                estimatedRecipientCredits =
                    if (packed[6] != 0L) amount(7, "recipient credits") else null,
                status = status,
            )
        }
    }
}

/**
 * The exact asset lock of a wallet drain. [txidWire] is the 32-byte txid in
 * wire order, the order
 * [PlatformWalletManager.shieldedResumeFundFromAssetLock] takes. Port of
 * Swift's `WalletDrainOutpoint`.
 */
class WalletDrainOutpoint(txidWire: ByteArray, val vout: Int) {
    init {
        require(txidWire.size == 32) { "txidWire must be 32 bytes, got ${txidWire.size}" }
        require(vout >= 0) { "vout must be non-negative, got $vout" }
    }

    private val txid: ByteArray = txidWire.copyOf()

    /** A copy of the 32-byte txid in wire order. */
    val txidWire: ByteArray get() = txid.copyOf()

    override fun equals(other: Any?): Boolean =
        other is WalletDrainOutpoint && txid.contentEquals(other.txid) && vout == other.vout

    override fun hashCode(): Int = 31 * txid.contentHashCode() + vout

    override fun toString(): String =
        txid.reversed().joinToString("") { "%02x".format(it) } + ":" + vout
}

/**
 * What Rust reports for ONE stored operation id. Port of Swift's
 * `WalletDrainOutcome`.
 *
 * [state] and [error] are complementary: an error does not erase
 * [outpoint], and neither an error nor an absent outpoint allows a new
 * drain. Do what [WalletDrainRules] returns for it.
 */
class WalletDrainOutcome(
    val state: State,
    val outpoint: WalletDrainOutpoint?,
    /** The error the call returned beside the outcome, if any. */
    val error: DashSdkError? = null,
) {
    enum class State(val raw: Int) {
        NotSent(0),
        Pending(1),
        Completed(2),
    }

    internal companion object {
        /**
         * Bytes of a packed outcome before its error message: state,
         * has-outpoint flag, txid, vout, error code, consensus code,
         * consensus kind, payment delivery, retryable flag, held-lock flag,
         * held lock (36).
         */
        const val HEADER_SIZE = 1 + 1 + 32 + 4 + 4 + 4 + 4 + 4 + 1 + 1 + 36

        /**
         * The outcome a failed call leaves when nothing is known about the
         * operation: pending, with no asset lock. A failure before or around
         * the native call cannot show that an earlier call under the same id
         * dispatched nothing.
         */
        fun unresolved(error: DashSdkError): WalletDrainOutcome =
            WalletDrainOutcome(State.Pending, outpoint = null, error = error)

        /**
         * Decode [WalletDrainNative.fundFromWalletDrainV2]'s (or
         * [WalletDrainNative.statusV2]'s) result. Never throws: whatever
         * cannot be decoded becomes a pending outcome with the reason as
         * its error, keeping any outpoint that was readable.
         */
        fun decode(packed: ByteArray): WalletDrainOutcome {
            if (packed.size < HEADER_SIZE) {
                return unresolved(
                    DashSdkError.SerializationError(
                        "wallet drain outcome has ${packed.size} bytes, expected at least $HEADER_SIZE",
                    ),
                )
            }
            val buffer = ByteBuffer.wrap(packed).order(ByteOrder.LITTLE_ENDIAN)
            val stateRaw = buffer.get().toInt() and 0xFF
            val hasOutpoint = buffer.get().toInt() != 0
            val txid = ByteArray(32).also { buffer.get(it) }
            val vout = buffer.getInt()
            val errorCode = buffer.getInt()
            val consensusCode = buffer.getInt()
            val consensusKind = buffer.getInt()
            val paymentDelivery = buffer.getInt()
            val retryable = buffer.get().toInt() != 0
            val hasHeldAssetLock = buffer.get().toInt() != 0
            val heldAssetLock = ByteArray(36).also { buffer.get(it) }
            val message = String(packed, HEADER_SIZE, packed.size - HEADER_SIZE, Charsets.UTF_8)

            var problem: DashSdkError? = null
            // A vout past Int.MAX_VALUE reads negative here. No asset lock has
            // one; without a usable outpoint the row can only be read again.
            val outpoint = when {
                !hasOutpoint -> null
                vout < 0 -> {
                    problem = DashSdkError.SerializationError(
                        "wallet drain outcome names an output index out of range",
                    )
                    null
                }
                else -> WalletDrainOutpoint(txid, vout)
            }
            val returned: DashSdkError? = if (errorCode == 0) {
                null
            } else {
                // Everything a thrown exception carries, the payment class
                // included, so a failed drain reads as it does on Swift.
                DashSdkError.fromNative(
                    DashSDKException(
                        errorCode,
                        message,
                        consensusCode,
                        consensusKind,
                        paymentDelivery,
                        retryable,
                        heldAssetLock.takeIf { hasHeldAssetLock },
                    ),
                )
            }
            val state = State.entries.firstOrNull { it.raw == stateRaw }
            if (state == null) {
                problem = problem ?: DashSdkError.SerializationError(
                    "Unknown wallet drain outcome state: $stateRaw",
                )
            }
            return WalletDrainOutcome(
                // An outcome that cannot be read in full stays pending.
                state = if (problem == null) state ?: State.Pending else State.Pending,
                outpoint = outpoint,
                error = returned ?: problem,
            )
        }
    }
}

/** What one read of [PlatformWalletManager.shieldedWalletDrainStatus] produced. */
sealed interface WalletDrainStatusRead {
    /** The read succeeded and reported [state]. */
    data class Reported(val state: WalletDrainOutcome.State) : WalletDrainStatusRead

    /** The read failed. That is never evidence that nothing was sent. */
    data object Failed : WalletDrainStatusRead
}

/** The one thing a host offers for a stored operation id. */
enum class WalletDrainRecoveryAction {
    /** One of the host's attempts for this operation is running. */
    InProgress,

    /** Resume the asset lock the last status read named, by its exact outpoint. */
    Resume,

    /** Read the status again. Sends nothing. */
    CheckAgain,
}

/** Where a tracked asset lock is listed, as far as wallet drains are concerned. */
enum class WalletDrainLockListing {
    /** Not decided here: the host's own rule for resumable locks applies. */
    Unaffected,

    /** The host's stored-operation row owns this lock: list it nowhere else. */
    Hidden,

    /** List it with the resumable locks, although it is only Built. */
    Offered,
}

/**
 * The rules a host follows for the operation ids it stores for wallet
 * drains. Every function is one call into Rust
 * (`platform_wallet::wallet::shielded::wallet_drain_recovery`), which holds
 * the rules, so this app and the Swift one cannot drift apart: the caller
 * gathers the facts it owns and does what these return.
 *
 * None of them throws. A call that fails (a value Rust does not know, a
 * native library that cannot be loaded) returns the fail-closed answer named
 * on each function, the same one the C export writes before it validates
 * and Swift's `WalletDrainRules` returns: keep the stored id, check again,
 * pending, not finished, not allowed, not offered.
 *
 * Swift callers of the same rules: `WalletDrainCoordinator`,
 * `ShieldedFundFromAssetLockView` and `PendingPlatformFundFromAssetLocksList`
 * in `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/`.
 */
object WalletDrainRules {
    /**
     * Whether the host keeps the operation id it stored, after one status
     * read. [attemptOverlappedRead] says whether one of the host's attempts
     * for the id ran or started while that read was in progress. A failed
     * call keeps the id: forgetting is the one step that cannot be undone.
     */
    fun keepsStoredId(read: WalletDrainStatusRead, attemptOverlappedRead: Boolean): Boolean =
        failClosed(true) {
            WalletDrainNative.keepsStoredId(
                readSucceeded = read is WalletDrainStatusRead.Reported,
                reportedState = (read as? WalletDrainStatusRead.Reported)?.state?.raw ?: 0,
                attemptOverlappedRead = attemptOverlappedRead,
            )
        }

    /**
     * What the host offers for a stored operation id. [lastRead] is null
     * before any status read has finished; [readNamedAssetLock] says whether
     * that read's outcome carried an outpoint. A failed call, or an action
     * this build does not know, is [WalletDrainRecoveryAction.CheckAgain],
     * which never spends.
     */
    fun recoveryAction(
        attemptInFlight: Boolean,
        lastRead: WalletDrainStatusRead?,
        readNamedAssetLock: Boolean,
    ): WalletDrainRecoveryAction {
        val raw = failClosed(RECOVERY_ACTION_CHECK_AGAIN) {
            WalletDrainNative.recoveryAction(
                attemptInFlight = attemptInFlight,
                hasRead = lastRead != null,
                readSucceeded = lastRead is WalletDrainStatusRead.Reported,
                reportedState = (lastRead as? WalletDrainStatusRead.Reported)?.state?.raw ?: 0,
                readNamedAssetLock = readNamedAssetLock,
            )
        }
        return when (raw) {
            0 -> WalletDrainRecoveryAction.InProgress
            1 -> WalletDrainRecoveryAction.Resume
            else -> WalletDrainRecoveryAction.CheckAgain
        }
    }

    /**
     * The state to report for a status read that returned [reported] while
     * one of the host's own drain calls for the same operation id has not
     * returned ([hostCallActive]): such a call may not have reached Rust
     * yet, and Rust answers "not sent" for an id it has never seen. A failed
     * call, or a state this build does not know, is
     * [WalletDrainOutcome.State.Pending].
     */
    fun statusDuringHostCall(
        reported: WalletDrainOutcome.State,
        hostCallActive: Boolean,
    ): WalletDrainOutcome.State {
        val raw = failClosed(WalletDrainOutcome.State.Pending.raw) {
            WalletDrainNative.statusDuringHostCall(reported.raw, hostCallActive)
        }
        return WalletDrainOutcome.State.entries.firstOrNull { it.raw == raw }
            ?: WalletDrainOutcome.State.Pending
    }

    /**
     * Whether a returned attempt needs nothing more. An attempt that is not
     * finished is shown as failed, with the error its call returned. A
     * failed call is "not finished".
     */
    fun attemptFinished(state: WalletDrainOutcome.State): Boolean =
        failClosed(false) { WalletDrainNative.attemptFinished(state.raw) }

    /**
     * Whether a drain may be started after a preview with this [status]. A
     * failed call does not allow it.
     */
    fun previewAllowsStart(status: WalletDrainPreview.Status): Boolean =
        failClosed(false) { WalletDrainNative.previewAllowsStart(status.raw) }

    /**
     * Whether a NEW drain is offered on [network]. Recovering an existing
     * one is offered on every network and does not ask this. A failed call
     * does not offer it.
     *
     * [shieldedFundFromWalletDrain] does not ask this itself: a caller that
     * starts a new drain asks first and does not call it where the answer is
     * false (mainnet, until dashpay/platform#5580).
     */
    fun startOffered(network: Network): Boolean =
        failClosed(false) { WalletDrainNative.startOffered(network.ffiValue) }

    /**
     * Where the host lists the asset lock with these stored values, or null
     * when Rust does not know [fundingTypeRaw] or [statusRaw] (or the call
     * fails): such a lock is offered nowhere. [hasOperationId] says whether
     * the lock carries a valid 16-byte operation id, and [hostHoldsStoredId]
     * whether the host holds a stored operation id equal to it. Swift:
     * `WalletDrainRules.lockListingIfKnown`.
     */
    fun lockListingOrNull(
        fundingTypeRaw: Int,
        statusRaw: Int,
        hasOperationId: Boolean,
        hostHoldsStoredId: Boolean,
    ): WalletDrainLockListing? {
        val raw = failClosed(LOCK_LISTING_UNKNOWN) {
            WalletDrainNative.lockListing(
                fundingType = fundingTypeRaw,
                status = statusRaw,
                hasOperationId = hasOperationId,
                hostHoldsStoredId = hostHoldsStoredId,
            )
        }
        return when (raw) {
            0 -> WalletDrainLockListing.Unaffected
            1 -> WalletDrainLockListing.Hidden
            2 -> WalletDrainLockListing.Offered
            else -> null
        }
    }

    /**
     * [lockListingOrNull], with [WalletDrainLockListing.Unaffected] for a
     * funding type or status Rust does not know.
     */
    fun lockListing(
        fundingTypeRaw: Int,
        statusRaw: Int,
        hasOperationId: Boolean,
        hostHoldsStoredId: Boolean,
    ): WalletDrainLockListing =
        lockListingOrNull(fundingTypeRaw, statusRaw, hasOperationId, hostHoldsStoredId)
            ?: WalletDrainLockListing.Unaffected

    private const val RECOVERY_ACTION_CHECK_AGAIN = 2

    /** Not one of the listing bytes Rust returns. */
    private const val LOCK_LISTING_UNKNOWN = -1

    /**
     * Run a rule call, returning [fallback] when it cannot answer: the
     * native layer threw, or the library (or one of its symbols) cannot be
     * loaded. Other errors, such as running out of memory, are not a rule's
     * answer and pass through.
     */
    private inline fun <T> failClosed(fallback: T, call: () -> T): T =
        try {
            call()
        } catch (_: Exception) {
            fallback
        } catch (_: LinkageError) {
            fallback
        }
}

/** A wallet drain operation id as the 16 bytes Rust stores, most significant first. */
fun UUID.toWalletDrainOperationIdBytes(): ByteArray =
    ByteBuffer.allocate(16)
        .putLong(mostSignificantBits)
        .putLong(leastSignificantBits)
        .array()

/** Inverse of [toWalletDrainOperationIdBytes]; null for anything that is not 16 bytes. */
fun walletDrainOperationIdFromBytes(bytes: ByteArray): UUID? {
    if (bytes.size != 16) return null
    val buffer = ByteBuffer.wrap(bytes)
    return UUID(buffer.getLong(), buffer.getLong())
}
