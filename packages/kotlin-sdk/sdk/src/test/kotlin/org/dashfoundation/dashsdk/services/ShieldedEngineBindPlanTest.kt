package org.dashfoundation.dashsdk.services

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Coverage for [engineBindOtherWallets] — the pure iteration seam behind
 * the multi-wallet shielded engine-bind in
 * `AppContainer.rebindWalletScopedServices()` (port of iOS
 * `ShieldedEngineBindPlanTests`).
 *
 * The app engine-binds EVERY loaded wallet into the shared network-scoped
 * shielded coordinator so a single sync pass trial-decrypts against the
 * union of all wallets' viewing keys (SH-14/15/16 cross-wallet flows).
 * The mirror wallet (the lexicographically-first) is bound separately via
 * `ShieldedService.bind`, so this seam binds every OTHER wallet.
 *
 * The real `ShieldedService.bindEngine` calls into JNI and needs a
 * configured `PlatformWalletManager`, so the loop logic — "visit every
 * non-mirror id" and "one id's failure doesn't stop the rest" — is
 * factored into the pure seam and tested with a recording action.
 */
class ShieldedEngineBindPlanTest {

    private fun id(byte: Int) = "%02x".format(byte).repeat(32)

    /**
     * Every non-mirror wallet is engine-bound exactly once; the mirror is
     * skipped (it is bound separately via the UI-mirror path).
     */
    @Test
    fun bindsEveryWalletExceptMirror() = runTest {
        val mirror = id(0x01)
        val others = listOf(id(0x02), id(0x03), id(0x04))
        val all = listOf(mirror) + others

        val bound = mutableListOf<String>()
        engineBindOtherWallets(all, mirror) { bound.add(it) }

        assertEquals(
            "every non-mirror wallet must be engine-bound",
            others.toSet(),
            bound.toSet(),
        )
        assertFalse(
            "the mirror wallet is bound separately and must be skipped here",
            bound.contains(mirror),
        )
        assertEquals(
            "each non-mirror wallet must be bound exactly once",
            others.size,
            bound.size,
        )
    }

    /**
     * A throwing bind for ONE wallet must not stop the others — the
     * production requirement that one wallet's missing mnemonic / declined
     * keystore read cannot dark every other wallet's shielded state.
     */
    @Test
    fun oneFailureDoesNotStopTheRest() = runTest {
        val mirror = id(0x01)
        val failing = id(0x03)
        val all = listOf(mirror, id(0x02), failing, id(0x04))

        val attempted = mutableListOf<String>()
        engineBindOtherWallets(all, mirror) { walletId ->
            attempted.add(walletId)
            if (walletId == failing) throw IllegalStateException("bind failed")
        }

        assertEquals(
            "a throwing bind for one wallet must not skip the remaining wallets",
            setOf(id(0x02), failing, id(0x04)),
            attempted.toSet(),
        )
    }

    /**
     * A single-wallet device (only the mirror loaded) binds nothing extra —
     * the mirror's own engine registration is the UI-mirror path's job.
     */
    @Test
    fun singleMirrorWalletBindsNothing() = runTest {
        val mirror = id(0x01)

        val bound = mutableListOf<String>()
        engineBindOtherWallets(listOf(mirror), mirror) { bound.add(it) }

        assertTrue(
            "with only the mirror wallet loaded there is nothing else to engine-bind",
            bound.isEmpty(),
        )
    }

    /**
     * The mirror is skipped even when it is not the first element — map key
     * sets have no guaranteed order, so the skip must be by value, not
     * position.
     */
    @Test
    fun mirrorSkippedRegardlessOfPosition() = runTest {
        val mirror = id(0x05)
        val all = listOf(id(0x02), id(0x05), id(0x08)) // mirror in the middle

        val bound = mutableListOf<String>()
        engineBindOtherWallets(all, mirror) { bound.add(it) }

        assertEquals(setOf(id(0x02), id(0x08)), bound.toSet())
        assertFalse(bound.contains(mirror))
    }
}
