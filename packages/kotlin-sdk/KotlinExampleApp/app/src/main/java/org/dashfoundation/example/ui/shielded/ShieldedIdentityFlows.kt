package org.dashfoundation.example.ui.shielded

import java.math.BigDecimal
import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.wallet.ShieldedIdentityDebitRecoveryRecord
import org.dashfoundation.dashsdk.wallet.ShieldedIdentityDebitRecoveryStatus
import org.dashfoundation.example.ui.wallet.formatCreditsExact
import org.dashfoundation.example.ui.wallet.shieldedOperationMayHaveGoneThrough
import org.dashfoundation.example.util.toHex

// The two transitions between an identity's credit balance and the wallet's
// shielded pool, and how the app reports them:
//
// - Type 21, shield from an identity's balance (Send → Send From → Identity),
//   ported from `SendViewModel.swift` and `ShieldedIdentityDebitRecoveryView.swift`.
// - Type 22, top up an identity from the shielded pool (Top Up → Shielded
//   Balance), ported from `TopUpIdentityView.swift`.
//
// (all under packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/).
//
// Nothing here decides whether a payment may be made. Whether the identity
// or the notes cover the amount plus the fee is Rust's refusal, made against
// the proved balance before anything is saved or sent; the recovery record's
// status is Rust's; this file only words what Rust reported.

/**
 * What became of one identity shield (Type 21) or shielded top-up (Type 22)
 * call. Amounts are shown exactly: one credit is 0.00000000001 DASH, never 0.
 */
sealed interface ShieldedPaymentOutcome {
    /** The call returned: submitted, not yet confirmed by the scan. */
    data object Submitted : ShieldedPaymentOutcome

    /** It may be on chain. Never retried; listed by the recovery views. */
    data object MayHaveGoneThrough : ShieldedPaymentOutcome

    /** Refused or failed; [message] is shown in the error dialog. */
    data class Failed(val message: String) : ShieldedPaymentOutcome
}

/** Exact DASH text for [credits] (11 decimals, nothing rounded away). */
internal fun formatCreditsExact(credits: ULong): String =
    BigDecimal(credits.toString()).movePointLeft(11).stripTrailingZeros().toPlainString() + " DASH"

/** Wording for the identity shield (Type 21). ← Swift `SendViewModel`'s identity shield messages. */
internal object IdentityShieldMessages {
    const val SUBMITTED =
        "Shield from identity submitted. Its Shielded Activity row stays " +
            "pending until the shielded sync sees the new note."

    const val MAY_HAVE_GONE_THROUGH =
        "The shield from this identity may have gone through. Do not retry. " +
            "It is listed under In-Progress Identity Shields in Shielded " +
            "Activity, and the wallet keeps checking it on each shielded sync."

    /**
     * Wallet-scoped on purpose: an unreadable record blocks shields from
     * every identity in the wallet, and the error then names the requested
     * identity, not the record's.
     */
    const val DEBIT_PENDING =
        "An earlier shield from an identity in this wallet is still unresolved, " +
            "or its record could not be read, so a new one was not sent. See " +
            "In-Progress Identity Shields in Shielded Activity: the wallet resolves " +
            "readable ones on a shielded sync, and you can stop retrying any of them there."

    const val IDENTITY_CHANGED =
        "The source identity changed after Send was tapped. Review it and tap Send again."

    const val ALREADY_RUNNING =
        "A shield from this identity is still running, or its outcome was not shown yet. " +
            "Nothing new was sent."

    const val FEE_FLOOR_UNAVAILABLE ="The fee floor is not available yet, so Send stays disabled."

    const val FOOTER =
        "The identity's transfer key signs. It must hold the amount plus the " +
            "estimated fee. The note goes to this wallet's own shielded address, " +
            "so enter that as the recipient."

    /**
     * The wallet's refusals made before anything is saved, built or sent,
     * worded for this flow; null for any other error, whose own message is
     * shown. ← Swift `SendViewModel.identityShieldRefusalMessage`.
     */
    fun refusal(error: Throwable): String? = when (error) {
        is DashSdkError.PlatformWallet.ShieldedIdentityDebitPending -> DEBIT_PENDING
        is DashSdkError.PlatformWallet.InsufficientIdentityCredits ->
            "The identity's balance on Platform is too low for this shield: it needs " +
                "${formatCreditsExact(error.requiredCredits)} including the fee and has " +
                "${formatCreditsExact(error.availableCredits)}. Nothing was sent."
        is DashSdkError.PlatformWallet.IdentityBalanceUnavailable ->
            "The identity's balance could not be read from Platform, so the shield " +
                "was not attempted. Nothing was sent."
        is DashSdkError.PlatformWallet.ProtocolVersionNotActive ->
            "This network has not activated shielding from an identity yet. Nothing was sent."
        is DashSdkError.PlatformWallet.SdkPinnedBelowProtocolVersion ->
            "This app's SDK is pinned below the protocol version that shields from " +
                "an identity. Nothing was sent."
        else -> null
    }

