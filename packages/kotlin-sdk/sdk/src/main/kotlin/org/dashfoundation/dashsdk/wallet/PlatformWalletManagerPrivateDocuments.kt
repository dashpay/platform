package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.PrivateDocumentCreateNative

// Creating an entry of any private document type (protocol version 15). Port of the
// "Creating an entry" part of
// `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PrivateDocuments.swift`
// (`PlatformWalletManager.createPrivateDocumentEntry`, `privateDocumentRecipientKeys`,
// `privateDocumentCreateFee`). They live on the manager, as in Swift, because the spend needs
// the manager's shielded coordinator.
//
// Which recipient keys qualify, which of the sender's keys seals, the schema check, the
// entry's capacity, the fee and the outcome are Rust's (`rs-platform-wallet`). These wrappers
// only marshal, and keep nothing.

/**
 * How a created entry tells its recipient who wrote it. A signed entry is not offered: which
 * keys may sign for a private type is not decided.
 *
 * @property ffiValue the byte `platform_wallet_manager_create_private_document_entry` takes.
 */
enum class PrivateDocumentSenderAuth(internal val ffiValue: Int) {
    /** The entry names no sender. */
    ANONYMOUS(0),

    /**
     * The entry names the sender's identity and encryption key, with a MAC only the sender and
     * the recipient can make.
     */
    DENIABLE_MAC(1),
}

/**
 * A key of an identity an entry of a private document type may be sealed to, as Rust decides
 * it (an enabled ECDSA_SECP256K1 DECRYPTION or ENCRYPTION key).
 */
data class PrivateDocumentRecipientKey(
    /** The key's id. */
    val keyId: Long,
    /** Its purpose as dpp numbers it: 1 ENCRYPTION, 2 DECRYPTION. */
    val purpose: Int,
) {
    /** The purpose's name, for display. */
    val purposeName: String
        get() = when (purpose) {
            1 -> "Encryption"
            2 -> "Decryption"
            else -> "Purpose $purpose"
        }

    internal companion object {
        /** The `[keyId, purpose]` pairs the JNI layer flattened, decoded. */
        fun fromFlat(flat: LongArray?): List<PrivateDocumentRecipientKey> {
            if (flat == null) {
                throw DashSdkError.SerializationError("privateDocumentRecipientKeys returned no answer")
            }
            if (flat.size % 2 != 0) {
                throw DashSdkError.SerializationError(
                    "privateDocumentRecipientKeys returned ${flat.size} values, not key and purpose pairs",
                )
            }
            return (flat.indices step 2).map {
                PrivateDocumentRecipientKey(keyId = flat[it], purpose = flat[it + 1].toInt())
            }
        }
    }
}

/** What a created entry is known by. */
class PrivateDocumentEntryCreated internal constructor(
    /**
     * The entry's commitment, 32 bytes: a read of the store reports the entry with it
     * ([PrivateDocumentEntry.commitment]).
     */
    val commitment: ByteArray,
    /**
     * The sender's key the entry was sealed with: its outgoing viewing key recovers the entry,
     * and a deniable MAC names it.
     */
    val senderKeyId: Long,
) {
    internal companion object {
        /** The 36 bytes the JNI layer wrote: the commitment, then the sender key id (u32, little-endian). */
        fun fromBytes(bytes: ByteArray?): PrivateDocumentEntryCreated {
            if (bytes == null || bytes.size != 36) {
                throw DashSdkError.SerializationError(
                    "createPrivateDocumentEntry returned ${bytes?.size ?: "no"} bytes, expected 36",
                )
            }
            var keyId = 0L
            for (i in 3 downTo 0) {
                keyId = (keyId shl 8) or (bytes[32 + i].toLong() and 0xFF)
            }
            return PrivateDocumentEntryCreated(commitment = bytes.copyOfRange(0, 32), senderKeyId = keyId)
        }
    }
}

/**
 * The keys of [identityId] (32 bytes), as the network proves the identity now, that an entry
 * of any private document type may be sealed to, by key id. An identity with no such key gives
 * none. Port of Swift's `PlatformWalletManager.privateDocumentRecipientKeys(identityId:)`.
 *
 * Throws [DashSdkError.NotFound] when the network holds no such identity.
 */
