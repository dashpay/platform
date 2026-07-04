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
            TokenConfigChange.MaxSupply(1_000_000L).payloadJson,
        )
        assertEquals(0, TokenConfigChange.MaxSupply(1L).tag)
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
        // -1L is u64::MAX (18446744073709551615) — the payload must carry the
        // unsigned rendering so the full range survives the signed Long param.
        assertEquals(
            "{\"newMaxSupply\":\"18446744073709551615\"}",
            TokenConfigChange.MaxSupply(-1L).payloadJson,
        )
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
