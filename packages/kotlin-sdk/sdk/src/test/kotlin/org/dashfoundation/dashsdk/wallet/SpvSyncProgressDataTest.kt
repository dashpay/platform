package org.dashfoundation.dashsdk.wallet

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Flat-array → [SpvSyncProgressData] marshalling (mirror of the JNI
 * `spvSyncProgress` out-array layout) and the sync-liveness derivation.
 */
class SpvSyncProgressDataTest {

    @Test
    fun fromNativeMapsFieldsAndPresenceFlags() {
        // longs[17]: overallState=2(Syncing); headers present, syncing, 50/100;
        // filterHeaders absent; filters absent; masternodes present, synced.
        val longs = longArrayOf(
            2, // overallState
            1, 2, 50, 100, // headers: has, state=Syncing, cur, tgt
            0, 0, 0, 0, // filterHeaders: absent
            0, 0, 0, 0, // filters: absent
            1, 3, 10, 10, // masternodes: has, state=Synced, cur, tgt
        )
        val percentages = doubleArrayOf(0.5, 0.5, 0.0, 0.0, 1.0)

        val p = SpvSyncProgressData.fromNative(longs, percentages)
        assertEquals(SpvSyncState.SYNCING, p.overallState)
        assertEquals(0.5, p.overallPercentage, 1e-9)

        assertTrue(p.headers != null)
        assertEquals(SpvSyncState.SYNCING, p.headers!!.state)
        assertEquals(50L, p.headers!!.currentHeight)
        assertEquals(100L, p.headers!!.targetHeight)
        assertEquals(0.5, p.headers!!.percentage, 1e-9)

        assertNull(p.filterHeaders)
        assertNull(p.filters)

        assertTrue(p.masternodes != null)
        assertEquals(SpvSyncState.SYNCED, p.masternodes!!.state)
    }

    @Test
    fun isSyncingReflectsOverallState() {
        assertTrue(SpvSyncProgressData.EMPTY.copy(overallState = SpvSyncState.SYNCING).isSyncing)
        assertTrue(
            SpvSyncProgressData.EMPTY
                .copy(overallState = SpvSyncState.WAITING_FOR_CONNECTIONS).isSyncing,
        )
        assertEquals(false, SpvSyncProgressData.EMPTY.isSyncing)
        assertEquals(
            false,
            SpvSyncProgressData.EMPTY.copy(overallState = SpvSyncState.SYNCED).isSyncing,
        )
    }

    @Test
    fun unknownStateRawFallsBackToWaitForEvents() {
        assertEquals(SpvSyncState.WAIT_FOR_EVENTS, SpvSyncState.fromRaw(99))
    }
}
