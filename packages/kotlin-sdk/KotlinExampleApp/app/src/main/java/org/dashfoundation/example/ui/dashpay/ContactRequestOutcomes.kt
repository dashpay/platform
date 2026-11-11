package org.dashfoundation.example.ui.dashpay

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.tokens.Dashpay

/**
 * How a contact request send or accept that threw ended, for the DashPay
 * screens. Follows the send screen (and iOS `SendViewModel.swift`): a shielded
 * spend whose result could not be confirmed (code 18) may already be on chain
 * (Rust keeps its notes reserved), and so may a state transition the SDK could
 * not confirm after it may have been accepted (code 20), so the screen keeps
 * either pending and never offers a fresh submission; the next sync settles
 * it. For a public request the wallet holds the pair itself after a 20, the
 * signed transition kept, until a public contact request sync settles it,
 * and refuses every later request or accept to it with code 76: the screen
 * shows "may have gone through; it is being checked" until the wallet's
 * per-contact read says settled. A pair Rust reports as
 * held for a private request (code 71), which a public send or accept can
 * also meet, sent nothing new: the screen shows it as pending until the
 * wallet's per-contact read says the pair is free. Unrestored private
 * payments (code 72) hold the wallet, not one contact: the screen shows
 * "setting up" until the wallet's readiness reads restored, or why the last
 * restoration failed; no private sync lifts it. A private store read that did
 * not finish (code 74) sent nothing and is not pending: the user may try
 * again. Every other error is an ordinary failure the user may retry; a
 * storage backend that cannot protect private payments (73) and an SDK
 * pinned below the protocol version (75) are shown as configuration problems.
 * ← Swift `ContactRequestSubmissionOutcome` (`ContactRequestOutcomes.swift`).
 */
sealed interface ContactRequestSubmissionOutcome {
    /** The private reply may have gone through: do not submit again. */
    data object MayHaveGoneThrough : ContactRequestSubmissionOutcome

    /** The wallet holds the pair for a private request; nothing was sent. */
    data object PrivateRequestPending : ContactRequestSubmissionOutcome

    /**
     * The wallet holds the pair for a public request whose outcome is not
     * proved yet (code 76); nothing was signed. A public contact request
     * sync settles it.
     */
    data object PublicRequestUnconfirmed : ContactRequestSubmissionOutcome

    /** The wallet's private payments are not restored yet; nothing was sent. */
    data object PrivatePaymentsNotRestored : ContactRequestSubmissionOutcome

    /**
     * The private store read every contact request runs first did not
     * finish; nothing was sent, and trying again continues it.
     */
    data object PrivateStoreNotRead : ContactRequestSubmissionOutcome

    /** It failed; the message is the error's. */
    data class Failed(val message: String) : ContactRequestSubmissionOutcome

