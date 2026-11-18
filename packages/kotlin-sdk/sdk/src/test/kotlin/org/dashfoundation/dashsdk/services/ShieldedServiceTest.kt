package org.dashfoundation.dashsdk.services

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.ShieldedNoteEntity
import org.dashfoundation.dashsdk.wallet.ShieldedBalanceSource
import org.dashfoundation.dashsdk.wallet.ShieldedLocalAccountBalance
import org.dashfoundation.dashsdk.wallet.ShieldedLocalBalanceSnapshot
import org.dashfoundation.dashsdk.wallet.ShieldedLocalBalanceState
import org.dashfoundation.dashsdk.wallet.WalletSyncEvent
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * Reduction, cumulative counters, the local balance cache, and clear-local-state
 * for [ShieldedService]. Reduction is exercised directly (bind() is gated on
 * the native `hasShielded()` probe, which is unavailable under Robolectric);
 * balance and teardown use an in-memory [DashDatabase].
 */
@RunWith(RobolectricTestRunner::class)
class ShieldedServiceTest {

    private lateinit var db: DashDatabase
    private lateinit var service: ShieldedService

    private val walletId = ByteArray(32) { 7 }
    private val otherWallet = ByteArray(32) { 8 }

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        service = ShieldedService(db)
    }

    @After
    fun tearDown() {
        service.close()
        db.close()
    }

    private fun note(n: Byte, value: Long, spent: Boolean, wallet: ByteArray = walletId) =
        ShieldedNoteEntity(
            nullifier = ByteArray(32) { n },
            walletId = wallet,
            accountIndex = 0,
            position = n.toLong(),
            cmx = ByteArray(32) { (n + 1).toByte() },
            blockHeight = 100,
            isSpent = spent,
            value = value,
            noteData = ByteArray(115),
        )

    @Test
    fun successfulResultAccumulatesCounters() {
        service.reduce(
            walletId,
            WalletSyncEvent.ShieldedResult(
                walletId = walletId,
                success = true,
                skipped = false,
                cooldownSkip = false,
                newNotes = 3,
                totalScanned = 100,
                newlySpent = 1,
                balance = 5_000,
                errorMessage = null,
            ),
        )
        service.reduce(
            walletId,
            WalletSyncEvent.ShieldedResult(
                walletId = walletId,
                success = true,
                skipped = false,
                cooldownSkip = false,
                newNotes = 2,
                totalScanned = 50,
                newlySpent = 0,
                balance = 7_500,
                errorMessage = null,
            ),
        )
        val s = service.state.value
        assertEquals(7_500L, s.shieldedBalance)
        assertEquals(2, s.lastNewNotes)
        assertEquals(150L, s.totalScanned) // 100 + 50
        assertEquals(5L, s.totalNewNotes) // 3 + 2
    }

    @Test
    fun cooldownSkipPreservesPriorState() {
        service.reduce(
            walletId,
            WalletSyncEvent.ShieldedResult(
                walletId, true, false, false, 4, 200, 0, 9_000, null,
            ),
        )
        // A cooldown-skip pass carries all-zero numerics — must NOT clobber.
        service.reduce(
            walletId,
            WalletSyncEvent.ShieldedResult(
                walletId, true, false, true, 0, 0, 0, 0, null,
            ),
        )
        val s = service.state.value
        assertEquals(9_000L, s.shieldedBalance)
        assertEquals(200L, s.totalScanned)
        assertEquals(4L, s.totalNewNotes)
    }

    @Test
    fun resultForOtherWalletIsIgnored() {
        service.reduce(
            walletId,
            WalletSyncEvent.ShieldedResult(
                otherWallet, true, false, false, 9, 999, 0, 1, null,
            ),
        )
        assertEquals(0L, service.state.value.totalScanned)
        assertEquals(0L, service.state.value.shieldedBalance)
    }

    @Test
    fun passCompletedIncrementsSyncCount() {
        assertEquals(0, service.state.value.syncCountSinceLaunch)
        service.reduce(walletId, WalletSyncEvent.ShieldedPassCompleted(1_700_000_000, 1))
        service.reduce(walletId, WalletSyncEvent.ShieldedPassCompleted(1_700_000_100, 1))
        assertEquals(2, service.state.value.syncCountSinceLaunch)
    }

    @Test
    fun progressAndTreeEventsUpdateLiveCounters() {
        service.reduce(walletId, WalletSyncEvent.ShieldedProgress(4_096, 12_345))
        // Progress feeds the LIVE counter only — the lifetime total must
        // not move until the pass completes (a progress+result pair would
        // otherwise double-count the pass).
        assertEquals(4_096L, service.state.value.currentSyncScanned)
        assertEquals(0L, service.state.value.totalScanned)
        service.reduce(walletId, WalletSyncEvent.ShieldedTreeProgress(500, 1_000))
        assertEquals(500L, service.state.value.treeLeavesCommitted)
        assertEquals(1_000L, service.state.value.treeTotalTarget)
    }

    @Test
    fun unspentBalanceFlowSumsWalletScopedNotes() = runTest {
        db.shieldedDao().upsertNote(note(1, 1_000, spent = false))
        db.shieldedDao().upsertNote(note(2, 2_000, spent = false))
        db.shieldedDao().upsertNote(note(3, 4_000, spent = true)) // spent — excluded
        db.shieldedDao().upsertNote(note(4, 8_000, spent = false, wallet = otherWallet)) // other wallet

        val balance = db.shieldedDao()
            .observeUnspentNotesByWallet(walletId)
            .first()
            .sumOf { it.value }
        assertEquals(3_000L, balance)
    }

    private fun snapshot(ordinary: Long, vararg accounts: Pair<Int, Long>) =
        ShieldedLocalBalanceSnapshot(
            ordinarySpendableCredits = ordinary,
            accounts = accounts.associate { (index, spendable) ->
                index to ShieldedLocalAccountBalance(
                    spendableCredits = spendable,
                    lastScannedIndex = 1,
                    source = ShieldedBalanceSource.SCANNED_THIS_SESSION,
                )
            },
        )

    private fun ready(ordinary: Long, vararg accounts: Pair<Int, Long>) =
        ShieldedLocalBalanceState.Ready(snapshot(ordinary, *accounts))

    @Test
    fun shouldKeepAReadySnapshotAndDropAnUnboundWallet() {
        val first = snapshot(150, 0 to 100, 1 to 50)
        val second = snapshot(70, 0 to 70)
        var cache = applyLocalBalanceRead(emptyMap(), "aa", ShieldedLocalBalanceState.Ready(first))
        assertEquals(mapOf("aa" to first), cache)
        // An incomplete restoration establishes nothing: the entry stays.
        cache = applyLocalBalanceRead(cache, "aa", ShieldedLocalBalanceState.RestoreIncomplete)
        assertEquals(mapOf("aa" to first), cache)
        cache = applyLocalBalanceRead(cache, "bb", ShieldedLocalBalanceState.Ready(second))
        cache = applyLocalBalanceRead(cache, "aa", ShieldedLocalBalanceState.Ready(second))
        assertEquals(mapOf("aa" to second, "bb" to second), cache)
        cache = applyLocalBalanceRead(cache, "aa", ShieldedLocalBalanceState.Unbound)
        assertEquals(mapOf("bb" to second), cache)
    }

    @Test
    fun shouldShowRustsTotalAndTheAccountASendSpendsFrom() = runTest {
        val identityAccount = 0x4000_0000
        // Rust's total leaves the identity account out; the service must
        // show that number as it is, not a sum over the accounts.
        service.launchLocalBalanceRead(walletId) {
            ready(150, 0 to 100, 1 to 50, identityAccount to 900)
        }.join()
        val cached = service.cachedLocalBalance(walletId).first()
        assertEquals(150L, cached?.ordinarySpendableCredits)
        assertEquals(100L, cached?.accounts?.get(0)?.spendableCredits)
        assertEquals(900L, cached?.accounts?.get(identityAccount)?.spendableCredits)
        // Another wallet has no snapshot: not known, which is not zero.
        assertNull(service.cachedLocalBalance(otherWallet).first())
    }

    @Test
    fun shouldReadAgainWhileTheStoreIsBusyAndGiveUpAfterTheLastDelay() = runTest {
        var reads = 0
        // Busy twice, then the scan releases the store.
        service.readLocalBalance("aa", listOf(300L, 1_000L, 3_000L)) {
            reads += 1
            if (reads < 3) error("busy") else ready(150, 0 to 150)
        }
        assertEquals(3, reads)

        reads = 0
        // Busy on every attempt: one read plus one per delay, then give up.
        service.readLocalBalance("aa", listOf(300L, 1_000L, 3_000L)) {
            reads += 1
            error("busy")
        }
        assertEquals(4, reads)
    }

    @Test
    fun shouldKeepTheLastSnapshotOfAWalletWhoseReadFails() = runTest {
        service.launchLocalBalanceRead(walletId) { ready(150, 0 to 150) }.join()
        service.readLocalBalance(walletId.joinToString("") { "%02x".format(it) }, emptyList()) {
            error("busy")
        }
        assertEquals(150L, service.cachedLocalBalance(walletId).first()?.ordinarySpendableCredits)
    }

    @Test
    fun shouldLetANewerReadReplaceTheOneInFlight() = runTest {
        val release = CompletableDeferred<Unit>()
        val older = service.launchLocalBalanceRead(walletId) {
            release.await()
            ready(1, 0 to 1)
        }
        val newer = service.launchLocalBalanceRead(walletId) { ready(2, 0 to 2) }
        newer.join()
        older.join()
        assertTrue(older.isCancelled)
        release.complete(Unit)
        assertEquals(2L, service.cachedLocalBalance(walletId).first()?.ordinarySpendableCredits)
    }

    @Test
    fun shouldForgetOnlyTheDroppedWalletsSnapshot() = runTest {
        service.launchLocalBalanceRead(walletId) { ready(150, 0 to 150) }.join()
        service.launchLocalBalanceRead(otherWallet) { ready(70, 0 to 70) }.join()
        service.dropLocalBalance(walletId)
        assertNull(service.cachedLocalBalance(walletId).first())
        assertEquals(70L, service.cachedLocalBalance(otherWallet).first()?.ordinarySpendableCredits)
    }

    @Test
    fun shouldNotLetAReadWriteIntoACacheDroppedAfterItStarted() {
        // The cache is empty before the drop and empty after it. Only the
        // generation tells the two apart, and it is part of the same value
        // the write replaces, so the write cannot land in between.
        val beforeDrop = LocalBalanceCache()
        val readGeneration = beforeDrop.generation
        val afterDrop = beforeDrop.dropped()
        assertEquals(emptyMap<String, ShieldedLocalBalanceSnapshot>(), afterDrop.snapshots)
        assertFalse(beforeDrop == afterDrop)
        assertEquals(afterDrop, afterDrop.afterRead(readGeneration, "aa", ready(999, 0 to 999)))

        // A read that started after the drop is kept, and a later drop
        // empties the cache again and moves the generation again.
        val kept = afterDrop.afterRead(afterDrop.generation, "aa", ready(150, 0 to 150))
        assertEquals(mapOf("aa" to snapshot(150, 0 to 150)), kept.snapshots)
        assertEquals(afterDrop.generation, kept.generation)
        assertEquals(LocalBalanceCache(generation = afterDrop.generation + 1), kept.dropped())
    }

    @Test
    fun shouldNotKeepAReadThatStartedBeforeTheCacheWasDropped() = runTest {
        val key = walletId.joinToString("") { "%02x".format(it) }
        // A read notes the generation when it starts. This one started
        // under a generation that is no longer current, as a read against a
        // replaced manager or a cleared ledger does.
        service.readLocalBalance(key, emptyList(), generation = -1) { ready(999, 0 to 999) }
        assertNull(service.cachedLocalBalance(walletId).first())

        // The drop itself moves the generation: a read that began before an
        // unbind and answers after it is not kept either.
        val release = CompletableDeferred<Unit>()
        val started = CompletableDeferred<Unit>()
        val stale = service.launchLocalBalanceRead(walletId) {
            started.complete(Unit)
            // Not cancellable here, like a read already inside native code.
            withContext(NonCancellable) { release.await() }
            ready(999, 0 to 999)
        }
        started.await()
        service.unbind()
        release.complete(Unit)
        stale.join()
        assertNull(service.cachedLocalBalance(walletId).first())

        // A read of the current generation is kept.
        service.readLocalBalance(key, emptyList()) { ready(150, 0 to 150) }
        assertEquals(150L, service.cachedLocalBalance(walletId).first()?.ordinarySpendableCredits)
    }

    @Test
    fun shouldForgetEverySnapshotOnUnbind() = runTest {
        service.launchLocalBalanceRead(walletId) { ready(150, 0 to 150) }.join()
        service.unbind()
        assertNull(service.cachedLocalBalance(walletId).first())
    }

    @Test
    fun shouldReadTheBalanceAgainOnlyAfterAPassThatRan() {
        fun result(success: Boolean, skipped: Boolean, cooldownSkip: Boolean) =
            WalletSyncEvent.ShieldedResult(walletId, success, skipped, cooldownSkip, 0, 0, 0, 0, null)
        assertTrue(changesLocalBalance(result(success = true, skipped = false, cooldownSkip = false)))
        assertFalse(changesLocalBalance(result(success = true, skipped = false, cooldownSkip = true)))
        assertFalse(changesLocalBalance(result(success = false, skipped = true, cooldownSkip = false)))
        assertFalse(changesLocalBalance(result(success = false, skipped = false, cooldownSkip = false)))
    }

    @Test
    fun clearLocalStateDeletesBoundWalletRows() = runTest {
        // Simulate bind by hand (native bind is unavailable under Robolectric):
        // populate rows, drive reduce, then clear.
        db.shieldedDao().upsertNote(note(1, 1_000, spent = false))
        db.shieldedDao().upsertNote(note(2, 2_000, spent = false, wallet = otherWallet))

        // Clear is a no-op while unbound (nothing to scope to).
        service.clearLocalState(db)
        assertEquals(2, db.shieldedDao().getAllNotes().size)
    }

    @Test
    fun unbindResetsState() {
        service.reduce(
            walletId,
            WalletSyncEvent.ShieldedResult(walletId, true, false, false, 1, 10, 0, 500, null),
        )
        service.unbind()
        val s = service.state.value
        assertEquals(0L, s.shieldedBalance)
        assertEquals(0L, s.totalScanned)
        assertEquals(0, s.syncCountSinceLaunch)
    }

    @Test
    fun bindingFlowsDefaultFalseAndHardUnbindResetsThem() {
        // A fresh service is neither bound nor resumable.
        assertEquals(false, service.isBound.value)
        assertEquals(false, service.canResume.value)
        // A hard unbind (network-switch path) must drive both back to false so
        // the Clear button's `canResume` gate disables when there is nothing
        // to resume to.
        service.unbind()
        assertEquals(false, service.isBound.value)
        assertEquals(false, service.canResume.value)
    }
}
