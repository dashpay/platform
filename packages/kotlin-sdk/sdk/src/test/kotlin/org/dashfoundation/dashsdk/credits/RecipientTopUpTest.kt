package org.dashfoundation.dashsdk.credits

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.IdentityTopUpNativeResult
import org.dashfoundation.dashsdk.wallet.TrackedAssetLock
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * [IdentityCredits]' bindings of the top-up to any identity
 * (dashpay/platform#5924): each forwards its arguments unchanged to one JNI
 * call and decides nothing — which locks resume, and to whom, is Rust's.
 */
class RecipientTopUpTest {

    private val lockTxid = ByteArray(32) { 5 }

    private fun lock(
        type: TrackedAssetLock.FundingType,
        recipient: ByteArray? = null,
        unassigned: Boolean = false,
    ) = TrackedAssetLock(
        outpointTxid = lockTxid,
        outpointVout = 9,
        fundingType = type,
        status = TrackedAssetLock.Status.CHAIN_LOCKED,
        registrationIndex = 3,
        instantLockPresent = true,
        chainLockHeight = 100,
        identityRecipient = recipient,
        identityRecipientUnassigned = unassigned,
    )

    /** Records every call; answers with [resumeResult] or throws [failure]. */
    private class FakeNative(
        val resumeResult: IdentityTopUpNativeResult =
            IdentityTopUpNativeResult(ByteArray(32) { 7 }, 4_000L),
        val failure: DashSDKException? = null,
    ) : RecipientTopUpNative {
        val calls = mutableListOf<List<Any>>()

        override fun topUpExternalIdentityFromCore(
            walletHandle: Long,
            identityId: ByteArray,
            amountDuffs: Long,
            accountIndex: Int,
            coreSignerHandle: Long,
        ): Long {
            calls += listOf("external", walletHandle, identityId, amountDuffs, accountIndex, coreSignerHandle)
            failure?.let { throw it }
            return 9_000L
        }

        override fun resumeTopUpToRecordedRecipient(
            walletHandle: Long,
            outpointTxid: ByteArray,
            outpointVout: Int,
            coreSignerHandle: Long,
        ): IdentityTopUpNativeResult {
            calls += listOf("resume", walletHandle, outpointTxid, outpointVout, coreSignerHandle)
            failure?.let { throw it }
            return resumeResult
        }

        override fun assignTopUpRecipient(
            walletHandle: Long,
            outpointTxid: ByteArray,
            outpointVout: Int,
            identityId: ByteArray,
        ) {
            calls += listOf("assign", walletHandle, outpointTxid, outpointVout, identityId)
            failure?.let { throw it }
        }
    }

    @Test
    fun shouldForwardAnExternalTopUpUnchangedAndReturnTheProvedBalance() = runTest {
        val native = FakeNative()
        val identity = ByteArray(32) { 3 }

        val balance = IdentityCredits(recipientTopUpNative = native)
            .topUpExternalIdentityFromCore(1L, identity, 60_000L, 0, 2L)

        assertEquals(9_000L, balance)
        val call = native.calls.single()
        assertEquals(listOf("external", 1L), call.take(2))
        assertArrayEquals(identity, call[2] as ByteArray)
        assertEquals(60_000L, call[3])
        assertEquals(0, call[4])
        assertEquals(2L, call[5])
    }

    @Test
    fun shouldResumeEveryIdentityTopUpLockByOutpointWithoutNamingAnIdentity() = runTest {
        val paid = ByteArray(32) { 7 }
        val native = FakeNative(resumeResult = IdentityTopUpNativeResult(paid, 4_000L))
        val credits = IdentityCredits(recipientTopUpNative = native)

        // Bound, unbound with a recipient, and unassigned alike: Rust decides.
        for (lock in listOf(
            lock(TrackedAssetLock.FundingType.IDENTITY_TOP_UP),
            lock(TrackedAssetLock.FundingType.IDENTITY_TOP_UP_NOT_BOUND, recipient = paid),
            lock(TrackedAssetLock.FundingType.IDENTITY_TOP_UP_NOT_BOUND, unassigned = true),
        )) {
            assertEquals(
                IdentityTopUpResult(paid, 4_000L),
                credits.resumeTopUpToRecordedRecipient(1L, lock, 2L),
            )
        }

        assertEquals(3, native.calls.size)
        for (call in native.calls) {
            assertEquals("resume", call[0])
            assertArrayEquals(lockTxid, call[2] as ByteArray)
            assertEquals(9, call[3])
            assertEquals(2L, call[4])
        }
    }

    @Test
    fun shouldSurfaceAnUnassignedLockRefusalAsAssetLockRecipientUnknown() = runTest {
        val native = FakeNative(
            failure = DashSDKException(DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 81, "assign a recipient"),
        )
        val failure = runCatching {
            IdentityCredits(recipientTopUpNative = native).resumeTopUpToRecordedRecipient(
                1L,
                lock(TrackedAssetLock.FundingType.IDENTITY_TOP_UP_NOT_BOUND, unassigned = true),
                2L,
            )
        }.exceptionOrNull()

        assertTrue(failure is DashSdkError.PlatformWallet.AssetLockRecipientUnknown)
    }

    @Test
    fun shouldForwardAnAssignmentUnchangedAndMapAMismatch() = runTest {
        val identity = ByteArray(32) { 4 }
        val native = FakeNative()
        val unassigned = lock(TrackedAssetLock.FundingType.IDENTITY_TOP_UP_NOT_BOUND, unassigned = true)

        IdentityCredits(recipientTopUpNative = native).assignTopUpRecipient(1L, unassigned, identity)

        val call = native.calls.single()
        assertEquals(listOf("assign", 1L), call.take(2))
        assertArrayEquals(lockTxid, call[2] as ByteArray)
        assertEquals(9, call[3])
        assertArrayEquals(identity, call[4] as ByteArray)

        val mismatch = FakeNative(
            failure = DashSDKException(DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 61, "cannot be resumed to"),
        )
        val failure = runCatching {
            IdentityCredits(recipientTopUpNative = mismatch).assignTopUpRecipient(1L, unassigned, identity)
        }.exceptionOrNull()
        assertTrue(failure is DashSdkError.PlatformWallet.AssetLockRecipientMismatch)
    }

    @Test
    fun shouldRefuseAnIdentityIdThatIsNot32BytesBeforeJni() = runTest {
        val native = FakeNative()
        val credits = IdentityCredits(recipientTopUpNative = native)

        assertTrue(
            runCatching {
                credits.topUpExternalIdentityFromCore(1L, ByteArray(31), 60_000L, 0, 2L)
            }.exceptionOrNull() is IllegalArgumentException,
        )
        assertTrue(
            runCatching {
                credits.assignTopUpRecipient(
                    1L,
                    lock(TrackedAssetLock.FundingType.IDENTITY_TOP_UP_NOT_BOUND),
                    ByteArray(33),
                )
            }.exceptionOrNull() is IllegalArgumentException,
        )
        assertTrue(native.calls.isEmpty())
    }
}
