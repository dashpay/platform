package org.dashfoundation.dashsdk.credits

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.wallet.TrackedAssetLock
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class IdentityTopUpRecoveryTest {

    @Test
    fun `topup resume accepts types one and two and always refuses voucher consumption`() = runTest {
        val calls = mutableListOf<Triple<Int, Int, Boolean>>()
        val credits = IdentityCredits(
            resumeTopUpNative = ResumeTopUpNativeCall { _, _, vout, _, _, consume ->
                calls += Triple(vout, calls.size + 1, consume)
                123L
            },
        )
        for (type in listOf(
            TrackedAssetLock.FundingType.IDENTITY_TOP_UP,
            TrackedAssetLock.FundingType.IDENTITY_TOP_UP_NOT_BOUND,
        )) {
            assertEquals(
                123L,
                credits.resumeTopUpWithExistingAssetLock(1, ByteArray(32), lock(type), 2),
            )
        }
        assertEquals(2, calls.size)
        assertTrue(calls.all { it.first == 9 })
        assertTrue(calls.none { it.third })
    }

    @Test
    fun `topup rejects registration lock before JNI`() = runTest {
        var called = false
        val credits = IdentityCredits(
            resumeTopUpNative = ResumeTopUpNativeCall { _, _, _, _, _, _ ->
                called = true
                0
            },
        )
        val failure = runCatching {
            credits.resumeTopUpWithExistingAssetLock(
                1,
                ByteArray(32),
                lock(TrackedAssetLock.FundingType.IDENTITY_REGISTRATION),
                2,
            )
        }
        assertTrue(failure.exceptionOrNull() is IllegalArgumentException)
        assertFalse(called)
    }

    private fun lock(type: TrackedAssetLock.FundingType) = TrackedAssetLock(
        outpointTxid = ByteArray(32) { 5 },
        outpointVout = 9,
        fundingType = type,
        status = TrackedAssetLock.Status.CHAIN_LOCKED,
        registrationIndex = 3,
        instantLockPresent = true,
        chainLockHeight = 100,
    )
}
