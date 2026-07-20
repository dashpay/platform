package org.dashfoundation.dashsdk.identity

import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.ffi.IdentityRegistrationNativeResult
import org.dashfoundation.dashsdk.wallet.TeardownGate
import org.dashfoundation.dashsdk.wallet.TrackedAssetLock
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class IdentityAssetLockRecoveryTest {

    @Test
    fun `registration resume forwards same outpoint and false voucher flag then frees handle`() = runTest {
        val lock = lock(TrackedAssetLock.FundingType.IDENTITY_REGISTRATION)
        val destroyed = mutableListOf<Long>()
        var capturedTxid: ByteArray? = null
        var capturedVout = -1
        var consumeVoucher = true
        val registration = IdentityRegistration(
            resumeNative = ResumeIdentityNativeCall { _, txid, vout, _, _, _, _, consume ->
                capturedTxid = txid.copyOf()
                capturedVout = vout
                consumeVoucher = consume
                IdentityRegistrationNativeResult(ByteArray(32) { 8 }, 77L)
            },
            destroyManagedIdentity = destroyed::add,
        )

        val identityId = registration.resumeWithExistingAssetLock(
            walletHandle = 1,
            lock = lock,
            identityIndex = 7,
            keys = listOf(key()),
            signerHandle = 2,
            coreSignerHandle = 3,
        )

        assertTrue(lock.outpointTxid.contentEquals(capturedTxid!!))
        assertEquals(lock.outpointVout, capturedVout)
        assertFalse(consumeVoucher)
        assertEquals(listOf(77L), destroyed)
        assertEquals(32, identityId.size)
    }

    @Test
    fun `managed identity handle is freed when native result validation fails`() = runTest {
        val destroyed = mutableListOf<Long>()
        val registration = IdentityRegistration(
            resumeNative = ResumeIdentityNativeCall { _, _, _, _, _, _, _, _ ->
                IdentityRegistrationNativeResult(ByteArray(31), 91L)
            },
            destroyManagedIdentity = destroyed::add,
        )

        val failure = runCatching {
            registration.resumeWithExistingAssetLock(
                1, lock(TrackedAssetLock.FundingType.IDENTITY_REGISTRATION),
                7, listOf(key()), 2, 3,
            )
        }
        assertTrue(failure.exceptionOrNull() is IllegalStateException)
        assertEquals(listOf(91L), destroyed)
    }

    @Test
    fun `registration rejects topup lock before JNI`() = runTest {
        var called = false
        val registration = IdentityRegistration(
            resumeNative = ResumeIdentityNativeCall { _, _, _, _, _, _, _, _ ->
                called = true
                IdentityRegistrationNativeResult(ByteArray(32), 1)
            },
        )

        val failure = runCatching {
            registration.resumeWithExistingAssetLock(
                1, lock(TrackedAssetLock.FundingType.IDENTITY_TOP_UP),
                7, listOf(key()), 2, 3,
            )
        }
        assertTrue(failure.exceptionOrNull() is IllegalArgumentException)
        assertFalse(called)
    }

    @Test
    fun `registration rejects identity index that differs from tracked lock before JNI`() = runTest {
        var called = false
        val registration = IdentityRegistration(
            resumeNative = ResumeIdentityNativeCall { _, _, _, _, _, _, _, _ ->
                called = true
                IdentityRegistrationNativeResult(ByteArray(32), 1)
            },
        )

        val failure = runCatching {
            registration.resumeWithExistingAssetLock(
                1, lock(TrackedAssetLock.FundingType.IDENTITY_REGISTRATION),
                8, listOf(key()), 2, 3,
            )
        }
        assertTrue(failure.exceptionOrNull() is IllegalArgumentException)
        assertFalse(called)
    }

    @Test
    fun `manager teardown waits for registration resolver borrow`() = runBlocking {
        val gate = TeardownGate()
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val registration = IdentityRegistration(
            gate = gate,
            resumeNative = ResumeIdentityNativeCall { _, _, _, _, _, _, _, _ ->
                entered.countDown()
                check(release.await(5, TimeUnit.SECONDS))
                IdentityRegistrationNativeResult(ByteArray(32), 55)
            },
            destroyManagedIdentity = {},
        )
        val operation = async(Dispatchers.Default) {
            registration.resumeWithExistingAssetLock(
                1, lock(TrackedAssetLock.FundingType.IDENTITY_REGISTRATION),
                7, listOf(key()), 2, 3,
            )
        }
        assertTrue(entered.await(5, TimeUnit.SECONDS))
        var closed = false
        val closer = launch(Dispatchers.Default) {
            gate.closeAndAwait()
            closed = true
        }
        delay(100)
        assertFalse(closed)
        release.countDown()
        operation.await()
        closer.join()
        assertTrue(closed)
    }

    // Resume carries the rich base MASTER row; the resume path only checks the
    // list is non-empty (the key set is fixed by the tracked lock, not
    // re-validated per key here).
    private fun key() = IdentityPubkey(
        keyId = 0,
        keyType = KeyType.ECDSA_SECP256K1,
        purpose = KeyPurpose.AUTHENTICATION,
        securityLevel = SecurityLevel.MASTER,
        pubkeyBytes = ByteArray(33) { 2 },
    )

    private fun lock(type: TrackedAssetLock.FundingType) = TrackedAssetLock(
        outpointTxid = ByteArray(32) { 4 },
        outpointVout = 3,
        fundingType = type,
        status = TrackedAssetLock.Status.BUILT,
        registrationIndex = 7,
        instantLockPresent = false,
        chainLockHeight = 0,
    )
}
