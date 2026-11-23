package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.PrivateDocumentsNative
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Round trips of the private document read JNI export
 * (`rs-unified-sdk-jni/src/private_documents.rs`) through `platform-wallet-ffi` on a device:
 * the symbol name, the method descriptor, the argument marshalling and the error codes Kotlin
 * maps. Every call here is refused before a wallet, a resolver or the network is reached, so
 * none is needed. The read itself needs a protocol version 15 network holding a private type
 * and is exercised through KotlinExampleApp's Read Entries screen.
 */
@RunWith(AndroidJUnit4::class)
class PrivateDocumentsFfiTest {

    private val contractId = ByteArray(32) { 0x11 }

    @Before
    fun loadLibrary() {
        NativeLoader.ensureLoaded()
    }

    private fun read(
        walletHandle: Long = 1L,
        coreSignerHandle: Long = 0L,
        contractId: ByteArray = this.contractId,
        typeName: String = "note",
        keySet: ByteArray? = null,
        store: ByteArray? = null,
        maxPages: Int = 1,
    ) = PrivateDocumentsNative.readPrivateDocumentEntries(
        walletHandle, coreSignerHandle, contractId, typeName,
        0L, 0L, false, keySet, store, maxPages,
    )

    /** The JNI layer refuses what it cannot marshal with InvalidParameter (1). */
    @Test
    fun shouldRefuseWhatTheJniLayerCannotMarshal() {
        assertCode(1) { read(maxPages = -1) }
        assertCode(1) { read(contractId = ByteArray(31)) }
        assertCode(1) { read(typeName = "no\u0000te") }
        // A cursor needs both fingerprints, each 32 bytes
        assertCode(1) { read(keySet = ByteArray(32)) }
        assertCode(1) { read(keySet = ByteArray(32), store = ByteArray(31)) }
    }

    /**
     * platform-wallet-ffi refuses a null resolver handle with ErrorNullPointer (3), which the
     * JNI layer throws shifted by 1000, before looking the wallet up.
     */
    @Test
    fun shouldRefuseANullResolverInPlatformWalletFfi() {
        assertCode(1003) { read(coreSignerHandle = 0L) }
    }

    private fun assertCode(code: Int, call: () -> Unit): DashSDKException {
        val error = assertThrows(DashSDKException::class.java) { call() }
        assertEquals(error.message, code, error.code)
        return error
    }
}
