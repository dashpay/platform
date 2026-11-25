package org.dashfoundation.dashsdk.identity

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test

/**
 * The key limits preview crosses JNI as six `long` slots (`preview_slots` in
 * `rs-unified-sdk-jni/src/identity_key_limits.rs`). The reviewed update hands the same slots
 * back, and Rust refuses anything other than what it reviewed, so the round trip must be
 * exact, including values past the signed range.
 */
class IdentityKeyLimitsUpdatePreviewTest {

    private val identityId = ByteArray(32) { 7 }

    @Test
    fun shouldDecodeEveryLimitAndItsFlag() {
        val preview = IdentityKeyLimitsUpdatePreview.fromSlots(
            identityId,
            keyId = 2,
            slots = longArrayOf(0, 0b1111, 1_000, 1_700_000_000_000, 1_500, 1_800_000_000_000),
        )
        assertEquals(2, preview.keyId)
        assertEquals(0, preview.signingKeyId)
        assertEquals(1_000uL, preview.currentTotalBudget)
        assertEquals(1_700_000_000_000uL, preview.currentExpiresAt)
        assertEquals(1_500uL, preview.newTotalBudget)
        assertEquals(1_800_000_000_000uL, preview.newExpiresAt)
        assertArrayEquals(identityId, preview.identityId)
    }

    @Test
    fun shouldReadAClearedFlagAsAbsentWhateverItsSlotHolds() {
        val preview = IdentityKeyLimitsUpdatePreview.fromSlots(
            identityId,
            keyId = 2,
            slots = longArrayOf(1, 0b0001, 1_000, 99, 99, 99),
        )
        assertEquals(1_000uL, preview.currentTotalBudget)
        assertNull(preview.currentExpiresAt)
        assertNull(preview.newTotalBudget)
        assertNull(preview.newExpiresAt)
    }

    @Test
    fun shouldRoundTripValuesPastTheSignedRangeUnchanged() {
        val preview = IdentityKeyLimitsUpdatePreview(
            identityId = identityId,
            keyId = -1, // u32::MAX as the persister writes key ids
            signingKeyId = Int.MIN_VALUE, // 2^31
            currentTotalBudget = ULong.MAX_VALUE,
            currentExpiresAt = null,
            newTotalBudget = ULong.MAX_VALUE - 1uL,
            newExpiresAt = null,
        )
        val slots = preview.toSlots()
        assertEquals(2_147_483_648L, slots[0])
        assertEquals(0b0101L, slots[1])
        assertEquals(-1L, slots[2])
        assertEquals(0L, slots[3])
        assertEquals(preview, IdentityKeyLimitsUpdatePreview.fromSlots(identityId, -1, slots))
    }

    @Test
    fun shouldRefuseSlotsThatAreNotAPreview() {
        assertThrows(IllegalArgumentException::class.java) {
            IdentityKeyLimitsUpdatePreview.fromSlots(identityId, 1, LongArray(5))
        }
        assertThrows(IllegalArgumentException::class.java) {
            IdentityKeyLimitsUpdatePreview.fromSlots(identityId, 1, longArrayOf(-1, 0, 0, 0, 0, 0))
        }
        assertThrows(IllegalArgumentException::class.java) {
            IdentityKeyLimitsUpdatePreview.fromSlots(identityId, 1, longArrayOf(0, 0b10000, 0, 0, 0, 0))
        }
        assertThrows(IllegalArgumentException::class.java) {
            IdentityKeyLimitsUpdatePreview.fromSlots(ByteArray(31), 1, LongArray(6))
        }
    }

    @Test
    fun shouldNotLetACallerChangeTheReviewedIdentity() {
        val preview = IdentityKeyLimitsUpdatePreview.fromSlots(identityId, 1, LongArray(6))
        preview.identityId.fill(0)
        assertArrayEquals(identityId, preview.identityId)
    }
}
