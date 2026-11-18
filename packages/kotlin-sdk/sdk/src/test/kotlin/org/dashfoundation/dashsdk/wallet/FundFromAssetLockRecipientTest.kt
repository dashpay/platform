package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.Companion.decodePaymentIntent
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.Companion.encodeRecipients
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.AssetLockBuiltOutPoint
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.FundRecipient
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import java.nio.ByteBuffer

/**
 * Pins the host side of the asset-lock funding surface: marshalling
 * recipients faithfully and decoding the reported outpoint. Recipient rules
 * are platform-wallet's alone, so none are checked here.
 *
 * The fee-paying output (`ReduceOutput(index)`) is deliberately NOT derived
 * here: it is positional against the transition's outputs map (a consensus
 * ordering rule), so `platform-wallet` derives it (`remainder_fee_strategy`).
 */
class FundFromAssetLockRecipientTest {

    private fun recipient(tag: Int, credits: Long?) =
        FundRecipient(addressType = 0, hash = ByteArray(20) { tag.toByte() }, credits = credits)

    private data class Row(val type: Int, val hash: ByteArray, val hasBalance: Boolean, val balance: Long)

    private fun decode(blob: ByteArray): List<Row> {
        val buffer = ByteBuffer.wrap(blob)
        val rows = List(buffer.int) {
            val type = buffer.get().toInt() and 0xFF
            val hash = ByteArray(20).also { buffer.get(it) }
            val hasBalance = buffer.get().toInt() != 0
            Row(type, hash, hasBalance, buffer.long)
        }
        assertEquals("trailing bytes after the last row", 0, buffer.remaining())
        return rows
    }

    // ── Marshalling ───────────────────────────────────────────────────

    @Test
    fun shouldEncodeRecipientsOneForOneInCallerOrder() {
        val alice = recipient(0x0A, credits = null)
        val bob = recipient(0xBB, credits = 500)
        val carol = recipient(0xCC, credits = 700)

        val rows = decode(encodeRecipients(listOf(bob, carol, alice)))

        assertEquals(3, rows.size)
        rows.zip(listOf(bob, carol, alice)).forEach { (row, r) -> assertArrayEquals(r.hash, row.hash) }
        assertEquals(listOf(true, true, false), rows.map { it.hasBalance })
        assertEquals(listOf(500L, 700L, 0L), rows.map { it.balance })
        assertEquals(listOf(0, 0, 0), rows.map { it.type })
    }

    @Test
    fun shouldEncodeNoFeeStrategyOnlyTheRemainderFlag() {
        val blob = encodeRecipients(listOf(recipient(0xCC, credits = 700), recipient(0x0A, credits = null)))
        // The blob is exactly the address rows: 4-byte count + 30 bytes per
        // row. The remainder is identified by hasBalance = 0, never by an
        // index computed on this side of the boundary.
        assertEquals(4 + 2 * 30, blob.size)
        assertEquals(1, decode(blob).count { !it.hasBalance })
    }

    @Test
    fun shouldNotConstructARecipientWithAWrongLengthHash() {
        // So a malformed hash never reaches the native blob. Every recipient
        // RULE (remainder count, P2PKH only, ownership) is platform-wallet's.
        listOf(19, 21, 0).forEach { size ->
            assertThrows(IllegalArgumentException::class.java) {
                FundRecipient(addressType = 0, hash = ByteArray(size), credits = 500)
            }
        }
    }

    // ── Asset-lock-built outpoint ─────────────────────────────────────

    @Test
    fun shouldDecodeTheReportedOutpointLikeThePersistedRowKey() {
        val txidWire = ByteArray(32) { it.toByte() }
        val raw = txidWire + byteArrayOf(0x02, 0x01, 0x00, 0x00) // vout 258, little-endian

        val outPoint = AssetLockBuiltOutPoint.fromRawBytes(raw)

        assertArrayEquals(txidWire, outPoint.txid)
        assertEquals(258, outPoint.vout)
        // Display order is the byte-reversed txid — the AssetLockEntity key.
        val displayHex = txidWire.reversedArray().joinToString("") { "%02x".format(it) }
        assertEquals("$displayHex:258", outPoint.outPointHex)
    }

    @Test
    fun shouldRejectAnOutpointThatIsNot36Bytes() {
        listOf(0, 35, 37).forEach { size ->
            assertThrows(IllegalArgumentException::class.java) {
                AssetLockBuiltOutPoint.fromRawBytes(ByteArray(size))
            }
        }
    }

    // ── Payment intent ────────────────────────────────────────────────

    /**
     * The recorded recipients come back as the list the external resume
     * takes: the kind byte, then the same row shape [encodeRecipients] writes.
     */
    @Test
    fun `should decode a recorded payment intent into the resume recipient list`() {
        val recipients = listOf(recipient(1, 1_000_000L), recipient(2, null))
        val blob = byteArrayOf(1) + encodeRecipients(recipients)
        assertEquals(recipients, decodePaymentIntent(blob))
    }

    @Test
    fun `should refuse an unknown payment intent kind or trailing bytes`() {
        val rows = encodeRecipients(listOf(recipient(1, null)))
        assertThrows(IllegalStateException::class.java) { decodePaymentIntent(byteArrayOf(9) + rows) }
        assertThrows(IllegalStateException::class.java) {
            decodePaymentIntent(byteArrayOf(1) + rows + byteArrayOf(0))
        }
    }
}
