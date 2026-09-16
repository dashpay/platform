package org.dashfoundation.dashsdk.security

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Pins [Hash160] against the RIPEMD-160 reference vectors
 * (Dobbertin/Bosselaers/Preneel) and the canonical Bitcoin/Dash HASH160 =
 * `RIPEMD160(SHA256(pubkey))` composition DPP uses for `ECDSA_HASH160` /
 * `EDDSA_25519_HASH160` identity keys. If this drifts, the identity-key
 * repair ownership check for HASH160-typed keys (dashpay/platform#4183) would
 * compare against a wrong hash and either wrongly reject a valid repair or
 * (worse) wrongly accept a mismatched key.
 */
class Hash160Test {

    private fun hex(bytes: ByteArray): String =
        bytes.joinToString("") { "%02x".format(it) }

    private fun unhex(s: String): ByteArray =
        s.chunked(2).map { it.toInt(16).toByte() }.toByteArray()

    @Test
    fun ripemd160ReferenceVectors() {
        assertEquals("9c1185a5c5e9fc54612808977ee8f548b2258d31", hex(Hash160.ripemd160(ByteArray(0))))
        assertEquals("8eb208f7e05d987a9b044a8e98c6b087f15a0bfc", hex(Hash160.ripemd160("abc".toByteArray())))
        assertEquals(
            "5d0689ef49d2fae572b881b123a85ffa21595f36",
            hex(Hash160.ripemd160("message digest".toByteArray())),
        )
        assertEquals(
            "12a053384a9c0c88e405a06c27dcf49ada62eb2b",
            hex(Hash160.ripemd160("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".toByteArray())),
        )
    }

    @Test
    fun ripemd160MultiBlockAndMillionA() {
        // 80 bytes — crosses the 64-byte block boundary.
        assertEquals(
            "9b752e45573d4b39f4dbd3323cab82bf63326bfb",
            hex(Hash160.ripemd160("1234567890".repeat(8).toByteArray())),
        )
        assertEquals(
            "52783243c1697bdbe16d37f97f68f08325dc1528",
            hex(Hash160.ripemd160(ByteArray(1_000_000) { 'a'.code.toByte() })),
        )
    }

    @Test
    fun hash160OfKnownCompressedPubkey() {
        // Canonical Bitcoin/Dash vector: the compressed secp256k1 pubkey for
        // the classic sipa example private key. This is exactly the shape a
        // HASH160-typed identity key stores on-chain as its "public key data".
        val pubkey = unhex("0250863ad64a87ae8a2fe83c1af1a8403cb53f53e486d8511dad8a04887e5b2352")
        assertEquals("f54a5851e9372b87810a8e60cdd2e7cfd80b6e31", hex(Hash160.hash160(pubkey)))
    }
}
