package org.dashfoundation.dashsdk.identity

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Decoding of the withdrawals BLOB. Port of Swift's `IdentityWithdrawalDecodingTests.swift`,
 * pinned to the bytes the JNI layer writes by [GOLDEN_HEX].
 */
class IdentityWithdrawalsDecodingTest {

    @Test
    fun `should decode the golden rows in Rust's order with every optional gated by its flag`() {
        val rows = decodeIdentityWithdrawals(GOLDEN_HEX.hex())

        assertEquals(2, rows.size)
        val pooled = rows[0]
        assertArrayEquals(ByteArray(32) { 1 }, pooled.id)
        assertArrayEquals(ByteArray(32) { 2 }, pooled.ownerId)
        assertEquals(150_000L, pooled.amountCredits)
        assertEquals(1L, pooled.coreFeePerByte)
        assertEquals(WithdrawalStatus.POOLED, pooled.status)
        assertEquals("Pooled", pooled.statusLabel)
        assertFalse(pooled.isFinal)
        assertEquals("yAbc", pooled.destination)
        assertEquals("76a914", pooled.outputScriptHex)
        assertEquals(42L, pooled.transactionIndex)
        assertNull(pooled.transactionSignHeight)
        assertEquals(1_700_000_000_000L, pooled.createdAtMs)
        assertEquals(1_700_000_600_000L, pooled.updatedAtMs)

        val complete = rows[1]
        assertEquals("03030303", complete.idHex.take(8))
        assertEquals(WithdrawalStatus.COMPLETE, complete.status)
        assertEquals("Complete", complete.statusLabel)
        assertTrue(complete.isFinal)
        assertNull(complete.destination)
        assertEquals("6a00", complete.outputScriptHex)
        assertEquals(7L, complete.transactionIndex)
        assertEquals(1_234_567L, complete.transactionSignHeight)
    }

    @Test
    fun `should decode a zero count as an empty list`() {
        assertEquals(emptyList<IdentityWithdrawal>(), decodeIdentityWithdrawals("00000000".hex()))
    }

    @Test
    fun `should refuse a status code this build does not know`() {
        val bytes = GOLDEN_HEX.hex()
        bytes[STATUS_OFFSET] = 6
        assertThrows(DashSdkError.SerializationError::class.java) { decodeIdentityWithdrawals(bytes) }
    }

    @Test
    fun `should refuse a row without its status label`() {
        // One row, status 0, empty label.
        val row = "00000001" + "00".repeat(64) + "0000000000000000" + "00000000" + "00" +
            "00000000" + "00" + "00" + "00000000" + "00" + "00" + "0000000000000000" +
            "0000000000000000"
        assertThrows(DashSdkError.SerializationError::class.java) { decodeIdentityWithdrawals(row.hex()) }
    }

    @Test
    fun `should refuse a truncated blob and trailing bytes`() {
        val bytes = GOLDEN_HEX.hex()
        assertThrows(DashSdkError.SerializationError::class.java) {
            decodeIdentityWithdrawals(bytes.copyOf(bytes.size - 1))
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            decodeIdentityWithdrawals(bytes + 0)
        }
    }

    @Test
    fun `should map every contract status code and refuse the next one`() {
        assertEquals(
            WithdrawalStatus.entries.toList(),
            (0..5).map { WithdrawalStatus.fromCode(it) },
        )
        assertNull(WithdrawalStatus.fromCode(6))
    }

    private fun String.hex(): ByteArray = chunked(2).map { it.toInt(16).toByte() }.toByteArray()

    private companion object {
        /** count + id + ownerId + amount + coreFeePerByte. */
        const val STATUS_OFFSET = 4 + 32 + 32 + 8 + 4

        /**
         * Mirrors `identity_withdrawals::tests::GOLDEN_HEX` in rs-unified-sdk-jni, which
         * asserts the JNI encoder writes exactly these bytes; change both together.
         */
        const val GOLDEN_HEX =
            "00000002" +
                "0101010101010101010101010101010101010101010101010101010101010101" +
                "0202020202020202020202020202020202020202020202020202020202020202" +
                "00000000000249f0" +
                "00000001" +
                "01" +
                "00000006" + "506f6f6c6564" +
                "00" +
                "01" + "00000004" + "79416263" +
                "00000003" + "76a914" +
                "01" + "000000000000002a" +
                "00" +
                "0000018bcfe56800" +
                "0000018bcfee8fc0" +
                "0303030303030303030303030303030303030303030303030303030303030303" +
                "0202020202020202020202020202020202020202020202020202020202020202" +
                "0000000000000064" +
                "00000002" +
                "03" +
                "00000008" + "436f6d706c657465" +
                "01" +
                "00" +
                "00000002" + "6a00" +
                "01" + "0000000000000007" +
                "01" + "0012d687" +
                "0000000000000001" +
                "0000000000000002"
    }
}
