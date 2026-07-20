package org.dashfoundation.example.ui.identity

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CreateIdentityFundingPolicyTest {

    @Test
    fun `Core minimum scales with the number of registration keys`() {
        assertEquals(228_000L, minimumCoreFundingDuffsForKeyCount(4))
        assertEquals(241_000L, minimumCoreFundingDuffsForKeyCount(6))
    }

    @Test
    fun `fresh six-key Core funding rejects amounts below the protocol floor`() {
        assertFalse(
            isCreateIdentityFundingAmountValid(
                CreateIdentityFundingSource.CoreBalance,
                240_999L,
            ),
        )
        assertTrue(
            isCreateIdentityFundingAmountValid(
                CreateIdentityFundingSource.CoreBalance,
                241_000L,
            ),
        )
    }

    @Test
    fun `resume has no new funding amount to validate`() {
        assertTrue(
            isCreateIdentityFundingAmountValid(
                CreateIdentityFundingSource.AssetLockResume,
                null,
            ),
        )
    }
}
