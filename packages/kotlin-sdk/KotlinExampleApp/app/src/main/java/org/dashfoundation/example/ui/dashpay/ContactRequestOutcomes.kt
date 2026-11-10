package org.dashfoundation.example.ui.dashpay

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.tokens.Dashpay

/**
 * How a contact request send or accept that threw ended, for the DashPay
 * screens. Follows the send screen (and iOS `SendViewModel.swift`): a shielded
 * spend whose result could not be confirmed (code 18) may already be on chain
 * (Rust keeps its notes reserved), and so may a state transition the SDK could
 * not confirm after it may have been accepted (code 20), so the screen keeps
 * either pending and never offers a fresh submission; the next sync settles
 * it. A pair Rust reports as
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
            is DashSdkError.PlatformWallet.PrivateContactPaymentsNotRestored -> PrivatePaymentsNotRestored
            is DashSdkError.PlatformWallet.PrivateContactStoreNotRead -> PrivateStoreNotRead
            is DashSdkError.PlatformWallet.PrivateContactRecoveryUnsupported ->
                Failed(RECOVERY_UNSUPPORTED_MESSAGE)
            is DashSdkError.PlatformWallet.SdkPinnedBelowProtocolVersion -> Failed(SDK_PINNED_MESSAGE)
            else -> Failed(error.message ?: "unknown error")
        }

        /**
         * The message a request row shows: the restoring notice while the
         * wallet's private payments are not restored, the pending notice
         * while the wallet holds the pair, the unavailable notice while the
         * wallet could not be read, and whatever error an earlier attempt
         * left otherwise.
         */
        fun rowMessage(
            pending: Boolean,
            restoring: Boolean = false,
            restoreFailure: String? = null,
            shieldedStoreFailure: String? = null,
            unavailable: Boolean = false,
            error: String?,
        ): String? = when {
            restoring -> restoringMessage(restoreFailure, shieldedStoreFailure)
            pending -> PRIVATE_REQUEST_PENDING_MESSAGE
            unavailable -> PrivateRepliesPendingSnapshot.UNAVAILABLE_MESSAGE
            else -> error
        }
    }
}

/**
 * Which contacts (lower hex ids) the wallet holds a pair with for a private
 * request, as the screens last read it (`privateRepliesPending`, one pair
 * read per contact, which answers as a send decides it). While one is held,
 * Rust refuses another send or reply to that contact, so the screens turn off
 * what would submit one (Accept, Ignore, Send). A failed read is not "nothing
 * pending": the last successful snapshot stays, and those controls stay off
 * for every contact until a read covering it succeeds.
 *
 * A send or accept Rust refused as pending (code 71) marks the contact
 * pending at once; the next successful read covering it decides, as it reads
 * what the refusal read. Controls for a pair re-enable only when that read
 * says it is free: a payment that will never land frees it only once the
 * wallet proves so, which the read shows.
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
    /** The held contacts the last successful read found. */
    val pending: Set<String> = emptySet(),
    /** The contacts the last successful read covered. */
    val covered: Set<String> = emptySet(),
    /** False after a failed read, until a read succeeds. */
    val available: Boolean = true,
    /** Contacts a send or accept was refused for as pending (code 71) since the last successful read covering them. */
    val refused: Set<String> = emptySet(),
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
     * This snapshot after a read of [contactIds]: [read] holds the held
     * ones, or is null when the read failed. A successful read decides each
     * contact it covered, a code 71 refusal included.
     */
    fun recorded(read: Set<String>?, contactIds: Collection<String>): PrivateRepliesPendingSnapshot =
        if (read == null) {
            copy(available = false)
        } else {
            copy(pending = read, covered = contactIds.toSet(), refused = refused - contactIds.toSet(), available = true)
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
     * Whether a send or reply to [contactId] may be offered: a successful
     * read covered it and found the pair free.
     */
    fun allowsSubmission(contactId: String): Boolean =
        available && contactId in covered && !isPending(contactId) && !isRestoringPrivatePayments

    companion object {
        const val UNAVAILABLE_MESSAGE = "Could not read whether a private request to this " +
            "contact is pending. Sync private requests to check again."
    }
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
 * deferred restoration completes later, changing nothing a screen observes),
 * the screen refreshes again on a bounded timer, so the gate lifts without a
 * bind, a restart or a submitted request. The screens change no hold
 * themselves.
 * ← Swift `PrivatePaymentsRecovery` (`ContactRequestOutcomes.swift`).
 */
object PrivatePaymentsRecovery {
    /** How many times a screen refreshes again on its own while the payments stay unrestored. */
    const val RECHECK_LIMIT = 20

    /** How long a screen waits before it refreshes again. */
    const val RECHECK_INTERVAL_MS = 3_000L

    /**
     * Whether a refresh asks the wallet to retry before it reads the
     * readiness it shows: the store did not open last time, or the last
     * readiness read says the payments are not restored.
     */
    fun retries(readiness: Dashpay.PrivateContactReadiness?, storeFailure: String?): Boolean =
        storeFailure != null || readiness?.privatePaymentsRestored == false

    /**
     * Whether a screen refreshes again on its own after [attempt] automatic
     * refreshes: the payments are still not restored and the limit is not
     * reached. A failed read (null) is left to the next refresh the screen
     * makes anyway.
     */
    fun rechecks(after: Dashpay.PrivateContactReadiness?, attempt: Int): Boolean =
        after?.privatePaymentsRestored == false && attempt < RECHECK_LIMIT

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
