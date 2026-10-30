package org.dashfoundation.dashsdk.identity

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test

class KeyDisableVerdictTest {

    @Test
    fun `should decode an allowed verdict`() {
        assertNull(decodeKeyDisableVerdict(longArrayOf(0, 0, 0, 0)))
    }

    @Test
    fun `should decode both refusals`() {
        assertEquals(
            KeyDisableRefusal.LastAuthenticationKey(1, SecurityLevel.CRITICAL),
            decodeKeyDisableVerdict(longArrayOf(1, 1, 1, 0)),
        )
        assertEquals(
            KeyDisableRefusal.LastEnabledKey(3, KeyPurpose.TRANSFER),
            decodeKeyDisableVerdict(longArrayOf(2, 3, 0, 3)),
        )
    }

    @Test
    fun `should raise the typed SDK error for values it does not know`() {
        listOf(
            longArrayOf(9, 1, 0, 0),
            longArrayOf(1, 1, 99, 0),
            longArrayOf(2, 1, 0, 99),
        ).forEach { verdict ->
            val error = assertThrows(DashSdkError.PlatformWallet.Generic::class.java) {
                decodeKeyDisableVerdict(verdict)
            }
            assertEquals(2, error.nativeCode)
        }
    }
}
