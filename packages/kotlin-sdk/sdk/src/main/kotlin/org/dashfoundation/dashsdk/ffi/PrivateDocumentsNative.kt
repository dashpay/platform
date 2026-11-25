package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for reading a private document type's entries (protocol version 15):
 * mirrors `rs-unified-sdk-jni/src/private_documents.rs`, which bridges
 * `platform_wallet_read_private_document_entries` and frees its result with
 * `platform_wallet_private_document_read_free` before returning.
 *
 * Internal: the public API is [org.dashfoundation.dashsdk.wallet.PrivateDocuments]. Errors throw
 * [DashSDKException]: an argument the JNI layer cannot marshal (a contract id or cursor
 * fingerprint that is not 32 bytes, a negative count, a null or NUL-holding type name) with
 * code 1, and the platform-wallet refusals with their code shifted by 1000.
 */
internal object PrivateDocumentsNative {

    /**
     * Read the store of [documentTypeName] of [contractId] for every identity of the wallet,
     * at most [maxPages] proved pages. The keys are derived inside [coreSignerHandle], the
     * manager's `MnemonicResolverHandle`.
     *
     * With [fromKeySet] and [fromStore] null the read starts at the store's start; otherwise
     * the five `from*` arguments are a previous read's cursor, field by field, as it came.
     * Returns the result as JSON (decoded by `PrivateDocumentEntriesRead.fromJson`), or null
     * after throwing. Blocking (network).
     */
    @Suppress("LongParameterList")
    external fun readPrivateDocumentEntries(
        walletHandle: Long,
        coreSignerHandle: Long,
        contractId: ByteArray,
        documentTypeName: String,
        fromPosition: Long,
        fromProvedTotalCount: Long,
        fromHasProvedTotalCount: Boolean,
        fromKeySet: ByteArray?,
        fromStore: ByteArray?,
        maxPages: Int,
    ): String?
}
