package org.dashfoundation.example.ui.identity

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class RegisterNameAvailabilityTest {

    @Test
    fun `should enable Register only for a label the check reported available`() {
        val available = parseNameAvailability(
            """{"label":"alice","valid":true,"available":true,"isContested":false,""" +
                """"message":"✅ Username is available"}""",
        )
        assertTrue(available.available)
        assertEquals("Available", available.text)
        assertTrue(canRegisterName("alice", available, isSubmitting = false))
    }

    @Test
    fun `should keep Register disabled for a taken label`() {
        // The tombstoned label of the QA run: valid, but not available.
        val taken = parseNameAvailability(
            """{"label":"qa","valid":true,"available":false,"isContested":true,""" +
                """"message":"❌ Username is already taken"}""",
        )
        assertFalse(taken.available)
        assertTrue(taken.isContested)
        assertEquals("❌ Username is already taken", taken.text)
        assertFalse(canRegisterName("qa", taken, isSubmitting = false))
    }

    @Test
    fun `should keep Register disabled for an invalid label`() {
        val invalid = parseNameAvailability(
            """{"label":"a","valid":false,"available":false,"message":"❌ Invalid username format"}""",
        )
        assertEquals("Invalid label", invalid.text)
        assertFalse(canRegisterName("a", invalid, isSubmitting = false))
    }

    @Test
    fun `should keep Register disabled without an answer`() {
        assertFalse(parseNameAvailability(null).available)
        assertFalse(parseNameAvailability("""{"valid":true}""").available)
        // Pending check (null), blank label, or a registration in flight.
        assertFalse(canRegisterName("alice", null, isSubmitting = false))
        val available = NameAvailability("Available", available = true)
        assertFalse(canRegisterName("", available, isSubmitting = false))
        assertFalse(canRegisterName("alice", available, isSubmitting = true))
    }
}