suspend fun PlatformWalletManager.privateDocumentRecipientKeys(
    identityId: ByteArray,
): List<PrivateDocumentRecipientKey> = teardownGate.op {
    require(identityId.size == 32) { "identityId must be exactly 32 bytes, got ${identityId.size}" }
    PrivateDocumentRecipientKey.fromFlat(
        mapNativeErrors { PrivateDocumentCreateNative.recipientKeys(managerHandle, identityId) },
    )
}

/**
 * The fee, in credits, a create of [entryCount] entries of the private type
 * [documentTypeName] of [contractId] reserves from the notes when it spends one or two of
 * them, computed in Rust at the network's protocol version with the type's entry size: the
 * least the spend pays. Port of Swift's `PlatformWalletManager.privateDocumentCreateFee`.
 *
 * Throws, with Rust's reason, for a type that is not private or an entry count one create
 * cannot append, and
 * [DashSdkError.PlatformWallet.ProtocolVersionNotActive] below protocol version 15.
 */
suspend fun PlatformWalletManager.privateDocumentCreateFee(
    contractId: ByteArray,
    documentTypeName: String,
    entryCount: Int = 1,
): Long = teardownGate.op {
    require(contractId.size == 32) { "contractId must be exactly 32 bytes, got ${contractId.size}" }
    mapNativeErrors {
        PrivateDocumentCreateNative.createFee(managerHandle, contractId, documentTypeName, entryCount)
    }
}

/**
 * Create one entry of the private type [documentTypeName] of [contractId] from the wallet's
 * identity [senderIdentityId] to the key [recipientKeyId] of [recipientIdentityId] (one of
 * [privateDocumentRecipientKeys]), paying for it from [account]'s shielded notes of
 * [walletId]: nothing on chain names either identity (type 29). Port of Swift's
 * `PlatformWalletManager.createPrivateDocumentEntry`. Ids are 32 bytes. Only on a shielded
 * build ([org.dashfoundation.dashsdk.Sdk.hasShielded]).
 *
 * [propertiesJson] is the entry's properties as a document create takes them (byte arrays as
 * hex, identifiers as base58); Rust checks them against the type's schema. The entry is sealed
 * with the sender's encryption key inside this manager's mnemonic resolver, which also
 * supplies the Orchard spend authority, so
 * [PrivateDocuments.readPrivateDocumentEntries] on the sender's wallet recovers it as sent.
 * Blocks for the Halo 2 proof.
 *
 * Throws, with Rust's reason, for what is refused before anything is paid (a sender without
 * an encryption key, a recipient key an entry cannot be sealed to, properties the schema
 * refuses, an identity neither the wallet nor the network holds),
 * [DashSdkError.PlatformWallet.ProtocolVersionNotActive] below protocol version 15, and
 * [DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed] once the entry may already be in the
 * store: do not create it again.
 */
@Suppress("LongParameterList")
suspend fun PlatformWalletManager.createPrivateDocumentEntry(
    walletId: ByteArray,
    account: Int = 0,
    contractId: ByteArray,
    documentTypeName: String,
    senderIdentityId: ByteArray,
    recipientIdentityId: ByteArray,
    recipientKeyId: Long,
    auth: PrivateDocumentSenderAuth,
    propertiesJson: String,
): PrivateDocumentEntryCreated = teardownGate.op {
    require(account >= 0) { "account must be non-negative, got $account" }
    for ((name, id) in listOf(
        "walletId" to walletId,
        "contractId" to contractId,
        "senderIdentityId" to senderIdentityId,
        "recipientIdentityId" to recipientIdentityId,
    )) {
        require(id.size == 32) { "$name must be exactly 32 bytes, got ${id.size}" }
    }
    PrivateDocumentEntryCreated.fromBytes(
        mapNativeErrors {
            PrivateDocumentCreateNative.createEntry(
                managerHandle,
                walletId,
                mnemonicResolverHandle,
                account,
                contractId,
                documentTypeName,
                senderIdentityId,
                recipientIdentityId,
                recipientKeyId,
                auth.ffiValue,
                propertiesJson,
            )
        },
    )
}