    /**
     * The outcome of a `shieldedShieldFromIdentity` call that threw [error],
     * or returned when [error] is null. A returned call is "submitted": the
     * wallet proved the identity's balance at a block, and the activity row
     * stays pending until the scan sees the note.
     */
    fun outcome(error: Throwable?): ShieldedPaymentOutcome = when {
        error == null -> ShieldedPaymentOutcome.Submitted
        error is CancellationException -> throw error
        shieldedOperationMayHaveGoneThrough(error) -> ShieldedPaymentOutcome.MayHaveGoneThrough
        else -> ShieldedPaymentOutcome.Failed(
            refusal(error) ?: error.message ?: "Failed to shield from the identity.",
        )
    }
}

/**
 * Plain-language text for a durable identity shield (Type 21) record. The
 * status itself is decided in Rust; this only labels it. A record whose
 * signed bytes do not decode has no identity id: the wallet cannot resend
 * it, and until it is stopped it blocks new identity shields from every
 * identity in the wallet, so it gets its own wording.
 * ← Swift `ShieldedIdentityDebitRecoveryDisplay`.
 */
internal object IdentityShieldRecoveryDisplay {
    const val HEADER = "In-Progress Identity Shields"

    const val FOOTER =
        "Shields from an identity's balance whose outcome is not known yet. A " +
            "shielded sync checks each shield for its note; each row shows what " +
            "the wallet can still do with it."

    const val STOP_TITLE = "Stop retrying this shield?"

    fun statusTitle(status: ShieldedIdentityDebitRecoveryStatus, identityId: ByteArray?): String {
        if (identityId == null && status != ShieldedIdentityDebitRecoveryStatus.UNKNOWN) {
            return "Unreadable record"
        }
        return when (status) {
            ShieldedIdentityDebitRecoveryStatus.RETRYING -> "Retrying"
            ShieldedIdentityDebitRecoveryStatus.PARKED -> "Waiting for the note"
            ShieldedIdentityDebitRecoveryStatus.UNKNOWN -> "Outcome unknown"
        }
    }

    fun statusDetail(status: ShieldedIdentityDebitRecoveryStatus, identityId: ByteArray?): String {
        if (identityId == null) {
            return if (status == ShieldedIdentityDebitRecoveryStatus.UNKNOWN) {
                "Automatic retry was stopped. The identity that funded it is unknown, " +
                    "and it may still have executed."
            } else {
                "The identity that funded this shield is unknown and its saved " +
                    "transaction cannot be resent. It blocks new identity shields in " +
                    "this wallet until you stop retrying it."
            }
        }
        return when (status) {
            ShieldedIdentityDebitRecoveryStatus.RETRYING ->
                "Automatic retry is enabled. The wallet may resend the original " +
                    "signed shield during a shielded sync."
            ShieldedIdentityDebitRecoveryStatus.PARKED ->
                "Its identity nonce is used up, so it either already executed or " +
                    "never will. The wallet is waiting for the shielded scan to see its note."
            ShieldedIdentityDebitRecoveryStatus.UNKNOWN ->
                "Automatic retry was stopped. It may still have executed; the wallet " +
                    "keeps the record until the scan sees its note."
        }
    }

    /** Only an active record can be abandoned; an unknown one already was. */
    fun canStopRetrying(status: ShieldedIdentityDebitRecoveryStatus): Boolean =
        status != ShieldedIdentityDebitRecoveryStatus.UNKNOWN

    /** Shown before Stop Retrying. Stopping never cancels an earlier broadcast. */
    fun stopRetryingWarning(identityId: ByteArray?): String {
        val cancel = "This does not cancel the payment. It may already have executed, " +
            "or it may still execute later."
        return if (identityId == null) {
            "$cancel The identity it belongs to is unknown, so a new shield from any " +
                "identity in this wallet may become an additional debit if it comes " +
                "from that identity."
        } else {
            "$cancel The wallet stops resending it, and a new shield from this " +
                "identity may become an additional debit."
        }
    }

