package org.dashfoundation.example.util

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.Locale
import java.util.TimeZone

/**
 * How a key's budget and expiry read on the key screens, and how typed DASH and a picked
 * date become protocol units. Ports `SwiftExampleAppTests/KeyLimitsFormattingTests.swift`.
 */
class KeyLimitsFormattingTest {

    @Test
    fun shouldRenderCreditsAsDashWithoutTrailingZeros() {
        assertEquals("1 DASH", KeyLimitsFormatting.dash(100_000_000_000uL))
        assertEquals("0.01 DASH", KeyLimitsFormatting.dash(1_000_000_000uL))
        assertEquals("1.23456789012 DASH", KeyLimitsFormatting.dash(123_456_789_012uL))
        assertEquals("0.00000000001 DASH", KeyLimitsFormatting.dash(1uL))
        assertEquals("0 DASH", KeyLimitsFormatting.dash(0uL))
    }

    @Test
    fun shouldParseDashTextToExactCredits() {
        fun parse(text: String) = KeyLimitsFormatting.credits(text, decimalSeparator = '.')
        assertEquals(100_000_000_000uL, parse("1"))
        assertEquals(1_000_000_000uL, parse("0.01"))
        assertEquals(123_456_789_012uL, parse("1.23456789012"))
        assertEquals(1uL, parse("0.00000000001"))
        assertEquals(50_000_000_000uL, parse(".5"))
        assertEquals(200_000_000_000uL, parse("2."))
        assertEquals(10_000_000_000uL, parse(" 0.1 "))
        // Zero parses: whether it raises anything is Rust's answer.
        assertEquals(0uL, parse("0"))
        // 0.1 + 0.2 in binary floating point is not 0.3; the parser never goes through it.
        assertEquals(30_000_000_000uL, parse("0.3"))
    }

    @Test
    fun shouldParseDashTextAsTheInverseOfTheRendering() {
        for (credits in listOf(0uL, 1uL, 999uL, 1_000_000_000uL, 123_456_789_012uL, 100_000_000_000uL, ULong.MAX_VALUE)) {
            assertEquals(credits, KeyLimitsFormatting.credits(KeyLimitsFormatting.dashNumber(credits), '.'))
        }
    }

    @Test
    fun shouldAcceptTheLocaleDecimalSeparator() {
        assertEquals(50_000_000_000uL, KeyLimitsFormatting.credits("0,5", decimalSeparator = ','))
        assertEquals(50_000_000_000uL, KeyLimitsFormatting.credits("0.5", decimalSeparator = ','))
        assertNull(KeyLimitsFormatting.credits("0,5", decimalSeparator = '.'))
    }

    @Test
    fun shouldNotParseTextThatIsNotAnAmount() {
        for (text in listOf("", " ", ".", "abc", "1.2.3", "-1", "+1", "1e3", "1 000", "0x10", "1.5 DASH", "١")) {
            assertNull("$text must not parse", KeyLimitsFormatting.credits(text, '.'))
        }
    }

    @Test
    fun shouldNotParseAnAmountFinerThanACredit() {
        assertNull(KeyLimitsFormatting.credits("0.000000000001", '.'))
        assertNull(KeyLimitsFormatting.credits("1.000000000000", '.'))
    }

    @Test
    fun shouldNotParseAnAmountPastTheLargestCreditAmount() {
        // u64::MAX credits is 184467440.73709551615 DASH.
        assertEquals(ULong.MAX_VALUE, KeyLimitsFormatting.credits("184467440.73709551615", '.'))
        assertNull(KeyLimitsFormatting.credits("184467440.73709551616", '.'))
        assertNull(KeyLimitsFormatting.credits("184467441", '.'))
        assertNull(KeyLimitsFormatting.credits("99999999999999999999999", '.'))
    }

    @Test
    fun shouldConvertAPickedInstantToMillisecondsOnlyFrom1970() {
        assertEquals(1_700_000_000_500uL, KeyLimitsFormatting.millis(1_700_000_000_500))
        assertEquals(0uL, KeyLimitsFormatting.millis(0))
        assertNull(KeyLimitsFormatting.millis(-1))
    }

    @Test
    fun shouldMoveThePickedDayAndTimeInTheLocalZone() {
        val zone = TimeZone.getTimeZone("Asia/Bangkok") // UTC+7
        // 2023-11-14 22:13:20 UTC is 2023-11-15 05:13:20 in Bangkok.
        val picked = 1_700_000_000_000L
        assertEquals(1_700_006_400_000L, KeyLimitsFormatting.dateUtcMillis(picked, zone)) // 2023-11-15 UTC midnight
        assertEquals(5 to 13, KeyLimitsFormatting.timeOfDay(picked, zone))
        // 2024-01-02 picked: same local time of day, on that day.
        val moved = KeyLimitsFormatting.withDate(picked, 1_704_153_600_000L, zone)
        assertEquals(1_704_153_600_000L, KeyLimitsFormatting.dateUtcMillis(moved, zone))
        assertEquals(5 to 13, KeyLimitsFormatting.timeOfDay(moved, zone))
        // 23:45 local on the same day, seconds cleared.
        val late = KeyLimitsFormatting.withTime(picked, 23, 45, zone)
        assertEquals(23 to 45, KeyLimitsFormatting.timeOfDay(late, zone))
        assertEquals(0L, late % 60_000)
        assertEquals(1_700_006_400_000L, KeyLimitsFormatting.dateUtcMillis(late, zone))
    }

    @Test
    fun shouldShowRemainingOfTotalWhenKnown() {
        assertEquals("0.004 of 0.01 DASH left", KeyLimitsFormatting.budget(1_000_000_000uL, 400_000_000uL))
        assertEquals("0.01 DASH", KeyLimitsFormatting.budget(1_000_000_000uL, null))
    }

    @Test
    fun shouldCountTheExpiryInstantAsExpired() {
        val expiresAt = 1_700_000_000_000uL
        assertFalse(KeyLimitsFormatting.isExpired(expiresAt, 1_699_999_999_999))
        assertTrue(KeyLimitsFormatting.isExpired(expiresAt, 1_700_000_000_000))
    }

    @Test
    fun shouldNameBothDirectionsOfTheExpiry() {
        val expiresAt = 1_700_000_000_000uL
        assertEquals("Expires in 1 hour", KeyLimitsFormatting.expiryRelative(expiresAt, 1_699_996_400_000))
        assertEquals("Expired 2 days ago", KeyLimitsFormatting.expiryRelative(expiresAt, 1_700_172_800_000))
    }

    @Test
    fun shouldRenderTheExpiryDateInTheGivenLocaleAndZone() {
        assertEquals(
            "Nov 14, 2023, 10:13 PM",
            KeyLimitsFormatting.expiryDate(1_700_000_000_000uL, Locale.US, TimeZone.getTimeZone("UTC"))
                .replace(' ', ' '),
        )
    }

    @Test
    fun shouldReadRemainingBudgetsByKeyId() {
        val read = KeyLimitsFormatting.remainingBudgets("""{"5": "1000", "6": null, "4294967295": "18446744073709551615"}""")
        assertEquals(1_000uL, read[5])
        assertTrue(read.containsKey(6))
        assertNull(read[6])
        assertEquals(ULong.MAX_VALUE, read[-1])
        assertTrue(KeyLimitsFormatting.remainingBudgets(null).isEmpty())
    }
}
