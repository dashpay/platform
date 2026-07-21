package org.dashfoundation.dashsdk.keywallet

import java.nio.ByteBuffer
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Host-JVM tests of [TransactionDecoder.parseBlob] — the Kotlin half of the
 * JNI decode contract. The fixture blob is the EXACT byte string the Rust
 * side pins in `rs-unified-sdk-jni/src/tx_decode.rs`
 * (`fixture_blob_hex_is_pinned_for_kotlin`): decoded from a deterministic
 * one-input / two-output P2PKH spend on testnet, so the layout is verified
 * from both ends without loading the native library. Regenerate BOTH
 * constants together if the layout ever changes.
 */
class TransactionDecoderTest {

    @Test
    fun `parses the Rust-pinned fixture blob`() {
        val decoded = TransactionDecoder.parseBlob(FIXTURE_BLOB_HEX.hexToBytes())

        // txid is consensus (internal) order; display rendering reverses it.
        assertEquals(
            "d5b51c39a335f82c33beee64bbcdf9c62418884c6511bf606fa75b6e217974bf",
            decoded.txid.joinToString("") { "%02x".format(it) }
        )
        assertEquals(
            "bf7479216e5ba76f60bf11654c881824c6f9cdbb64eebe332cf835a3391cb5d5",
            decoded.txidDisplayHex
        )

        assertEquals(1, decoded.inputs.size)
        val input = decoded.inputs[0]
        assertArrayEquals(ByteArray(32) { 0x11 }, input.prevTxid)
        assertEquals(3, input.prevVout)
        assertEquals("yNDj28QBMm5sY6bLjFcNdWRNef24KLQNuQ", input.address)
        assertEquals("11".repeat(32), input.prevTxidDisplayHex)

        assertEquals(2, decoded.outputs.size)
        val paid = decoded.outputs[0]
        assertEquals("yNDj28QBMm5sY6bLjFcNdWRNef24KLQNuQ", paid.address)
        assertEquals(151_072L, paid.valueDuffs)
        assertEquals(
            "76a91414db4138d56a2ecfb10881a9be394d9f321985b288ac",
            paid.scriptPubkey.joinToString("") { "%02x".format(it) }
        )

        val opReturn = decoded.outputs[1]
        assertNull("OP_RETURN output has no address", opReturn.address)
        assertEquals(0L, opReturn.valueDuffs)
        assertEquals(
            "6a04aaaaaaaa",
            opReturn.scriptPubkey.joinToString("") { "%02x".format(it) }
        )
    }

    @Test
    fun `null address markers and empty lists round-trip`() {
        // Hand-built blob: coinbase-like tx — one input without an address,
        // one output without an address (non-standard script), empty script.
        val blob = ByteBuffer.allocate(32 + 4 + (32 + 4 + 2) + 4 + (8 + 2 + 4))
        blob.put(ByteArray(32) { 0xAB.toByte() })
        blob.putInt(1)
        blob.put(ByteArray(32)) // null prev txid (coinbase)
        blob.putInt(-1) // coinbase vout 0xFFFFFFFF
        blob.putShort(0) // no address
        blob.putInt(1)
        blob.putLong(5_000_000_000L)
        blob.putShort(0) // no address
        blob.putInt(0) // empty script

        val decoded = TransactionDecoder.parseBlob(blob.array())
        assertEquals(1, decoded.inputs.size)
        assertNull(decoded.inputs[0].address)
        assertEquals(-1, decoded.inputs[0].prevVout) // u32 max crosses as -1 bits
        assertEquals(1, decoded.outputs.size)
        assertNull(decoded.outputs[0].address)
        assertEquals(5_000_000_000L, decoded.outputs[0].valueDuffs)
        assertEquals(0, decoded.outputs[0].scriptPubkey.size)
    }

    @Test
    fun `truncated blob throws`() {
        val fixture = FIXTURE_BLOB_HEX.hexToBytes()
        assertThrows(Exception::class.java) {
            TransactionDecoder.parseBlob(fixture.copyOfRange(0, fixture.size - 3))
        }
    }

    @Test
    fun `trailing bytes throw`() {
        val fixture = FIXTURE_BLOB_HEX.hexToBytes()
        val padded = fixture + byteArrayOf(0x00)
        val error = assertThrows(IllegalArgumentException::class.java) {
            TransactionDecoder.parseBlob(padded)
        }
        assertTrue(error.message!!.contains("trailing"))
    }

    private fun String.hexToBytes(): ByteArray =
        chunked(2).map { it.toInt(16).toByte() }.toByteArray()

    companion object {
        /** Pinned in `tx_decode.rs::fixture_blob_hex_is_pinned_for_kotlin`. */
        private const val FIXTURE_BLOB_HEX =
            "d5b51c39a335f82c33beee64bbcdf9c62418884c6511bf606fa75b6e217974bf" +
                "000000011111111111111111111111111111111111111111111111111111111111111111" +
                "000000030022794e446a323851424d6d35735936624c6a46634e6457524e656632344b4c514e7551" +
                "000000020000000000024e200022794e446a323851424d6d35735936624c6a46634e6457524e656632344b4c514e7551" +
                "0000001976a91414db4138d56a2ecfb10881a9be394d9f321985b288ac" +
                "00000000000000000000000000066a04aaaaaaaa"
    }
}
