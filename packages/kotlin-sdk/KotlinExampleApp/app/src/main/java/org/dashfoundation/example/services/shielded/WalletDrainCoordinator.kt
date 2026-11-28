package org.dashfoundation.example.services.shielded

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.PaymentDelivery
import org.dashfoundation.dashsdk.errors.PaymentRecovery
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.AssetLockBuiltOutPoint
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.dashsdk.wallet.WalletDrainOutcome
import org.dashfoundation.dashsdk.wallet.WalletDrainOutpoint
import org.dashfoundation.dashsdk.wallet.WalletDrainRecoveryAction
import org.dashfoundation.dashsdk.wallet.WalletDrainRules
import org.dashfoundation.dashsdk.wallet.WalletDrainStatusRead
import org.dashfoundation.dashsdk.wallet.shieldedFundFromWalletDrain
import org.dashfoundation.dashsdk.wallet.shieldedWalletDrainStatus
import org.dashfoundation.example.services.shielded.ShieldedFundFromAssetLockCoordinator.StartFundingResult
import org.dashfoundation.example.util.PaymentFailure
import java.util.UUID

/**
 * The three SDK calls a Max transfer makes, so the coordinator can be
 * tested without a loaded wallet. Production is [PlatformWalletManager]
 * through [asWalletDrainWallet].
 */
interface WalletDrainWallet {
    /** `shieldedFundFromWalletDrain` for [record]. */
    suspend fun drain(record: WalletDrainOperationRecord): WalletDrainOutcome

    /** `shieldedResumeFundFromAssetLock` of [outpoint], paying [record]'s recipient. */
    suspend fun resume(record: WalletDrainOperationRecord, outpoint: WalletDrainOutpoint)

    /** `shieldedWalletDrainStatus` for [record]. Throws when the read fails. */
    suspend fun status(record: WalletDrainOperationRecord): WalletDrainOutcome
}

/** [WalletDrainWallet] over this manager's wrappers. */
fun PlatformWalletManager.asWalletDrainWallet(): WalletDrainWallet = object : WalletDrainWallet {
    override suspend fun drain(record: WalletDrainOperationRecord): WalletDrainOutcome =
        shieldedFundFromWalletDrain(
            walletId = record.walletId,
            operationId = record.operationId,
            fundingAccountIndex = record.fundingAccountIndex,
            recipientRaw43 = record.recipientRaw43,
        )

    override suspend fun resume(record: WalletDrainOperationRecord, outpoint: WalletDrainOutpoint) =
        shieldedResumeFundFromAssetLock(
            walletId = record.walletId,
            outPointTxid = outpoint.txidWire,
            outPointVout = outpoint.vout,
            recipientRaw43 = record.recipientRaw43,
        )

    override suspend fun status(record: WalletDrainOperationRecord): WalletDrainOutcome =
        shieldedWalletDrainStatus(walletId = record.walletId, operationId = record.operationId)
}

/**
 * The rules the coordinator asks Rust for, behind a seam: a JVM unit test
 * cannot load the native library. Production is [WalletDrainRules].
 */
interface WalletDrainRuleSet {
    fun keepsStoredId(read: WalletDrainStatusRead, attemptOverlappedRead: Boolean): Boolean

    fun recoveryAction(
        attemptInFlight: Boolean,
        lastRead: WalletDrainStatusRead?,
        readNamedAssetLock: Boolean,
    ): WalletDrainRecoveryAction

    fun attemptFinished(state: WalletDrainOutcome.State): Boolean

    /** Rust's rules, through `WalletDrainNative`. */
    object Native : WalletDrainRuleSet {
        override fun keepsStoredId(read: WalletDrainStatusRead, attemptOverlappedRead: Boolean) =
            WalletDrainRules.keepsStoredId(read, attemptOverlappedRead)

