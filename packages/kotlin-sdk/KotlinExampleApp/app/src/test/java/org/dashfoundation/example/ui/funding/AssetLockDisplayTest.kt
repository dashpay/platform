package org.dashfoundation.example.ui.funding

import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Display-helper + outpoint-parse tests for the ADDR-03 resume surface —
 * mirror of the invariants encoded in `PersistentAssetLockDisplay.swift`
 * (`isVisibleAsResumable`, `canFundIdentity`, `statusLabel`,
 * `shortOutPointDisplay`) and the byte-order contract of
 * `FundFromAssetLockPlatformAddressView.parseOutPoint`.
 */
class AssetLockDisplayTest {

    private fun lock(
        statusRaw: Int,
        outPointHex: String = "${"ab".repeat(32)}:0",
        amountDuffs: Long = 100_000L,
        fundingTypeRaw: Int = 4,
    ): AssetLockEntity = AssetLockEntity(
        outPointHex = outPointHex,
        walletId = ByteArray(32),
        transactionBytes = ByteArray(0),
        fundingTypeRaw = fundingTypeRaw,
        identityIndexRaw = 0,
        amountDuffs = amountDuffs,
        statusRaw = statusRaw,
    )

    @Test
    fun `isVisibleAsResumable covers 1 through 3 plus RecoveredFromChain`() {
        assertFalse(lock(0).isVisibleAsResumable) // Built
        assertTrue(lock(1).isVisibleAsResumable) // Broadcast
        assertTrue(lock(2).isVisibleAsResumable) // InstantSendLocked
        assertTrue(lock(3).isVisibleAsResumable) // ChainLocked
        assertFalse(lock(4).isVisibleAsResumable) // Consumed
        assertTrue(lock(5).isVisibleAsResumable) // RecoveredFromChain
    }

    /**
     * The exclusion of `4` is by NAME, not by an upper bound — `5` sits
     * above it numerically and is resumable. A `statusRaw <= 3` bound reads
     * as equivalent and silently hides every restored lock.
     */
    @Test
    fun `isVisibleAsResumable excludes Consumed without bounding above it`() {
        assertFalse(lock(4).isVisibleAsResumable)
        assertTrue(lock(5).isVisibleAsResumable)
    }

    /**
     * RecoveredFromChain (5) is FUNDABLE. The restore scan and the
     * chainlock-promotion path attach a real `ChainAssetLockProof` before
     * writing it, so Core-side finality is proven — treating it as
     * not-yet-final routed the row into the "waiting for InstantSend /
     * ChainLock finality" copy and told the user to wait for something that
     * had already happened.
     */
    @Test
    fun `canFundIdentity covers 2, 3 and RecoveredFromChain`() {
        assertFalse(lock(0).canFundIdentity)
        assertFalse(lock(1).canFundIdentity)
        assertTrue(lock(2).canFundIdentity)
        assertTrue(lock(3).canFundIdentity)
        assertFalse(lock(4).canFundIdentity)
        assertTrue(lock(5).canFundIdentity)
    }

    @Test
    fun `statusLabel maps every discriminant`() {
        assertEquals("Built", lock(0).statusLabel)
        assertEquals("Broadcast", lock(1).statusLabel)
        assertEquals("InstantSendLocked", lock(2).statusLabel)
        assertEquals("ChainLocked", lock(3).statusLabel)
        assertEquals("Consumed", lock(4).statusLabel)
        // Regression: rendered as "Unknown(5)" before the branch existed.
        assertEquals("RecoveredFromChain", lock(5).statusLabel)
        assertEquals("Unknown(7)", lock(7).statusLabel)
    }

    @Test
    fun `shortOutPointDisplay takes 8 txid chars plus vout`() {
        val hex = "0011223344556677" + "88".repeat(28) // 64 chars
        assertEquals("00112233:5", lock(1, "$hex:5").shortOutPointDisplay)
    }

    @Test
    fun `parseOutPoint reverses display hex to wire order and reads vout`() {
        // Display order 00,01,...,1f (32 bytes). Wire order is the reverse.
        val displayHex = (0 until 32).joinToString("") { "%02x".format(it) }
        val parsed = parseOutPoint("$displayHex:7")
        assertTrue(parsed != null)
        val (txid, vout) = parsed!!
        assertEquals(7, vout)
        assertEquals(32, txid.size)
        val expectedWire = ByteArray(32) { (31 - it).toByte() }
        assertArrayEquals(expectedWire, txid)
    }

    @Test
    fun `parseOutPoint round-trips a resumable lock outpoint to 32 wire bytes`() {
        // The most load-bearing invariant: whatever hex the persister wrote,
        // parseOutPoint must yield exactly 32 bytes (resumeFundFromAssetLock
        // require(size == 32)) in reversed (wire) order.
        val displayHex = "ab".repeat(32)
        val (txid, vout) = parseOutPoint("$displayHex:0")!!
        assertEquals(32, txid.size)
        assertEquals(0, vout)
        // ab reversed is still ab, so all bytes stay 0xab here — the length +
        // reversal contract is asserted by the previous test.
        assertArrayEquals(ByteArray(32) { 0xAB.toByte() }, txid)
    }

    @Test
    fun `parseOutPoint rejects malformed input`() {
        assertNull(parseOutPoint("nocolon"))
        assertNull(parseOutPoint("${"ab".repeat(32)}:notanumber"))
        assertNull(parseOutPoint("${"ab".repeat(30)}:0")) // 60 chars, not 64
        assertNull(parseOutPoint("${"zz".repeat(32)}:0")) // non-hex
    }
}