    companion object {
        const val MAY_HAVE_GONE_THROUGH_MESSAGE = "This may have gone through. It shows up " +
            "after the next sync confirms it; do not submit it again."

        /**
         * A public request or accept that may have gone through, as the
         * wallet holds it until a sync settles its outcome.
         */
        const val PUBLIC_REQUEST_UNCONFIRMED_MESSAGE = "This request may have gone through; " +
            "it is being checked. The next sync settles it."

        const val PRIVATE_REQUEST_PENDING_MESSAGE = "A private request to this contact is " +
            "pending. It clears once the request is read back from the private store, " +
            "or once its payment can no longer land, which can take a while. Sync " +
            "private requests to check."

        const val SETTING_UP_PRIVATE_PAYMENTS_MESSAGE = "Setting up this wallet's private " +
            "payments. Contact requests wait until it finishes."

        const val PRIVATE_STORE_NOT_READ_MESSAGE = "Still reading private contact requests. " +
            "Nothing was sent; try again in a moment."

        /**
         * Code 73 is a configuration error of the app's storage, which
         * declares what a private payment needs; shown as such.
         */
        const val RECOVERY_UNSUPPORTED_MESSAGE = "This storage backend can't protect " +
            "private payments, so private contact requests are off."

        /**
         * Code 75 is the developer's setting: only raising or removing the
         * SDK's protocol version pin lifts it.
         */
        const val SDK_PINNED_MESSAGE = "This build's SDK is pinned below the protocol " +
            "version private contact requests need. Raise or remove the SDK's protocol " +
            "version pin."

        /** The wallet's last restoration of its private payments failed. */
        fun restoreFailedMessage(reason: String): String =
            "Restoring this wallet's private payments failed: $reason. Contact requests " +
                "wait until a restore succeeds."

        /** The app could not open the shielded store, which restores the wallet's private payments. */
        fun shieldedStoreFailedMessage(reason: String): String =
            "The shielded store could not be opened: $reason. Contact requests wait " +
                "until it opens."

        /**
         * The notice shown while the wallet's private payments are not
         * restored: why opening the shielded store failed, why the last
         * restoration failed, or that they are being set up.
         */
        fun restoringMessage(restoreFailure: String?, shieldedStoreFailure: String?): String = when {
            shieldedStoreFailure != null -> shieldedStoreFailedMessage(shieldedStoreFailure)
            restoreFailure != null -> restoreFailedMessage(restoreFailure)
            else -> SETTING_UP_PRIVATE_PAYMENTS_MESSAGE
        }

        fun of(error: Throwable): ContactRequestSubmissionOutcome = when (error) {
            is DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed,
            is DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed,
            is DashSdkError.StateTransitionResultUnconfirmed,
            -> MayHaveGoneThrough
            is DashSdkError.PlatformWallet.PrivateContactRequestPending -> PrivateRequestPending
            is DashSdkError.PlatformWallet.PublicContactRequestUnconfirmed -> PublicRequestUnconfirmed
            is DashSdkError.PlatformWallet.PrivateContactPaymentsNotRestored -> PrivatePaymentsNotRestored
            is DashSdkError.PlatformWallet.PrivateContactStoreNotRead -> PrivateStoreNotRead
            is DashSdkError.PlatformWallet.PrivateContactRecoveryUnsupported ->
                Failed(RECOVERY_UNSUPPORTED_MESSAGE)
            is DashSdkError.PlatformWallet.SdkPinnedBelowProtocolVersion -> Failed(SDK_PINNED_MESSAGE)
            else -> Failed(error.message ?: "unknown error")
        }

        /**
         * The message a request row shows: the restoring notice while the
         * wallet's private payments are not restored, the being-checked
         * notice while a request to the contact awaits its outcome, the
         * pending notice while the wallet holds the pair for a private
         * request, the unavailable notice while the wallet could not be
         * read, and whatever error an earlier attempt left otherwise.
         */
        fun rowMessage(
            pending: Boolean,
            awaitingOutcome: Boolean = false,
            restoring: Boolean = false,
            restoreFailure: String? = null,
            shieldedStoreFailure: String? = null,
            unavailable: Boolean = false,
            error: String?,
        ): String? = when {
            restoring -> restoringMessage(restoreFailure, shieldedStoreFailure)
            awaitingOutcome -> PUBLIC_REQUEST_UNCONFIRMED_MESSAGE
            pending -> PRIVATE_REQUEST_PENDING_MESSAGE
            unavailable -> PrivateRepliesPendingSnapshot.UNAVAILABLE_MESSAGE
            else -> error
        }
    }
}

/**
 * Which contacts (lower hex ids) the wallet holds a pair with for a private
 * request, and which for a public request awaiting its outcome, as the
 * screens last read it ([contactRequestHolds]: per contact, the private pair
 * read, which answers as a send decides it, and the public one). While either
 * holds a pair, Rust refuses another send or reply to that contact, so the
 * screens turn off
 * what would submit one (Accept, Ignore, Send). A failed read is not "nothing
 * pending": the last successful snapshot stays, and those controls stay off
 * for every contact until a read covering it succeeds.
 *
 * A send or accept Rust refused as pending (code 71) marks the contact
 * pending at once; the next successful read covering it decides, as it reads
 * what the refusal read. Controls for a pair re-enable only when that read
 * says it is free: a payment that will never land frees it only once the
 * wallet proves so, which the read shows. So does a send or accept that may
 * have gone through (18 or 20), or was refused as awaiting a public request's
 * outcome (76): the contact shows "being checked" at once, and a read decides
 * it the same way, so a reload whose sync did not settle the wallet's hold
 * keeps it so. A read always covers both holds: a read of the private hold
 * alone would offer a public request awaiting its outcome again.
 *
 * Whether the wallet's private payments are restored is the wallet's
 * readiness (`Dashpay.privateContactReadiness`), not anything the screens
 * conclude: a code 72 refusal, or a readiness read that says not restored,
 * shows "setting up" (or why the last restoration failed) for every contact,
 * and only a readiness read that says restored lifts it. A private sync
 * changes neither on its own.
 * ← Swift `PrivateRepliesPendingSnapshot` (`ContactRequestOutcomes.swift`).
 */
