package org.dashfoundation.dashsdk.wallet

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the decode of `AddressPrivateKeyNative.revealCoreAddressPrivateKey`'s
 * blob (`u8 hexLen ‖ hex ‖ u8 wifLen ‖ wif`, written by
 * `rs-unified-sdk-jni/src/address_private_key.rs`) and that every copy of the
 * key Kotlin holds can be wiped.
 */
class CoreAddressPrivateKeyTest {

    private val hex = "ab".repeat(32)
    private val wif = "cTestWifValue"

    private fun blob(hexPart: String = hex, wifPart: String = wif): ByteArray =
        byteArrayOf(hexPart.length.toByte()) + hexPart.toByteArray(Charsets.US_ASCII) +
            byteArrayOf(wifPart.length.toByte()) + wifPart.toByteArray(Charsets.US_ASCII)

    @Test
    fun `should decode hex and WIF and zero the blob`() {
        val raw = blob()
        val key = CoreAddressPrivateKey.decodeAndWipe(raw)
        assertEquals(hex, String(key.hex))
        assertEquals(wif, String(key.wif))
        assertArrayEquals(ByteArray(raw.size), raw)
    }

    @Test
    fun `should zero the blob when it is truncated`() {
        val full = blob()
        val raw = full.copyOf(full.size - 3)
        assertThrows(IllegalArgumentException::class.java) {
            CoreAddressPrivateKey.decodeAndWipe(raw)
        }
        assertArrayEquals(ByteArray(raw.size), raw)
    }

    @Test
    fun `should refuse trailing bytes and zero the blob`() {
        val raw = blob() + byteArrayOf(7)
        assertThrows(IllegalArgumentException::class.java) {
            CoreAddressPrivateKey.decodeAndWipe(raw)
        }
        assertArrayEquals(ByteArray(raw.size), raw)
    }

    @Test
    fun `should refuse an empty blob`() {
        assertThrows(IllegalArgumentException::class.java) {
            CoreAddressPrivateKey.decodeAndWipe(ByteArray(0))
        }
    }

    @Test
    fun `should wipe both forms on clear and never print the key`() {
        val key = CoreAddressPrivateKey.decodeAndWipe(blob())
        assertFalse(key.toString().contains(hex))
        assertFalse(key.toString().contains(wif))
        key.clear()
        assertTrue(key.hex.all { it == '\u0000' })
        assertTrue(key.wif.all { it == '\u0000' })
        key.clear()
    }
}
