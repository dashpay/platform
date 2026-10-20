package org.dashfoundation.dashsdk.tokens

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * [InvitationPreview.fromJson] pins the JSON contract emitted by
 * `DashpayNative.parseInvitation` (Rust `rs-unified-sdk-jni/src/dashpay.rs`),
 * including the gate rule: contact features key off a non-null
 * `inviterUsername`, never off `hasInviter` alone.
 */
@RunWith(RobolectricTestRunner::class)
class InvitationPreviewTest {

    @Test
    fun parsesAValidPreviewWithUsername() {
        val preview = InvitationPreview.fromJson(
            """{"structurallyValid":true,"isInstant":true,"hasInviter":true,""" +
                """"inviterUsername":"alice","amountDuffs":0,"expiryUnix":0}""",
        )
        assertTrue(preview.structurallyValid)
        assertTrue(preview.isInstant)
        assertTrue(preview.hasInviter)
        assertEquals("alice", preview.inviterUsername)
        assertEquals(0L, preview.amountDuffs)
        assertEquals(0, preview.expiryUnix)
    }

    @Test
    fun metadataOnlyLinkHasInviterButNullUsername() {
        val preview = InvitationPreview.fromJson(
            """{"structurallyValid":true,"isInstant":false,"hasInviter":true,""" +
                """"inviterUsername":null,"amountDuffs":0,"expiryUnix":0}""",
        )
        assertTrue(preview.structurallyValid)
        assertFalse(preview.isInstant)
        assertTrue(preview.hasInviter)
        assertNull(preview.inviterUsername)
    }

    @Test
    fun malformedLinkPreviewIsInvalidNotAnError() {
        val preview = InvitationPreview.fromJson(
            """{"structurallyValid":false,"isInstant":false,"hasInviter":false,""" +
                """"inviterUsername":null,"amountDuffs":0,"expiryUnix":0}""",
        )
        assertEquals(InvitationPreview.INVALID, preview)
    }

    @Test
    fun nullOrGarbageJsonDegradesToInvalid() {
        assertEquals(InvitationPreview.INVALID, InvitationPreview.fromJson(null))
        assertEquals(InvitationPreview.INVALID, InvitationPreview.fromJson(""))
        assertEquals(InvitationPreview.INVALID, InvitationPreview.fromJson("not json"))
    }
}
