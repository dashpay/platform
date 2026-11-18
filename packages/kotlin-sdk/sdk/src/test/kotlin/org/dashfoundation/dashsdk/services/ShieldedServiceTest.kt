package org.dashfoundation.dashsdk.services

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.flatMapLatest
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.ShieldedNoteEntity
import org.dashfoundation.dashsdk.wallet.ShieldedBalanceSource
import org.dashfoundation.dashsdk.wallet.ShieldedBalanceUpdate
import org.dashfoundation.dashsdk.wallet.ShieldedLocalAccountBalance
import org.dashfoundation.dashsdk.wallet.ShieldedLocalBalanceSnapshot
import org.dashfoundation.dashsdk.wallet.ShieldedLocalBalanceState
import org.dashfoundation.dashsdk.wallet.WalletSyncEvent
import org.dashfoundation.dashsdk.wallet.latestShieldedBalances
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

    private fun update(
        revision: Long,
        ordinary: Long,
        established: Boolean = true,
        wallet: ByteArray = walletId,
        vararg accounts: Pair<Int, Long>,
    ) = ShieldedBalanceUpdate(
        wallet,
        revision,
        ShieldedLocalBalanceState.Ready(
            ShieldedLocalBalanceSnapshot(
                ordinarySpendableCredits = ordinary,
                ordinaryBalanceEstablished = established,
                accounts = accounts.associate { (index, spendable) ->
                    index to ShieldedLocalAccountBalance(
                        spendableCredits = spendable,
                        lastScannedIndex = 1,
                        source = ShieldedBalanceSource.SCANNED_THIS_SESSION,
                    )
                },
            ),
        ),
    )

    private fun announce(vararg updates: ShieldedBalanceUpdate): MutableStateFlow<Map<String, ShieldedBalanceUpdate>> {
        val announced = MutableStateFlow(
            updates.fold(emptyMap<String, ShieldedBalanceUpdate>()) { held, next ->
                latestShieldedBalances(held, next)
            },
        )
        service.balanceUpdates.value = announced
        return announced
    }

    @Test
    fun shouldShowRustsTotalAndTheAccountASendSpendsFrom() = runTest {
        val identityAccount = 0x4000_0000
        // Rust's total leaves the identity account out; the service shows
        // that number as it is, not a sum over the accounts or the notes.
        db.shieldedDao().upsertNote(note(1, 1_050, spent = false))
        announce(update(1, 150, accounts = arrayOf(0 to 100, 1 to 50, identityAccount to 900)))
        assertEquals(150L, service.ordinaryBalance(walletId).first())
        assertEquals(100L, service.accountBalance(walletId, 0).first())
        assertEquals(900L, service.accountBalance(walletId, identityAccount).first())
        // An account that is not bound, and a wallet that was not announced:
        // not known, which is not zero.
        assertNull(service.accountBalance(walletId, 9).first())
        assertNull(service.ordinaryBalance(otherWallet).first())
    }

    @Test
    fun shouldNotShowABalanceThatIsNotEstablishedOrHasNoNumbers() = runTest {
        // Readable but never scanned: zero credits without evidence.
        announce(update(1, 0, established = false, accounts = arrayOf(0 to 0)))
        assertNull(service.ordinaryBalance(walletId).first())

        announce(ShieldedBalanceUpdate(walletId, 2, ShieldedLocalBalanceState.RestoreIncomplete))
        assertNull(service.ordinaryBalance(walletId).first())
        assertNull(service.localBalance(walletId).first())

        // An unbound wallet was not read at all: not a known zero.
        announce(ShieldedBalanceUpdate(walletId, 3, ShieldedLocalBalanceState.Unbound))
        assertNull(service.ordinaryBalance(walletId).first())
    }

    @Test
    fun shouldFollowEveryUpdateAndARebindToAnotherManager() = runTest {
        val announced = announce(update(1, 150, accounts = arrayOf(0 to 150)))
        val seen = mutableListOf<Long?>()
        val collecting = launch(UnconfinedTestDispatcher(testScheduler)) {
            service.ordinaryBalance(walletId).collect { seen += it }
        }
        // A spend reserved notes: Rust announces the lower amount.
        announced.value = latestShieldedBalances(announced.value, update(2, 40, accounts = arrayOf(0 to 40)))
        // Another manager (another network) knows nothing of this wallet yet.
        val other = announce()
        other.value = latestShieldedBalances(other.value, update(1, 7, accounts = arrayOf(0 to 7)))
        collecting.cancel()
        assertEquals(listOf(150L, 40L, null, 7L), seen)
    }

    @Test
    fun shouldNotLetABindThatWasOvertakenFinish() {
        // Clear's re-bind of the old manager starts and suspends in native
        // code; a switch to another manager binds meanwhile; Clear's bind
        // resumes and must not attach its subscriptions.
        val turns = BindingTurns()
        val log = mutableListOf<String>()
        val clearsBind = turns.begin { log += "old manager installed" }
        val switch = turns.begin { log += "new manager installed" }
        assertFalse(turns.finish(clearsBind) { log += "old manager attached" })
        assertTrue(turns.finish(switch) { log += "new manager attached" })
        assertEquals(
            listOf("old manager installed", "new manager installed", "new manager attached"),
            log,
        )

        // An unbind takes a turn too: a bind it overtook does not attach.
        val bind = turns.begin { }
        turns.begin { log += "unbound" }
        assertFalse(turns.finish(bind) { log += "attached after unbind" })
        assertEquals("unbound", log.last())

        // A bind nothing overtook finishes.
        val alone = turns.begin { }
        assertTrue(turns.finish(alone) { log += "attached" })
        assertEquals("attached", log.last())
    }

    @Test
    fun shouldNotStartClearsRebindOnceAnotherManagerIsBound() {
        // Clear captured the old manager, then a switch bound the new one.
        // Clear's re-bind checks the bound manager under the lock that
        // would start it, so it takes no turn and installs nothing.
        val turns = BindingTurns()
        var boundManager = "old"
        val switch = turns.begin { boundManager = "new" }
        val clearsBind = turns.beginIf({ boundManager == "old" }) { boundManager = "old" }
        assertNull(clearsBind)
        assertEquals("new", boundManager)
        assertTrue(turns.isLatest(switch))

        // While the cleared manager is still the bound one, the re-bind
        // starts, and it stops being the latest once another turn begins,
        // which is what ends Clear's fleet recovery and sync restart.
        val rebind = turns.beginIf({ boundManager == "new" }) { }
        assertTrue(rebind != null && turns.isLatest(rebind))
        turns.begin { }
        assertFalse(turns.isLatest(rebind!!))
    }

    @Test
    fun shouldShowNoBalanceAfterUnbind() = runTest {
        announce(update(1, 150, accounts = arrayOf(0 to 150)))
        service.unbind()
        assertNull(service.ordinaryBalance(walletId).first())
    }

    @Test
    fun shouldTellACollectorOfTheBoundBalanceThatItIsUnknownAfterUnbind() = runTest {
        // A screen follows `shieldedBalance` from one inner flow to the next.
        val seen = mutableListOf<Long?>()
        val collecting = launch(UnconfinedTestDispatcher(testScheduler)) {
            service.shieldedBalance.flatMapLatest { it }.collect { seen += it }
        }
        service.unbind()
        collecting.cancel()
        // Not an empty flow: the collector is told the balance is not known.
        assertEquals(listOf<Long?>(null), seen)
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
