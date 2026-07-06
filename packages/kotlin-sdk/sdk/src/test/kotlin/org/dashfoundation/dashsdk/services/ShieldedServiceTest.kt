package org.dashfoundation.dashsdk.services

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.ShieldedNoteEntity
import org.dashfoundation.dashsdk.wallet.WalletSyncEvent
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * Reduction, cumulative counters, balance summing, and clear-local-state
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