    /**
     * The identity's label from the wallet's rows ([labels], keyed by
     * lowercase hex id) when known, else a short id.
     */
    fun identityLabel(identityId: ByteArray?, labels: Map<String, String>): String {
        if (identityId == null) return "Unknown identity"
        val hex = identityId.toHex()
        return labels[hex] ?: (hex.take(12) + "…")
    }

    /** A record is identified by its account and activity id within a wallet. */
    fun key(record: ShieldedIdentityDebitRecoveryRecord): String =
        "${record.accountIndex}-${record.activityId.toHex()}"
}

/**
 * The records last read for one wallet. A failed read keeps the last
 * successful records, so an in-progress shield never disappears because one
 * read failed. ← Swift `RecoveryReadState`.
 */
internal data class RecoveryReadState<R>(
    val records: List<R> = emptyList(),
    val loaded: Boolean = false,
    val failureMessage: String? = null,
) {
    fun applySuccess(records: List<R>) = RecoveryReadState(records, loaded = true, failureMessage = null)

    fun applyFailure(message: String) = copy(failureMessage = message)

    /** The latest read succeeded and found nothing. */
    val isKnownEmpty: Boolean get() = loaded && failureMessage == null && records.isEmpty()
}

/** Wording for a top-up from the shielded pool (Type 22). ← Swift `ShieldedTopUpSubmitCheck`. */
internal object ShieldedTopUpMessages {
    const val FEE_NOTE =
        "The fee is spent from the shielded notes on top of the amount, so amount " +
            "plus fee must fit the spendable shielded balance. The fee is more than " +
            "the minimum if the wallet has to combine more than two notes. Building " +
            "the proof takes tens of seconds."

    const val PENDING_HEADER = "Pending Shielded Top-Ups"

    const val PENDING_FOOTER =
        "Not confirmed. They may already have gone through, so do not submit them " +
            "again. The wallet checks them on each shielded sync."

    /** The proof covers the spent notes, not that this transition added the amount. */
    fun completedText(amount: Long): String =
        "The spent notes were confirmed. Requested: ${formatCreditsExact(amount)}."

    /**
     * [pendingListedBelow]: this identity has pending top-ups shown under the
     * result. None of them is claimed to be this submission (two can have the
     * same amount), and without any the text promises no entry.
     */
    fun unconfirmedText(amount: Long, pendingListedBelow: Boolean): String {
        val warning = "The top-up of ${formatCreditsExact(amount)} could not be confirmed. " +
            "Do not retry: it may already be on chain. The wallet checks it on each " +
            "shielded sync."
        return if (pendingListedBelow) {
            "$warning Pending top-ups for this identity are listed below."
        } else {
            warning
        }
    }

    /** Refusals worded for this screen; null for any other error, whose own message is shown. */
    fun refusal(error: Throwable): String? = when (error) {
        is DashSdkError.PlatformWallet.ProtocolVersionNotActive ->
            "This network has not activated topping up an identity from the shielded " +
                "pool yet. Nothing was sent."
        is DashSdkError.PlatformWallet.SdkPinnedBelowProtocolVersion ->
            "This app's SDK is pinned below the protocol version that tops up an " +
                "identity from the shielded pool. Nothing was sent."
        else -> null
    }

    /**
     * The outcome of a `shieldedIdentityTopUpFromPool` call that threw
     * [error], or returned when [error] is null.
     */
    fun outcome(error: Throwable?): ShieldedPaymentOutcome = when {
        error == null -> ShieldedPaymentOutcome.Submitted
        error is CancellationException -> throw error
        shieldedOperationMayHaveGoneThrough(error) -> ShieldedPaymentOutcome.MayHaveGoneThrough
        else -> ShieldedPaymentOutcome.Failed(refusal(error) ?: error.message ?: "Top up failed")
    }
}

/**
 * The line under a Check Now button, from that call's own outcome.
 * Null when there is nothing to say. ← Swift `ShieldedCheckNowStatus`.
 */
internal sealed interface CheckNowStatus {
    data object Idle : CheckNowStatus
    data object Checking : CheckNowStatus
    /** `syncShieldedNow` returned; not proof that this wallet was scanned. */
    data object Finished : CheckNowStatus
    data class Failed(val message: String) : CheckNowStatus

    val text: String?
        get() = when (this) {
            Idle, Checking -> null
            // The sync call does not say whether this wallet's scan
            // succeeded, so this claims only that one was asked for.
            Finished -> "Sync requested. The list shows the wallet's records as they " +
                "are now; this screen cannot tell whether the scan for this wallet succeeded."
            is Failed -> "Check failed: $message"
        }

    val isFailure: Boolean get() = this is Failed
}
