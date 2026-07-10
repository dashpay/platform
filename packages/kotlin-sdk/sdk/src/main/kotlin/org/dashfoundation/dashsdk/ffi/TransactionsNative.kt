package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for the identity / document / voting write paths —
 * mirrors `rs-unified-sdk-jni/src/transactions.rs`.
 *
 * Internal: the public API is
 * [org.dashfoundation.dashsdk.identity.IdentityUpdates],
 * [org.dashfoundation.dashsdk.documents.DocumentTransactions], and
 * [org.dashfoundation.dashsdk.voting.VoteCasting]. Handles are raw Rust
 * pointers as [Long] (wallet handle, SDK handle, `SignerHandle`); passing a
 * stale or foreign value is undefined behavior, so ownership is confined to
 * the SDK wrapper classes. Errors throw [DashSDKException].
 *
 * Each function is a thin marshaler over a SINGLE FFI entry point — no
 * orchestration crosses this boundary (see `packages/kotlin-sdk/CLAUDE.md`).
 */
internal object TransactionsNative {

    /**
     * Add public keys and/or disable existing key ids on an identity,
     * signing the resulting `IdentityUpdateTransition` via [signerHandle]
     * (the identity's MASTER auth key). Bridges
     * `platform_wallet_update_identity_with_signer` — the exact call Swift
     * `AddIdentityKeyView.submit` makes through `wallet.updateIdentity(...)`.
     *
     * @param addPubkeysBlob big-endian rows for the keys to add: `u32
     *   rowCount` then per row `u32 keyId, u8 keyType, u8 purpose, u8
     *   securityLevel, u8 readOnly, u8 contractBoundsKind, u16 pubkeyLen,
     *   pubkey`, plus (when `contractBoundsKind != 0`) a 32-byte contract id
     *   and (when `== 2`) `u16 docTypeLen, docType`. May be empty.
     * @param disablePublicKeyIds key ids to disable; may be empty. At least
     *   one of add / disable must be non-empty.
     */
    external fun updateIdentity(
        walletHandle: Long,
        identityId: ByteArray,
        addPubkeysBlob: ByteArray,
        disablePublicKeyIds: IntArray,
        signerHandle: Long,
    )

    /**
     * Purchase for-sale [documentId] on [contractId]'s [documentType] for
     * [price] credits, with [purchaserId] as the buyer — signed via
     * [signerHandle] with key [signingKeyId]. Bridges
     * `platform_wallet_document_purchase`. Returns the confirmed document's
     * canonical JSON (the confirmed 32-byte id is its `$id` field).
     */
    external fun documentPurchase(
        walletHandle: Long,
        purchaserId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        documentId: ByteArray,
        price: Long,
        signingKeyId: Int,
        signerHandle: Long,
    ): String

    /**
     * Set the trade price of [documentId] on [contractId]'s [documentType],
     * owned by [ownerId], to [price] credits — signed via [signerHandle] with
     * key [signingKeyId]. Bridges `platform_wallet_document_set_price`.
     * Returns the confirmed document's canonical JSON (now carrying
     * `$price`).
     */
    external fun documentSetPrice(
        walletHandle: Long,
        ownerId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        documentId: ByteArray,
        price: Long,
        signingKeyId: Int,
        signerHandle: Long,
    ): String

    /**
     * Create + broadcast a new document on [contractId]'s [documentType],
     * owned by [ownerId], signed via [signerHandle]. Bridges
     * `platform_wallet_create_document_with_signer` (Swift
     * `ManagedPlatformWallet.createDocument`). Unlike purchase / set-price
     * there is **no** `signingKeyId` — the Rust side selects an
     * AUTHENTICATION + ECDSA key satisfying the document type's security
     * level from the wallet's `IdentityManager`.
     *
     * @param propertiesJson JSON object keyed by property name (byte-array
     *   fields as hex, identifier fields as base58); `"{}"` for a type with
     *   no required properties.
     * @return the confirmed document's canonical JSON (its 32-byte id is the
     *   base58 `$id` field).
     */
    external fun documentCreate(
        walletHandle: Long,
        ownerId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        propertiesJson: String,
        signerHandle: Long,
    ): String

    /**
     * Replace + broadcast the full property set of [documentId] on
     * [contractId]'s [documentType], owned by [ownerId], signed via
     * [signerHandle] with key [signingKeyId]. Bridges
     * `platform_wallet_document_replace` (Swift
     * `ManagedPlatformWallet.replaceDocument`). The revision is bumped on
     * the Rust side — the caller does not pass a revision.
     *
     * @param propertiesJson the FULL replacement property object (byte-array
     *   fields as hex, identifier fields as base58), same encoding as
     *   [documentCreate].
     * @return the confirmed document's canonical JSON (its 32-byte id is the
     *   base58 `$id` field).
     */
    external fun documentReplace(
        walletHandle: Long,
        ownerId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        documentId: ByteArray,
        propertiesJson: String,
        signingKeyId: Int,
        signerHandle: Long,
    ): String

