package org.dashfoundation.example.util

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class Base58Test {

    @Test
    fun zeroIdentifierRoundTrips() {
        // 32 zero bytes encode as thirty-two '1's; the decode must not
        // grow a synthetic value byte (BigInteger.ZERO.toByteArray()).
        val encoded = "1".repeat(32)
        val decoded = Base58.decode(encoded)!!
        assertArrayEquals(ByteArray(32), decoded)
        assertArrayEquals(ByteArray(32), Base58.decodeIdentifier(encoded))
        assertEquals(encoded, Base58.encode(ByteArray(32)))
    }

    @Test
    fun nonZeroIdentifierRoundTrips() {
        val id = ByteArray(32) { (it + 1).toByte() }
        val decoded = Base58.decode(Base58.encode(id))!!
        assertArrayEquals(id, decoded)
    }

    @Test
    fun leadingZerosPreserved() {
        val bytes = byteArrayOf(0, 0, 1, 2, 3)
        assertArrayEquals(bytes, Base58.decode(Base58.encode(bytes)))
    }

    @Test
    fun invalidCharacterRejected() {
        assertNull(Base58.decode("0OIl"))
    }
}
