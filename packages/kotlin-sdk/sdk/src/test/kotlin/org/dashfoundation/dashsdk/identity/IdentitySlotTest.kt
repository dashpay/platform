package org.dashfoundation.dashsdk.identity

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.credits.FundingInput
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Wrapper-seam tests for the identity-slot lease calls of
 * [IdentityRegistration]: what each forwards to the native layer and how it
 * reads the native result (`rs-unified-sdk-jni/src/identity.rs` pins the
 * same layouts on the Rust side).
 */
class IdentitySlotTest {

    @Test
    fun `registration slot reads index then the opaque token`() = runTest {
        var capturedWallet = 0L
        val registration = IdentityRegistration(
            acquireRegistrationSlotNative = { wallet ->
                capturedWallet = wallet
                longArrayOf(3L, -1L)
            },
        )

        val slot = registration.acquireRegistrationSlot(walletHandle = 11)

        assertEquals(11L, capturedWallet)
        assertEquals(RegistrationSlot(identityIndex = 3, slotToken = -1L), slot)
    }

    @Test
    fun `registration slot release forwards the token`() = runTest {
        val released = mutableListOf<Pair<Long, Long>>()
        val registration = IdentityRegistration(
            releaseRegistrationSlotNative = { wallet, token -> released += wallet to token },
        )

        registration.releaseRegistrationSlot(walletHandle = 5, slot = RegistrationSlot(2, 99L))

        assertEquals(listOf(5L to 99L), released)
    }

    @Test
    fun `address-funded registration forwards the slot index and token`() = runTest {
        var captured: Triple<Long, Int, Long>? = null
        var capturedInputs: ByteArray? = null
        val registration = IdentityRegistration(
            registerFromAddressesNative = RegisterFromAddressesNativeCall { wallet, index, token, _, _, inputs ->
                captured = Triple(wallet, index, token)
                capturedInputs = inputs
                ByteArray(32) { 1 }
            },
        )
        val inputs = listOf(FundingInput(addressType = 0, hash = ByteArray(20) { 2 }, credits = 5L))

        val id = registration.registerFromAddresses(
            walletHandle = 8,
            slot = RegistrationSlot(identityIndex = 6, slotToken = Long.MIN_VALUE),
            keys = emptyList(),
            signerHandle = 9,
            inputs = inputs,
        )

        assertEquals(Triple(8L, 6, Long.MIN_VALUE), captured)
        assertArrayEquals(FundingInput.encode(inputs), capturedInputs)
        assertArrayEquals(ByteArray(32) { 1 }, id)
    }

    @Test
    fun `claim slot decodes index outpoint and submitted`() = runTest {
        val outPoint = ByteArray(36) { it.toByte() }
        var capturedUri: String? = null
        val registration = IdentityRegistration(
            acquireInvitationClaimSlotNative = { _, uri ->
                capturedUri = uri
                byteArrayOf(0, 0, 1, 2) + outPoint + byteArrayOf(1)
            },
        )

        val slot = registration.acquireInvitationClaimSlot(1, "dashpay://invite?x")

        assertEquals("dashpay://invite?x", capturedUri)
        assertEquals(258, slot.identityIndex)
        assertArrayEquals(outPoint, slot.invitationOutPoint)
        assertTrue(slot.submitted)
    }

    @Test
    fun `claim slot release forwards the outpoint`() = runTest {
        var captured: ByteArray? = null
        val registration = IdentityRegistration(
            releaseInvitationClaimSlotNative = ReleaseInvitationClaimSlotNativeCall { _, outPoint ->
                captured = outPoint
            },
        )
        val slot = InvitationClaimSlot(4, ByteArray(36) { 7 }, submitted = false)

        registration.releaseInvitationClaimSlot(1, slot)

        assertArrayEquals(ByteArray(36) { 7 }, captured)
    }

    @Test
    fun `claim slot decode rejects a blob of the wrong size or index range`() {
        assertTrue(runCatching { InvitationClaimSlot.decode(ByteArray(40)) }.isFailure)
        val tooHigh = byteArrayOf(0x80.toByte(), 0, 0, 0) + ByteArray(36) + byteArrayOf(0)
        assertTrue(runCatching { InvitationClaimSlot.decode(tooHigh) }.isFailure)
        val notSubmitted = InvitationClaimSlot.decode(ByteArray(41))
        assertEquals(0, notSubmitted.identityIndex)
        assertFalse(notSubmitted.submitted)
    }
}
