package org.dashfoundation.dashsdk.wallet

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertThrows
import org.junit.Test

/** Reading a shielded balance update, and keeping the latest one per wallet. */
class ShieldedLocalBalanceTest {

    private val walletId = ByteArray(32) { 7 }
    private val otherWallet = ByteArray(32) { 8 }
    private val identityAccount = 0x4000_0000

    private fun ready(
        revision: Long,
        ordinary: Long,
        established: Boolean = true,
        accounts: LongArray = longArrayOf(0, ordinary, 1, 1, 2),
        wallet: ByteArray = walletId,
    ) = decodeShieldedBalanceUpdate(wallet, revision, 2, ordinary, established, accounts)

    @Test
    fun shouldReadAReadyUpdateWithRustsTotalAndEveryAccount() {
        val update = decodeShieldedBalanceUpdate(
            walletId,
            revision = 4,
            status = 2,
            ordinarySpendableCredits = 150,
            ordinaryBalanceEstablished = true,
            accounts = longArrayOf(
                0, 100, 0, 0, 1,
                1, 50, 1, 0, 2,
                identityAccount.toLong(), 900, 1, 42, 2,
            ),
        )
        assertEquals(4L, update.revision)
        val snapshot = (update.state as ShieldedLocalBalanceState.Ready).snapshot
        // The total is the one Rust sent, not a sum of the rows.
        assertEquals(150L, snapshot.ordinarySpendableCredits)
        assertEquals(150L, snapshot.ordinaryBalance)
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
        assertEquals(900L, snapshot.accountBalance(identityAccount))
        // The update is shared by every collector: its accounts are read-only.
        assertThrows(UnsupportedOperationException::class.java) {
            (snapshot.accounts as MutableMap<Int, ShieldedLocalAccountBalance>).clear()
        }
    }

    @Test
    fun shouldNotTakeAnAmountWithoutEvidenceForABalance() {
        // A wallet restored without history: readable, zero, never scanned.
        val unscanned = decodeShieldedBalanceUpdate(
            walletId, 1, 2, 0, false, longArrayOf(0, 0, 0, 0, 0),
        )
        val snapshot = (unscanned.state as ShieldedLocalBalanceState.Ready).snapshot
        assertEquals(0L, snapshot.ordinarySpendableCredits)
        assertNull(snapshot.ordinaryBalance)
        assertNull(snapshot.accountBalance(0))
        // An account that is not bound has no balance either.
        assertNull(snapshot.accountBalance(5))

        // The same zero after a completed scan is a balance of zero.
        val scanned = decodeShieldedBalanceUpdate(
            walletId, 2, 2, 0, true, longArrayOf(0, 0, 1, 0, 2),
        )
        val known = (scanned.state as ShieldedLocalBalanceState.Ready).snapshot
        assertEquals(0L, known.ordinaryBalance)
        assertEquals(0L, known.accountBalance(0))
    }

    @Test
    fun shouldReadStatesThatCarryNoNumbers() {
        assertEquals(
            ShieldedLocalBalanceState.Unbound,
            decodeShieldedBalanceUpdate(walletId, 1, 0, 0, false, longArrayOf()).state,
        )
        assertEquals(
            ShieldedLocalBalanceState.RestoreIncomplete,
            decodeShieldedBalanceUpdate(walletId, 1, 1, 0, false, longArrayOf()).state,
        )
    }

    @Test
    fun shouldRefuseAnUpdateItCannotReadExactly() {
        val row = longArrayOf(0, 100, 0, 0, 1)
        val malformed: List<() -> Unit> = listOf(
            { decodeShieldedBalanceUpdate(ByteArray(31), 1, 2, 100, true, row) },
            { decodeShieldedBalanceUpdate(walletId, 0, 2, 100, true, row) },
            // Ready without accounts, and accounts on a state without numbers.
            { decodeShieldedBalanceUpdate(walletId, 1, 2, 0, true, longArrayOf()) },
            { decodeShieldedBalanceUpdate(walletId, 1, 0, 0, false, row) },
            { decodeShieldedBalanceUpdate(walletId, 1, 1, 0, false, row) },
            // A row cut short, and an unknown status, flag and source.
            { decodeShieldedBalanceUpdate(walletId, 1, 2, 100, true, longArrayOf(0, 100, 0, 0)) },
            { decodeShieldedBalanceUpdate(walletId, 1, 3, 0, false, longArrayOf()) },
            { decodeShieldedBalanceUpdate(walletId, 1, 2, 100, true, longArrayOf(0, 100, 2, 0, 1)) },
            { decodeShieldedBalanceUpdate(walletId, 1, 2, 100, true, longArrayOf(0, 100, 0, 0, 3)) },
            // An account index outside Int, and the same account twice.
            { decodeShieldedBalanceUpdate(walletId, 1, 2, 100, true, longArrayOf(0x1_0000_0000, 100, 0, 0, 1)) },
            { decodeShieldedBalanceUpdate(walletId, 1, 2, 100, true, longArrayOf(-1, 100, 0, 0, 1)) },
            { decodeShieldedBalanceUpdate(walletId, 1, 2, 100, true, row + row) },
        )
        malformed.forEachIndexed { index, decode ->
            assertThrows("accepted malformed update $index", IllegalStateException::class.java) {
                decode()
            }
        }
    }

    @Test
    fun shouldKeepTheLatestUpdateOfEachWallet() {
        val first = ready(revision = 1, ordinary = 100)
        val second = ready(revision = 2, ordinary = 70)
        val other = ready(revision = 1, ordinary = 900, wallet = otherWallet)

        var held = latestShieldedBalances(emptyMap(), first)
        held = latestShieldedBalances(held, other)
        held = latestShieldedBalances(held, second)
        assertEquals(second, held[shieldedBalanceKey(walletId)])
        assertEquals(other, held[shieldedBalanceKey(otherWallet)])

        // An update that arrives late, or twice, changes nothing.
        assertSame(held, latestShieldedBalances(held, first))
        assertSame(held, latestShieldedBalances(held, second))

        // A later state without numbers replaces the numbers.
        val cleared = decodeShieldedBalanceUpdate(walletId, 3, 1, 0, false, longArrayOf())
        held = latestShieldedBalances(held, cleared)
        assertEquals(ShieldedLocalBalanceState.RestoreIncomplete, held[shieldedBalanceKey(walletId)]?.state)
        assertEquals(other, held[shieldedBalanceKey(otherWallet)])
    }
}
