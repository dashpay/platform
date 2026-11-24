package org.dashfoundation.example.ui.dashpay

import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.wallet.ShieldedIdentityBalance
import org.dashfoundation.dashsdk.wallet.ShieldedTipRecipientChange
import org.dashfoundation.dashsdk.wallet.ShieldedTipReview
import org.dashfoundation.dashsdk.wallet.ShieldedTipSource
import org.dashfoundation.example.ui.shielded.ShieldedPaymentOutcome
import org.dashfoundation.example.ui.wallet.formatCreditsExact
import org.dashfoundation.example.ui.wallet.shieldedOperationMayHaveGoneThrough
import org.dashfoundation.example.util.Base58

// How the app words a shielded tip to a DashPay username. The wallet
// (`platform-wallet`'s `wallet/shielded/tip_payment.rs`) decides everything
// about a tip: the amount the typed text means, whom the name pays, whether
// that changed since the wallet last paid the name, which account pays,
// whether an earlier payment to the recipient is unresolved and whether the
// tip may be sent now. Nothing here decides any of that; it words what the
// wallet returned.

/** The wallet's main shielded account, the account the other shielded sends pay from. */
internal const val SHIELDED_TIP_MAIN_ACCOUNT = 0

/**
 * Which account the user picked to pay a tip from. The wallet resolves the
 * account and refuses one it may not pay from.
 */
internal enum class ShieldedTipSourceChoice(val label: String, val tag: String) {
    /** The sending identity's own shielded account, proved with the seed when the tip is sent. */
    IDENTITY("This identity", "dashpay.tip.source.identity"),

    /** The wallet's main shielded account (account 0). */
    MAIN_ACCOUNT("Main shielded", "dashpay.tip.source.main"),
    ;

    fun source(identityId: ByteArray): ShieldedTipSource = when (this) {
        IDENTITY -> ShieldedTipSource.Identity(identityId)
        MAIN_ACCOUNT -> ShieldedTipSource.Account(SHIELDED_TIP_MAIN_ACCOUNT)
    }
}

/**
 * What the user reviewed: the amount the wallet parsed from the typed text,
 * the name as it was reviewed, and the wallet's review of it. The send takes
 * all three back unchanged.
 */
internal data class ReviewedShieldedTip(
    val username: String,
    val credits: Long,
    val review: ShieldedTipReview,
)

/** The fields a review reads, as they were when it started. */
internal data class ShieldedTipDraft(val username: String, val amountText: String)

/** What the screen does with a review the wallet returned. */
internal sealed interface ShieldedTipReviewStep {
    /** The draft changed while the wallet reviewed it: drop the result. */
    data object Discard : ShieldedTipReviewStep

    /**
     * A re-check of a review that said an earlier payment is unresolved found
     * it resolved. The review is dropped, not turned into an enabled Send:
     * the earlier payment may have executed, and another tip is a new one.
     */
    data object HoldEnded : ShieldedTipReviewStep

    /** The recipient changed since the last confirmation: ask before showing it. */
    data class ConfirmChange(val tip: ReviewedShieldedTip) : ShieldedTipReviewStep

    data class Show(val tip: ReviewedShieldedTip) : ShieldedTipReviewStep
}

/**
 * Route a review the wallet returned for [started], with the form now at
 * [now]. [recheck] marks the automatic read again of a review that was
 * unresolved.
 */
internal fun shieldedTipReviewStep(
    started: ShieldedTipDraft,
    now: ShieldedTipDraft,
    recheck: Boolean,
    tip: ReviewedShieldedTip,
): ShieldedTipReviewStep = when {
    started != now -> ShieldedTipReviewStep.Discard
    recheck && !tip.review.unresolved -> ShieldedTipReviewStep.HoldEnded
    tip.review.change is ShieldedTipRecipientChange.Changed -> ShieldedTipReviewStep.ConfirmChange(tip)
    else -> ShieldedTipReviewStep.Show(tip)
}

/** Wording for a shielded tip. */
internal object ShieldedTipMessages {
    const val SENDING =
        "Sending the tip. Building the proof takes tens of seconds; leaving this " +
            "screen does not stop it, and its outcome is shown when you come back."

    const val PENDING =
        "This attempt sent nothing. Another tip from this wallet is being sent, or an " +
            "earlier payment to this recipient may still execute. A shielded sync resolves " +
            "it either way, so the refusal ending does not mean that payment failed: check " +
            "Shielded Activity before you tip again."

