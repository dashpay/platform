package org.dashfoundation.dashsdk.identity

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.ffi.IdentityRegistrationNativeResult
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Wrapper-seam tests for [IdentityRegistration.claimInvitation] — the
 * invitee side of a DIP-13 invitation. Mirrors the
 * `IdentityAssetLockRecoveryTest` resume coverage: argument forwarding,
 * managed-handle adoption/free on both success and validation failure,
 * and the host-side key-slot guard.
 */
class InvitationClaimTest {

    @Test
    fun `claim forwards uri and index then frees the adopted handle`() = runTest {
        val destroyed = mutableListOf<Long>()
        var capturedUri: String? = null
        var capturedIndex = -1
        var capturedNow = 0
        val registration = IdentityRegistration(
            claimInvitationNative = ClaimInvitationNativeCall { _, uri, index, _, _, now ->
                capturedUri = uri
                capturedIndex = index
                capturedNow = now
                IdentityRegistrationNativeResult(ByteArray(32) { 9 }, 55L)
            },
            destroyManagedIdentity = destroyed::add,
        )

        val identityId = registration.claimInvitation(
            walletHandle = 1,
            uri = "dashpay://invite?assetlocktx=aa&pk=bb",
            identityIndex = 4,
            keys = keySet(identityIndex = 4),
            signerHandle = 2,
            nowUnix = 1_800_000_000,
        )

        assertEquals("dashpay://invite?assetlocktx=aa&pk=bb", capturedUri)
        assertEquals(4, capturedIndex)
        assertEquals(1_800_000_000, capturedNow)
        assertEquals(listOf(55L), destroyed)
        assertEquals(32, identityId.size)
        assertTrue(identityId.all { it == 9.toByte() })
    }

    @Test
    fun `claim frees the handle when native result validation fails`() = runTest {
        val destroyed = mutableListOf<Long>()
        val registration = IdentityRegistration(
            claimInvitationNative = ClaimInvitationNativeCall { _, _, _, _, _, _ ->
                IdentityRegistrationNativeResult(ByteArray(31), 91L)
            },
            destroyManagedIdentity = destroyed::add,
        )

        val failure = runCatching {
            registration.claimInvitation(1, "dashpay://invite?x", 4, keySet(4), 2)
        }
        assertTrue(failure.exceptionOrNull() is IllegalStateException)
        assertEquals(listOf(91L), destroyed)
    }

    @Test
    fun `claim rejects a key set derived for a different slot before JNI`() = runTest {
        var called = false
        val registration = IdentityRegistration(
            claimInvitationNative = ClaimInvitationNativeCall { _, _, _, _, _, _ ->
                called = true
                IdentityRegistrationNativeResult(ByteArray(32), 1)
            },
        )

        val failure = runCatching {
            registration.claimInvitation(1, "dashpay://invite?x", 4, keySet(identityIndex = 5), 2)
        }
        assertTrue(failure.exceptionOrNull() is IllegalArgumentException)
        assertFalse(called)
    }

    private fun keySet(identityIndex: Int) = RegistrationKeySet(
        identityIndex = identityIndex,
        rows = listOf(
            IdentityPubkey(
                keyId = 0,
                keyType = KeyType.ECDSA_SECP256K1,
                purpose = KeyPurpose.AUTHENTICATION,
                securityLevel = SecurityLevel.MASTER,
                pubkeyBytes = ByteArray(33) { 2 },
            ),
        ),
    )
}
