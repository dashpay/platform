package org.dashfoundation.dashsdk.persistence

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Pure-function tests for the persistence-handler encoding helpers:
 * plain base58 (follow-up (a)) and DIP-0018 bech32m platform-address
 * decode (follow-up (c)). No Android / native dependency.
 */
class EncodingHelpersTest {

    // ── base58 (plain, no checksum — matches DPP Identifier::to_string) ──

    @Test
    fun base58EmptyIsEmpty() {
        assertEquals("", base58Encode(ByteArray(0)))
    }

    @Test
    fun base58LeadingZerosBecomeLeadingOnes() {
        // Each leading 0x00 → one leading '1'. All-zero 32 bytes → 32 '1's.
        assertEquals("1".repeat(32), ByteArray(32).toBase58String())
    }

    @Test
    fun base58KnownVectors() {
        // Standard bs58 vectors (Bitcoin/IPFS alphabet, no checksum).
        assertEquals("2g", base58Encode(byteArrayOf(0x61)))
        assertEquals("a3gV", base58Encode("bbb".toByteArray(Charsets.US_ASCII)))
        assertEquals("aPEr", base58Encode("ccc".toByteArray(Charsets.US_ASCII)))
        assertEquals(
            "2NEpo7TZRRrLZSi2U",
            base58Encode("Hello World!".toByteArray(Charsets.US_ASCII)),
        )
    }

    @Test
    fun base58Is32ByteIdentityLength() {
        // A 32-byte id encodes to a 43-or-44-char base58 string (no leading
        // zeros here), never hex length (64). Sanity that we're not hexing.
        val id = ByteArray(32) { (it + 1).toByte() }
        val s = id.toBase58String()
        assertEquals(true, s.length in 42..44)
    }

    // ── bech32m platform-address decode (DIP-0018) ──────────────────────

    @Test
    fun bech32mRejectsNonPlatformHrp() {
        // A valid bech32m string with the wrong HRP → null (policy: dash/tdash only).
        // "abcdef" hrp is not a platform address.
        assertNull(decodePlatformAddress("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4"))
    }

    @Test
    fun bech32mRejectsGarbage() {
        assertNull(decodePlatformAddress("not-an-address"))
        assertNull(decodePlatformAddress(""))
        assertNull(decodePlatformAddress("dash1"))
    }

    @Test
    fun bech32mRoundTripFromEncodedPlatformAddress() {
        // Build a DIP-0018 address (hrp=tdash, type 0xb0 = P2PKH + 20-byte
        // hash), encode with our own encoder, and assert decode recovers
        // (addressType=0, hash). This exercises the checksum + convertBits
        // path end-to-end without hardcoding a wallet's address.
        val hash = ByteArray(20) { (it + 3).toByte() }
        val payload = ByteArray(21).also {
            it[0] = 0xb0.toByte()
            System.arraycopy(hash, 0, it, 1, 20)
        }
        val encoded = bech32mEncodeForTest("tdash", payload)
        val decoded = decodePlatformAddress(encoded)
        assertEquals(0, decoded?.first)
        assertEquals(true, decoded?.second?.contentEquals(hash))
    }

    @Test
    fun bech32mP2shTypeMapsToOne() {
        val hash = ByteArray(20) { 7 }
        val payload = ByteArray(21).also {
            it[0] = 0x80.toByte()
            System.arraycopy(hash, 0, it, 1, 20)
        }
        val encoded = bech32mEncodeForTest("dash", payload)
        assertEquals(1, decodePlatformAddress(encoded)?.first)
    }

    // ── Test-only bech32m encoder (mirror of the decoder's math) ────────

    private fun bech32mEncodeForTest(hrp: String, data8: ByteArray): String {
        val charset = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
        val data5 = convertBits(data8.map { it.toInt() and 0xFF }, 8, 5, true)!!
        val values = hrpExpand(hrp) + data5
        val polymod = polymod(values + listOf(0, 0, 0, 0, 0, 0)) xor 0x2bc830a3
        val checksum = (0 until 6).map { (polymod shr (5 * (5 - it))) and 31 }
        val combined = data5 + checksum
        return hrp + "1" + combined.joinToString("") { charset[it].toString() }
    }

    private fun hrpExpand(hrp: String): List<Int> =
        hrp.map { it.code shr 5 } + listOf(0) + hrp.map { it.code and 31 }

    private fun polymod(values: List<Int>): Int {
        val gen = intArrayOf(0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3)
        var chk = 1
        for (v in values) {
            val top = chk shr 25
            chk = ((chk and 0x1ffffff) shl 5) xor v
            for (i in 0 until 5) if (((top shr i) and 1) != 0) chk = chk xor gen[i]
        }
        return chk
    }

    private fun convertBits(data: List<Int>, from: Int, to: Int, pad: Boolean): List<Int>? {
        var acc = 0
        var bits = 0
        val out = ArrayList<Int>()
        val maxv = (1 shl to) - 1
        for (value in data) {
            if (value < 0 || (value shr from) != 0) return null
            acc = (acc shl from) or value
            bits += from
            while (bits >= to) {
                bits -= to
                out.add((acc shr bits) and maxv)
            }
        }
        if (pad && bits > 0) out.add((acc shl (to - bits)) and maxv)
        return out
    }
}
