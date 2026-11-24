package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.PrivateDocumentCreateNative
import org.dashfoundation.dashsdk.ffi.PrivateDocumentsNative
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Round trips of the private document JNI exports (`rs-unified-sdk-jni/src/private_documents.rs`
 * for the read, `private_document_create.rs` for the create, its recipient keys and its fee)
 * through `platform-wallet-ffi` on a device: the symbol names, the method descriptors, the
 * argument marshalling and the error codes Kotlin maps. Every call here is refused before a
 * wallet, a resolver or the network is reached, so none is needed. The read and the create
 * themselves need a protocol version 15 network holding a private type and are exercised
 * through KotlinExampleApp's Read Entries and Create Entry screens.
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

    @Suppress("LongParameterList")
    private fun create(
        account: Int = 0,
        resolverHandle: Long = 0L,
        walletId: ByteArray = ByteArray(32),
        typeName: String = "note",
        recipientKeyId: Long = 1L,
        authMode: Int = 0,
        properties: String = "{}",
    ) = PrivateDocumentCreateNative.createEntry(
        1L, walletId, resolverHandle, account, contractId, typeName,
        ByteArray(32), ByteArray(32), recipientKeyId, authMode, properties,
    )

    /**
     * The recipient key and fee exports, present on every build, refuse what their C arguments
     * cannot hold with InvalidParameter (1): a Java string with an unpaired surrogate is refused,
     * not replaced.
     */
    @Test
    fun shouldRefuseWhatTheKeyAndFeeJniLayerCannotMarshal() {
        assertCode(1) { PrivateDocumentCreateNative.recipientKeys(1L, ByteArray(31)) }
        assertCode(1) { PrivateDocumentCreateNative.createFee(1L, contractId, "note", -1) }
        assertCode(1) { PrivateDocumentCreateNative.createFee(1L, ByteArray(31), "note", 1) }
        assertCode(1) { PrivateDocumentCreateNative.createFee(1L, contractId, "no\u0000te", 1) }
        assertCode(1) { PrivateDocumentCreateNative.createFee(1L, contractId, "no\uD800te", 1) }
    }

    /**
     * A valid supplementary character crosses the VM boundary intact: beside a NUL the refusal
     * names the NUL, not the surrogate pair, and alone it reaches platform-wallet-ffi, which
     * refuses the unknown manager (1001) before the network.
     */
    @Test
    fun shouldReadASupplementaryCharacterAsOneAndNameARealNul() {
        val nul = assertCode(1) {
            PrivateDocumentCreateNative.createFee(1L, contractId, "\uD83D\uDE00\u0000note", 1)
        }
        assertTrue(nul.message, nul.message.orEmpty().contains("holds a NUL character"))
        assertCode(1001) { PrivateDocumentCreateNative.createFee(-1L, contractId, "\uD83D\uDE00", 1) }
    }

    /**
     * platform-wallet-ffi refuses an unknown manager handle with ErrorInvalidHandle (1), shifted
     * by 1000, before the network.
     */
    @Test
    fun shouldRefuseAnUnknownManagerInPlatformWalletFfi() {
        assertCode(1001) { PrivateDocumentCreateNative.recipientKeys(-1L, ByteArray(32)) }
        assertCode(1001) { PrivateDocumentCreateNative.createFee(-1L, contractId, "note", 1) }
    }

    /** The create export, on a shielded build, refuses what its C arguments cannot hold with 1. */
    @Test
    fun shouldRefuseWhatTheCreateJniLayerCannotMarshal() {
        assumeTrue("the native library has no shielded support", Sdk.hasShielded())
        assertCode(1) { create(account = -1) }
        assertCode(1) { create(recipientKeyId = -1L) }
        assertCode(1) { create(recipientKeyId = 1L shl 32) }
        assertCode(1) { create(authMode = 256) }
        assertCode(1) { create(walletId = ByteArray(31)) }
        assertCode(1) { create(properties = "{\"a\":\"\u0000\"}") }
        assertCode(1) { create(properties = "{\"a\":\"\uDC00\"}") }
    }

    /** platform-wallet-ffi refuses a null resolver with ErrorNullPointer (3), shifted by 1000. */
    @Test
    fun shouldRefuseANullResolverForACreateInPlatformWalletFfi() {
        assumeTrue("the native library has no shielded support", Sdk.hasShielded())
        assertCode(1003) { create(resolverHandle = 0L) }
    }

    private fun assertCode(code: Int, call: () -> Unit): DashSDKException {
        val error = assertThrows(DashSDKException::class.java) { call() }
        assertEquals(error.message, code, error.code)
        return error
    }
}
