package org.dashfoundation.dashsdk.wallet

import java.util.concurrent.CountDownLatch
import kotlin.concurrent.thread
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/** Decoding of the flat `long[]` the native local balance snapshot returns. */
class ShieldedLocalBalanceTest {

    private val identityAccount = 0x4000_0000

    @Test
    fun shouldDecodeAReadySnapshotWithItsOrdinaryTotalAndEveryAccount() {
        val state = decodeLocalShieldedBalance(
            longArrayOf(
                2, 150, 3,
                0, 100, 0, 0, 1,
                1, 50, 1, 0, 2,
                identityAccount.toLong(), 900, 1, 42, 2,
            ),
        )
        val snapshot = (state as ShieldedLocalBalanceState.Ready).snapshot
        // The total is the one Rust sent, not a sum of the rows.
        assertEquals(150L, snapshot.ordinarySpendableCredits)
        assertEquals(setOf(0, 1, identityAccount), snapshot.accounts.keys)
        assertEquals(
            ShieldedLocalAccountBalance(100, null, ShieldedBalanceSource.RESTORED),
            snapshot.accounts[0],
        )
        // A completed scan that ended at index zero is not "no scan".
        assertEquals(
            ShieldedLocalAccountBalance(50, 0, ShieldedBalanceSource.SCANNED_THIS_SESSION),
            snapshot.accounts[1],
        )
        assertEquals(
            ShieldedLocalAccountBalance(900, 42, ShieldedBalanceSource.SCANNED_THIS_SESSION),
            snapshot.accounts[identityAccount],
        )
    }

    @Test
    fun shouldDecodeStatesThatCarryNoNumbers() {
        assertEquals(
            ShieldedLocalBalanceState.Unbound,
            decodeLocalShieldedBalance(longArrayOf(0, 0, 0)),
        )
        assertEquals(
            ShieldedLocalBalanceState.RestoreIncomplete,
            decodeLocalShieldedBalance(longArrayOf(1, 0, 0)),
        )
    }

    @Test
    fun shouldRefuseALayoutItCannotReadExactly() {
        val malformed = listOf(
            longArrayOf(),
            longArrayOf(2, 150),
            // Count says one account, no row follows.
            longArrayOf(2, 150, 1),
            // A row is cut short.
            longArrayOf(2, 150, 1, 0, 100, 0, 0),
            // Values after the last row.
            longArrayOf(2, 150, 1, 0, 100, 0, 0, 1, 9),
            longArrayOf(2, 150, -1),
            // Ready without accounts, and accounts on a state without numbers.
            longArrayOf(2, 0, 0),
            longArrayOf(0, 0, 1, 0, 100, 0, 0, 1),
            longArrayOf(1, 0, 1, 0, 100, 0, 0, 1),
            // Unknown status, flag and source.
            longArrayOf(3, 0, 0),
            longArrayOf(2, 150, 1, 0, 100, 2, 0, 1),
            longArrayOf(2, 150, 1, 0, 100, 0, 0, 3),
            // An account index outside Int, and the same account twice.
            longArrayOf(2, 150, 1, 0x1_0000_0000, 100, 0, 0, 1),
            longArrayOf(2, 150, 1, -1, 100, 0, 0, 1),
            longArrayOf(2, 150, 2, 0, 100, 0, 0, 1, 0, 50, 0, 0, 1),
        )
        for (values in malformed) {
            assertThrows(
                "accepted ${values.toList()}",
                IllegalStateException::class.java,
            ) { decodeLocalShieldedBalance(values) }
        }
    }

    @Test
    fun shouldReturnAReadThatNoBindOrClearOverlaps() {
        val fence = ShieldedBindingFence()
        assertEquals(7, fence.reading { 7 })
        // A bind or clear that finished earlier does not taint a later read.
        fence.changing { }
        assertEquals(8, fence.reading { 8 })
        // Nor does one that failed.
        assertThrows(IllegalStateException::class.java) { fence.changing { error("bind failed") } }
        assertEquals(9, fence.reading { 9 })
    }

    @Test
    fun shouldRefuseAReadThatABindOrClearStartsAndFinishesInside() {
        val fence = ShieldedBindingFence()
        assertThrows(ShieldedLocalBalanceBindingChanged::class.java) {
            fence.reading {
                // The native read captured the old binding; the bind then
                // replaced it before the read came back.
                fence.changing { }
                1
            }
        }
    }

    @Test
    fun shouldRefuseAReadThatStartsWhileABindOrClearIsRunning() {
        val fence = ShieldedBindingFence()
        var readRan = false
        fence.changing {
            // The change has started and has not returned: a read here could
            // capture either binding, so it is refused before it runs.
            assertThrows(ShieldedLocalBalanceBindingChanged::class.java) {
                fence.reading { readRan = true }
            }
        }
        assertFalse(readRan)
    }

    @Test
    fun shouldRefuseAReadThatReturnsWhileABindOrClearIsStillRunning() {
        val fence = ShieldedBindingFence()
        val readCaptured = CountDownLatch(1)
        val changeStarted = CountDownLatch(1)
        val readDelivered = CountDownLatch(1)
        var failure: Throwable? = null
        val reader = thread {
            failure = runCatching {
                fence.reading {
                    // Captured the old binding, then the change starts and is
                    // still in native code when this read returns.
                    readCaptured.countDown()
                    changeStarted.await()
                    1
                }
            }.exceptionOrNull()
            readDelivered.countDown()
        }
        readCaptured.await()
        fence.changing {
            changeStarted.countDown()
            readDelivered.await()
        }
        reader.join()
        assertTrue(failure is ShieldedLocalBalanceBindingChanged)
    }
}
