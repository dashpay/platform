package org.dashfoundation.dashsdk.wallet

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The watch-only arm of `unlockWalletFromKeystore` — Swift parity with
 * `verifySeedBinding` (PlatformWalletManager.swift:764-770).
 *
 * The regression these pin: the mnemonic-existence check used to return
 * watch-only BEFORE any status update, so a `seedMismatch` published by an
 * earlier failed verify survived the removal of the Keystore entry and the
 * unlock banner never went away.
 */
class WatchOnlySeedMismatchTest {

    @Test
    fun noStoredMnemonicClearsSeedMismatchBeforeReportingWatchOnly() {
        val order = mutableListOf<String>()

        val watchOnly = isGenuineWatchOnly(
            hasMnemonic = false,
            clearSeedMismatch = { order += "clear" },
        )
        order += "returned"

        assertTrue("no mnemonic must report genuine watch-only", watchOnly)
        // Ordering is the fix: the clear cannot run after the early return.
        assertEquals(listOf("clear", "returned"), order)
    }

    @Test
    fun storedMnemonicDoesNotTouchSeedMismatchAndFallsThroughToVerify() {
        var cleared = false

        val watchOnly = isGenuineWatchOnly(
            hasMnemonic = true,
            clearSeedMismatch = { cleared = true },
        )

        assertFalse("a wallet holding a mnemonic is not watch-only", watchOnly)
        // The verify publishes the real result; this path must not pre-empt it.
        assertFalse("seedMismatch must be left to the binding verify", cleared)
    }

    @Test
    fun clearingDropsAPreviouslyPublishedMismatchAndLeavesSiblingFieldsAlone() {
        // The transform the manager hands to `isGenuineWatchOnly`, applied to
        // the state an earlier failed verify would have left behind.
        var status = DashPayUnlockStatus(
            draining = true,
            seedMismatch = true,
            pendingAccountBuilds = 3,
        )

        isGenuineWatchOnly(
            hasMnemonic = false,
            clearSeedMismatch = { status = status.copy(seedMismatch = false) },
        )

        assertFalse(status.seedMismatch)
        assertTrue("unrelated unlock state must survive", status.draining)
        assertEquals(3, status.pendingAccountBuilds)
    }
}
