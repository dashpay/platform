package org.dashfoundation.dashsdk.wallet

import java.nio.ByteBuffer
import java.nio.ByteOrder
import org.junit.Assert.assertEquals
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the 57-byte outcome blob ABI shared with the JNI serializer in
 * `rs-unified-sdk-jni/src/wallet_manager.rs` — the layout table lives on
 * both sides and this test is the Kotlin half of the contract.
 */
class WalletStartupTest {

    private fun blob(
        status: Int = 0,
        hasIdentity: Boolean = true,
        identity: ByteArray = ByteArray(32) { it.toByte() },
        discoveryAttempts: Int = 3,
        dashPaySyncRan: Boolean = true,
        seedBindingUnverified: Boolean = false,
        identityScanIncomplete: Boolean = false,
        drained: Int = 4,
        pending: Int = 0,
        elapsedMs: Long = 12_345L,
    ): ByteArray {
        val buf = ByteBuffer.allocate(WalletStartupOutcome.BLOB_SIZE).order(ByteOrder.BIG_ENDIAN)
        buf.put(status.toByte())
        buf.put(if (hasIdentity) 1 else 0)
        buf.put(identity)
        buf.putInt(discoveryAttempts)
        buf.put(if (dashPaySyncRan) 1 else 0)
        buf.put(if (seedBindingUnverified) 1 else 0)
        buf.put(if (identityScanIncomplete) 1 else 0)
        buf.putInt(drained)
        buf.putInt(pending)
        buf.putLong(elapsedMs)
        return buf.array()
    }

    @Test
    fun decodesEveryFieldOfAReadyOutcome() {
        val identity = ByteArray(32) { (it * 3).toByte() }
        val outcome = WalletStartupOutcome.decode(
            blob(
                status = 0,
                hasIdentity = true,
                identity = identity,
                discoveryAttempts = 2,
                dashPaySyncRan = true,
                seedBindingUnverified = false,
                identityScanIncomplete = false,
                drained = 5,
                pending = 1,
                elapsedMs = 9_876L,
            ),
        )
        assertEquals(WalletStartupStatus.READY, outcome.status)
        assertArrayEquals(identity, outcome.identityId)
        assertEquals(2, outcome.discoveryAttempts)
        assertTrue(outcome.dashPaySyncRan)
        assertFalse(outcome.seedBindingUnverified)
        assertFalse(outcome.identityScanIncomplete)
        assertEquals(5, outcome.contactAccountsDrained)
        assertEquals(1, outcome.contactAccountsPending)
        assertEquals(9_876L, outcome.elapsedMs)
    }

    @Test
    fun absentIdentityDecodesToNullEvenWhenBytesAreSet() {
        // The JNI side zeroes the array when has_identity_id is false, but
        // the decoder must key on the flag, not the bytes.
        val outcome = WalletStartupOutcome.decode(
            blob(status = 1, hasIdentity = false, identity = ByteArray(32) { 7 }),
        )
        assertEquals(WalletStartupStatus.NO_IDENTITY, outcome.status)
        assertNull(outcome.identityId)
    }

    @Test
    fun everyAbiDiscriminantRoundTrips() {
        for (status in WalletStartupStatus.entries) {
            val outcome = WalletStartupOutcome.decode(blob(status = status.raw))
            assertEquals(status, outcome.status)
        }
    }

    @Test
    fun unknownDiscriminantThrows() {
        assertThrows(IllegalArgumentException::class.java) {
            WalletStartupOutcome.decode(blob(status = 200))
        }
    }

    @Test
    fun wrongBlobSizeThrows() {
        assertThrows(IllegalArgumentException::class.java) {
            WalletStartupOutcome.decode(ByteArray(WalletStartupOutcome.BLOB_SIZE - 1))
        }
        assertThrows(IllegalArgumentException::class.java) {
            WalletStartupOutcome.decode(ByteArray(WalletStartupOutcome.BLOB_SIZE + 1))
        }
    }

    @Test
    fun statusHelperSemanticsMatchTheSwiftBinding() {
        // discoveryWorthRetrying: only the two "Platform not fully asked" cases.
        assertTrue(WalletStartupStatus.PARTIAL_NO_IDENTITY.discoveryWorthRetrying)
        assertTrue(WalletStartupStatus.IDENTITY_SCAN_INCOMPLETE.discoveryWorthRetrying)
        assertFalse(WalletStartupStatus.READY.discoveryWorthRetrying)
        assertFalse(WalletStartupStatus.NO_IDENTITY.discoveryWorthRetrying)
        assertFalse(WalletStartupStatus.DISCOVERY_FAILED.discoveryWorthRetrying)
        assertFalse(WalletStartupStatus.SEED_BINDING_UNVERIFIED.discoveryWorthRetrying)
        assertFalse(WalletStartupStatus.PARTIAL_ACCOUNTS_PENDING.discoveryWorthRetrying)
        // identityIsSettled: not the inverse — DISCOVERY_FAILED is unsettled
        // AND not worth retrying; IDENTITY_SCAN_INCOMPLETE is settled AND
        // worth retrying.
        assertFalse(WalletStartupStatus.PARTIAL_NO_IDENTITY.identityIsSettled)
        assertFalse(WalletStartupStatus.DISCOVERY_FAILED.identityIsSettled)
        assertTrue(WalletStartupStatus.IDENTITY_SCAN_INCOMPLETE.identityIsSettled)
        assertTrue(WalletStartupStatus.READY.identityIsSettled)
        assertTrue(WalletStartupStatus.NO_IDENTITY.identityIsSettled)
        assertTrue(WalletStartupStatus.SEED_BINDING_UNVERIFIED.identityIsSettled)
        assertTrue(WalletStartupStatus.PARTIAL_ACCOUNTS_PENDING.identityIsSettled)
    }
}
