package org.dashfoundation.example.util

import java.security.MessageDigest

/**
 * Pure-Kotlin RIPEMD-160 (ISO/IEC 10118-3, the standard public-domain
 * algorithm by Dobbertin/Bosselaers/Preneel) plus the Bitcoin/Dash
 * HASH160 composition RIPEMD160(SHA256(x)).
 *
 * Why it exists: iOS computes identity-key HASH160s via the Rust
 * `platform_wallet_hash160` FFI helper
 * (`KeychainManager.computePublicKeyHashHex`), but that helper has no JNI
 * export yet, and the JDK ships no RIPEMD-160 `MessageDigest`. This is a
 * local hash of already-public bytes — no key material, no protocol
 * decisions — so it stays within the kotlin-sdk "no crypto in Kotlin"
 * doctrine's spirit; swap it for a JNI-bridged `platform_wallet_hash160`
 * when one lands.
 *
 * Verified against the RIPEMD-160 reference vectors and a known
 * compressed-pubkey → hash160 vector (see `Ripemd160Test`).
 */
object Ripemd160 {

    /** HASH160 = RIPEMD160(SHA256(input)) — 20 bytes. */
    fun hash160(input: ByteArray): ByteArray =
        digest(MessageDigest.getInstance("SHA-256").digest(input))

    /** RIPEMD-160 of [input] — 20 bytes. */
    fun digest(input: ByteArray): ByteArray {
        var h0 = 0x67452301
        var h1 = 0xEFCDAB89.toInt()
        var h2 = 0x98BADCFE.toInt()
        var h3 = 0x10325476
        var h4 = 0xC3D2E1F0.toInt()

        // MD4-style padding: 0x80, zeros, then the 64-bit little-endian
        // bit length, to a multiple of 64 bytes.
        val bitLength = input.size.toLong() * 8
        val paddedLength = ((input.size + 8) / 64 + 1) * 64
        val padded = input.copyOf(paddedLength)
        padded[input.size] = 0x80.toByte()
        for (i in 0 until 8) {
            padded[paddedLength - 8 + i] = (bitLength ushr (8 * i)).toByte()
        }

        val x = IntArray(16)
        var offset = 0
        while (offset < paddedLength) {
            for (i in 0 until 16) {
                val base = offset + 4 * i
                x[i] = (padded[base].toInt() and 0xFF) or
                    ((padded[base + 1].toInt() and 0xFF) shl 8) or
                    ((padded[base + 2].toInt() and 0xFF) shl 16) or
                    ((padded[base + 3].toInt() and 0xFF) shl 24)
            }

            var a = h0; var b = h1; var c = h2; var d = h3; var e = h4
            var ap = h0; var bp = h1; var cp = h2; var dp = h3; var ep = h4

            for (j in 0 until 80) {
                val round = j / 16
                var t = a + f(round, b, c, d) + x[R_LEFT[j]] + K_LEFT[round]
                t = Integer.rotateLeft(t, S_LEFT[j]) + e
                a = e; e = d; d = Integer.rotateLeft(c, 10); c = b; b = t

                var tp = ap + f(4 - round, bp, cp, dp) + x[R_RIGHT[j]] + K_RIGHT[round]
                tp = Integer.rotateLeft(tp, S_RIGHT[j]) + ep
                ap = ep; ep = dp; dp = Integer.rotateLeft(cp, 10); cp = bp; bp = tp
            }

            val t = h1 + c + dp
            h1 = h2 + d + ep
            h2 = h3 + e + ap
            h3 = h4 + a + bp
            h4 = h0 + b + cp
            h0 = t

            offset += 64
        }

        val out = ByteArray(20)
        intArrayOf(h0, h1, h2, h3, h4).forEachIndexed { i, word ->
            for (bIdx in 0 until 4) {
                out[4 * i + bIdx] = (word ushr (8 * bIdx)).toByte()
            }
        }
        return out
    }

    /** The five round functions f1..f5, selected by round index 0..4. */
    private fun f(round: Int, x: Int, y: Int, z: Int): Int = when (round) {
        0 -> x xor y xor z
        1 -> (x and y) or (x.inv() and z)
        2 -> (x or y.inv()) xor z
        3 -> (x and z) or (y and z.inv())
        else -> x xor (y or z.inv())
    }

    private val K_LEFT = intArrayOf(
        0x00000000, 0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC.toInt(), 0xA953FD4E.toInt(),
    )
    private val K_RIGHT = intArrayOf(
        0x50A28BE6, 0x5C4DD124, 0x6D703EF3, 0x7A6D76E9, 0x00000000,
    )

    // Message-word selection order, left and right lines.
    private val R_LEFT = intArrayOf(
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        7, 4, 13, 1, 10, 6, 15, 3, 12, 0, 9, 5, 2, 14, 11, 8,
        3, 10, 14, 4, 9, 15, 8, 1, 2, 7, 0, 6, 13, 11, 5, 12,
        1, 9, 11, 10, 0, 8, 12, 4, 13, 3, 7, 15, 14, 5, 6, 2,
        4, 0, 5, 9, 7, 12, 2, 10, 14, 1, 3, 8, 11, 6, 15, 13,
    )
    private val R_RIGHT = intArrayOf(
        5, 14, 7, 0, 9, 2, 11, 4, 13, 6, 15, 8, 1, 10, 3, 12,
        6, 11, 3, 7, 0, 13, 5, 10, 14, 15, 8, 12, 4, 9, 1, 2,
        15, 5, 1, 3, 7, 14, 6, 9, 11, 8, 12, 2, 10, 0, 4, 13,
        8, 6, 4, 1, 3, 11, 15, 0, 5, 12, 2, 13, 9, 7, 10, 14,
        12, 15, 10, 4, 1, 5, 8, 7, 6, 2, 13, 14, 0, 3, 9, 11,
    )

    // Per-step left-rotation amounts, left and right lines.
    private val S_LEFT = intArrayOf(
        11, 14, 15, 12, 5, 8, 7, 9, 11, 13, 14, 15, 6, 7, 9, 8,
        7, 6, 8, 13, 11, 9, 7, 15, 7, 12, 15, 9, 11, 7, 13, 12,
        11, 13, 6, 7, 14, 9, 13, 15, 14, 8, 13, 6, 5, 12, 7, 5,
        11, 12, 14, 15, 14, 15, 9, 8, 9, 14, 5, 6, 8, 6, 5, 12,
        9, 15, 5, 11, 6, 8, 13, 12, 5, 12, 13, 14, 11, 8, 5, 6,
    )
    private val S_RIGHT = intArrayOf(
        8, 9, 9, 11, 13, 15, 15, 5, 7, 7, 8, 11, 14, 14, 12, 6,
        9, 13, 15, 7, 12, 8, 9, 11, 7, 7, 12, 7, 6, 15, 13, 11,
        9, 7, 15, 11, 8, 6, 6, 14, 12, 13, 5, 14, 13, 13, 7, 5,
        15, 5, 8, 11, 14, 14, 6, 14, 6, 9, 12, 9, 12, 5, 15, 8,
        8, 5, 12, 9, 12, 5, 14, 6, 8, 13, 6, 5, 15, 13, 11, 11,
    )
}