        override fun recoveryAction(
            attemptInFlight: Boolean,
            lastRead: WalletDrainStatusRead?,
            readNamedAssetLock: Boolean,
        ) = WalletDrainRules.recoveryAction(attemptInFlight, lastRead, readNamedAssetLock)

        override fun attemptFinished(state: WalletDrainOutcome.State) =
            WalletDrainRules.attemptFinished(state)
    }
}

/**
 * The error a drain attempt fails with when Rust did not report it finished.
 * [recovery] is Rust's class of the failure, which [PaymentFailure] reads
 * instead of treating this app-side error as unclassified.
 */
class WalletDrainAttemptException(message: String, val recovery: PaymentRecovery) : Exception(message)

/**
 * Starts, resumes and reconciles Core to shielded "Max" transfers. Port of
 * `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Services/WalletDrainCoordinator.swift`.
 *
 * Kotlin decides nothing about amounts, inputs or fees, and nothing about
 * what a status means. It stores each operation id before the drain is
 * called ([WalletDrainOperationStore]), calls the SDK wrappers, and does
 * what Rust's rules ([WalletDrainRuleSet]) return for the facts it gathers:
 * whether one of its attempts is running, and what the last status read
 * said. Execution runs through the [ShieldedFundFromAssetLockCoordinator],
 * so the per-wallet single-flight rule and the progress screen are shared
 * with the fixed-amount shield.
 *
 * Only [start] calls the drain. A resume calls the exact-outpoint resume
 * with the asset lock the last status read named, which can never build a
 * new lock; a repeated drain call could, while Rust's state for the id is
 * still "not sent".
 *
 * A stored id is removed by [reconcile], from a status Rust returned, and by
 * the SDK's wallet-deletion pass (one more exception: a start the funding
 * coordinator refused before running anything).
 */
