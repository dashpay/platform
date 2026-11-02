package org.dashfoundation.example.services.tokens

import java.math.BigDecimal
import java.math.BigInteger

/**
 * Display-unit ⇄ raw-unit token amount conversion — the Kotlin
 * counterpart of the Swift `parseTokenAmount(_:decimals:)` /
 * `formatTokenAmount(_:decimals:)` helpers the token-action forms share.
 *
 * Raw amounts are u64 on-chain and remain [ULong] throughout the Kotlin UI.
 */
object TokenAmounts {

    /**
     * Parse user input in display units and scale to raw on-chain units
     * by `10^decimals`. Null when unparseable, negative, fractional
     * beyond [decimals], or out of range.
     */
    fun parse(text: String, decimals: Int): ULong? = try {
        val raw = BigDecimal(text.trim())
            .movePointRight(decimals)
            .toBigIntegerExact()
        if (raw.signum() >= 0 && raw <= MAX_U64) raw.toString().toULong() else null
    } catch (_: NumberFormatException) {
        null
    } catch (_: ArithmeticException) {
        null
    }

    /**
     * Normalize a raw on-chain amount given as a decimal string. Null when it
     * is not an integer, is negative, or does not fit in a u64.
     */
    fun parseRaw(text: String): String? = try {
        val raw = BigInteger(text.trim())
        if (raw.signum() >= 0 && raw <= MAX_U64) raw.toString() else null
    } catch (_: NumberFormatException) {
        null
    }

    /** Format a raw amount into display units, trimming trailing zeros. */
    fun format(raw: ULong, decimals: Int): String = format(raw.toString(), decimals)

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

    private val MAX_U64 = BigInteger(ULong.MAX_VALUE.toString())
}