    const val UNRESOLVED =
        "An earlier payment from this wallet to this recipient may still execute, so " +
            "the wallet refuses another one. It clears once a shielded sync sees that " +
            "payment executed or proves it cannot execute."

    /**
     * Shown when a re-check no longer reports the hold of [UNRESOLVED]; the
     * review is dropped. The review answers for the recipient as it is now,
     * not for the earlier payment, so this claims nothing about that payment.
     */
    const val HOLD_ENDED =
        "The refreshed review no longer reports a hold on this recipient. That does not " +
            "say what became of the earlier payment: check it in Shielded Activity before " +
            "you tip again. Reviewing again starts a new tip, which is a new payment."

    const val IDENTITY_NOT_HELD =
        "This wallet does not hold the paying identity. Nothing was sent."

    const val RECIPIENT_CHANGED_TITLE = "Tip recipient changed"

    const val RECIPIENT_CHANGED =
        "This username now pays a different identity or shielded address than the " +
            "recipient this wallet last confirmed for it. Check the change with the " +
            "recipient before you send."

    const val ALREADY_RUNNING =
        "A tip from this identity is still being sent, or its outcome was not shown yet. " +
            "Nothing new was sent."

    /** A returned send: submitted, not yet seen by a shielded sync. */
    fun submittedText(credits: Long): String =
        "Tip of ${formatCreditsExact(credits)} submitted. Your shielded balance shows the " +
            "spend after the next shielded sync."

    /** A send that may have left the wallet. Never offered again. */
    fun mayHaveGoneThroughText(credits: Long): String =
        "The tip of ${formatCreditsExact(credits)} may have gone through. Do not send it " +
            "again. A shielded sync resolves it; until then the wallet refuses another tip " +
            "to this recipient, and the refusal ending does not mean the tip failed."

    /**
     * The review line for how the recipient compares with the one this
     * wallet last confirmed for the name. A recipient is confirmed when a
     * send to it is admitted, before anything is built, so none of these
     * says a tip was paid.
     */
    fun changeText(change: ShieldedTipRecipientChange): String = when (change) {
        ShieldedTipRecipientChange.New ->
            "No recipient was confirmed for this username from this wallet before."
        ShieldedTipRecipientChange.Unchanged ->
            "Same recipient as the one last confirmed for this username."
        is ShieldedTipRecipientChange.Changed ->
            "Differs from the recipient last confirmed for this username. Previously: " +
                "identity ${change.previous.identityId.toBase58Short()}."
    }

    /**
     * The paying identity's own shielded balance as the wallet reports it,
     * or null before the first read. Unknown is never shown as zero.
     */
    fun identityBalanceText(balance: ShieldedIdentityBalance?): String = when (balance) {
        null -> "Not read yet"
        ShieldedIdentityBalance.Unverified ->
            "Not known until this identity's shielded account is prepared in this session"
        is ShieldedIdentityBalance.Unavailable ->
            "Not known until a shielded sync restores account ${balance.account}"
        is ShieldedIdentityBalance.Available -> formatCreditsExact(balance.credits)
    }

    /** The main account's balance, or null while it is not known. */
    fun mainBalanceText(credits: Long?): String =
        credits?.let { formatCreditsExact(it) } ?: "Not known yet"

    /**
     * The wallet's refusals made before anything was selected, built or
     * sent, worded for this flow; null for any other error, whose own
     * message is shown.
     */
    fun refusal(error: Throwable): String? = when (error) {
        is DashSdkError.PlatformWallet.ShieldedTipPending -> PENDING
        // `ErrorIdentityNotFound` (7). The wallet's own NotFound (98, a
        // removed wallet) keeps its message.
        is DashSdkError.NotFound -> IDENTITY_NOT_HELD
        else -> null
    }

    /**
     * The outcome of a `sendShieldedTip` call that threw [error], or
     * returned when [error] is null. The SDK keeps the result of a call that
     * completed after its caller was cancelled, so a cancellation here means
     * the call never completed; it is rethrown, never retried.
     */
    fun outcome(error: Throwable?): ShieldedPaymentOutcome = when {
        error == null -> ShieldedPaymentOutcome.Submitted
        error is CancellationException -> throw error
        shieldedOperationMayHaveGoneThrough(error) -> ShieldedPaymentOutcome.MayHaveGoneThrough
        else -> ShieldedPaymentOutcome.Failed(
            refusal(error) ?: error.message ?: "The tip was not sent.",
        )
    }
}

private fun ByteArray.toBase58Short(): String =
    Base58.encode(this).let { if (it.length > 16) it.take(16) + "…" else it }
