package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.queries.AddressBalanceChange
import org.dashfoundation.dashsdk.queries.AddressLeafBoundary
import org.dashfoundation.dashsdk.queries.AddressTreeElement
import org.dashfoundation.dashsdk.queries.CompactedAddressChange
import org.dashfoundation.dashsdk.queries.CreditOperation
import org.dashfoundation.dashsdk.queries.decodeBranchState
import org.dashfoundation.dashsdk.queries.decodeCompactedBalanceChanges
import org.dashfoundation.dashsdk.queries.decodeRecentBalanceChanges
import org.dashfoundation.dashsdk.queries.decodeTrunkState
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

/**
 * Decoding of the balance history and address-sync BLOBs, pinned to the bytes the JNI layer
 * writes: each golden mirrors the constant of the same name in
 * `rs-unified-sdk-jni/src/address_history.rs`, whose tests assert the encoders write exactly
 * those bytes. Change both together.
 */
class AddressHistoryDecodingTest {

    @Test
    fun `should decode both history sections in Rust's order`() {
        val history = decodePlatformAddressBalanceHistory(BALANCE_HISTORY_GOLDEN_HEX.hex())

        assertEquals(12L, history.provedHeight)
        assertEquals(
            listOf(BalanceHistoryEntry(12, 0, "07".repeat(20), "tb1", BalanceHistoryEntry.Change.Added(400))),
            history.recent,
        )
        assertEquals(
            listOf(
                CompactedBalanceHistoryEntry(
                    6, 9, 0, "03".repeat(20), "tb3",
                    CompactedBalanceHistoryEntry.Change.Additions(
                        total = 30,
                        entries = listOf(
                            CompactedBalanceHistoryEntry.Addition(8, 20),
                            CompactedBalanceHistoryEntry.Addition(6, 10),
                        ),
                    ),
                ),
                CompactedBalanceHistoryEntry(
                    1, 5, 1, "01".repeat(20), "tb2",
                    CompactedBalanceHistoryEntry.Change.SetFinal(50),
                ),
            ),
            history.compacted,
        )
    }

    @Test
    fun `should decode an empty history`() {
        val history = decodePlatformAddressBalanceHistory("00000000000000070000000000000000".hex())
        assertEquals(PlatformAddressBalanceHistory(emptyList(), emptyList(), 7), history)
    }

    @Test
    fun `should refuse an unknown change kind, an entry without its address and a truncated blob`() {
        val bytes = BALANCE_HISTORY_GOLDEN_HEX.hex()
        // provedHeight, count, blockHeight, addressType, hash, address (len + "tb1").
        val kindAt = 8 + 4 + 8 + 1 + 20 + 4 + 3
        assertThrows(DashSdkError.SerializationError::class.java) {
            decodePlatformAddressBalanceHistory(bytes.copyOf().also { it[kindAt] = 2 })
        }
        val noAddress = "0000000000000001" + "00000001" + "0000000000000001" + "00" + "00".repeat(20) +
            "00000000" + "01" + "0000000000000001" + "00000000"
        assertThrows(DashSdkError.SerializationError::class.java) {
            decodePlatformAddressBalanceHistory(noAddress.hex())
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            decodePlatformAddressBalanceHistory(bytes.copyOf(bytes.size - 1))
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            decodePlatformAddressBalanceHistory(bytes + 0)
        }
    }

    @Test
    fun `should decode recent changes block by block`() {
        val blocks = decodeRecentBalanceChanges(RECENT_CHANGES_GOLDEN_HEX.hex())
        assertEquals(listOf(100L, 101L), blocks.map { it.blockHeight })
        assertEquals(
            listOf(AddressBalanceChange("00" + "09".repeat(20), CreditOperation.ADD, 5)),
            blocks[0].changes,
        )
        assertEquals(emptyList<AddressBalanceChange>(), blocks[1].changes)
    }

    @Test
    fun `should decode compacted changes with their add entries`() {
        val ranges = decodeCompactedBalanceChanges(COMPACTED_CHANGES_GOLDEN_HEX.hex())
        assertEquals(1, ranges.size)
        assertEquals(1L to 10L, ranges[0].startBlockHeight to ranges[0].endBlockHeight)
        assertEquals(
            listOf(
                CompactedAddressChange("00" + "04".repeat(20), CompactedAddressChange.Operation.SetCredits(77)),
                CompactedAddressChange(
                    "01" + "05".repeat(20),
                    CompactedAddressChange.Operation.AddToCredits(listOf(CompactedAddressChange.Entry(3, 33))),
                ),
            ),
            ranges[0].changes,
        )
    }

    @Test
    fun `should decode the trunk and a branch with the same element layout`() {
        val trunk = decodeTrunkState(TRUNK_STATE_GOLDEN_HEX.hex())
        val elements = listOf(AddressTreeElement("aabb", 3, 1000))
        val leaves = listOf(AddressLeafBoundary("cc", "11".repeat(32), 42))
        assertEquals(900L, trunk.checkpointHeight)
        assertEquals(elements, trunk.elements)
        assertEquals(leaves, trunk.leafBoundaries)

        val branch = decodeBranchState(TRUNK_STATE_GOLDEN_HEX.substring(16).hex())
        assertEquals(elements, branch.elements)
        assertEquals(leaves, branch.leafBoundaries)
    }

    private fun String.hex(): ByteArray = chunked(2).map { it.toInt(16).toByte() }.toByteArray()

    private companion object {
        const val BALANCE_HISTORY_GOLDEN_HEX =
            "000000000000000c00000001000000000000000c0007070707070707070707070707070707070707070000000374623101000000000000019000000002000000000000000600000000000000090003030303030303030303030303030303030303030000000374623301000000000000001e00000002000000000000000800000000000000140000000000000006000000000000000a000000000000000100000000000000050101010101010101010101010101010101010101010000000374623200000000000000003200000000"
        const val RECENT_CHANGES_GOLDEN_HEX =
            "0000000200000000000000640000000100000015000909090909090909090909090909090909090909010000000000000005000000000000006500000000"
        const val COMPACTED_CHANGES_GOLDEN_HEX =
            "000000010000000000000001000000000000000a000000020000001500040404040404040404040404040404040404040400000000000000004d00000000000000150105050505050505050505050505050505050505050100000000000000000000000100000000000000030000000000000021"
        const val TRUNK_STATE_GOLDEN_HEX =
            "00000000000003840000000100000002aabb0000000300000000000003e80000000100000001cc1111111111111111111111111111111111111111111111111111111111111111000000000000002a"
    }
}
