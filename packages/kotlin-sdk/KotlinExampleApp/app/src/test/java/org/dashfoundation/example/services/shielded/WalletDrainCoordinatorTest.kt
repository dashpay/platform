package org.dashfoundation.example.services.shielded

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.wallet.WalletDrainOutcome
import org.dashfoundation.dashsdk.wallet.WalletDrainOutcome.State
import org.dashfoundation.dashsdk.wallet.WalletDrainOutpoint
import org.dashfoundation.dashsdk.wallet.WalletDrainRecoveryAction
import org.dashfoundation.dashsdk.wallet.WalletDrainStatusRead
import org.dashfoundation.example.services.shielded.ShieldedFundFromAssetLockController.Phase
import org.dashfoundation.example.services.shielded.ShieldedFundFromAssetLockCoordinator.StartFundingResult
import org.dashfoundation.example.services.shielded.WalletDrainCoordinator.RowAction
import org.dashfoundation.example.services.shielded.WalletDrainCoordinator.RowStatus
import org.dashfoundation.example.services.shielded.WalletDrainCoordinator.StartResult
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.UUID

/**
 * [WalletDrainCoordinator]: what it stores, calls and lists around a Core to
 * shielded Max transfer. Port of the coordinator cases of
 * `packages/swift-sdk/SwiftExampleApp/SwiftExampleAppTests/WalletDrainRecoveryTests.swift`.
 *
 * The rules themselves are Rust's and are tested there
 * (`wallet_drain_recovery.rs`) and through the native library
 * (`WalletDrainRulesFfiTest`). A JVM test cannot load that library, so
 * [RustRules] stands in for it; what is under test here is the facts the
 * coordinator gathers for the rules and what it does with their answers.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class WalletDrainCoordinatorTest {

    private val walletA = ByteArray(32) { 0xAA.toByte() }
    private val walletB = ByteArray(32) { 0xBB.toByte() }
    private val recipient = ByteArray(43) { 0x20 }
    private val outpoint = WalletDrainOutpoint(ByteArray(32) { 7 }, 0)

    /** The decision tables of `wallet_drain_recovery.rs`, restated for the JVM. */
    private object RustRules : WalletDrainRuleSet {
        override fun keepsStoredId(read: WalletDrainStatusRead, attemptOverlappedRead: Boolean) =
            when (read) {
                WalletDrainStatusRead.Failed -> true
                is WalletDrainStatusRead.Reported -> when (read.state) {
                    State.Completed -> false
                    State.NotSent -> attemptOverlappedRead
                    State.Pending -> true
                }
            }

        override fun recoveryAction(
            attemptInFlight: Boolean,
            lastRead: WalletDrainStatusRead?,
            readNamedAssetLock: Boolean,
        ) = when {
            attemptInFlight -> WalletDrainRecoveryAction.InProgress
            lastRead == WalletDrainStatusRead.Reported(State.Pending) && readNamedAssetLock ->
                WalletDrainRecoveryAction.Resume
            else -> WalletDrainRecoveryAction.CheckAgain
        }

        override fun attemptFinished(state: State) = state == State.Completed
    }

    private class FakeStore : WalletDrainOperationStore {
        val rows = mutableListOf<WalletDrainOperationRecord>()
        var failReads = false
        var failInserts = false
        var failRemovals = false
        var removals = 0

        override suspend fun records(): List<WalletDrainOperationRecord> {
            if (failReads) error("disk read failed")
            return rows.sortedBy { it.createdAtMillis }
        }

        override suspend fun insert(record: WalletDrainOperationRecord) {
            if (failInserts) error("disk full")
            rows += record
        }

        override suspend fun remove(operationId: UUID) {
            removals++
            if (failRemovals) error("disk write failed")
            rows.removeAll { it.operationId == operationId }
        }
    }

    private class FakeWallet : WalletDrainWallet {
        val drained = mutableListOf<WalletDrainOperationRecord>()
        val resumed = mutableListOf<Pair<WalletDrainOperationRecord, WalletDrainOutpoint>>()
        val statusReads = mutableListOf<UUID>()

        /** What the store held when the drain was called. */
        var storedAtDrain: List<UUID>? = null
        var store: FakeStore? = null

        var drain: suspend (WalletDrainOperationRecord) -> WalletDrainOutcome =
            { WalletDrainOutcome(State.Completed, null) }
        var resume: suspend (WalletDrainOperationRecord, WalletDrainOutpoint) -> Unit = { _, _ -> }
        var status: suspend (WalletDrainOperationRecord) -> WalletDrainOutcome =
            { WalletDrainOutcome(State.NotSent, null) }

        override suspend fun drain(record: WalletDrainOperationRecord): WalletDrainOutcome {
            storedAtDrain = store?.rows?.map { it.operationId }
            drained += record
            return drain.invoke(record)
        }

        override suspend fun resume(record: WalletDrainOperationRecord, outpoint: WalletDrainOutpoint) {
            resumed += record to outpoint
            resume.invoke(record, outpoint)
        }

        override suspend fun status(record: WalletDrainOperationRecord): WalletDrainOutcome {
            statusReads += record.operationId
            return status.invoke(record)
        }
    }

    private class Fixture(
        val coordinator: WalletDrainCoordinator,
        val funding: ShieldedFundFromAssetLockCoordinator,
        val store: FakeStore,
        val wallet: FakeWallet,
    )

    private fun TestScope.fixture(rules: WalletDrainRuleSet = RustRules): Fixture {
        val store = FakeStore()
        val wallet = FakeWallet().also { it.store = store }
        // On the test scope, like the funding coordinator's own tests: a
        // body launched on `backgroundScope` is not run by `advanceUntilIdle`.
        // Every test lets its controllers finish, which ends their sweeps.
        val funding = ShieldedFundFromAssetLockCoordinator(
            scope = this,
            now = { testScheduler.currentTime },
        )
        var minted = 0L
        val coordinator = WalletDrainCoordinator(
            store = store,
            fundingCoordinator = funding,
            rules = rules,
            now = { testScheduler.currentTime },
            newOperationId = { UUID(0, ++minted) },
        )
        return Fixture(coordinator, funding, store, wallet)
    }

    private fun record(
        id: Long,
        walletId: ByteArray = walletA,
        network: Network = Network.TESTNET,
        createdAtMillis: Long = id,
    ) = WalletDrainOperationRecord(
        operationId = UUID(0, id),
        walletId = walletId,
        networkRaw = network.ffiValue,
        fundingAccountIndex = 0,
        recipientRaw43 = recipient,
        createdAtMillis = createdAtMillis,
    )

    private suspend fun Fixture.start() =
        coordinator.start(walletA, Network.TESTNET, 0, recipient, wallet)

    private suspend fun Fixture.reconcile(walletIds: List<ByteArray> = listOf(walletA)) =
        coordinator.reconcile(Network.TESTNET, walletIds, wallet)

    // ── start ─────────────────────────────────────────────────────────

    @Test
    fun `stores the operation id before the drain is called`() = runTest {
        val f = fixture()
        f.wallet.status = { WalletDrainOutcome(State.Completed, outpoint) }

        val started = f.start()
        assertTrue(started is StartResult.Started)
        assertEquals("nothing is called before the body runs", 0, f.wallet.drained.size)
        assertEquals(listOf(UUID(0, 1)), f.store.rows.map { it.operationId })

        advanceUntilIdle()

        assertEquals(listOf(UUID(0, 1)), f.wallet.storedAtDrain)
        val drained = f.wallet.drained.single()
        assertEquals(UUID(0, 1), drained.operationId)
        assertEquals(Network.TESTNET.ffiValue, drained.networkRaw)
        assertTrue(drained.recipientRaw43.contentEquals(recipient))
    }

    @Test
    fun `a completed drain is forgotten after the read that follows it`() = runTest {
        val f = fixture()
        f.wallet.status = { WalletDrainOutcome(State.Completed, outpoint) }

        val controller = (f.start() as StartResult.Started).controller
        advanceUntilIdle()

        assertEquals(Phase.Completed, controller.phase.value)
        assertEquals("read once, after the attempt", listOf(UUID(0, 1)), f.wallet.statusReads)
        assertTrue(f.store.rows.isEmpty())
        assertTrue(f.coordinator.entries.value.isEmpty())
        assertTrue(f.coordinator.markedOperationIds.value.isEmpty())
    }

    @Test
    fun `nothing is called when the operation id cannot be stored`() = runTest {
        val f = fixture()
        f.store.failInserts = true

        val result = f.start()
        advanceUntilIdle()

        assertTrue(result is StartResult.MarkerWriteFailed)
        assertTrue((result as StartResult.MarkerWriteFailed).message.contains("nothing was sent"))
        assertTrue(f.wallet.drained.isEmpty())
        assertTrue(f.funding.controllers.value.isEmpty())
    }

    @Test
    fun `an unfinished drain keeps its id and offers a resume of the named lock`() = runTest {
        val f = fixture()
        val unresolved = DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed("proof wait timed out")
        f.wallet.drain = { WalletDrainOutcome(State.Pending, outpoint, unresolved) }
        f.wallet.status = { WalletDrainOutcome(State.Pending, outpoint) }

        val controller = (f.start() as StartResult.Started).controller
        advanceUntilIdle()

        val failed = controller.phase.value as Phase.Failed
        assertTrue(failed.message.startsWith("proof wait timed out"))
        assertTrue(failed.message.contains("do not start a new Max"))

        val entry = f.coordinator.entries.value.single()
        assertEquals(WalletDrainStatusRead.Reported(State.Pending), entry.lastRead)
        assertEquals(outpoint, entry.outpoint)
        assertEquals(RowAction.Resume(outpoint), f.coordinator.rowAction(false, entry))
        assertEquals(RowStatus.Unfinished, f.coordinator.rowStatus(false, entry))
        assertEquals(RowAction.InProgress, f.coordinator.rowAction(true, entry))
        assertEquals(setOf(UUID(0, 1)), f.coordinator.markedOperationIds.value)
    }

    @Test
    fun `a completed drain that also returned an error is not shown as failed`() = runTest {
        val f = fixture()
        val used = DashSdkError.PlatformWallet.AssetLockAlreadyConsumed("already consumed")
        assertNull(f.coordinator.attemptError(WalletDrainOutcome(State.Completed, outpoint, used)))
        assertEquals(
            "Nothing was sent.",
            f.coordinator.attemptError(WalletDrainOutcome(State.NotSent, null))!!.message,
        )
        assertEquals(
            "another wallet drain is pending",
            f.coordinator.attemptError(
                WalletDrainOutcome(
                    State.NotSent,
                    null,
                    DashSdkError.PlatformWallet.WalletOperation("another wallet drain is pending"),
                ),
            )!!.message,
        )
    }

    @Test
    fun `a start blocked by another funding sends nothing and stores nothing`() = runTest {
        val f = fixture()
        val gate = CompletableDeferred<Unit>()
        f.funding.startFunding(walletA, ByteArray(43) { 0x33 }, "shield:other") { gate.await() }
        runCurrent()

        val result = f.start()
        // Not `advanceUntilIdle`: the blocker's sweep polls for as long as it
        // is in flight.
        runCurrent()

        assertTrue(result is StartResult.BlockedByOtherWalletFunding)
        assertTrue(f.wallet.drained.isEmpty())
        assertTrue("the unused id is removed", f.store.rows.isEmpty())
        gate.complete(Unit)
        advanceUntilIdle()
        assertTrue("and nothing ran afterwards", f.wallet.drained.isEmpty())
    }

    // ── reconcile ─────────────────────────────────────────────────────

    @Test
    fun `reconcile forgets what Rust is done with and keeps the rest`() = runTest {
        val f = fixture()
        f.store.rows += listOf(record(1), record(2), record(3), record(4))
        f.wallet.status = { r ->
            when (r.operationId.leastSignificantBits) {
                1L -> WalletDrainOutcome(State.Completed, outpoint)
                2L -> WalletDrainOutcome(State.NotSent, null)
                3L -> WalletDrainOutcome(State.Pending, null)
                else -> throw DashSdkError.PlatformWallet.WalletOperation("wallet missing from snapshot")
            }
        }

        f.reconcile()

        assertEquals(listOf(UUID(0, 3), UUID(0, 4)), f.store.rows.map { it.operationId })
        val (pending, unreadable) = f.coordinator.entries.value
        assertEquals(RowStatus.UnfinishedLockUnknown, f.coordinator.rowStatus(false, pending))
        assertEquals(
            "pending without a named lock never offers a spend",
            RowAction.CheckAgain,
            f.coordinator.rowAction(false, pending),
        )
        assertEquals(WalletDrainStatusRead.Failed, unreadable.lastRead)
        assertEquals(
            RowStatus.ReadFailed("wallet missing from snapshot"),
            f.coordinator.rowStatus(false, unreadable),
        )
        assertEquals(RowAction.CheckAgain, f.coordinator.rowAction(false, unreadable))
    }

    @Test
    fun `reconcile reads only the given network and wallets`() = runTest {
        val f = fixture()
        f.store.rows += listOf(
            record(1),
            record(2, walletId = walletB),
            record(3, network = Network.MAINNET),
        )
        f.wallet.status = { WalletDrainOutcome(State.Completed, outpoint) }

        f.reconcile()

        assertEquals(listOf(UUID(0, 1)), f.wallet.statusReads)
        assertEquals(listOf(UUID(0, 2), UUID(0, 3)), f.store.rows.map { it.operationId })
        // The unread ones are still listed, as "checking".
        val unread = f.coordinator.entries.value
        assertEquals(2, unread.size)
        assertTrue(unread.all { it.lastRead == null })
        assertEquals(RowStatus.Checking, f.coordinator.rowStatus(false, unread.first()))
        assertEquals(RowAction.CheckAgain, f.coordinator.rowAction(false, unread.first()))
    }

    @Test
    fun `a not-sent read that an attempt overlapped keeps the id`() = runTest {
        val f = fixture()
        val readGate = CompletableDeferred<Unit>()
        val drainGate = CompletableDeferred<Unit>()
        f.store.rows += record(1)
        f.wallet.status = { readGate.await(); WalletDrainOutcome(State.NotSent, null) }
        f.wallet.resume = { _, _ -> drainGate.await() }

        // A read starts and waits; an attempt for the same id starts meanwhile.
        val read = launch { f.reconcile() }
        runCurrent()
        f.coordinator.resume(record(1), outpoint, Network.TESTNET, f.wallet)
        runCurrent()
        readGate.complete(Unit)
        read.join()

        assertEquals(
            "the attempt may have sent after the read",
            listOf(UUID(0, 1)),
            f.store.rows.map { it.operationId },
        )
        val entry = f.coordinator.entries.value.single()
        assertEquals(RowStatus.NotSentWhileAttemptStarted, f.coordinator.rowStatus(false, entry))

        drainGate.complete(Unit)
        advanceUntilIdle()
        assertTrue("the read after the attempt forgets it", f.store.rows.isEmpty())
    }

    @Test
    fun `an older read that finishes after a newer one is dropped`() = runTest {
        val f = fixture()
        f.store.rows += record(1)
        val olderGate = CompletableDeferred<Unit>()
        var call = 0
        f.wallet.status = {
            if (++call == 1) {
                olderGate.await()
                WalletDrainOutcome(State.Pending, outpoint)
            } else {
                WalletDrainOutcome(State.Completed, outpoint)
            }
        }

        val older = launch { f.reconcile() }
        runCurrent()
        f.reconcile()
        assertTrue("the newer read forgot it", f.store.rows.isEmpty())

        // Put the id back, as if only the removal had failed: the older
        // "pending" must not turn it into a resumable row again.
        f.store.rows += record(1)
        olderGate.complete(Unit)
        older.join()

        val entry = f.coordinator.entries.value.single()
        assertNull("the older report was not applied", entry.lastRead)
        assertEquals(RowAction.CheckAgain, f.coordinator.rowAction(false, entry))
    }

    @Test
    fun `an id that cannot be removed stays listed and never offers a spend`() = runTest {
        val f = fixture()
        f.store.rows += record(1)
        f.store.failRemovals = true
        f.wallet.status = { WalletDrainOutcome(State.Completed, outpoint) }

        f.reconcile()

        val entry = f.coordinator.entries.value.single()
        assertEquals("disk write failed", entry.removalError)
        assertEquals(RowStatus.FinishedNotCleared("disk write failed"), f.coordinator.rowStatus(false, entry))
        assertEquals(RowAction.CheckAgain, f.coordinator.rowAction(false, entry))

        // "Check again" retries the removal.
        f.store.failRemovals = false
        f.coordinator.checkAgain(entry.record, Network.TESTNET, f.wallet)
        assertEquals(2, f.store.removals)
        assertTrue(f.coordinator.entries.value.isEmpty())
    }

    @Test
    fun `a failed read of the stored ids is shown and keeps the last list`() = runTest {
        val f = fixture()
        f.store.rows += record(1)
        f.wallet.status = { WalletDrainOutcome(State.Pending, outpoint) }
        f.reconcile()
        assertNull(f.coordinator.storeError.value)

        f.store.failReads = true
        f.reconcile()

        assertNotNull(f.coordinator.storeError.value)
        assertTrue(f.coordinator.storeError.value!!.contains("disk read failed"))
        assertEquals("the last known list stays", 1, f.coordinator.entries.value.size)
        assertEquals("no status was read", 1, f.wallet.statusReads.size)

        f.store.failReads = false
        f.coordinator.refreshFromStore()
        assertNull(f.coordinator.storeError.value)
    }

    // ── resume ────────────────────────────────────────────────────────

    @Test
    fun `a resume finishes the named lock and never calls the drain`() = runTest {
        val f = fixture()
        f.store.rows += record(1)
        f.wallet.status = { WalletDrainOutcome(State.Completed, outpoint) }

        val result = f.coordinator.resume(record(1), outpoint, Network.TESTNET, f.wallet)
        advanceUntilIdle()

        val controller = (result as StartFundingResult.Started).controller
        assertEquals(Phase.Completed, controller.phase.value)
        assertEquals("drain:${UUID(0, 1)}", controller.operationId)
        val (resumedRecord, resumedOutpoint) = f.wallet.resumed.single()
        assertEquals(UUID(0, 1), resumedRecord.operationId)
        assertEquals(outpoint, resumedOutpoint)
        assertTrue(f.wallet.drained.isEmpty())
        assertTrue(f.store.rows.isEmpty())
    }

    @Test
    fun `a resume of a lock the wallet does not track says how to recover`() = runTest {
        val f = fixture()
        f.store.rows += record(1)
        f.wallet.status = { WalletDrainOutcome(State.Pending, outpoint) }
        f.wallet.resume = { _, _ ->
            throw DashSdkError.PlatformWallet.AssetLockNotTracked("asset lock is not tracked")
        }

        val result = f.coordinator.resume(record(1), outpoint, Network.TESTNET, f.wallet)
        advanceUntilIdle()

        val failed = (result as StartFundingResult.Started).controller.phase.value as Phase.Failed
        assertEquals(WalletDrainCoordinator.ASSET_LOCK_NOT_TRACKED_MESSAGE, failed.message)
        assertEquals("the id is kept", 1, f.store.rows.size)
    }

    @Test
    fun `a re-tapped resume does not count as a second running attempt`() = runTest {
        val f = fixture()
        f.store.rows += record(1)
        val gate = CompletableDeferred<Unit>()
        f.wallet.resume = { _, _ -> gate.await() }
        f.wallet.status = { WalletDrainOutcome(State.NotSent, null) }

        f.coordinator.resume(record(1), outpoint, Network.TESTNET, f.wallet)
        runCurrent()
        val again = f.coordinator.resume(record(1), outpoint, Network.TESTNET, f.wallet)
        assertFalse((again as StartFundingResult.Started).bodyScheduled)
        assertEquals("the body ran once", 1, f.wallet.resumed.size)

        gate.complete(Unit)
        advanceUntilIdle()

        // Had the re-tap left a running count behind, every later not-sent
        // read would look overlapped and the id would never be forgotten.
        assertTrue(f.store.rows.isEmpty())
    }

    // ── rules that cannot be asked ────────────────────────────────────

    @Test
    fun `a rule that cannot be asked keeps the id and offers no spend`() = runTest {
        val broken = object : WalletDrainRuleSet {
            override fun keepsStoredId(read: WalletDrainStatusRead, attemptOverlappedRead: Boolean) =
                error("native library missing")

            override fun recoveryAction(
                attemptInFlight: Boolean,
                lastRead: WalletDrainStatusRead?,
                readNamedAssetLock: Boolean,
            ): WalletDrainRecoveryAction = error("native library missing")

            override fun attemptFinished(state: State): Boolean = error("native library missing")
        }
        val f = fixture(broken)
        f.store.rows += record(1)
        f.wallet.status = { WalletDrainOutcome(State.Completed, outpoint) }

        f.reconcile()

        assertEquals(1, f.store.rows.size)
        assertEquals(0, f.store.removals)
        assertEquals(RowAction.CheckAgain, f.coordinator.rowAction(false, f.coordinator.entries.value.single()))
        assertNotNull(f.coordinator.attemptError(WalletDrainOutcome(State.Completed, outpoint)))
    }
}
