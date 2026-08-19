package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.persistence.toHex
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The watch-only guard of `unlockWalletFromKeystore` — Swift parity with
 * `verifySeedBinding` (PlatformWalletManager.swift:764-770).
 *
 * The regression these pin: the mnemonic-existence check used to return
 * watch-only BEFORE any status update, so a `seedMismatch` published by an
 * earlier failed verify survived the removal of the Keystore entry and the
 * unlock banner never went away.
 *
 * These run against [isGenuineWatchOnly] — the seam the production call site
 * delegates to WHOLE. The storage read, the hex status-key derivation, the
 * clearing transform, and the early-return decision are all inside the unit
 * under test, exercised through a fake storage probe and a manager-shaped
 * status map; nothing here tests a lambda against itself. Reverting the
 * guard to the pre-fix call-site shape (return watch-only straight off the
 * missing mnemonic, no clear) fails the first and third tests.
 */
class WatchOnlySeedMismatchTest {

    private val walletId = ByteArray(32) { (it + 1).toByte() }
    private val statusKey = walletId.toHex()

    /** Manager-shaped status store: keyed map, transform-based updates. */
    private class RecordingStatusMap {
        val map = mutableMapOf<String, DashPayUnlockStatus>()
        val updatedKeys = mutableListOf<String>()

        fun update(key: String, transform: (DashPayUnlockStatus) -> DashPayUnlockStatus) {
            updatedKeys += key
            map[key] = transform(map[key] ?: DashPayUnlockStatus())
        }
    }

    @Test
    fun noStoredMnemonicClearsSeedMismatchBeforeReportingWatchOnly() = runTest {
        val order = mutableListOf<String>()
        val statuses = RecordingStatusMap()
        // The state an earlier failed verify would have left behind.
        statuses.map[statusKey] = DashPayUnlockStatus(seedMismatch = true)

        val watchOnly = isGenuineWatchOnly(
            walletId = walletId,
            hasMnemonic = { id ->
                order += "read:${id.toHex()}"
                false
            },
            updateUnlockStatus = { key, transform ->
                order += "clear:$key"
                statuses.update(key, transform)
            },
        )
        order += "returned"

        assertTrue("no mnemonic must report genuine watch-only", watchOnly)
        // Ordering is the fix: the probe runs on OUR wallet id, the clear
        // lands on OUR status key, and the clear cannot run after the early
        // return.
        assertEquals(
            listOf("read:$statusKey", "clear:$statusKey", "returned"),
            order,
        )
        assertFalse(
            "the stale mismatch must be gone once watch-only is reported",
            statuses.map.getValue(statusKey).seedMismatch,
        )
    }

    @Test
    fun storedMnemonicDoesNotTouchSeedMismatchAndFallsThroughToVerify() = runTest {
        val statuses = RecordingStatusMap()
        statuses.map[statusKey] = DashPayUnlockStatus(seedMismatch = true)

        val watchOnly = isGenuineWatchOnly(
            walletId = walletId,
            hasMnemonic = { true },
            updateUnlockStatus = statuses::update,
        )

        assertFalse("a wallet holding a mnemonic is not watch-only", watchOnly)
        // The verify publishes the real result; this path must not pre-empt it.
        assertEquals(
            "seedMismatch must be left to the binding verify",
            emptyList<String>(),
            statuses.updatedKeys,
        )
        assertTrue(statuses.map.getValue(statusKey).seedMismatch)
    }

    @Test
    fun clearingDropsAPreviouslyPublishedMismatchAndLeavesSiblingFieldsAlone() = runTest {
        val statuses = RecordingStatusMap()
        statuses.map[statusKey] = DashPayUnlockStatus(
            draining = true,
            seedMismatch = true,
            pendingAccountBuilds = 3,
        )

        val watchOnly = isGenuineWatchOnly(
            walletId = walletId,
            hasMnemonic = { false },
            updateUnlockStatus = statuses::update,
        )

        assertTrue(watchOnly)
        assertEquals(
            "exactly one status write, on the wallet's own key",
            listOf(statusKey),
            statuses.updatedKeys,
        )
        val status = statuses.map.getValue(statusKey)
        assertFalse(status.seedMismatch)
        assertTrue("unrelated unlock state must survive", status.draining)
        assertEquals(3, status.pendingAccountBuilds)
    }
}
