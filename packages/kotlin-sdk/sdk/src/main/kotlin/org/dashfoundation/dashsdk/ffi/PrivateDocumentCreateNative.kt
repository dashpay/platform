package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for creating an entry of any private document type (protocol version 15):
 * mirrors `rs-unified-sdk-jni/src/private_document_create.rs`, which bridges
 * `platform_wallet_manager_private_document_recipient_keys` (and its free function),
 * `platform_wallet_manager_private_document_create_fee` and
 * `platform_wallet_manager_create_private_document_entry`.
 *
 * Internal: the public API is the `PlatformWalletManager` extensions in
 * `PlatformWalletManagerPrivateDocuments.kt`. Errors throw [DashSDKException]: an argument the
 * JNI layer cannot marshal (an id that is not 32 bytes, a negative count or account, a key id
 * outside `u32`, an auth mode outside a byte, a null or NUL-holding string) with code 1, and
 * the platform-wallet refusals with their code shifted by 1000.
 */
internal object PrivateDocumentCreateNative {

    /**
     * The keys of [identityId] an entry may be sealed to, as `[keyId, purpose]` pairs
     * flattened into one array (purpose 1 ENCRYPTION, 2 DECRYPTION), empty for none, or null
     * after throwing. Blocking (network).
     */
    external fun recipientKeys(managerHandle: Long, identityId: ByteArray): LongArray?

    /**
     * The fee, in credits, a create of [entryCount] entries of [documentTypeName] of
     * [contractId] reserves when it spends one or two notes. Blocking (network).
     */
    external fun createFee(
        managerHandle: Long,
        contractId: ByteArray,
        documentTypeName: String,
        entryCount: Int,
    ): Long

    /**
     * Create one entry, paid from [account]'s shielded notes of [walletId]. [authMode] is the
     * FFI's byte (0 anonymous, 1 deniable MAC). Returns 36 bytes: the commitment, then the
     * sender key id as a little-endian u32; null after throwing. Blocking (Halo 2 proof,
     * network). Only on a shielded build.
     */
    @Suppress("LongParameterList")
    external fun createEntry(
        managerHandle: Long,
        walletId: ByteArray,
        resolverHandle: Long,
        account: Int,
        contractId: ByteArray,
        documentTypeName: String,
        senderIdentityId: ByteArray,
        recipientIdentityId: ByteArray,
        recipientKeyId: Long,
        authMode: Int,
        propertiesJson: String,
    ): ByteArray?
}
