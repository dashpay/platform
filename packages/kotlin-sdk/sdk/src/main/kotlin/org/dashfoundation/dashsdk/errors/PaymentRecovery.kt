package org.dashfoundation.dashsdk.errors

import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.AssetLockBuiltOutPoint

/**
 * What a host may offer a person after a failed payment, read from the
 * native wallet's class alone ([DashSdkError.delivery],
 * [DashSdkError.isRetryable], [DashSdkError.heldAssetLock]), never from the
 * error's type, code or text. Show [suggestion]; offer a retry only under
 * [Retry] and a resume only under [Resume] with its outpoint.
 *
 * The Android analog of Swift's `PlatformPaymentRecovery`, with the same texts.
 */
sealed class PaymentRecovery {
    /** Text fit to show a person: what happened and the one action allowed. */
    abstract val suggestion: String

    /**
     * [suggestion] for a failed invitation create, whose held lock is the
     * invitation's voucher: the generic resume refuses a voucher lock, and its
     * funds come back through the voucher's reclaim. The host knows it from
     * the operation that failed; the class is the same.
     */
    open val invitationVoucherSuggestion: String get() = suggestion

    /** Nothing left the wallet and the native wallet allows running it again. */
    data object Retry : PaymentRecovery() {
        override val suggestion = "Nothing was sent. You can try again."
    }

    /**
     * Nothing left the wallet, and running it again as it is is not
     * allowed: an earlier payment may still be settling. Offer no retry.
     */
    data object Wait : PaymentRecovery() {
        override val suggestion =
            "Nothing was sent. An earlier payment may still be settling; wait for it to finish."
    }

    /** The payment holds the tracked asset lock at [outPoint]: resume it, never fund a new one. */
    data class Resume(val outPoint: AssetLockBuiltOutPoint) : PaymentRecovery() {
        override val suggestion =
            "The funds are held in an asset lock. Resume that lock to finish; do not start a new payment."

        override val invitationVoucherSuggestion =
            "The funds are held in the invitation's voucher lock. Reclaim the voucher from Sent " +
                "invitations to get them back; do not create a new invitation."
    }

    /** The payment may have left the wallet: let sync settle it, never offer a second one. */
    data object Reconcile : PaymentRecovery() {
        override val suggestion =
            "The payment may have gone through. Wait for the wallet to sync before doing anything else; " +
                "do not send it again."
    }

    /** No native wallet error classified the failure: unconfirmed, never retried. */
    data object Unconfirmed : PaymentRecovery() {
        override val suggestion =
            "Submission result is unconfirmed. Check history and reconcile the original operation " +
                "before submitting again."
    }

    companion object {
        /**
         * The recovery for a class. A [PaymentDelivery.LOCK_HELD] without its
         * outpoint names no lock to resume, so it waits like
         * [PaymentDelivery.MAYBE_SENT].
         */
        fun of(
            delivery: PaymentDelivery?,
            isRetryable: Boolean,
            heldAssetLock: AssetLockBuiltOutPoint?,
        ): PaymentRecovery = when (delivery) {
            null -> Unconfirmed
            PaymentDelivery.NOT_SENT -> if (isRetryable) Retry else Wait
            PaymentDelivery.MAYBE_SENT -> Reconcile
            PaymentDelivery.LOCK_HELD -> heldAssetLock?.let(::Resume) ?: Reconcile
        }

        /** The recovery for [error]; anything that is not a [DashSdkError] is [Unconfirmed]. */
        fun of(error: Throwable): PaymentRecovery =
            (error as? DashSdkError)?.let { of(it.delivery, it.isRetryable, it.heldAssetLock) }
                ?: Unconfirmed
    }
}
