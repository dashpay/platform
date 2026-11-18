package org.dashfoundation.dashsdk.wallet

import java.util.concurrent.atomic.AtomicLong
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
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
    fun shouldRefuseAReadThatOverlapsABindOrClear() {
        val generation = AtomicLong(0)
        // No bind or clear while the read ran: the value is returned.
        assertEquals(7, fencedByShieldedBinding(generation) { 7 })
        // An earlier bind does not taint a later read.
        generation.incrementAndGet()
        assertEquals(8, fencedByShieldedBinding(generation) { 8 })
        // A bind or clear during the read: the value is not returned.
        assertThrows(ShieldedLocalBalanceBindingChanged::class.java) {
            fencedByShieldedBinding(generation) {
                generation.incrementAndGet()
                9
            }
        }
    }
}
