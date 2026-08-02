package org.dashfoundation.dashsdk.documents

import org.dashfoundation.dashsdk.wallet.op
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
class DocumentTransactions internal constructor(
    private val gate: org.dashfoundation.dashsdk.wallet.TeardownGate? = null,
) {

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
    ): String = gate.op {
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
    ): String = gate.op {
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
    ): String = gate.op {
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

    /**
     * Replace + broadcast the full property set of [documentId] on
     * [contractId]'s [documentType], owned by [ownerId] — signed via
     * [signerHandle] with key [signingKeyId]. Mirrors Swift
     * `ManagedPlatformWallet.replaceDocument`. The document revision is
     * bumped on the Rust side; the caller does not pass a revision.
     *
     * @param propertiesJson the FULL replacement property object (byte-array
     *   fields as hex, identifier fields as base58), same encoding as
     *   [create].
     * @return the confirmed document's canonical JSON (its 32-byte id is the
     *   `$id` field).
     */
    suspend fun replace(
        walletHandle: Long,
        ownerId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        documentId: ByteArray,
        propertiesJson: String,
        signingKeyId: Int,
        signerHandle: Long,
    ): String = gate.op {
        require(ownerId.size == 32) { "ownerId must be 32 bytes" }
        require(contractId.size == 32) { "contractId must be 32 bytes" }
        require(documentId.size == 32) { "documentId must be 32 bytes" }
        require(signingKeyId >= 0) { "signingKeyId must be non-negative, got $signingKeyId" }
        mapNativeErrors {
            TransactionsNative.documentReplace(
                walletHandle,
                ownerId,
                contractId,
                documentType,
                documentId,
                propertiesJson,
                signingKeyId,
                signerHandle,
            )
        }
    }

    /**
     * Delete + broadcast [documentId] on [contractId]'s [documentType],
     * owned by [ownerId] — signed via [signerHandle] with key [signingKeyId].
     * Mirrors Swift `ManagedPlatformWallet.deleteDocument`. Delete returns no
     * document body.
     *
     * @return the deleted document's 32-byte id, for confirmation.
     */
    suspend fun delete(
        walletHandle: Long,
        ownerId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        documentId: ByteArray,
        signingKeyId: Int,
        signerHandle: Long,
    ): ByteArray = gate.op {
        require(ownerId.size == 32) { "ownerId must be 32 bytes" }
        require(contractId.size == 32) { "contractId must be 32 bytes" }
        require(documentId.size == 32) { "documentId must be 32 bytes" }
        require(signingKeyId >= 0) { "signingKeyId must be non-negative, got $signingKeyId" }
        mapNativeErrors {
            TransactionsNative.documentDelete(
                walletHandle,
                ownerId,
                contractId,
                documentType,
                documentId,
                signingKeyId,
                signerHandle,
            )
        }
    }

    /**
     * Transfer + broadcast [documentId] on [contractId]'s [documentType],
     * from [ownerId] to [recipientId] — signed via [signerHandle] with key
     * [signingKeyId]. Mirrors Swift `ManagedPlatformWallet.transferDocument`.
     * Only valid for document types whose schema is `transferable`; the
     * caller's UI gates against non-transferable types and self-transfer.
     *
     * @return the confirmed document's canonical JSON, now reflecting the
     *   new owner (its 32-byte id is the `$id` field).
     */
    suspend fun transfer(
        walletHandle: Long,
        ownerId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        documentId: ByteArray,
        recipientId: ByteArray,
        signingKeyId: Int,
        signerHandle: Long,
    ): String = gate.op {
        require(ownerId.size == 32) { "ownerId must be 32 bytes" }
        require(contractId.size == 32) { "contractId must be 32 bytes" }
        require(documentId.size == 32) { "documentId must be 32 bytes" }
        require(recipientId.size == 32) { "recipientId must be 32 bytes" }
        require(signingKeyId >= 0) { "signingKeyId must be non-negative, got $signingKeyId" }
        mapNativeErrors {
            TransactionsNative.documentTransfer(
                walletHandle,
                ownerId,
                contractId,
                documentType,
                documentId,
                recipientId,
                signingKeyId,
                signerHandle,
            )
        }
    }

    /**
     * Create + broadcast an ENCRYPTED wallet-contract document (the wire-
     * compatible `txMetadata` shape) on [contractId]'s [documentType], owned by
     * [ownerId] — signed via [signerHandle]. Implements the create half of the
     * legacy `BlockchainIdentity.publishTxMetaData` retirement: the SDK derives
     * the identity encryption key, seals [payload] into the legacy
     * `version ‖ IV ‖ AES-256-CBC` blob, and writes
     * `{keyIndex, encryptionKeyIndex, encryptedMetadata}`.
     *
     * Batching stays app-side: the caller serializes its items into [payload]
     * (a protobuf `TxMetadataBatch`). The identity encryption key id (the
     * `keyIndex` field) is chosen SDK-side to match the legacy stack, so the key
     * never crosses the FFI boundary.
     *
     * ### `encryptionKeyIndex` allocation
     * Leave [encryptionKeyIndex] `null` (the default) to let the SDK allocate
     * the per-document index in Rust from authoritative Platform state — the
     * host-thin path. Rust counts the identity's existing documents of this
     * contract and document type on Platform and uses `1 + count`, the same
     * series the legacy stack produced, serialized in process so concurrent
     * creates never pick the same index. The index is best-effort unique PER
     * DEVICE; a cross-device duplicate is not data loss, because each document
     * stores its own index and the reader derives that document's key from it,
     * so both decrypt independently. It follows that the index is an
     * encryption-key selector, NOT a document sequence number: do not order,
     * count, address, or gap-check documents by it.
     *
     * Passing an explicit non-negative [encryptionKeyIndex] is retained ONLY for
     * migration / tests and is discouraged: a caller-supplied counter can
     * collide across concurrent callers and devices, and choosing the index is
     * the SDK's job.
     *
     * ### What is not scrubbed
     * The SDK zeroizes the native copies it makes of [payload]. It cannot
     * scrub [payload] itself: that is a JVM `ByteArray` the caller owns, as are
     * any buffers that produced it, and the runtime may have copied it while
     * compacting the heap. Treat it and every JVM copy as plaintext-equivalent
     * for as long as they are reachable: keep them short-lived, never log them,
     * and overwrite your own array once this call returns where that is
     * feasible. Overwriting the array you hold does not reach any copy the
     * runtime made of it, so this reduces exposure rather than eliminating it.
     *
     * @param encryptionKeyIndex `null` to let the SDK allocate the index
     *   (preferred); or an explicit non-negative per-document index
     *   (migration / tests only).
     * @param version payload version byte. Which values are meaningful is
     *   decided by the wallet core; an unsupported one is rejected there and
     *   surfaced as a platform-wallet invalid-parameter error.
     * @param payload already-serialized opaque plaintext; the SDK does not
     *   parse it.
     * [mnemonicResolverHandle] is the host mnemonic-resolver handle
     * ([org.dashfoundation.dashsdk.wallet.PlatformWalletManager.mnemonicResolverHandle]):
     * required for external-signable wallets (the app's shape — the AES key
     * derives on demand through the resolver), ignored for wallets with
     * resident private keys.
     *
     * @return the confirmed document's canonical JSON (its 32-byte id is the
     *   base58 `$id` field).
     */
    suspend fun createEncryptedDocument(
        walletHandle: Long,
        mnemonicResolverHandle: Long,
        ownerId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        version: Int,
        payload: ByteArray,
        signerHandle: Long,
        encryptionKeyIndex: Int? = null,
    ): String = gate.op {
        require(ownerId.size == 32) { "ownerId must be 32 bytes" }
        require(contractId.size == 32) { "contractId must be 32 bytes" }
        require(encryptionKeyIndex == null || encryptionKeyIndex >= 0) {
            "encryptionKeyIndex, when supplied, must be non-negative, got $encryptionKeyIndex"
        }
        mapNativeErrors {
            TransactionsNative.documentCreateEncrypted(
                walletHandle,
                mnemonicResolverHandle,
                ownerId,
                contractId,
                documentType,
                // -1 is the JNI sentinel for "let Rust allocate the index".
                encryptionKeyIndex ?: -1,
                version,
                payload,
                signerHandle,
            )
        }
    }

    /**
     * Fetch + DECRYPT every encrypted wallet-contract document owned by
     * [ownerId] on [contractId]'s [documentType] updated at or after [sinceMs]
     * (epoch-millis). Implements the read half of the legacy
     * `BlockchainIdentity.getTxMetaData(since, key)` retirement: the SDK fetches
     * the owner-scoped, since-timestamp documents and decrypts each with the
     * identity's derived key. Documents that fail to decrypt, and documents
     * carrying an unsupported wire version, are skipped Rust-side — a bad
     * document never aborts the fetch.
     *
     * ### Decryption is not authentication
     * The envelope is AES-256-CBC with PKCS7 and carries no integrity tag, so a
     * successful decrypt does not mean the bytes are genuine. A wrong key or a
     * modified ciphertext usually fails the unpad and is skipped, but PKCS7
     * accepts a wrong plaintext often enough that an element can carry opaque
     * garbage. Parse every `payload` strictly — CBOR for `version` 0, protobuf
     * for 1 — and discard anything that does not parse, rather than trusting it
     * because it appeared in the array.
     *
     * @return a JSON array; each element is `{ "id", "ownerId" (base58),
     *   "keyIndex", "encryptionKeyIndex", "version", "updatedAt" (number|null),
     *   "payload" (base64 of the decrypted opaque plaintext) }`. The caller
     *   parses each `payload` itself and MUST dispatch on `version`: `0` is a
     *   CBOR payload, `1` a protobuf `TxMetadataBatch`. Those are the only
     *   versions the legacy format defines; a document carrying anything else
     *   is skipped by the SDK and never reaches this array. Reconcile memo /
     *   taxCategory / exchangeRate / service / giftCard fields into the local
     *   store from the parsed payload.
     *
     * ### What is not scrubbed
     * The SDK zeroizes the native decrypted-payload and JSON buffers it owns.
     * It cannot scrub the returned `String`: that is a JVM object the runtime
     * manages, as are every copy of it and every object parsed out of it, and
     * the runtime may have copied it while compacting the heap. Treat it and
     * everything derived from it as plaintext-equivalent for as long as they
     * are reachable: parse promptly, never log them, and do not retain or
     * persist them longer than required. Unlike a `ByteArray` there is no
     * overwrite to attempt here at all, so short retention is the only control
     * the caller has.
     *
     * [mnemonicResolverHandle] is the host mnemonic-resolver handle
     * ([org.dashfoundation.dashsdk.wallet.PlatformWalletManager.mnemonicResolverHandle]):
     * required for external-signable wallets (the app's shape — the AES key
     * derives on demand through the resolver), ignored for wallets with
     * resident private keys.
     */
    suspend fun fetchEncryptedDocuments(
        walletHandle: Long,
        mnemonicResolverHandle: Long,
        ownerId: ByteArray,
        contractId: ByteArray,
        documentType: String,
        sinceMs: Long,
    ): String = gate.op {
        require(ownerId.size == 32) { "ownerId must be 32 bytes" }
        require(contractId.size == 32) { "contractId must be 32 bytes" }
        require(sinceMs >= 0) { "sinceMs must be non-negative, got $sinceMs" }
        mapNativeErrors {
            TransactionsNative.documentFetchEncrypted(
                walletHandle,
                mnemonicResolverHandle,
                ownerId,
                contractId,
                documentType,
                sinceMs,
            )
        }
    }
}