data class PrivateRepliesPendingSnapshot(
    /** The contacts the last successful read found held for a private request. */
    val pending: Set<String> = emptySet(),
    /** The contacts the last successful read covered. */
    val covered: Set<String> = emptySet(),
    /** False after a failed read, until a read succeeds. */
    val available: Boolean = true,
    /** Contacts a send or accept was refused for as pending (code 71) since the last successful read covering them. */
    val refused: Set<String> = emptySet(),
    /** The contacts the last successful read found held for a public request awaiting its outcome. */
    val publicUnconfirmed: Set<String> = emptySet(),
    /**
     * Contacts a send or accept may have gone through to (18, 20), or was
     * refused for as awaiting a public request's outcome (76), since the last
     * successful read covering them.
     */
    val awaitingOutcome: Set<String> = emptySet(),
    /** The wallet's readiness for the identity, as last read; null until a read succeeds. */
    val readiness: Dashpay.PrivateContactReadiness? = null,
    /** Why the last restoration failed, as last read with the readiness; null when none failed or it could not be read. */
    val restoreFailure: String? = null,
    /** Set by a code 72 refusal; cleared only by a readiness read that says restored. */
    val restoringRefused: Boolean = false,
) {
    /** Whether the last read failed. */
    val isUnavailable: Boolean get() = !available

    /**
     * This snapshot after a read of both holds for [contactIds]
     * ([contactRequestHolds]), or after a failed one ([read] null). A
     * successful read decides each contact it covered, a refusal or a
     * submission that may have gone through included.
     */
    fun recorded(read: ContactRequestHolds?, contactIds: Collection<String>): PrivateRepliesPendingSnapshot =
        if (read == null) {
            copy(available = false)
        } else {
            val decided = contactIds.toSet()
            copy(
                pending = read.privatelyHeld,
                publicUnconfirmed = read.publicUnconfirmed,
                covered = decided,
                refused = refused - decided,
                awaitingOutcome = awaitingOutcome - decided,
                available = true,
            )
        }

    /**
     * This snapshot after a send or accept to [contactId] threw with
     * [outcome]: a 71 refusal marks it pending and a 72 refusal holds every
     * contact, as [withRefusal] and [withRestoringRefusal] do; one that may
     * have gone through, or a 76 refusal, marks it awaiting its outcome.
     * Nothing else changes a hold.
     */
    fun withOutcome(outcome: ContactRequestSubmissionOutcome, contactId: String): PrivateRepliesPendingSnapshot =
        when (outcome) {
            ContactRequestSubmissionOutcome.PrivateRequestPending -> withRefusal(contactId)
            ContactRequestSubmissionOutcome.MayHaveGoneThrough,
            ContactRequestSubmissionOutcome.PublicRequestUnconfirmed,
            -> copy(awaitingOutcome = awaitingOutcome + contactId)
            ContactRequestSubmissionOutcome.PrivatePaymentsNotRestored -> withRestoringRefusal()
            ContactRequestSubmissionOutcome.PrivateStoreNotRead,
            is ContactRequestSubmissionOutcome.Failed,
            -> this
        }

    /** This snapshot after a send or accept to [contactId] was refused as pending (code 71). */
    fun withRefusal(contactId: String): PrivateRepliesPendingSnapshot = copy(refused = refused + contactId)

    /** This snapshot after a send or accept refused because the private payments are not restored (code 72). */
    fun withRestoringRefusal(): PrivateRepliesPendingSnapshot = copy(restoringRefused = true)

    /**
     * This snapshot after a read of the wallet's readiness and, when it flags
     * a failed restoration, the reason read with it; a null readiness (the
     * read failed) keeps what the last read said. A readiness that says
     * restored lifts a code 72 refusal and its failure.
     */
    fun withReadiness(
        read: Dashpay.PrivateContactReadiness?,
        restoreFailure: String? = null,
    ): PrivateRepliesPendingSnapshot =
        if (read == null) {
            this
        } else {
            copy(
                readiness = read,
                restoreFailure = restoreFailure.takeIf { read.privatePaymentsRestoreFailed },
                restoringRefused = restoringRefused && !read.privatePaymentsRestored,
            )
        }

    /**
     * Whether the wallet's private payments are being restored: a send or
     * accept was refused with code 72, or the last readiness read says they
     * are not restored. Every contact waits while they are.
     */
    val isRestoringPrivatePayments: Boolean
        get() = restoringRefused || readiness?.privatePaymentsRestored == false

    /**
     * Whether [contactId] is pending: the last successful read found the
     * pair held, or a send or accept to it was refused as pending since.
     */
    fun isPending(contactId: String): Boolean = contactId in pending || contactId in refused

    /**
     * Whether a request to [contactId] awaits its outcome: the last
     * successful read found the pair held for a public request awaiting it,
     * or a send or accept to the contact may have gone through, or was
     * refused so (76), since.
     */
    fun isAwaitingOutcome(contactId: String): Boolean =
        contactId in publicUnconfirmed || contactId in awaitingOutcome

    /**
     * Whether a send or reply to [contactId] may be offered: a successful
     * read covered it and found the pair free of both holds.
     */
    fun allowsSubmission(contactId: String): Boolean =
        available && contactId in covered && !isPending(contactId) &&
            !isAwaitingOutcome(contactId) && !isRestoringPrivatePayments

    companion object {
        const val UNAVAILABLE_MESSAGE = "Could not read whether a private request to this " +
            "contact is pending. Sync private requests to check again."
    }
}

