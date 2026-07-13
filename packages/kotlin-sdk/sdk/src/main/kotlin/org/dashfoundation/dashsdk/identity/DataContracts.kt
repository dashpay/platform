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
}
