package org.dashfoundation.dashsdk

import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.Dispatchers
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.SdkNative
import org.dashfoundation.dashsdk.ffi.SignerNative
import org.dashfoundation.dashsdk.ffi.WalletManagerNative
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.PlatformWalletPersistenceHandler
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
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

        // Assert only what this smoke test can prove: the native symbol binds
        // and derive-and-sign returns a compact recoverable signature. The
        // array is zeroed by the test's own `finally` above, so asserting it
        // is now zero would only re-check that `fill(0)` works — it would say
        // nothing about JNI, which by contract never scrubs the caller's array.
        assertNotNull("native derive-and-sign should return a signature", signature)
        assertEquals("compact recoverable ECDSA signature", 65, signature!!.size)
    }

    /**
     * Rust↔Kotlin persistence-descriptor lockstep. The trampolines resolve
     * each bridge method by (name, descriptor) only when its slot first
     * fires — for the invitation upsert that would be mid-way through a
     * live, funded `create_invitation`. Resolving the whole table up front
     * turns a descriptor typo into a CI failure instead.
     */
    @Test
    fun persistenceBridgeDescriptorsAllResolve() {
        NativeLoader.ensureLoaded()
        val db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        try {
            val handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
            val firstMismatch =
                WalletManagerNative.nativeVerifyPersistenceBridgeDescriptors(handler)
            assertNull(
                "bridge method failed to resolve (Rust descriptor vs Kotlin " +
                    "signature drift): $firstMismatch",
                firstMismatch,
            )
        } finally {
            db.close()
        }
    }
}
