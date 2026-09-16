package org.dashfoundation.dashsdk.tokens

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Unit tests for the token parameter-marshalling logic — the flattening of
 * [GroupAction] into the native `(kind, position, actionId, actionIsProposer)`
 * tuple, and [TokenConfigChange] payload JSON rendering. The kind tags and
 * payload shapes must stay in sync with
 * `rs-platform-wallet-ffi/src/tokens/group_info.rs` and
 * `.../tokens/update_config.rs`.
 */
class GroupActionTest {

    @Test
    fun noneFlattensToKindZero() {
        val flat = GroupAction.None.flatten()
        assertEquals(0, flat.kind)
        assertEquals(0, flat.position)
        assertNull(flat.actionId)
        assertFalse(flat.actionIsProposer)
    }

    @Test
    fun proposeFlattensToKindOneWithPosition() {
        val flat = GroupAction.Propose(position = 7).flatten()
        assertEquals(1, flat.kind)
        assertEquals(7, flat.position)
        assertNull(flat.actionId)
        assertFalse(flat.actionIsProposer)
    }

    @Test
    fun signExistingFlattensToKindTwoCarryingActionId() {
        val actionId = ByteArray(32) { it.toByte() }
        val flat = GroupAction.SignExisting(
            position = 3,
            actionId = actionId,
            actionIsProposer = true,
        ).flatten()
        assertEquals(2, flat.kind)
        assertEquals(3, flat.position)
        assertArrayEquals(actionId, flat.actionId)
        assertTrue(flat.actionIsProposer)
    }

    @Test
    fun signExistingRejectsWrongLengthActionId() {
        val ex = assertThrows(IllegalArgumentException::class.java) {
            GroupAction.SignExisting(
                position = 0,
                actionId = ByteArray(16),
                actionIsProposer = false,
            ).flatten()
        }
        assertTrue(ex.message!!.contains("32 bytes"))
    }

    @Test
    fun proposeRejectsPositionOutsideU16() {
        assertThrows(IllegalArgumentException::class.java) {
            GroupAction.Propose(position = 70_000).flatten()
        }
    }

    @Test
    fun maxSupplyRendersDecimalStringPayload() {
        assertEquals(
            "{\"newMaxSupply\":\"1000000\"}",
            TokenConfigChange.MaxSupply(1_000_000uL).payloadJson,
        )
        assertEquals(0, TokenConfigChange.MaxSupply(1uL).tag)
    }

    @Test
    fun maxSupplyNullRemovesCap() {
        assertEquals(
            "{\"newMaxSupply\":null}",
            TokenConfigChange.MaxSupply(null).payloadJson,
        )
    }

    @Test
    fun maxSupplyEncodesFullU64RangeAsUnsigned() {
        // The decimal JSON representation must preserve u64::MAX.
        assertEquals(
            "{\"newMaxSupply\":\"18446744073709551615\"}",
            TokenConfigChange.MaxSupply(ULong.MAX_VALUE).payloadJson,
        )
    }

    @Test
    fun nativeLongCarrierPreservesUnsignedBoundaryBits() {
        listOf(0uL, Long.MAX_VALUE.toULong(), 1uL shl 63, ULong.MAX_VALUE).forEach { value ->
            assertEquals(value, value.toNativeLongBits().fromNativeLongBits())
        }
        assertEquals(Long.MIN_VALUE, (1uL shl 63).toNativeLongBits())
        assertEquals(-1L, ULong.MAX_VALUE.toNativeLongBits())
    }

    @Test
    fun distributionTypeDiscriminantsMatchFfi() {
        assertEquals(0, TokenDistributionType.PRE_PROGRAMMED.value)
        assertEquals(1, TokenDistributionType.PERPETUAL.value)
    }

    @Test
    fun groupStatusDiscriminantsMatchFfi() {
        assertEquals(0, Groups.Status.ACTIVE.value)
        assertEquals(1, Groups.Status.CLOSED.value)
    }
}
