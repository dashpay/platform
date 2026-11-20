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
 * resumes the named lock and never funds a new one (an invitation's voucher
 * lock is reclaimed instead, [invitationVoucher]), and everything else
 * waits (a non-retryable NOT_SENT, MAYBE_SENT, and an unclassified failure).
 * Whether the failure keeps its slot is [holdsSlot]: only a NOT_SENT one
 * releases it.
 */
data class PaymentFailure(
    val recovery: PaymentRecovery,
    val detail: String,
    /**
     * Whether the operation that failed created an invitation, so a held lock
     * is the invitation's voucher: its funds come back through the voucher's
     * reclaim, not the generic resume (which refuses a voucher). Known from
     * the operation, never from the error.
     */
    val invitationVoucher: Boolean = false,
) {

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

    /** The lock to resume, null unless the action is [Action.Resume]. */
    val heldLock: AssetLockBuiltOutPoint? get() = (action as? Action.Resume)?.outPoint

    /** A short heading for the failed state. */
    val title: String
        get() = when (recovery) {
            PaymentRecovery.Retry -> "Failed"
            PaymentRecovery.Wait -> "Not sent"
            is PaymentRecovery.Resume ->
                if (invitationVoucher) "Funds held in the invitation voucher" else "Funds held in an asset lock"
            PaymentRecovery.Reconcile -> "May have gone through"
            PaymentRecovery.Unconfirmed -> "Not confirmed"
        }

    /** The class's text, the held lock when there is one, then the detail. */
    val message: String
        get() = heldLock?.let {
            if (invitationVoucher) {
                "${recovery.invitationVoucherSuggestion} Voucher lock ${it.outPointHex} is listed " +
                    "under Sent invitations. $detail"
            } else {
                "${recovery.suggestion} Asset lock ${it.outPointHex} is listed with the wallet's " +
                    "resumable asset locks. $detail"
            }
        } ?: "${recovery.suggestion} $detail"

    /**
     * Whether the failure holds its slot (the screen's form, the
     * coordinator's controller, its dismissal) until it is resumed or the
     * user leaves the flow. Every gate on a failed payment reads this.
     * ← Swift `PaymentFailure.holdsSlot`.
     *
     * A failure that may have left the wallet (MAYBE_SENT), holds an asset
     * lock (LOCK_HELD) or carries no class holds it: a new request could pay
     * a second time. Any NOT_SENT failure, retryable or not, releases it:
     * nothing left the wallet, so dismissal and a corrected new request are
     * allowed (the native wallet refuses again if something still blocks).
     */
    val holdsSlot: Boolean
        get() = when (recovery) {
            PaymentRecovery.Retry, PaymentRecovery.Wait -> false
            is PaymentRecovery.Resume, PaymentRecovery.Reconcile, PaymentRecovery.Unconfirmed -> true
        }

    /**
     * Whether a failed attempt may be started again by an operation that
     * resumes [resuming] (null for a fresh one): anything after a failure
     * that releases its slot ([holdsSlot]), only the resume of the held lock
     * after a held one, nothing after any other.
     */
    fun permitsRestart(resuming: AssetLockBuiltOutPoint?): Boolean =
        !holdsSlot || (heldLock != null && resuming == heldLock)

    companion object {
        /**
         * The failure [error] stands for, with its display text as the detail.
         * [invitationVoucher] when the operation that failed created an
         * invitation.
         */
        fun of(error: Throwable, invitationVoucher: Boolean = false): PaymentFailure = PaymentFailure(
            PaymentRecovery.of(error),
            (error as? DashSdkError)?.userMessage?.takeIf { it.isNotBlank() }
                ?: error.message
                ?: error.javaClass.simpleName,
            invitationVoucher,
        )

        /**
         * The failure a payment screen's [error] stands for, or null when no
         * payment was attempted, which the screen shows as its own message
         * (← Swift `PaymentFailure.ofPayment`): the screen's own input check
         * (not an SDK error), or a wallet error the native wallet gave no
         * class. The native wallet classes every failure of a payment it ran,
         * so a wallet error without one is a refusal before any payment (an
         * input check, a native pointer or handle refusal). An unconfirmed
         * shielded identity create and an rs-sdk-ffi error stay failures.
         */
        fun ofPayment(error: Throwable, invitationVoucher: Boolean = false): PaymentFailure? = when (error) {
            is DashSdkError.PlatformWallet.ShieldedCreateUnconfirmed -> of(error, invitationVoucher)
            is DashSdkError.PlatformWallet -> error.delivery?.let { of(error, invitationVoucher) }
            is DashSdkError -> of(error, invitationVoucher)
            else -> null
        }

        /** The outpoint a resume call takes, from its wire-order txid and vout. */
        fun outPoint(txid: ByteArray, vout: Int): AssetLockBuiltOutPoint =
            AssetLockBuiltOutPoint.fromRawBytes(
                txid + ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN).putInt(vout).array(),
            )
    }
}
