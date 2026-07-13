package org.dashfoundation.dashsdk.identity

import org.dashfoundation.dashsdk.wallet.op

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.IdentityNative

/**
 * Data-contract create surface bound to a wallet handle — the Android
 * analog of Swift's `ManagedPlatformWallet.createDataContract`
 * (`RegisterContractSourceView` / `QuickBasicTokenView` submit path).
 *
 * Stateless: each call reads [walletHandle] afresh. The whole
 * build/validate/broadcast pipeline lives in platform-wallet; this only
 * marshals JSON in and the created contract id out.
 *
 * @param walletHandle a live `PlatformWallet` handle (the wallet that owns
 *   the acting identity).
 */
class DataContracts internal constructor(private val walletHandle: Long,
    private val gate: org.dashfoundation.dashsdk.wallet.TeardownGate? = null,
) {

    /**
     * Create + broadcast a new data contract.
     *
     * @param ownerIdentityId the 32-byte owner identity id.
     * @param documentsSchemaJson the documents-schema JSON (required; pass
     *   `"{}"` for a token-only contract).
     * @param tokensSchemaJson optional `tokenSchemas` JSON.
     * @param groupsSchemaJson optional groups JSON.
     * @param keywordsJson optional keywords JSON array.
     * @param description optional plain-text description.
     * @param configJson optional contract-config JSON.
     * @param signingKeyId the identity public-key id to sign with (unused
     *   by the FFI directly today — the signer resolves the key — but kept
     *   for call-site symmetry with token actions; may be `0`).
     * @param signerHandle a native `SignerHandle`.
     * @return the 32-byte created contract id.
     */
    suspend fun create(
        ownerIdentityId: ByteArray,
        documentsSchemaJson: String,
        tokensSchemaJson: String? = null,
        groupsSchemaJson: String? = null,
        keywordsJson: String? = null,
        description: String? = null,
        configJson: String? = null,
        @Suppress("UNUSED_PARAMETER") signingKeyId: Int = 0,
        signerHandle: Long,
    ): ByteArray = gate.op {
        mapNativeErrors {
            IdentityNative.createDataContract(
                walletHandle = walletHandle,
                ownerIdentityId = ownerIdentityId,
                documentsSchemaJson = documentsSchemaJson,
                tokensSchemaJson = tokensSchemaJson,
                groupsSchemaJson = groupsSchemaJson,
                keywordsJson = keywordsJson,
                description = description,
                configJson = configJson,
                signerHandle = signerHandle,
            )
        }
    }

    /**
     * Update + broadcast an existing data contract (DC-04). The wallet
     * fetches the live contract, bumps its version, and *merges* the
     * supplied sections additively (omitted keys keep their on-chain
     * definition), so a single-section update never wipes the rest.
     *
     * @param ownerIdentityId the 32-byte owner identity id.
     * @param contractId the 32-byte id of the contract to update.
     * @param documentsSchemaJson the documents-schema overlay (required; the
     *   FFI requires this section be present — pass the full documents map).
     * @param tokensSchemaJson optional `tokenSchemas` overlay.
     * @param groupsSchemaJson optional groups overlay.
     * @param keywordsJson optional keywords JSON array.
     * @param description optional plain-text description.
     * @param configJson optional contract-config overlay.
     * @param signerHandle a native `SignerHandle`. Unlike the document ops
     *   there is no `signingKeyId` — the wallet selects the key internally.
     * @return the 32-byte updated contract id.
     */
    suspend fun update(
        ownerIdentityId: ByteArray,
        contractId: ByteArray,
        documentsSchemaJson: String,
        tokensSchemaJson: String? = null,
        groupsSchemaJson: String? = null,
        keywordsJson: String? = null,
        description: String? = null,
        configJson: String? = null,
        signerHandle: Long,
    ): ByteArray = withContext(Dispatchers.IO) {
        require(ownerIdentityId.size == 32) { "ownerIdentityId must be 32 bytes" }
        require(contractId.size == 32) { "contractId must be 32 bytes" }
        mapNativeErrors {
            IdentityNative.updateDataContract(
                walletHandle = walletHandle,
                ownerIdentityId = ownerIdentityId,
                contractId = contractId,
                documentsSchemaJson = documentsSchemaJson,
                tokensSchemaJson = tokensSchemaJson,
                groupsSchemaJson = groupsSchemaJson,
                keywordsJson = keywordsJson,
                description = description,
                configJson = configJson,
                signerHandle = signerHandle,
            )
        }
    }
}
