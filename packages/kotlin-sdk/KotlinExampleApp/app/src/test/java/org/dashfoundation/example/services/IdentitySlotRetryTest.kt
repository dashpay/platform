package org.dashfoundation.example.services

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class IdentitySlotRetryTest {

    private fun takenAt(index: Int) =
        DashSdkError.PlatformWallet.IdentitySlotTakenOnChain(index, "id$index")

    @Test
    fun `a slot taken on chain moves the attempt to the next slot the wallet leases`() = runTest {
        val leases = ArrayDeque(listOf(4, 5))
        val attempted = mutableListOf<Int>()

        val result = IdentitySlotRetry.run(
            first = 3,
            reacquire = { leases.removeFirst() },
            attempt = { slot ->
                attempted += slot
                if (slot < 5) throw takenAt(slot)
                "landed at $slot"
            },
        )

        assertEquals("landed at 5", result)
        assertEquals(listOf(3, 4, 5), attempted)
    }

    @Test
    fun `the refusal after the last retry is thrown`() = runTest {
        var acquired = 0
        val failure = runCatching {
            IdentitySlotRetry.run(
                first = 0,
                reacquire = { ++acquired },
                attempt = { slot -> throw takenAt(slot) },
                limit = 2,
            )
        }.exceptionOrNull()

        assertEquals(2, acquired)
        assertTrue(failure is DashSdkError.PlatformWallet.IdentitySlotTakenOnChain)
        assertEquals(2, (failure as DashSdkError.PlatformWallet.IdentitySlotTakenOnChain).identityIndex)
    }

    @Test
    fun `any other failure is thrown without a new slot`() = runTest {
        var acquired = 0
        val failure = runCatching {
            IdentitySlotRetry.run<Int, Unit>(
                first = 0,
                reacquire = { ++acquired },
                attempt = { error("insufficient funds") },
            )
        }.exceptionOrNull()

        assertEquals(0, acquired)
        assertTrue(failure is IllegalStateException)
    }
}
