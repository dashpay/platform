package org.dashfoundation.example.ui.identity

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class RegisterNameAvailabilityTest {

    @Test
    fun `should enable Register only for a label the check reported available`() {
        val available = parseNameAvailability(
            "alice",
            """{"label":"alice","valid":true,"available":true,"isContested":false,""" +
                """"message":"✅ Username is available"}""",
        )
        assertTrue(available.available)
        assertEquals("alice", available.label)
        assertEquals("Available", available.text)
        assertTrue(canRegisterName("alice", available, isSubmitting = false))
    }

    @Test
    fun `should keep Register disabled for a taken label`() {
        // The tombstoned label of the QA run: valid, but not available.
        val taken = parseNameAvailability(
            "qa",
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
            "a",
            """{"label":"a","valid":false,"available":false,"message":"❌ Invalid username format"}""",
        )
        assertEquals("Invalid label", invalid.text)
        assertFalse(canRegisterName("a", invalid, isSubmitting = false))
    }

    @Test
    fun `should keep Register disabled without an answer`() {
        assertFalse(parseNameAvailability("alice", null).available)
        assertFalse(parseNameAvailability("alice", """{"valid":true}""").available)
        // Pending check (null), blank label, or a registration in flight.
        assertFalse(canRegisterName("alice", null, isSubmitting = false))
        val available = NameAvailability("alice", "Available", available = true)
        assertFalse(canRegisterName("", available, isSubmitting = false))
        assertFalse(canRegisterName("alice", available, isSubmitting = true))
    }

    @Test
    fun `should not use an available answer for a different label`() {
        // "alice" was checked and is available; the user then typed a taken
        // name and tapped before the screen redrew.
        val aliceAvailable = NameAvailability("alice", "Available", available = true)
        assertFalse(canRegisterName("qa", aliceAvailable, isSubmitting = false))
        assertTrue(canRegisterName(" alice ", aliceAvailable, isSubmitting = false))
    }

    @Test
    fun `should not offer again a name submitted from this screen`() {
        // Registration landed (or joined a contest); Rust's check can still
        // call the normalized name available, and a second preorder would be
        // paid and then refused.
        val again = parseNameAvailability(
            "Alice",
            """{"label":"Alice","valid":true,"available":true,"normalizedLabel":"a11ce",""" +
                """"isContested":true,"message":"✅ Username is available"}""",
        )
        assertEquals("a11ce", again.normalizedLabel)
        val shown = nameAvailabilityForVisit(again, setOf("a11ce"))
        assertFalse(shown.available)
        assertEquals("Already submitted from this screen", shown.text)
        assertFalse(canRegisterName("Alice", shown, isSubmitting = false))
        // A genuinely different name is still offered.
        val other = NameAvailability("bob", "Available", available = true, normalizedLabel = "b0b")
        assertTrue(nameAvailabilityForVisit(other, setOf("a11ce")).available)
    }
}
