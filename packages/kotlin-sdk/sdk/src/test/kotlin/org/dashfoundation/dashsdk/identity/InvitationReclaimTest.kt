package org.dashfoundation.dashsdk.identity

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.credits.IdentityCredits
import org.dashfoundation.dashsdk.credits.ResumeTopUpNativeCall
import org.dashfoundation.dashsdk.ffi.IdentityRegistrationNativeResult
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Wrapper-seam tests for the two DIP-13 invitation-reclaim entry points —
 * the ONLY call sites in the SDK that pass `consumeInvitationVoucher =
 * true` (Rust core refuses invitation locks everywhere else). The generic
 * resume paths' `false` discipline stays pinned by
 * `IdentityAssetLockRecoveryTest` / `IdentityTopUpRecoveryTest`.
 */
class InvitationReclaimTest {

    @Test
    fun `reclaim as top-up forwards the raw outpoint with the voucher flag set`() = runTest {
        var capturedTxid: ByteArray? = null
        var capturedVout = -1
        var capturedConsume = false
        val credits = IdentityCredits(
            resumeTopUpNative = ResumeTopUpNativeCall { _, txid, vout, _, _, consume ->
                capturedTxid = txid.copyOf()
                capturedVout = vout
                capturedConsume = consume
                777_000L
            },
        )

        val newBalance = credits.reclaimInvitationAsTopUp(
            walletHandle = 1,
            identityId = ByteArray(32) { 6 },
            outPointTxid = ByteArray(32) { 4 },
            outPointVout = 3,
            coreSignerHandle = 2,
        )

        assertTrue(ByteArray(32) { 4 }.contentEquals(capturedTxid!!))
        assertEquals(3, capturedVout)
        assertTrue(capturedConsume)
        assertEquals(777_000L, newBalance)
    }

    @Test
    fun `reclaim as new identity forwards the voucher flag and frees the handle`() = runTest {
        val destroyed = mutableListOf<Long>()
        var capturedConsume = false
        var capturedIndex = -1
        val registration = IdentityRegistration(
            resumeNative = ResumeIdentityNativeCall { _, _, _, index, _, _, _, consume ->
                capturedIndex = index
                capturedConsume = consume
                IdentityRegistrationNativeResult(ByteArray(32) { 8 }, 77L)
            },
            destroyManagedIdentity = destroyed::add,
        )

        val identityId = registration.reclaimInvitationAsNewIdentity(
            walletHandle = 1,
            outPointTxid = ByteArray(32) { 4 },
            outPointVout = 0,
            identityIndex = 7,
            keys = keySet(identityIndex = 7),
            signerHandle = 2,
            coreSignerHandle = 3,
        )

        assertTrue(capturedConsume)
        assertEquals(7, capturedIndex)
        assertEquals(listOf(77L), destroyed)
        assertEquals(32, identityId.size)
    }

    @Test
    fun `reclaim as new identity rejects a wrong-slot key set before JNI`() = runTest {
        var called = false
        val registration = IdentityRegistration(
            resumeNative = ResumeIdentityNativeCall { _, _, _, _, _, _, _, _ ->
                called = true
                IdentityRegistrationNativeResult(ByteArray(32), 1)
            },
        )

        val failure = runCatching {
            registration.reclaimInvitationAsNewIdentity(
                1, ByteArray(32), 0, 7, keySet(identityIndex = 6), 2, 3,
            )
        }
        assertTrue(failure.exceptionOrNull() is IllegalArgumentException)
        assertTrue(!called)
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