/**
 * One read of the wallet's two per-contact holds, as lower hex ids.
 * ← Swift `ContactRequestHolds` (`ContactRequestOutcomes.swift`).
 */
data class ContactRequestHolds(
    /** The contacts whose pair the wallet holds for a private request. */
    val privatelyHeld: Set<String> = emptySet(),
    /** The contacts whose pair the wallet holds for a public request awaiting its outcome. */
    val publicUnconfirmed: Set<String> = emptySet(),
)

/**
 * Read the wallet's two holds: the pairs held for a private request
 * ([readPrivatelyHeld]) and those held for a public request awaiting its
 * outcome ([readPublicUnconfirmed]). Null when either read failed, which the
 * screens keep apart from "nothing held" ([PrivateRepliesPendingSnapshot]);
 * an identity the wallet does not manage reads as nothing held in both.
 * ← Swift `contactRequestHolds(contactIds:privatePairHeld:publicUnconfirmed:)`.
 */
suspend fun contactRequestHolds(
    readPrivatelyHeld: suspend () -> Set<String>,
    readPublicUnconfirmed: suspend () -> Set<String>,
): ContactRequestHolds? = try {
    ContactRequestHolds(
        privatelyHeld = readPrivatelyHeld(),
        publicUnconfirmed = readPublicUnconfirmed(),
    )
} catch (e: CancellationException) {
    throw e
} catch (e: Exception) {
    null
}

/**
 * The DashPay screens' readiness refresh, one flow for Requests and Add
 * Contact. The wallet owns the recovery: on a store already open, a repeat
 * `configureShielded` retries restoring every wallet whose private payments
 * are still held, a restoration that failed after the store opened included,
 * and defers it to a task while a scan holds the store. So a refresh asks for
 * it whenever the payments are not restored, or the store did not open, and
 * only then reads the readiness it shows: the readiness is read after the
 * attempt, never cached from before it. While the payments stay unrestored (a
 * deferred restoration completes later, changing nothing a screen observes but
 * the end of the shielded scan that held the store), the screen keeps
 * refreshing on a timer for as long as it is shown, and again when a shielded
 * sync pass ends, so the gate lifts without a bind, a restart or a submitted
 * request. The screens change no hold themselves.
 * ← Swift `PrivatePaymentsRecovery` (`ContactRequestOutcomes.swift`).
 */
object PrivatePaymentsRecovery {
    /*
     * The cadence of the automatic refreshes while the payments stay
     * unrestored: every FAST_RECHECK_INTERVAL_MS for the first FAST_RECHECKS
     * of them (a minute), then every SLOW_RECHECK_INTERVAL_MS, for as long as
     * the screen is shown. It never stops on a count: a shielded scan can hold
     * the store for minutes, and the wallet runs a restoration it deferred
     * only once the scan frees it. Each refresh is local and cheap, reads of
     * in-memory wallet state and a repeat `configureShielded` that checks
     * which wallets are still held and queues at most one deferred
     * restoration per wallet, with no network call, so an open-ended poll at
     * this cadence costs little. It stops when the readiness reads restored
     * or the screen leaves the composition; a screen for another identity
     * starts its own.
     */

