package org.dashfoundation.dashsdk.identity

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertTrue
import org.junit.Test

/** Wrapper-seam tests for [IdentityRegistration.loadIdentityAtIndex]. */
class IdentityLoadAtIndexTest {

    @Test
    fun `load rejects a negative identity index before JNI`() = runTest {
        // JVM tests never load the JNI library, so reaching the native call
        // would fail with UnsatisfiedLinkError instead.
        val failure = runCatching {
            IdentityRegistration().loadIdentityAtIndex(
                walletHandle = 1,
                mnemonicResolverHandle = 2,
                identityIndex = -1,
            )
        }
        assertTrue(failure.exceptionOrNull() is IllegalArgumentException)
    }
}
