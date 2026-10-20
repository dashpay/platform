package org.dashfoundation.example.util

import java.math.BigInteger

/**
 * Minimal Base58 codec (Bitcoin alphabet, no checksum) for Platform
 * identifiers — the Kotlin counterpart of the iOS app's
 * `Data.identifier(fromBase58:)` / `Data.toBase58String()` helpers used
 * throughout `ContractsTabView.swift` and `LocalDataContractsView.swift`.
 *
 * Identifiers are always 32 bytes; [decodeIdentifier] enforces that so a
 * mistyped id fails fast instead of round-tripping to the network.
 */
object Base58 {

    private const val ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
    private val BASE = BigInteger.valueOf(58)

    fun encode(bytes: ByteArray): String {
        if (bytes.isEmpty()) return ""
        var value = BigInteger(1, bytes)
        val sb = StringBuilder()
        while (value > BigInteger.ZERO) {
            val (quotient, remainder) = value.divideAndRemainder(BASE)
            sb.append(ALPHABET[remainder.toInt()])
            value = quotient
        }
        // Leading zero bytes encode as '1's.
        for (b in bytes) {
            if (b.toInt() == 0) sb.append(ALPHABET[0]) else break
        }
        return sb.reverse().toString()
    }

    fun decode(input: String): ByteArray? {
        if (input.isEmpty()) return ByteArray(0)
        var value = BigInteger.ZERO
        for (c in input) {
            val digit = ALPHABET.indexOf(c)
            if (digit < 0) return null
            value = value.multiply(BASE).add(BigInteger.valueOf(digit.toLong()))
        }
        // BigInteger.ZERO.toByteArray() is a synthetic [0] — an all-'1'
        // input must contribute NO value bytes (its length is carried
        // entirely by the leading-zero restore below), or a 32-byte zero
        // id round-trips to 33 bytes.
        var decoded = if (value == BigInteger.ZERO) ByteArray(0) else value.toByteArray()
        // Strip BigInteger's sign byte.
        if (decoded.size > 1 && decoded[0].toInt() == 0) {
            decoded = decoded.copyOfRange(1, decoded.size)
        }
        // Restore leading zero bytes ('1' characters).
        val leadingZeros = input.takeWhile { it == ALPHABET[0] }.length
        return ByteArray(leadingZeros) + decoded
    }

    /**
     * Decode a 32-byte Platform identifier. Accepts base58 or 64-char hex
     * (the two encodings `isLikelyContractIdBytes` in `ContractsTabView.swift`
     * tries first); returns null unless the result is exactly 32 bytes.
     */
    fun decodeIdentifier(input: String): ByteArray? {
        val trimmed = input.trim()
        if (trimmed.isEmpty()) return null
        if (trimmed.length == 64 && trimmed.all { it.isDigit() || it in 'a'..'f' || it in 'A'..'F' }) {
            return trimmed.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
        }
        val decoded = decode(trimmed) ?: return null
        return decoded.takeIf { it.size == 32 }
    }
}
