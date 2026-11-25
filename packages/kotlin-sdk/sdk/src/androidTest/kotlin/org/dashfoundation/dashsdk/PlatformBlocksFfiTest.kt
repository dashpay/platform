package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.PlatformBlocksNative
import org.dashfoundation.dashsdk.ffi.SdkNative
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertThrows
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Round trips of the block and transition status JNI exports
 * (`rs-unified-sdk-jni/src/platform_blocks.rs`) through `rs-sdk-ffi` on a device: the symbol
 * names, the method descriptors, the argument marshalling and the error code Kotlin maps.
 * Every call here is refused before anything reaches the network (the JNI layer and the C
 * functions read every argument first), so no network at protocol version 15 is needed:
 * `createTrusted` only builds the client. The reads that reach Platform are exercised through
 * KotlinExampleApp's Block Explorer and Transition Status Checker.
 */
@RunWith(AndroidJUnit4::class)
class PlatformBlocksFfiTest {

    private val validHash = "07".repeat(32)
    private val validOwner = "11111111111111111111111111111111"

    private var handle = 0L

    @Before
    fun createSdk() {
        NativeLoader.ensureLoaded()
        handle = SdkNative.createTrusted(
            network = 1,
            dapiAddresses = null,
            quorumUrl = null,
            skipAssetLockProofVerification = false,
            requestRetryCount = 3,
            requestTimeoutMs = 30_000,
            platformVersion = 0,
        )
        assertNotEquals("createTrusted must return a live handle", 0L, handle)
    }

    @After
    fun destroySdk() {
        SdkNative.destroy(handle)
    }

    /** rs-sdk-ffi refuses a null SDK handle with InvalidParameter (1). */
    @Test
    fun shouldRefuseANullSdkHandleInEveryExport() {
        assertCode(1) { PlatformBlocksNative.getBlocks(0L, 1, 1) }
        assertCode(1) { PlatformBlocksNative.getOwnerTransactionsInBlock(0L, 1, validOwner) }
        assertCode(1) { PlatformBlocksNative.getRecentStateTransitionStatus(0L, validHash) }
        assertCode(1) { PlatformBlocksNative.getStateTransitionStatusByHash(0L, validHash) }
    }

    /** The JNI layer refuses what Kotlin's signed integers would turn into huge unsigned ones. */
    @Test
    fun shouldRefuseNegativeHeightsAndCounts() {
        assertCode(1) { PlatformBlocksNative.getBlocks(handle, -1, 1) }
        assertCode(1) { PlatformBlocksNative.getBlocks(handle, 1, -1) }
        assertCode(1) { PlatformBlocksNative.getOwnerTransactionsInBlock(handle, -1, validOwner) }
    }

    /** rs-sdk-ffi refuses a malformed identity id or hash before asking the network. */
    @Test
    fun shouldRefuseAMalformedOwnerOrHashBeforeTheNetwork() {
        assertCode(1) { PlatformBlocksNative.getOwnerTransactionsInBlock(handle, 1, "0OIl") }
        assertCode(1) {
            PlatformBlocksNative.getOwnerTransactionsInBlock(handle, 1, "1\u0000")
        }
        for (hash in listOf("zz".repeat(32), "07".repeat(31), "07\u0000")) {
            assertCode(1) { PlatformBlocksNative.getRecentStateTransitionStatus(handle, hash) }
            assertCode(1) { PlatformBlocksNative.getStateTransitionStatusByHash(handle, hash) }
        }
    }

    private fun assertCode(code: Int, call: () -> Unit): DashSDKException {
        val error = assertThrows(DashSDKException::class.java) { call() }
        assertEquals(error.message, code, error.code)
        return error
    }
}