    /**
     * Delete + broadcast [documentId] on [contractId]'s [documentType],
     * owned by [ownerId], signed via [signerHandle] with key [signingKeyId].
     * Bridges `platform_wallet_document_delete` (Swift
     * `ManagedPlatformWallet.deleteDocument`). Delete returns no document
     * body, so this returns the deleted document's 32-byte id for
     * confirmation.
     */
    external fun documentDelete(
        walletHandle: Long,
        ownerId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        documentId: ByteArray,
        signingKeyId: Int,
        signerHandle: Long,
    ): ByteArray

    /**
     * Transfer + broadcast [documentId] on [contractId]'s [documentType],
     * from [ownerId] to [recipientId], signed via [signerHandle] with key
     * [signingKeyId]. Bridges `platform_wallet_document_transfer` (Swift
     * `ManagedPlatformWallet.transferDocument`). Only valid for document
     * types whose schema is `transferable`.
     *
     * @return the confirmed document's canonical JSON, now reflecting the
     *   new owner (its 32-byte id is the base58 `$id` field).
     */
    external fun documentTransfer(
        walletHandle: Long,
        ownerId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        documentId: ByteArray,
        recipientId: ByteArray,
        signingKeyId: Int,
        signerHandle: Long,
    ): String

    /**
     * Create + broadcast an ENCRYPTED wallet-contract document (the wire-
     * compatible `txMetadata` shape) on [contractId]'s [documentType], owned by
     * [ownerId], signed via [signerHandle]. Bridges
     * `platform_wallet_create_encrypted_document_with_signer`.
     *
     * The Rust side selects the identity's ENCRYPTION key id (the `keyIndex`
     * field), derives the AES key from the wallet HD tree, and seals [payload]
     * into the legacy `version ‖ IV ‖ AES-256-CBC` blob — decryptable by the
     * legacy `org.dashj.platform` stack and vice versa.
     *
     * @param encryptionKeyIndex the app's per-document index (dash-wallet's
     *   monotonic `1 + countAllRequests()` counter); non-negative.
     * @param version payload version byte (`1` = protobuf, as the wallet writes).
     * @param payload the already-serialized opaque plaintext (a protobuf
     *   `TxMetadataBatch`); the SDK does not parse it.
     * @return the confirmed document's canonical JSON (its 32-byte id is the
     *   base58 `$id` field).
     */
    external fun documentCreateEncrypted(
        walletHandle: Long,
        ownerId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        encryptionKeyIndex: Int,
        version: Int,
        payload: ByteArray,
        signerHandle: Long,
    ): String

    /**
     * Fetch + DECRYPT every encrypted wallet-contract document owned by
     * [ownerId] on [contractId]'s [documentType] updated at or after [sinceMs]
     * (epoch-millis). Bridges `platform_wallet_fetch_encrypted_documents` — the
     * wire-compatible read counterpart of the legacy `getTxMetaData(since, key)`.
     *
     * @return a JSON array; each element is `{ "id", "ownerId" (base58),
     *   "keyIndex", "encryptionKeyIndex", "version", "updatedAt" (number|null),
     *   "payload" (base64 of the decrypted opaque plaintext) }`. Documents that
     *   fail to decrypt are skipped Rust-side.
     */
    external fun documentFetchEncrypted(
        walletHandle: Long,
        ownerId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        sinceMs: Long,
    ): String

    /**
     * Cast a masternode contested-resource vote and wait for the response.
     * Bridges `dash_sdk_contested_resource_cast_vote` (Swift
     * `SDK.castContestedResourceVote`).
     *
     * @param voteChoice `0` TowardsIdentity (requires [contenderIdentityId]),
     *   `1` Abstain, `2` Lock.
     * @param contenderIdentityId base58 contender id; required only when
     *   [voteChoice] == 0, else may be null.
     * @param voterProTxHash 32-byte masternode pro_tx_hash.
     * @param votingPrivateKey 32-byte masternode voting private key (both the
     *   signer and the ECDSA_HASH160 voting key are derived Rust-side; the
     *   bytes are not stored).
     * @param networkOrd `Network.ffiValue` (0 Mainnet, 1 Testnet, 2 Devnet,
     *   3 Regtest).
     */
    external fun castContestedResourceVote(
        sdkHandle: Long,
        contractId: String,
        documentType: String,
        indexName: String,
        indexValuesJson: String,
        voteChoice: Int,
        contenderIdentityId: String?,
        voterProTxHash: ByteArray,
        votingPrivateKey: ByteArray,
        networkOrd: Int,
    )
}
