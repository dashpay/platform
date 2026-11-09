package org.dashfoundation.dashsdk.wallet

import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The one-time Orchard key helpers are handle-less and may be the first SDK
 * call in a process. Without the native library they must fail inside
 * [org.dashfoundation.dashsdk.Sdk.initialize] (the load step), not with an
 * [UnsatisfiedLinkError] from the external fun itself. This host JVM has no
 * `libdash_sdk_jni.so`, so the load is what fails here.
 */
class OneTimeOrchardKeyTest {
    private fun assertFailsInLoad(nativeMethod: String, call: () -> Unit) {
        val error = assertThrows(Throwable::class.java) { call() }
        assertTrue(
            "expected the library load to fail, got ${error::class.simpleName}: ${error.message}",
            error is UnsatisfiedLinkError && error.message?.contains("dash_sdk_jni") == true,
        )
        assertFalse(
            "reached the external fun before loading the library",
            error.message?.contains(nativeMethod) == true,
        )
    }

    @Test
    fun `generateOneTimeOrchardKey initializes the native library before entering JNI`() =
        assertFailsInLoad("generateOneTimeOrchardKey") { generateOneTimeOrchardKey() }

    @Test
    fun `orchardAddressFromSpendingKey initializes the native library before entering JNI`() =
        assertFailsInLoad("orchardAddressFromSpendingKey") {
            orchardAddressFromSpendingKey(ByteArray(32))
        }
}
