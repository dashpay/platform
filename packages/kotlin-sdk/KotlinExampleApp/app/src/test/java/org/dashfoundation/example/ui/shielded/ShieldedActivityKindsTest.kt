package org.dashfoundation.example.ui.shielded

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The Shielded Activity row of a private document entry or a DashPay private
 * contact request paid from the shielded pool (type 29): named by the kind
 * Rust records, with Rust's amount, which is the fee.
 */
class ShieldedActivityKindsTest {
    @Test
    fun `should name a private document entry and a private contact request`() {
        assertEquals(13, KIND_PRIVATE_DOCUMENT_CREATE)
        assertEquals(14, KIND_PRIVATE_CONTACT_REQUEST)
        assertEquals("Private Document Entry", kindLabel(KIND_PRIVATE_DOCUMENT_CREATE))
        assertEquals("Private Contact Request", kindLabel(KIND_PRIVATE_CONTACT_REQUEST))
        // The residual kind, and a tag this build does not know, keep the generic label.
        assertEquals("Shielded Spend", kindLabel(7))
        assertEquals("Shielded Spend", kindLabel(15))
    }

    @Test
    fun `should show the fee Rust records as the amount of a fee-only spend`() {
        // The fee Rust stores as the row's amount, shown exactly as every credit amount is.
        val fee = 153_074_400L
        for (kind in listOf(KIND_PRIVATE_DOCUMENT_CREATE, KIND_PRIVATE_CONTACT_REQUEST)) {
            assertEquals("−0.001530744 DASH", signedAmount(kind, direction = 1, amount = fee))
            assertFalse(amountIsTokens(kind))
            assertEquals("153074400 credits", storedActivityAmountText(kind, fee))
        }
    }

    @Test
    fun `should say a fee-only spend's amount is its fee only while it may have been paid`() {
        for (kind in listOf(KIND_PRIVATE_DOCUMENT_CREATE, KIND_PRIVATE_CONTACT_REQUEST)) {
            assertEquals(
                "Confirmed. The amount is the fee, which is all that left the shielded pool.",
                supportingText(kind, 1),
            )
            assertTrue(supportingText(kind, 0).startsWith("Pending. Not confirmed yet"))
            assertEquals(
                "Failed. It did not go through; the fee shown was not charged.",
                supportingText(kind, 2),
            )
            assertTrue(supportingText(kind, 3).startsWith("Unknown. Not confirmed yet"))
        }
        assertEquals("the residual kind keeps its bare status", "Confirmed", supportingText(7, 1))
    }
}
