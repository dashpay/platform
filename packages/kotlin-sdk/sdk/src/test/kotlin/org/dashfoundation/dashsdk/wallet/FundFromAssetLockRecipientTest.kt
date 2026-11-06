package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.Companion.encodeRecipients
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.Companion.fundFromAssetLockExternalPreflight
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.AssetLockBuiltOutPoint
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.FundRecipient
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import java.nio.ByteBuffer

/**
 * Pins the two host-side responsibilities of the asset-lock funding surface —
 * marshalling recipients faithfully, and rejecting recipient lists Kotlin can
 * check synchronously. Port of Swift's `FundFromAssetLockRecipientTests`.
 *
 * The fee-paying output (`ReduceOutput(index)`) is deliberately NOT derived
 * here: it is positional against the transition's outputs map (a consensus
 * ordering rule), so `platform-wallet` derives it (`remainder_fee_strategy`).
 */
class FundFromAssetLockRecipientTest {

    private fun recipient(tag: Int, credits: Long?, type: Int = 0) =
        FundRecipient(addressType = type, hash = ByteArray(20) { tag.toByte() }, credits = credits)

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

    // ── External pre-flight ───────────────────────────────────────────

    @Test
    fun shouldAcceptThirdPartyPayeeWithOwnRemainder() {
        fundFromAssetLockExternalPreflight(
            listOf(recipient(0xBB, credits = 500), recipient(0x0A, credits = null)),
        )
    }

    @Test
    fun shouldRejectRemainderOnlyRequest() {
        // Nothing is paid externally — a call-site mistake; fundFromAssetLock
        // validates the destination properly.
        assertThrows(IllegalArgumentException::class.java) {
            fundFromAssetLockExternalPreflight(listOf(recipient(0x0A, credits = null)))
        }
    }

    @Test
    fun shouldRejectP2shAndBadHashLengths() {
        assertThrows(IllegalArgumentException::class.java) {
            fundFromAssetLockExternalPreflight(
                listOf(recipient(0xBB, credits = 500, type = 1), recipient(0x0A, credits = null)),
            )
        }
        // A wrong-length hash cannot even be constructed, so it never reaches
        // the pre-flight or the native blob.
        listOf(19, 21, 0).forEach { size ->
            assertThrows(IllegalArgumentException::class.java) {
                FundRecipient(addressType = 0, hash = ByteArray(size), credits = 500)
            }
        }
    }

    @Test
    fun shouldRejectWrongRemainderCount() {
        listOf(
            emptyList(),
            listOf(recipient(0x0A, credits = 500), recipient(0xBB, credits = 700)),
            listOf(recipient(0x0A, credits = null), recipient(0xBB, credits = null)),
            listOf(recipient(0x0A, credits = null), recipient(0xBB, credits = null), recipient(0xCC, credits = 1)),
        ).forEach { recipients ->
            assertThrows("accepted $recipients", IllegalArgumentException::class.java) {
                fundFromAssetLockExternalPreflight(recipients)
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
}
