package org.dashfoundation.dashsdk.credits

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.wallet.TrackedAssetLock
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class IdentityTopUpRecoveryTest {

    @Test
    fun `topup resume passes every tracked lock type to Rust and always refuses voucher consumption`() = runTest {
        val calls = mutableListOf<Triple<Int, Int, Boolean>>()
        val credits = IdentityCredits(
            resumeTopUpNative = ResumeTopUpNativeCall { _, _, vout, _, _, consume ->
                calls += Triple(vout, calls.size + 1, consume)
                123L
            },
        )
        // A registration lock tops up the identity in its own slot after its
        // resume found that slot taken; Rust checks the index binding.
        for (type in TrackedAssetLock.FundingType.entries) {
            assertEquals(
                123L,
                credits.resumeTopUpWithExistingAssetLock(1, ByteArray(32), lock(type), 2),
            )
        }
        assertEquals(3, calls.size)
        assertTrue(calls.all { it.first == 9 })
        assertTrue(calls.none { it.third })
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
