package org.dashfoundation.example.ui.scanner

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class QrScanResultRoutingTest {
    @Test
    fun invitationBearerNeverUsesSavedStateFallback() {
        val sensitive = listOf(
            "dashpay://invite?pk=secret",
            "DASHPAY://INVITE?PK=secret",
            "https://invitations.dashpay.io/applink?pk=secret",
            "HTTPS://INVITATIONS.DASHPAY.IO/APPLINK?PK=secret",
            "dashpay://invite?broken=%&pk=secret",
            "https://example.org/anything?PK=secret",
        )

        sensitive.forEach { raw ->
            assertTrue(raw, isSensitiveInvitationQr(raw))
        }
    }

    @Test
    fun genericQrContentKeepsSavedStateRoute() {
        val generic = listOf(
            "dash:Xabc123?amount=1.0",
            "https://example.org/payment?id=42",
            "XyZAddressOnly",
        )

        generic.forEach { raw ->
            assertFalse(raw, isSensitiveInvitationQr(raw))
        }
    }
}
