package org.dashfoundation.example.util

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * RIPEMD-160 reference vectors (Dobbertin/Bosselaers/Preneel paper) plus
 * the Bitcoin/Dash HASH160 composition [Ripemd160.hash160] uses for
 * ECDSA_HASH160 identity keys in `IdentityKeyAdditionFlow.prepareKeys`.
 */
class Ripemd160Test {

    private fun hex(bytes: ByteArray): String =
        bytes.joinToString("") { "%02x".format(it) }

    private fun unhex(s: String): ByteArray =
        s.chunked(2).map { it.toInt(16).toByte() }.toByteArray()

    @Test
    fun referenceVectors() {
        assertEquals(
            "9c1185a5c5e9fc54612808977ee8f548b2258d31",
            hex(Ripemd160.digest(ByteArray(0))),
        )
        assertEquals(
            "8eb208f7e05d987a9b044a8e98c6b087f15a0bfc",
            hex(Ripemd160.digest("abc".toByteArray())),
        )
        assertEquals(
            "5d0689ef49d2fae572b881b123a85ffa21595f36",
            hex(Ripemd160.digest("message digest".toByteArray())),
        )
        assertEquals(
            "12a053384a9c0c88e405a06c27dcf49ada62eb2b",
            hex(
                Ripemd160.digest(
                    "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".toByteArray(),
                ),
            ),
        )
    }

    @Test
    fun multiBlockVector() {
        // 80 bytes — crosses the 64-byte block boundary, exercising the
        // multi-block compression + padding path.
        assertEquals(
            "9b752e45573d4b39f4dbd3323cab82bf63326bfb",
            hex(Ripemd160.digest("1234567890".repeat(8).toByteArray())),
        )
    }

    @Test
    fun millionAVector() {
        assertEquals(
            "52783243c1697bdbe16d37f97f68f08325dc1528",
            hex(Ripemd160.digest(ByteArray(1_000_000) { 'a'.code.toByte() })),
        )
    }

    @Test
    fun hash160OfKnownCompressedPubkey() {
        // Canonical Bitcoin/Dash vector: the compressed secp256k1 pubkey
        // for private key 0x18E14A7B... (the classic sipa example).
        val pubkey = unhex("0250863ad64a87ae8a2fe83c1af1a8403cb53f53e486d8511dad8a04887e5b2352")
        assertEquals(
            "f54a5851e9372b87810a8e60cdd2e7cfd80b6e31",
            hex(Ripemd160.hash160(pubkey)),
        )
    }
}
