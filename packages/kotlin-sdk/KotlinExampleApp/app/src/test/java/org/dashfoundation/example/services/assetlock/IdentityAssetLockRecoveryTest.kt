package org.dashfoundation.example.services.assetlock

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.wallet.TrackedAssetLock
import org.dashfoundation.example.util.Base58
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test
import kotlinx.coroutines.test.runTest

class IdentityAssetLockRecoveryTest {

    @Test
    fun `registration and topup rows route to separate recovery UIs`() {
        val registration = lock(TrackedAssetLock.FundingType.IDENTITY_REGISTRATION, 1)
        val topUp = lock(TrackedAssetLock.FundingType.IDENTITY_TOP_UP, 2)
        val unbound = lock(TrackedAssetLock.FundingType.IDENTITY_TOP_UP_NOT_BOUND, 3)

        assertEquals(listOf(registration), IdentityAssetLockRecovery.registrations(listOf(topUp, registration, unbound)))
        assertEquals(
            listOf(topUp, unbound),
            IdentityAssetLockRecovery.topUps(listOf(topUp, registration, unbound), selectedIdentityIndex = 4),
        )
    }

    @Test
    fun `bound topup for another identity is excluded while unbound topup remains eligible`() {
        val selectedIdentityIndex = 4
        val boundToSelected = lock(TrackedAssetLock.FundingType.IDENTITY_TOP_UP, 2, registrationIndex = 4)
        val boundToOther = lock(TrackedAssetLock.FundingType.IDENTITY_TOP_UP, 3, registrationIndex = 5)
        val unbound = lock(TrackedAssetLock.FundingType.IDENTITY_TOP_UP_NOT_BOUND, 4, registrationIndex = 5)

        assertEquals(
            listOf(boundToSelected, unbound),
            IdentityAssetLockRecovery.topUps(
                listOf(boundToSelected, boundToOther, unbound),
                selectedIdentityIndex,
            ),
        )
    }

    @Test
    fun `display formatting does not mutate submission outpoint`() {
        val lock = lock(TrackedAssetLock.FundingType.IDENTITY_REGISTRATION, 7)
        val original = lock.outpointTxid.copyOf()

        val label = IdentityAssetLockRecovery.label(lock)

        assertTrue(label.endsWith(":7 · built"))
        assertTrue(original.contentEquals(lock.outpointTxid))
        assertEquals(7, lock.outpointVout)
    }

    @Test
    fun `registration resume dispatches the original outpoint without a fresh funding path`() = runTest {
        val lock = lock(TrackedAssetLock.FundingType.IDENTITY_REGISTRATION, 7)
        var resumed = false

        IdentityAssetLockRecovery.submitRegistrationResume(lock) { submitted ->
            resumed = true
            assertTrue(lock.outpointTxid.contentEquals(submitted.outpointTxid))
            assertEquals(lock.outpointVout, submitted.outpointVout)
            Unit
        }

        assertTrue(resumed)
    }

    @Test
    fun `topup resume dispatches the original outpoint without a fresh funding path`() = runTest {
        val lock = lock(TrackedAssetLock.FundingType.IDENTITY_TOP_UP, 8)
        var resumed = false

        IdentityAssetLockRecovery.submitTopUpResume(lock) { submitted ->
            resumed = true
            assertTrue(lock.outpointTxid.contentEquals(submitted.outpointTxid))
            assertEquals(lock.outpointVout, submitted.outpointVout)
            Unit
        }

        assertTrue(resumed)
    }

    @Test
    fun `registration resume refused for a taken slot offers a top-up of the identity there`() {
        val lock = lock(TrackedAssetLock.FundingType.IDENTITY_REGISTRATION, 7)
        val id = ByteArray(32) { (it + 1).toByte() }
        val base58 = Base58.encode(id)

        val offer = IdentityAssetLockRecovery.registrationResumeTopUp(
            lock,
            DashSdkError.PlatformWallet.IdentitySlotTakenOnChain(4, base58),
        )

        requireNotNull(offer)
        assertSame(lock, offer.lock)
        assertEquals(4, offer.identityIndex)
        assertTrue(id.contentEquals(offer.identityId))
        assertEquals(base58, offer.identityIdBase58)
    }

    @Test
    fun `other registration resume failures offer no top-up`() {
        val lock = lock(TrackedAssetLock.FundingType.IDENTITY_REGISTRATION, 7)

        assertNull(IdentityAssetLockRecovery.registrationResumeTopUp(lock, IllegalStateException("x")))
        assertNull(
            IdentityAssetLockRecovery.registrationResumeTopUp(
                lock,
                DashSdkError.PlatformWallet.IdentitySlotNotHeld(4),
            ),
        )
        // An id that is not a 32-byte identifier cannot be topped up.
        assertNull(
            IdentityAssetLockRecovery.registrationResumeTopUp(
                lock,
                DashSdkError.PlatformWallet.IdentitySlotTakenOnChain(4, "not-an-id"),
            ),
        )
    }

    private fun lock(
        type: TrackedAssetLock.FundingType,
        vout: Int,
        registrationIndex: Int = 4,
    ) = TrackedAssetLock(
        outpointTxid = ByteArray(32) { it.toByte() },
        outpointVout = vout,
        fundingType = type,
        status = TrackedAssetLock.Status.BUILT,
        registrationIndex = registrationIndex,
        instantLockPresent = false,
        chainLockHeight = 0,
    )
}
