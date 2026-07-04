package org.dashfoundation.example.services.tokens

import java.math.BigDecimal
import java.math.BigInteger

/**
 * Display-unit ⇄ raw-unit token amount conversion — the Kotlin
 * counterpart of the Swift `parseTokenAmount(_:decimals:)` /
 * `formatTokenAmount(_:decimals:)` helpers the token-action forms share.
 *
 * Raw amounts are u64 on-chain; they cross the JNI as [Long] bit
 * patterns. Parsing rejects values above [Long.MAX_VALUE] (2^63-1) —
 * the practical UI range — rather than juggling the sign bit.
 */
object TokenAmounts {

    /**
     * Parse user input in display units and scale to raw on-chain units
     * by `10^decimals`. Null when unparseable, negative, fractional
     * beyond [decimals], or out of range.
     */
    fun parse(text: String, decimals: Int): Long? = try {
        val raw = BigDecimal(text.trim())
            .movePointRight(decimals)
            .toBigIntegerExact()
        if (raw.signum() >= 0 && raw.bitLength() < 63) raw.toLong() else null
    } catch (_: NumberFormatException) {
        null
    } catch (_: ArithmeticException) {
        null
    }

    /** Format a raw amount into display units, trimming trailing zeros. */
    fun format(raw: Long, decimals: Int): String = format(raw.toULong().toString(), decimals)

    /** Format a decimal-string raw amount (u64-safe) into display units. */
    fun format(raw: String?, decimals: Int): String {
        val value = raw?.let {
            try {
                BigInteger(it)
            } catch (_: NumberFormatException) {
                null
            }
        } ?: return raw ?: "0"
        val display = BigDecimal(value).movePointLeft(decimals).stripTrailingZeros()
        return display.toPlainString()
    }
}
