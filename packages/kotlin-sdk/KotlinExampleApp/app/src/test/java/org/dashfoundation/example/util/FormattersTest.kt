package org.dashfoundation.example.util

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class FormattersTest {

    @Test
    fun parseDashToCreditsScalesAtCreditsPrecision() {
        // 1 DASH = 1e11 credits — the shielded/platform settlement scale.
        assertEquals(100_000_000_000L, parseDashToCredits("1"))
        assertEquals(10_000_000L, parseDashToCredits("0.0001"))
        // Exactly 11 fractional digits is the finest representable step.
        assertEquals(1L, parseDashToCredits("0.00000000001"))
    }

    @Test
    fun parseDashToCreditsRejectsInvalidInput() {
        assertNull(parseDashToCredits("0"))
        assertNull(parseDashToCredits("-1"))
        // 12 fractional digits under-runs one credit.
        assertNull(parseDashToCredits("0.000000000001"))
        assertNull(parseDashToCredits("abc"))
        assertNull(parseDashToCredits(""))
    }

    @Test
    fun parseDashToDuffsStaysOnTheDuffsScale() {
        // Regression guard: the two parsers differ by exactly 1000x.
        assertEquals(100_000_000L, parseDashToDuffs("1"))
        assertEquals(10_000L, parseDashToDuffs("0.0001"))
        assertNull(parseDashToDuffs("0.000000001"))
    }
}
