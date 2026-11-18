package org.dashfoundation.dashsdk.identity

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertTrue
import org.junit.Test

/** Wrapper-seam tests for [IdentityRegistration.recoverClaimedIdentity]. */
class IdentityRecoverClaimedTest {

    // JVM tests never load the JNI library, so reaching the native call
    // would fail with UnsatisfiedLinkError instead.

    @Test
    fun `recover rejects a negative identity index before JNI`() = runTest {
        val failure = runCatching {
            IdentityRegistration().recoverClaimedIdentity(
                walletHandle = 1,
                mnemonicResolverHandle = 2,
                identityIndex = -1,
                identityId = ByteArray(32),
            )
        }
        assertTrue(failure.exceptionOrNull() is IllegalArgumentException)
    }

    @Test
    fun `recover rejects an identity id that is not 32 bytes before JNI`() = runTest {
        val failure = runCatching {
            IdentityRegistration().recoverClaimedIdentity(
                walletHandle = 1,
                mnemonicResolverHandle = 2,
                identityIndex = 0,
                identityId = ByteArray(31),
            )
        }
        assertTrue(failure.exceptionOrNull() is IllegalArgumentException)
    }
}
