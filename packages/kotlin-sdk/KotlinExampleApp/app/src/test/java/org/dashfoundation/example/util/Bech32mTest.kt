package org.dashfoundation.example.util

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class Bech32mTest {

    /**
     * BIP-350 valid bech32m vector: hrp `abcdef`, 32 descending 5-bit
     * values (31..0) = exactly 20 payload bytes. Decode must accept it and
     * re-encoding the decoded payload must reproduce the string.
     */
    @Test
    fun bip350VectorRoundTrips() {
        val vector = "abcdef1l7aum6echk45nj3s0wdvt2fg8x9yrzpqzd3ryx"
        val decoded = Bech32m.decode(vector)
        assertNotNull(decoded)
        assertEquals("abcdef", decoded!!.hrp)
        assertEquals(20, decoded.data.size)
        assertEquals(vector, Bech32m.encode(decoded.hrp, decoded.data))
    }

    @Test
    fun corruptedChecksumRejected() {
        // Last character flipped from the valid vector above.
        assertNull(Bech32m.decode("abcdef1l7aum6echk45nj3s0wdvt2fg8x9yrzpqzd3ryy"))
    }

    @Test
    fun bech32ChecksumRejected() {
        // A valid BIP-173 bech32 (not bech32m) string must fail the
        // bech32m constant check.
        assertNull(Bech32m.decode("abcdef1qpzry9x8gf2tvdw0s3jn54khce6mua7lmqqqxw"))
    }

    @Test
    fun invalidCharacterRejected() {
        // 'b' is not in the bech32 charset.
        assertNull(Bech32m.decode("abcdef1bqzry9x8gf2tvdw0s3jn54khce6mua7lmqqqxw"))
    }

    @Test
    fun missingSeparatorRejected() {
        assertNull(Bech32m.decode("qpzry9x8gf2tvdw0s3jn54khce6mua7l"))
    }

    @Test
    fun orchardSizedPayloadRoundTrips() {
        // 44 bytes = the 0x10 type byte + a raw 43-byte Orchard address —
        // the exact shape the receive sheet encodes and the send flow decodes.
        val payload = ByteArray(44) { it.toByte() }
        val encoded = Bech32m.encode("tdash", payload)
        assertNotNull(encoded)
        val decoded = Bech32m.decode(encoded!!)
        assertNotNull(decoded)
        assertEquals("tdash", decoded!!.hrp)
        assertArrayEquals(payload, decoded.data)
    }

    @Test
    fun decodeIsCaseInsensitive() {
        val payload = ByteArray(21) { (it * 3).toByte() }
        val encoded = Bech32m.encode("tdash", payload)!!
        val decoded = Bech32m.decode(encoded.uppercase())
        assertNotNull(decoded)
        assertArrayEquals(payload, decoded!!.data)
    }
}
