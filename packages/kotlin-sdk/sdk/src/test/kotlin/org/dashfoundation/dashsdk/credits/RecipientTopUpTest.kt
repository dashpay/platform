package org.dashfoundation.dashsdk.credits

import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.withTimeout
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.IdentityTopUpNativeResult
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.dashsdk.wallet.TrackedAssetLock
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
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
        /** The 36-byte outpoint the external top-up reports, if it built a lock. */
        val built: ByteArray? = null,
    ) : UnexpectedTopUpNative() {
        val calls = mutableListOf<List<Any>>()

        override fun topUpExternalIdentityFromCore(
            walletHandle: Long,
            identityId: ByteArray,
            amountDuffs: Long,
            accountIndex: Int,
            coreSignerHandle: Long,
            outOutpoint: ByteArray,
        ): Long {
            calls += listOf("external", walletHandle, identityId, amountDuffs, accountIndex, coreSignerHandle)
            built?.copyInto(outOutpoint)
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
            acknowledgeUnknownConsumption: Boolean,
        ) {
            calls += listOf(
                "assign",
                walletHandle,
                outpointTxid,
                outpointVout,
                identityId,
                acknowledgeUnknownConsumption,
            )
            failure?.let { throw it }
        }
    }

    @Test
    fun shouldForwardAnExternalTopUpUnchangedAndReturnTheProvedBalance() = runTest {
        val native = FakeNative()
        val identity = ByteArray(32) { 3 }

        val balance = IdentityCredits(native = native)
            .topUpExternalIdentityFromCore(1L, identity, 60_000L, 0, 2L) { fail("nothing was built") }

        assertEquals(9_000L, balance)
        val call = native.calls.single()
        assertEquals(listOf("external", 1L), call.take(2))
        assertArrayEquals(identity, call[2] as ByteArray)
        assertEquals(60_000L, call[3])
        assertEquals(0, call[4])
        assertEquals(2L, call[5])
    }

    /**
     * A lock built before the call failed is reported before the error, so the
     * host resumes that outpoint instead of re-sending with a new lock.
     */
    @Test
    fun shouldReportTheBuiltOutpointBeforeAnUnconfirmedOutcome() = runTest {
        val raw = ByteArray(36) { (it + 1).toByte() }
        val native = FakeNative(
            failure = DashSDKException(DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 20, "may have been sent"),
            built = raw,
        )
        val reported = mutableListOf<ManagedPlatformWallet.AssetLockBuiltOutPoint>()

        val failure = runCatching {
            IdentityCredits(native = native)
                .topUpExternalIdentityFromCore(1L, ByteArray(32) { 3 }, 60_000L, 0, 2L) { reported += it }
        }.exceptionOrNull()

        assertTrue(failure is DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed)
        assertEquals(listOf(ManagedPlatformWallet.AssetLockBuiltOutPoint.fromRawBytes(raw)), reported)
        assertArrayEquals(raw.copyOf(32), reported.single().txid)
        assertEquals(0x24232221, reported.single().vout)
    }

    @Test
    fun shouldNotReportAnOutpointWhenNothingWasBuilt() = runTest {
        val native = FakeNative(
            failure = DashSDKException(DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 7, "identity not found"),
        )
        val failure = runCatching {
            IdentityCredits(native = native)
                .topUpExternalIdentityFromCore(1L, ByteArray(32) { 3 }, 60_000L, 0, 2L) {
                    fail("nothing was built")
                }
        }.exceptionOrNull()

        assertTrue(failure is DashSdkError.NotFound)
    }

    @Test
    fun shouldResumeEveryIdentityTopUpLockByOutpointWithoutNamingAnIdentity() = runTest {
        val paid = ByteArray(32) { 7 }
        val native = FakeNative(resumeResult = IdentityTopUpNativeResult(paid, 4_000L))
        val credits = IdentityCredits(native = native)

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
            IdentityCredits(native = native).resumeTopUpToRecordedRecipient(
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

        IdentityCredits(native = native).assignTopUpRecipient(1L, unassigned, identity)
        IdentityCredits(native = native).assignTopUpRecipient(
            1L,
            unassigned,
            identity,
            acknowledgeUnknownConsumption = true,
        )

        val (call, acknowledged) = native.calls
        assertEquals(listOf("assign", 1L), call.take(2))
        assertArrayEquals(lockTxid, call[2] as ByteArray)
        assertEquals(9, call[3])
        assertArrayEquals(identity, call[4] as ByteArray)
        assertEquals(false, call[5])
        assertEquals(true, acknowledged[5])

        val mismatch = FakeNative(
            failure = DashSDKException(DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 61, "cannot be resumed to"),
        )
        val failure = runCatching {
            IdentityCredits(native = mismatch).assignTopUpRecipient(1L, unassigned, identity)
        }.exceptionOrNull()
        assertTrue(failure is DashSdkError.PlatformWallet.AssetLockRecipientMismatch)
    }

    @Test
    fun shouldRefuseAnIdentityIdThatIsNot32BytesBeforeJni() = runTest {
        val native = FakeNative()
        val credits = IdentityCredits(native = native)

        assertTrue(
            runCatching {
                credits.topUpExternalIdentityFromCore(1L, ByteArray(31), 60_000L, 0, 2L) {}
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

    /** Blocks every payment until [release], as a native call that cannot see cancellation. */
    private class BlockingNative : UnexpectedTopUpNative() {
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)

        private fun block() {
            entered.countDown()
            release.await()
        }

        override fun topUpExternalIdentityFromCore(
            walletHandle: Long,
            identityId: ByteArray,
            amountDuffs: Long,
            accountIndex: Int,
            coreSignerHandle: Long,
            outOutpoint: ByteArray,
        ): Long {
            block()
            return 9_000L
        }

        override fun resumeTopUpToRecordedRecipient(
            walletHandle: Long,
            outpointTxid: ByteArray,
            outpointVout: Int,
            coreSignerHandle: Long,
        ): IdentityTopUpNativeResult {
            block()
            return IdentityTopUpNativeResult(ByteArray(32) { 7 }, 4_000L)
        }

        override fun assignTopUpRecipient(
            walletHandle: Long,
            outpointTxid: ByteArray,
            outpointVout: Int,
            identityId: ByteArray,
            acknowledgeUnknownConsumption: Boolean,
        ) = Unit
    }

    /**
     * A payment that completes after its caller was cancelled is still
     * returned: dropping it would hide a top-up that happened, and a retry
     * with a new lock would pay the identity twice.
     */
    @Test
    fun shouldReturnACompletedPaymentToACancelledCaller() = runBlocking {
        val payments: List<Pair<Long, suspend (IdentityCredits) -> Long>> = listOf(
            9_000L to { it.topUpExternalIdentityFromCore(1L, ByteArray(32) { 3 }, 60_000L, 0, 2L) {} },
            4_000L to {
                it.resumeTopUpToRecordedRecipient(
                    1L,
                    lock(TrackedAssetLock.FundingType.IDENTITY_TOP_UP_NOT_BOUND, ByteArray(32) { 7 }),
                    2L,
                ).newBalance
            },
        )
        for ((expected, pay) in payments) {
            val native = BlockingNative()
            val credits = IdentityCredits(native = native)
            val received = CompletableDeferred<Long>()
            val caller = launch(Dispatchers.Default) {
                received.complete(pay(credits))
            }
            assertTrue("payment never started", native.entered.await(5, TimeUnit.SECONDS))
            caller.cancel()
            native.release.countDown()
            withTimeout(5_000) { caller.join() }

            assertTrue("completed payment was dropped", received.isCompleted)
            assertEquals(expected, received.await())
        }
    }
}
