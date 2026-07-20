package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.SdkNative
import org.dashfoundation.dashsdk.ffi.SignerNative
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * A-M1 gate: proves the native library loads on-device, JNI_OnLoad ran,
 * symbols resolve, and an SDK handle round-trips create → getNetwork →
 * destroy. No network access is required — `createTrusted` only builds the
 * client; it does not connect.
 */
@RunWith(AndroidJUnit4::class)
class FfiSmokeTest {

    @Test
    fun nativeLibraryLoadsAndReportsVersion() {
        NativeLoader.ensureLoaded()
        val version = SdkNative.version()
        assertTrue("version should be non-empty", version.isNotEmpty())
    }

    @Test
    fun sdkHandleRoundTrip() {
        NativeLoader.ensureLoaded()
        // Testnet, default DAPI address list, trusted quorum defaults.
        val handle = SdkNative.createTrusted(
            network = 1,
            dapiAddresses = null,
            quorumUrl = null,
            skipAssetLockProofVerification = false,
            requestRetryCount = 3,
            requestTimeoutMs = 30_000,
            platformVersion = 0,
        )
        try {
            assertNotEquals("createTrusted must return a live handle", 0L, handle)
            assertEquals("handle should report testnet", 1, SdkNative.getNetwork(handle))
        } finally {
            SdkNative.destroy(handle)
        }
    }

    @Test
    fun mnemonicAndPathSignerSymbolLoadsAndSigns() {
        NativeLoader.ensureLoaded()
        val mnemonicUtf8 = (
            "abandon abandon abandon abandon abandon abandon abandon abandon " +
                "abandon abandon abandon about"
        ).toByteArray(Charsets.UTF_8)
        val signature = try {
            SignerNative.signWithMnemonicAndPathInto(
                mnemonicUtf8 = mnemonicUtf8,
                derivationPath = "m/9'/5'/3'/0/0",
                network = Network.TESTNET.ffiValue,
                data = "jni signer smoke".toByteArray(Charsets.UTF_8),
            )
        } finally {
            // JNI copies the phrase into Rust-owned zeroizing memory but
            // deliberately does not mutate the JVM array. Direct callers own
            // this cleanup; the production wrapper applies the same finally.
            mnemonicUtf8.fill(0)
        }

        assertNotNull("native derive-and-sign should return a signature", signature)
        assertEquals("compact recoverable ECDSA signature", 65, signature!!.size)
        assertArrayEquals(ByteArray(mnemonicUtf8.size), mnemonicUtf8)
    }
}
