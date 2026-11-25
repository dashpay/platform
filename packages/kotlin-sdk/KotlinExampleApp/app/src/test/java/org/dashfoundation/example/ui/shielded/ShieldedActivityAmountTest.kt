package org.dashfoundation.example.ui.shielded

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ShieldedActivityAmountTest {

    @Test
    fun `should show a credit amount exactly, never rounded to zero`() {
        // 0.00144 DASH used to read "−0.0014 DASH"; 1000 credits read "−0.0000 DASH".
        assertEquals("−0.00144 DASH", signedAmount(kindTag = 7, direction = 1, amount = 144_000_000))
        assertEquals("−0.00000001 DASH", signedAmount(kindTag = 7, direction = 1, amount = 1_000))
        assertEquals("+1.2 DASH", signedAmount(kindTag = 2, direction = 0, amount = 120_000_000_000))
        assertEquals("0.5 DASH", signedAmount(kindTag = 3, direction = 2, amount = 50_000_000_000))
    }

    @Test
    fun `should show the recorded fee of a payment that moves no value`() {
        // A private document payment (type 29): amount 0, the fee recorded apart.
        assertEquals("−0 DASH", signedAmount(kindTag = 7, direction = 1, amount = 0))
        assertEquals("Fee 0.00144 DASH", activityFeeText(hasFee = true, fee = 144_000_000))
        assertEquals("Fee 0 DASH", activityFeeText(hasFee = true, fee = 0))
        // A restored spend whose fee Rust could not derive shows none.
        assertNull(activityFeeText(hasFee = false, fee = 0))
    }
}
