package org.dashfoundation.example.util

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.PaymentRecovery
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.AssetLockBuiltOutPoint
import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * How the app presents a failed payment: the text to show and the one action
 * the native wallet's class allows. Every payment screen builds its failed
 * state from this, so none of them decides "retry", "resume" or "wait" from
 * an error's type, code or text. ← Swift `PaymentFailure`.
 *
 * The class is the error's delivery, retry and held lock, read through the
 * SDK's [PaymentRecovery]: a retryable NOT_SENT offers a retry, LOCK_HELD
 * resumes the named lock and never funds a new one, and everything else
 * waits (a non-retryable NOT_SENT, MAYBE_SENT, and an unclassified failure).
 */
data class PaymentFailure(val recovery: PaymentRecovery, val detail: String) {

    /** The one thing a screen may offer after the failure. */
    sealed interface Action {
        /** Run the operation again (a fresh registration or funding). */
        data object Retry : Action

        /** Resume the tracked asset lock at [outPoint] through the existing resume flow. */
        data class Resume(val outPoint: AssetLockBuiltOutPoint) : Action

        /** Offer nothing that pays again. */
        data object Wait : Action
    }

    val action: Action
        get() = when (val r = recovery) {
            PaymentRecovery.Retry -> Action.Retry
            is PaymentRecovery.Resume -> Action.Resume(r.outPoint)
            PaymentRecovery.Wait, PaymentRecovery.Reconcile, PaymentRecovery.Unconfirmed -> Action.Wait
        }

    val allowsRetry: Boolean get() = action == Action.Retry

    /** The lock to resume, null unless the action is [Action.Resume]. */
    val heldLock: AssetLockBuiltOutPoint? get() = (action as? Action.Resume)?.outPoint

    /** A short heading for the failed state. */
    val title: String
        get() = when (recovery) {
            PaymentRecovery.Retry -> "Failed"
            PaymentRecovery.Wait -> "Not sent"
            is PaymentRecovery.Resume -> "Funds held in an asset lock"
            PaymentRecovery.Reconcile -> "May have gone through"
            PaymentRecovery.Unconfirmed -> "Not confirmed"
        }

    /** The class's text, the held lock when there is one, then the detail. */
    val message: String
        get() = heldLock?.let {
            "${recovery.suggestion} Asset lock ${it.outPointHex} is listed with the wallet's " +
                "resumable asset locks. $detail"
        } ?: "${recovery.suggestion} $detail"

    /**
     * Whether a failed attempt may be started again by an operation that
     * resumes [resuming] (null for a fresh one): anything after a retryable
     * failure, only the resume of the held lock after a held one, nothing
     * after one that waits.
     */
    fun permitsRestart(resuming: AssetLockBuiltOutPoint?): Boolean = when (val a = action) {
        Action.Retry -> true
        is Action.Resume -> resuming == a.outPoint
        Action.Wait -> false
    }

    companion object {
        /** The failure [error] stands for, with its display text as the detail. */
        fun of(error: Throwable): PaymentFailure = PaymentFailure(
            PaymentRecovery.of(error),
            (error as? DashSdkError)?.userMessage?.takeIf { it.isNotBlank() }
                ?: error.message
                ?: error.javaClass.simpleName,
        )

        /**
         * The failure a payment screen's [error] stands for, or null when it
         * is not an SDK error: the screen's own input check, thrown before
         * anything was sent, which shows as its own message.
         */
        fun ofPayment(error: Throwable): PaymentFailure? = if (error is DashSdkError) of(error) else null

        /** The outpoint a resume call takes, from its wire-order txid and vout. */
        fun outPoint(txid: ByteArray, vout: Int): AssetLockBuiltOutPoint =
            AssetLockBuiltOutPoint.fromRawBytes(
                txid + ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN).putInt(vout).array(),
            )
    }
}