class WalletDrainCoordinator(
    private val store: WalletDrainOperationStore,
    private val fundingCoordinator: ShieldedFundFromAssetLockCoordinator,
    private val rules: WalletDrainRuleSet = WalletDrainRuleSet.Native,
    private val now: () -> Long = System::currentTimeMillis,
    private val newOperationId: () -> UUID = UUID::randomUUID,
) {
    /** One stored id with what the last status read reported for it. */
    data class Entry(
        val record: WalletDrainOperationRecord,
        /**
         * The asset lock the last successful read named. Null when the read
         * failed or named none.
         */
        val outpoint: WalletDrainOutpoint?,
        /**
         * What the last status read produced. Null before any read has
         * finished: a stored id is kept in either case, so its presence
         * alone says nothing about Rust.
         */
        val lastRead: WalletDrainStatusRead?,
        /** The last read's error, when it failed. */
        val readError: String?,
        /**
         * Set when Rust's rule said to forget the id but removing it failed.
         * The stored id is all that is left; reconcile retries the removal.
         */
        val removalError: String?,
    ) {
        val operationId: UUID get() = record.operationId

        companion object {
            /** A stored id no status read has reported on yet. */
            fun unread(record: WalletDrainOperationRecord) =
                Entry(record, outpoint = null, lastRead = null, readError = null, removalError = null)
        }
    }

    /** The one action a saved-transfer row offers. */
    sealed interface RowAction {
        /** An attempt for this operation is running. */
        data object InProgress : RowAction

        /** Resume the asset lock at this exact outpoint. */
        data class Resume(val outpoint: WalletDrainOutpoint) : RowAction

        /** Read the status again. Never spends. */
        data object CheckAgain : RowAction
    }

    /**
     * The status line a saved-transfer row shows. Wording only: each case
     * says what the entry's last read (or the running attempt) supports.
     */
    sealed interface RowStatus {
        val text: String

        data object InProgress : RowStatus {
            override val text = "In progress."
        }

        data object Unfinished : RowStatus {
            override val text =
                "Unfinished: resume to finish it. A new Max cannot start until it does."
        }

        data object UnfinishedLockUnknown : RowStatus {
            override val text = "Unfinished, but the status named no asset lock yet: check again."
        }

        data object Checking : RowStatus {
            override val text = "Checking status…"
        }

        data class ReadFailed(val error: String) : RowStatus {
            override val text = "Status could not be read: $error"
        }

        data class FinishedNotCleared(val error: String?) : RowStatus {
            override val text = error?.let { "Finished; the saved record could not be cleared: $it" }
                ?: "Finished; the saved record is not cleared yet."
        }

        data class NotSentNotCleared(val error: String) : RowStatus {
            override val text = "Not sent; the saved record could not be cleared: $error"
        }

        data object NotSentWhileAttemptStarted : RowStatus {
            override val text =
                "Nothing was sent when last read, but an attempt was starting: check again."
        }
    }

    sealed interface StartResult {
        data class Started(val controller: ShieldedFundFromAssetLockController) : StartResult

        data class BlockedByOtherWalletFunding(
            val blocker: ShieldedFundFromAssetLockController,
        ) : StartResult

        /** The id could not be stored, so nothing was called. */
        data class MarkerWriteFailed(val message: String) : StartResult
    }

    private val _entries = MutableStateFlow<List<Entry>>(emptyList())

    /**
     * Every stored id, on every network, oldest first, with its last
     * reported state. A stored id is not by itself an unfinished transfer.
     */
    val entries: StateFlow<List<Entry>> = _entries.asStateFlow()

    private val _markedOperationIds = MutableStateFlow<Set<UUID>>(emptySet())

    /** Every operation id with a stored record, read or not yet read. */
    val markedOperationIds: StateFlow<Set<UUID>> = _markedOperationIds.asStateFlow()

    private val _storeError = MutableStateFlow<String?>(null)

    /**
     * Set when the stored ids cannot be read, cleared by the next successful
     * read. Shown instead of treated as "no saved transfers". While set,
     * [entries] is what the last successful read listed, so it may be stale.
     */
    val storeError: StateFlow<String?> = _storeError.asStateFlow()

    /** Guards the maps below. Never held across a store or wallet call. */
    private val state = Mutex()
    private val latest = HashMap<UUID, Entry>()

    /** Attempts started per operation, to detect a read an attempt overlapped. */
    private val attemptCounts = HashMap<UUID, Int>()

    /** Attempts whose body may still be running, per operation. */
    private val runningCounts = HashMap<UUID, Int>()

    /** Status reads started per operation; overlapping reads finish in any order. */
    private val readsStarted = HashMap<UUID, Int>()

    /**
     * The start number of the newest read applied per operation. A read that
     * started before it is dropped, so an older "pending" cannot replace a
     * newer terminal report and offer Resume again.
     */
    private val newestAppliedRead = HashMap<UUID, Int>()

    /**
     * What the row for [entry] offers. Rust's rule decides; a Resume is
     * offered only with the outpoint the read named.
     */
    fun rowAction(attemptInFlight: Boolean, entry: Entry): RowAction =
        when (
            runCatching {
                rules.recoveryAction(attemptInFlight, entry.lastRead, entry.outpoint != null)
            }.getOrDefault(WalletDrainRecoveryAction.CheckAgain)
        ) {
            WalletDrainRecoveryAction.InProgress -> RowAction.InProgress
            WalletDrainRecoveryAction.Resume ->
                entry.outpoint?.let { RowAction.Resume(it) } ?: RowAction.CheckAgain
            WalletDrainRecoveryAction.CheckAgain -> RowAction.CheckAgain
        }

    /** The row's status line, from the entry and whether an attempt runs. */
    fun rowStatus(attemptInFlight: Boolean, entry: Entry): RowStatus {
        if (attemptInFlight) return RowStatus.InProgress
        return when (val read = entry.lastRead) {
            null -> RowStatus.Checking
            WalletDrainStatusRead.Failed -> RowStatus.ReadFailed(entry.readError ?: "unknown error")
            is WalletDrainStatusRead.Reported -> when (read.state) {
                WalletDrainOutcome.State.Pending ->
                    if (entry.outpoint == null) RowStatus.UnfinishedLockUnknown else RowStatus.Unfinished
                WalletDrainOutcome.State.Completed -> RowStatus.FinishedNotCleared(entry.removalError)
                WalletDrainOutcome.State.NotSent ->
                    entry.removalError?.let { RowStatus.NotSentNotCleared(it) }
                        ?: RowStatus.NotSentWhileAttemptStarted
            }
        }
    }

    /**
     * The error a returned attempt fails with, or null when Rust's rule says
     * it needs nothing more (even if the call also returned an error, such
     * as "already consumed" for a repeated operation id).
     *
     * The error keeps Rust's class: the returned error's, or NOT_SENT for a
     * not-sent outcome that returned none (Rust's state says nothing left
     * the wallet, and gives no retry). Any other outcome without an error is
     * unclassified, so it holds its slot.
     */
    fun attemptError(outcome: WalletDrainOutcome): WalletDrainAttemptException? {
        val finished = runCatching { rules.attemptFinished(outcome.state) }.getOrDefault(false)
        if (finished) return null
        val rustText = outcome.error?.message
        val recovery = outcome.error?.let { PaymentRecovery.of(it) }
            ?: when (outcome.state) {
                WalletDrainOutcome.State.NotSent ->
                    PaymentRecovery.of(PaymentDelivery.NOT_SENT, isRetryable = false, heldAssetLock = null)
                WalletDrainOutcome.State.Pending, WalletDrainOutcome.State.Completed ->
                    PaymentRecovery.Unconfirmed
            }
        return when (outcome.state) {
            WalletDrainOutcome.State.NotSent ->
                WalletDrainAttemptException(rustText ?: "Nothing was sent.", recovery)
            // `Completed` lands here only if the rule call itself failed:
            // shown as unfinished, the fail-closed reading.
            WalletDrainOutcome.State.Pending, WalletDrainOutcome.State.Completed -> {
                val notice = "This transfer is not finished: resume it from the wallet screen " +
                    "and do not start a new Max."
                WalletDrainAttemptException(rustText?.let { "$it $notice" } ?: notice, recovery)
            }
        }
    }

    /**
     * Reads the status of every stored id of [network] whose wallet is in
     * [walletIds], and removes or lists it as Rust's rule says. When reads
     * of one operation overlap, the one that started last wins.
     */
    suspend fun reconcile(network: Network, walletIds: List<ByteArray>, wallet: WalletDrainWallet) {
        val records = try {
            store.records().also { _storeError.value = null }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            _storeError.value = "Could not read saved Max transfers: ${e.message ?: e}"
            return
        }
        for (record in records) {
            if (record.networkRaw != network.ffiValue) continue
            if (walletIds.none { record.belongsTo(it) }) continue
            val id = record.operationId
            val (attemptsBefore, runningBefore, readNumber) = state.withLock {
                val number = (readsStarted[id] ?: 0) + 1
                readsStarted[id] = number
                Triple(attemptCounts[id] ?: 0, (runningCounts[id] ?: 0) > 0, number)
            }
            var outpoint: WalletDrainOutpoint? = null
            var readError: String? = null
            val read: WalletDrainStatusRead = try {
                val outcome = wallet.status(record)
                outpoint = outcome.outpoint
                WalletDrainStatusRead.Reported(outcome.state)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                readError = e.message ?: e.toString()
                WalletDrainStatusRead.Failed
            }
            // A read that started later has already been applied: this one is
            // older news, for the entry and for the stored id alike. The read
            // after an attempt starts after it, so it is never older than a
            // read that attempt could have changed.
            val overlapped = state.withLock {
                if (readNumber <= (newestAppliedRead[id] ?: 0)) return@withLock null
                newestAppliedRead[id] = readNumber
                runningBefore ||
                    (runningCounts[id] ?: 0) > 0 ||
                    (attemptCounts[id] ?: 0) != attemptsBefore
            } ?: continue
            // A rule that cannot be asked keeps the id: forgetting is the
            // only step that cannot be undone.
            val keep = runCatching { rules.keepsStoredId(read, overlapped) }.getOrDefault(true)
            val entry = Entry(record, outpoint, read, readError, removalError = null)
            if (keep) {
                state.withLock { latest[id] = entry }
            } else {
                try {
                    store.remove(id)
                    // An id is minted per transfer and never reused, so what
                    // is kept for it goes with its stored row: with no row,
                    // a late older read has nothing to list or offer. The
                    // running count is left to the attempt that holds it,
                    // which clears it when it returns.
                    state.withLock {
                        latest.remove(id)
                        attemptCounts.remove(id)
                        readsStarted.remove(id)
                        newestAppliedRead.remove(id)
                    }
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    // Rust is done with it and only the stored id is left.
                    // The report is kept, so the row offers "Check again"
                    // (which retries this removal), never a spend.
                    state.withLock {
                        latest[id] = entry.copy(removalError = e.message ?: e.toString())
                    }
                }
            }
        }
        refreshFromStore()
    }

    /**
     * Mints an operation id, stores it, then runs the drain. Nothing is
     * called when the id cannot be stored.
     */
    suspend fun start(
        walletId: ByteArray,
        network: Network,
        fundingAccountIndex: Int,
        recipientRaw43: ByteArray,
        wallet: WalletDrainWallet,
    ): StartResult {
        val record = WalletDrainOperationRecord(
            operationId = newOperationId(),
            walletId = walletId,
            networkRaw = network.ffiValue,
            fundingAccountIndex = fundingAccountIndex,
            recipientRaw43 = recipientRaw43,
            createdAtMillis = now(),
        )
        try {
            store.insert(record)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            return StartResult.MarkerWriteFailed(
                "Could not save the transfer id, so nothing was sent: ${e.message ?: e}",
            )
        }
        val result = run(record, network, wallet, resuming = null) {
            attemptError(wallet.drain(record))?.let { throw it }
        }
        return when (result) {
            is StartFundingResult.Started -> {
                // A failure that holds the slot refused the start, so the
                // body never ran and nothing was sent under this id: remove
                // it as for a blocked start (reconcile reads it otherwise).
                if (!result.bodyScheduled) runCatching { store.remove(record.operationId) }
                // Listed as unread until the reconcile after the attempt.
                refreshFromStore()
                StartResult.Started(result.controller)
            }
            is StartFundingResult.BlockedByOtherWalletFunding -> {
                // The body never ran, so nothing was sent under this id. If
                // the removal fails the id is left for reconcile, which will
                // read it as not sent.
                runCatching { store.remove(record.operationId) }
                refreshFromStore()
                StartResult.BlockedByOtherWalletFunding(result.blocker)
            }
        }
    }

    /**
     * Finishes the stored operation from the asset lock at [outpoint],
     * paying the stored recipient. Rust writes the drain's completion
     * receipt when it lands (or Platform reports the lock already
     * consumed), so the reconcile that follows forgets the stored id.
     * The attempt names [outpoint] as the lock it resumes, so a failure
     * that holds exactly that lock lets it run.
     */
    suspend fun resume(
        record: WalletDrainOperationRecord,
        outpoint: WalletDrainOutpoint,
        network: Network,
        wallet: WalletDrainWallet,
    ): StartFundingResult = run(
        record,
        network,
        wallet,
        resuming = PaymentFailure.outPoint(outpoint.txidWire, outpoint.vout),
    ) {
        try {
            wallet.resume(record, outpoint)
        } catch (e: DashSdkError.PlatformWallet.AssetLockNotTracked) {
            // Rust's "not tracked" text does not say what to do next; its
            // class stays the error's.
            throw WalletDrainAttemptException(ASSET_LOCK_NOT_TRACKED_MESSAGE, PaymentRecovery.of(e))
        }
    }

    /** Reads the operation's status again. Sends nothing. */
    suspend fun checkAgain(record: WalletDrainOperationRecord, network: Network, wallet: WalletDrainWallet) {
        reconcile(network, listOf(record.walletId), wallet)
    }

    /**
     * Lists every stored id. A failed read sets [storeError] and leaves
     * [entries] as the last successful read left it.
     */
    suspend fun refreshFromStore() {
        val records = try {
            store.records()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            _storeError.value = "Could not read saved Max transfers: ${e.message ?: e}"
            return
        }
        _storeError.value = null
        val ids = records.mapTo(HashSet()) { it.operationId }
        // Every stored id is listed, read or not: its lock is already hidden
        // from the resumable list, so dropping an unread one would hide the
        // transfer everywhere until a read reports on it.
        val listed = state.withLock {
            latest.keys.retainAll(ids)
            records.map { latest[it.operationId] ?: Entry.unread(it) }
        }
        _markedOperationIds.value = ids
        _entries.value = listed
    }

    /** The funding coordinator's id for a Max operation's controller. */
    fun controllerOperationId(operationId: UUID): String = CONTROLLER_OPERATION_ID_PREFIX + operationId

    /**
     * Runs [attempt] for [record] through the shielded-funding coordinator
     * under the operation's controller id, then reconciles the operation
     * and rethrows the attempt's error for the controller to show.
     * [resuming] is the asset lock the attempt resumes, null for the drain.
     */
    private suspend fun run(
        record: WalletDrainOperationRecord,
        network: Network,
        wallet: WalletDrainWallet,
        resuming: AssetLockBuiltOutPoint?,
        attempt: suspend () -> Unit,
    ): StartFundingResult {
        val id = record.operationId
        // Counted before the body is scheduled, so a status read that
        // started first sees that an attempt overlapped it.
        state.withLock {
            attemptCounts[id] = (attemptCounts[id] ?: 0) + 1
            runningCounts[id] = (runningCounts[id] ?: 0) + 1
        }
        val body: suspend () -> Unit = {
            var failure: Throwable? = null
            try {
                attempt()
            } catch (e: Throwable) {
                failure = e
            }
            // A cancelled attempt may still have sent something: finish the
            // bookkeeping and the read before letting the cancellation out.
            withContext(NonCancellable) {
                finishRunning(id)
                reconcile(network, listOf(record.walletId), wallet)
            }
            failure?.let { throw it }
        }
        val result = fundingCoordinator.startFunding(
            walletId = record.walletId,
            recipientRaw43 = record.recipientRaw43,
            operationId = controllerOperationId(id),
            resuming = resuming,
            body = body,
        )
        // Undo the running count when the body was not scheduled: a blocked
        // start, a re-tap that re-presented a controller already running
        // this operation, or a start a failure's class refused.
        val bodyScheduled = (result as? StartFundingResult.Started)?.bodyScheduled ?: false
        if (!bodyScheduled) finishRunning(id)
        return result
    }

    private suspend fun finishRunning(id: UUID) {
        state.withLock {
            val remaining = (runningCounts[id] ?: 0) - 1
            if (remaining > 0) runningCounts[id] = remaining else runningCounts.remove(id)
        }
    }

    companion object {
        const val CONTROLLER_OPERATION_ID_PREFIX = "drain:"

        /**
         * Shown when a Resume finds no tracked asset lock for the outpoint.
         * "Not tracked" only means the outpoint is not in the wallet's live
         * map: a stored row may still exist and be sent again after a
         * restart, so this claims nothing about what was sent and points at
         * the status read instead. ← Swift `assetLockNotTrackedMessage`.
         */
        const val ASSET_LOCK_NOT_TRACKED_MESSAGE =
            "The wallet is not tracking this asset lock right now. Restart the app, then " +
                "check this transfer again and follow the status it reports."
    }
}
