package org.dashfoundation.dashsdk.documents

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.TransactionsNative

/**
 * Document purchase + set-price bridge — port of the document
 * state-transition slice of `ManagedPlatformWallet.swift`
 * (`purchaseDocument(...)`, driven by Swift `DocumentWithPriceView`). Thin
 * wrapper over [TransactionsNative]; no orchestration lives here (see
 * `packages/kotlin-sdk/CLAUDE.md`). Handles are supplied by the caller —
 * `PlatformWalletManager` owns the [signerHandle], `ManagedPlatformWallet`
 * owns the wallet handle.
 *
 * All ids are 32-byte canonical form; [contractId] / [documentId] /
 * [purchaserId] / [ownerId] must decode from base58 or hex before the call.
 */
class DocumentTransactions internal constructor() {

    /**
     * Purchase for-sale [documentId] on [contractId]'s [documentType] for
     * [price] credits, with [purchaserId] as the buyer (and new owner) —
     * signed via [signerHandle] with key [signingKeyId]. Mirrors Swift
     * `ManagedPlatformWallet.purchaseDocument`'s parameter order. Consensus
     * rejects a purchase where the buyer is the current owner — the caller's
     * UI gates against that self-buy case.
     *
     * @return the confirmed document's canonical JSON (now owned by the
     *   purchaser; its 32-byte id is the `$id` field).
     */
    suspend fun purchase(
        walletHandle: Long,
        purchaserId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        documentId: ByteArray,
        price: Long,
        signingKeyId: Int,
        signerHandle: Long,
    ): String = withContext(Dispatchers.IO) {
        require(purchaserId.size == 32) { "purchaserId must be 32 bytes" }
        require(contractId.size == 32) { "contractId must be 32 bytes" }
        require(documentId.size == 32) { "documentId must be 32 bytes" }
        require(price > 0) { "price must be positive, got $price" }
        require(signingKeyId >= 0) { "signingKeyId must be non-negative, got $signingKeyId" }
        mapNativeErrors {
            TransactionsNative.documentPurchase(
                walletHandle,
                purchaserId,
                contractId,
                documentType,
                documentId,
                price,
                signingKeyId,
                signerHandle,
            )
        }
    }

    /**
     * Set (update) the trade price of [documentId] on [contractId]'s
     * [documentType], owned by [ownerId], to [price] credits — signed via
     * [signerHandle] with key [signingKeyId]. Mirrors the set-price flow
     * behind Swift `DocumentWithPriceView`.
     *
     * @return the confirmed document's canonical JSON (now carrying
     *   `$price`).
     */
    suspend fun setPrice(
        walletHandle: Long,
        ownerId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        documentId: ByteArray,
        price: Long,
        signingKeyId: Int,
        signerHandle: Long,
    ): String = withContext(Dispatchers.IO) {
        require(ownerId.size == 32) { "ownerId must be 32 bytes" }
        require(contractId.size == 32) { "contractId must be 32 bytes" }
        require(documentId.size == 32) { "documentId must be 32 bytes" }
        require(price >= 0) { "price must be non-negative, got $price" }
        require(signingKeyId >= 0) { "signingKeyId must be non-negative, got $signingKeyId" }
        mapNativeErrors {
            TransactionsNative.documentSetPrice(
                walletHandle,
                ownerId,
                contractId,
                documentType,
                documentId,
                price,
                signingKeyId,
                signerHandle,
            )
        }
    }

    /**
     * Create + broadcast a new document on [contractId]'s [documentType],
     * owned by [ownerId] — signed via [signerHandle]. Mirrors Swift
     * `ManagedPlatformWallet.createDocument` (driven by `CreateDocumentView`).
     * Unlike [purchase] / [setPrice] there is no `signingKeyId`:
     * `create_document_with_signer` selects an AUTHENTICATION + ECDSA key
     * satisfying the document type's security level from the wallet's
     * `IdentityManager`, so the key never crosses the FFI boundary.
     *
     * @param propertiesJson JSON object keyed by property name (byte-array
     *   fields as hex, identifier fields as base58); `"{}"` for a document
     *   type with no required properties.
     * @return the confirmed document's canonical JSON (now owned by
     *   [ownerId]; its 32-byte id is the `$id` field).
     */
    suspend fun create(
        walletHandle: Long,
        ownerId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        propertiesJson: String,
        signerHandle: Long,
    ): String = withContext(Dispatchers.IO) {
        require(ownerId.size == 32) { "ownerId must be 32 bytes" }
        require(contractId.size == 32) { "contractId must be 32 bytes" }
        mapNativeErrors {
            TransactionsNative.documentCreate(
                walletHandle,
                ownerId,
                contractId,
                documentType,
                propertiesJson,
                signerHandle,
            )
        }
    }
}
