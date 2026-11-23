package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for reviewing a data contract update before it is broadcast: mirrors
 * `rs-unified-sdk-jni/src/contract_update.rs`, one export per C function of
 * `platform-wallet-ffi` (`platform_wallet_preview_data_contract_update` and
 * `platform_wallet_split_contract_update_source`).
 *
 * Internal: the public API is [org.dashfoundation.dashsdk.identity.DataContracts.previewUpdate]
 * and [org.dashfoundation.dashsdk.identity.DataContractUpdateSource.split], one native call
 * each. Both return the JSON the C function wrote; errors throw [DashSDKException].
 */
internal object ContractUpdateNative {

    /**
     * What `IdentityNative.updateDataContractAtReviewedVersion` with the same arguments would change
     * (`platform_wallet_preview_data_contract_update`). Signs and broadcasts nothing.
     *
     * @param ownerIdentityId 32 bytes.
     * @param contractId 32 bytes.
     * @param documentsSchemaJson required; the other sections are null to keep the live ones.
     */
    external fun previewDataContractUpdate(
        walletHandle: Long,
        ownerIdentityId: ByteArray,
        contractId: ByteArray,
        documentsSchemaJson: String,
        tokensSchemaJson: String?,
        groupsSchemaJson: String?,
        keywordsJson: String?,
        description: String?,
        configJson: String?,
    ): String

    /** Contract JSON split into update sections (`platform_wallet_split_contract_update_source`). */
    external fun splitContractUpdateSource(text: String): String
}