    /** The automatic refreshes made at the fast cadence. */
    const val FAST_RECHECKS = 20

    /** The pause between automatic refreshes for the first minute. */
    const val FAST_RECHECK_INTERVAL_MS = 3_000L

    /** The pause between automatic refreshes after the first minute. */
    const val SLOW_RECHECK_INTERVAL_MS = 15_000L

    /**
     * Whether a refresh asks the wallet to retry before it reads the
     * readiness it shows: the store did not open last time, or the last
     * readiness read says the payments are not restored.
     */
    fun retries(readiness: Dashpay.PrivateContactReadiness?, storeFailure: String?): Boolean =
        storeFailure != null || readiness?.privatePaymentsRestored == false

    /**
     * The pause before the next automatic refresh after [attempt] of them, or
     * null when there is none: the readiness reads restored, or no read
     * succeeded yet (left to the refresh the screen makes anyway).
     */
    fun recheckDelayMs(after: Dashpay.PrivateContactReadiness?, attempt: Int): Long? = when {
        after?.privatePaymentsRestored != false -> null
        attempt < FAST_RECHECKS -> FAST_RECHECK_INTERVAL_MS
        else -> SLOW_RECHECK_INTERVAL_MS
    }

    /**
     * Wait for the next automatic refresh and say whether to make it: false at
     * once when none is due ([recheckDelayMs]), true after the pause. The
     * screens pass `delay`; a screen leaving the composition cancels the wait,
     * and the cancellation ends its effect.
     */
    suspend fun waitForRecheck(
        after: Dashpay.PrivateContactReadiness?,
        attempt: Int,
        sleep: suspend (Long) -> Unit = { delay(it) },
    ): Boolean {
        val pause = recheckDelayMs(after, attempt) ?: return false
        sleep(pause)
        return true
    }

    /** A refresh's readiness read, null when it failed, and why the store did not open, or null. */
    data class Refreshed(val read: PrivateContactReadinessRead?, val storeFailure: String?)

    /**
     * One refresh. [lastReadiness] is the readiness the screen shows now;
     * [readReadiness] reads the wallet's (with the reason a restoration
     * failed); [configureAgain] asks the wallet to retry and returns why the
     * store did not open, or null. When [retries] says so, the wallet is asked
     * first and the readiness is read after it; otherwise it is read once.
     */
    suspend fun refresh(
        lastReadiness: Dashpay.PrivateContactReadiness?,
        storeFailure: String?,
        readReadiness: suspend () -> PrivateContactReadinessRead?,
        configureAgain: suspend () -> String?,
    ): Refreshed {
        if (retries(lastReadiness, storeFailure)) {
            val failure = configureAgain()
            return Refreshed(readReadiness(), failure)
        }
        val read = readReadiness()
        if (retries(read?.readiness, storeFailure)) {
            val failure = configureAgain()
            return Refreshed(readReadiness(), failure)
        }
        return Refreshed(read, storeFailure)
    }
}

/**
 * What the request list's private-store refresh tells the user. A network
 * that does not serve the store yet (before protocol version 15) is expected
 * and stays quiet; an operational failure, or requests read but not saved,
 * is shown so the user knows to refresh again.
 * ← Swift `PrivateContactRefreshNotice` (`ContactRequestOutcomes.swift`).
 */
object PrivateContactRefreshNotice {
    /** The notice a pass that read the store leaves, or null when every identity's requests reached disk. */
    fun notice(unpersistedIdentityCount: Long): String? {
        if (unpersistedIdentityCount <= 0) return null
        val who = if (unpersistedIdentityCount == 1L) "identity were" else "identities were"
        return "Private requests for $unpersistedIdentityCount $who read but not saved. " +
            "Pull to refresh again."
    }

    /** The notice a pass that failed leaves, or null when the network does not serve the private store yet. */
    fun notice(error: Throwable): String? = when (error) {
        is DashSdkError.PlatformWallet.ProtocolVersionNotActive -> null
        is DashSdkError.PlatformWallet.SdkPinnedBelowProtocolVersion ->
            ContactRequestSubmissionOutcome.SDK_PINNED_MESSAGE
        else -> "Private requests could not be read: ${error.message ?: "unknown error"}"
    }
}
