package org.dashfoundation.dashsdk.wallet

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.Json
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.PrivateDocumentsNative

/**
 * Reading a private document type's entries (protocol version 15) — port of Swift's
 * `ManagedPlatformWallet.readPrivateDocumentEntries`
 * (`packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PrivateDocuments.swift`).
 *
 * A private type's store holds fixed-size encrypted entries that name nobody in the clear,
 * each sealed to one key of one identity and appended by an anonymous transition paid from
 * the shielded pool. A wallet finds its own by trying every entry with its identities' keys,
 * and recovers the ones its identities sealed with their outgoing viewing keys.
 * `rs-platform-wallet` reads a store that way and hands back what opened; which keys, which
 * identities, how a payload reads and where the next read starts are Rust's. This class only
 * marshals, and keeps nothing.
 *
 * Bound to one wallet's handle, like [org.dashfoundation.dashsdk.tokens.Dashpay]: get it from
 * [ManagedPlatformWallet.privateDocuments].
 */
class PrivateDocuments internal constructor(
    private val walletHandle: Long,
    private val gate: TeardownGate? = null,
) {

    /**
     * Read the store of the private document type [documentTypeName] of [contractId] (32
     * bytes) for every identity of this wallet, at most [maxPages] proved pages from the cursor
     * [from], or from the store's start when [from] is null: the entries sealed to them,
     * opened, and the entries they sealed, recovered from the store alone, each with its
     * payload read against the type's schema.
     *
     * The keys are derived inside [coreSignerHandle], the manager's `MnemonicResolverHandle`
     * (`PlatformWalletManager.mnemonicResolverHandle`), and stay there. The read keeps
     * nothing: keep the returned [PrivateDocumentEntriesRead.cursor] and pass it back,
     * unchanged, to read on. When the wallet's keys changed since that cursor the read starts
     * over and sets [PrivateDocumentEntriesRead.restarted]. A failed page fetch fails the read;
     * read again from the cursor you had. An entry's sender is what the entry claims
     * ([PrivateDocumentEntry.claimedSenderIdentityId]): whether that identity wrote it is not
     * checked here.
     *
     * Throws [DashSdkError.InvalidParameter] for a contract id that is not 32 bytes or a type
     * name holding a NUL, and the platform-wallet refusals: a cursor read from another store,
     * a mnemonic that is not this wallet's (before any page is read), a network below
     * protocol version 15 ([DashSdkError.PlatformWallet.ProtocolVersionNotActive]), and
     * the error the wallet reports when the contract declares no such private type.
     */
    suspend fun readPrivateDocumentEntries(
        contractId: ByteArray,
        documentTypeName: String,
        coreSignerHandle: Long,
        from: PrivateDocumentReadCursor? = null,
        maxPages: Int,
    ): PrivateDocumentEntriesRead = gate.op {
        val json = mapNativeErrors {
            PrivateDocumentsNative.readPrivateDocumentEntries(
                walletHandle = walletHandle,
                coreSignerHandle = coreSignerHandle,
                contractId = contractId,
                documentTypeName = documentTypeName,
                fromPosition = from?.position ?: 0L,
                fromProvedTotalCount = from?.provedTotalCount ?: 0L,
                fromHasProvedTotalCount = from?.provedTotalCount != null,
                fromKeySet = from?.keySet,
                fromStore = from?.store,
                maxPages = maxPages,
            )
        }
        PrivateDocumentEntriesRead.fromJson(json)
    }
}

/** Which way an entry went. */
enum class PrivateDocumentDirection {
    /** Sealed to a key of one of the wallet's identities, which opened it. */
    RECEIVED,

    /** Sealed by one of the wallet's identities, whose outgoing viewing key recovered it. */
    SENT,
}

/** How an entry tells its recipient who wrote it. */
enum class PrivateDocumentAuthMode {
    /** The entry names no sender. */
    ANONYMOUS,

    /**
     * The entry names a sender and carries a MAC only the sender and the recipient can make,
     * so it proves nothing to anyone else.
     */
    DENIABLE_MAC,

    /** The entry names a sender and carries a signature. */
    SIGNATURE,
}

/** What an entry's payload held, read against the type's schema. */
sealed class PrivateDocumentPayload {
    /**
     * The properties, decoded and checked against the type's schema, as a JSON object in the
     * encoding a document query returns: binary values as base64 strings, identifiers as
     * base58 strings.
     */
    data class Properties(val json: String) : PrivateDocumentPayload()

    /** Why the payload's properties were refused. */
    data class Refused(val reason: String) : PrivateDocumentPayload()
}

/**
 * An entry of a private document type's store one of the wallet's identities opened or
 * recovered. An entry from one of the wallet's identities to another is read twice, once each
 * way. Identifiers are 32 bytes.
 */
class PrivateDocumentEntry(
    /** The entry's position in the store. */
    val position: Long,
    /** The height of the block that appended it. */
    val blockHeight: Long,
    /** The time of the block that appended it, in milliseconds since 1970. */
    val blockTimeMs: Long,
    /** Which way it went. */
    val direction: PrivateDocumentDirection,
    /**
     * The wallet's identity that opened it, when received, or whose outgoing viewing key
     * recovered it, when sent.
     */
    val walletIdentityId: ByteArray,
    /**
     * That identity's key: the one the entry is sealed to, when received, or the one the
     * outgoing viewing key is derived from, when sent.
     */
    val walletKeyId: Long,
    /** The identity the entry is sealed to. */
    val recipientIdentityId: ByteArray,
    /** How the entry tells its recipient who wrote it. */
    val authMode: PrivateDocumentAuthMode,
    /**
     * The identity the entry names as its sender, null when it is anonymous. A claim: this
     * read does not check it against the sender identity's keys.
     */
    val claimedSenderIdentityId: ByteArray?,
    /** The key the entry names as its sender's, null when anonymous. A claim, as above. */
    val claimedSenderKeyId: Long?,
    /** The entry's commitment, 32 bytes: two entries with the same one are copies. */
    val commitment: ByteArray,
    /** The properties its payload holds, or why they are refused. */
    val payload: PrivateDocumentPayload,
)

/**
 * Where reading a private document type's store goes on from. Keep the one a read returned
 * and hand it back, as it came, as the next read's `from`: Rust decides from it where that
 * read starts.
 *
 * It carries the largest store size a page proved so far, so a page from a node behind does
 * not end the reading early, the fingerprint of the keys the reads used, so a read after the
 * wallet's keys changed starts over, and the fingerprint of the store read, so a read of
 * another contract or type refuses it.
 */
class PrivateDocumentReadCursor internal constructor(
    /** The position the next read starts at. */
    val position: Long,
    /**
     * The largest store size a page proved in the reads this cursor comes from; null while no
     * page was ever proved: the size is unknown, not 0.
     */
    val provedTotalCount: Long?,
    /** The fingerprint of the keys the reads scanned with, 32 bytes. Opaque: it only crosses back. */
    internal val keySet: ByteArray,
    /** The fingerprint of the store the reads read, 32 bytes. Opaque: it only crosses back. */
    internal val store: ByteArray,
)

/**
 * What one read of a private document type's store found for the wallet. Nothing of it is
 * kept anywhere: hold it in screen state.
 */
class PrivateDocumentEntriesRead internal constructor(
    /** The entries opened or recovered, in store order; at one position, the received one first. */
    val entries: List<PrivateDocumentEntry>,
    /** Where to read on from: pass it back as `from`. */
    val cursor: PrivateDocumentReadCursor,
    /**
     * The wallet's keys changed since the `from` cursor, so the read started over at the
     * store's start: its entries replace those of the earlier reads rather than follow them.
     */
    val restarted: Boolean,
    /** The store pages read. */
    val pagesRead: Long,
    /** Entries that decrypted under one of the wallet's keys but are refused. */
    val rejectedCount: Long,
    /**
     * The wallet's identities the store was read for: those with an HD index and a key an
     * entry can be sealed to. With none, nothing was read.
     */
    val identityCount: Long,
    /**
     * Keys of the wallet's identities an entry could be sealed to that the wallet does not
     * derive at their identity key slot, such as a DashPay Connect application key: the read
     * did not scan with them, so no entry sealed to them is in it.
     */
    val unscannedKeyCount: Long,
    /**
     * Whether the read reached the store's last entry, as large a store as any page proved,
     * with every key but the unscanned ones. `false` with [pagesRead] 0 means nothing was read,
     * and reading again changes nothing until that does.
     */
    val complete: Boolean,
) {
    internal companion object {
        private val json = Json { ignoreUnknownKeys = true }

        /**
         * The JSON `rs-unified-sdk-jni` wrote for one read, decoded. A direction or
         * authentication mode the JNI layer does not define, an entry with neither properties
         * nor a refusal, or an id that is not 32 bytes of hex is a [DashSdkError.SerializationError]:
         * the result is refused whole rather than shown in part.
         */
        fun fromJson(answer: String?): PrivateDocumentEntriesRead {
            if (answer == null) {
                throw DashSdkError.SerializationError("readPrivateDocumentEntries returned no answer")
            }
            val wire = try {
                json.decodeFromString<WireRead>(answer)
            } catch (e: SerializationException) {
                throw DashSdkError.SerializationError("private document read did not decode: ${e.message}", e)
            } catch (e: IllegalArgumentException) {
                throw DashSdkError.SerializationError("private document read did not decode: ${e.message}", e)
            }
            return PrivateDocumentEntriesRead(
                entries = wire.entries.map(WireEntry::toEntry),
                cursor = PrivateDocumentReadCursor(
                    position = wire.cursor.position,
                    provedTotalCount = wire.cursor.provedTotalCount,
                    keySet = id32(wire.cursor.keySet, "cursor key set"),
                    store = id32(wire.cursor.store, "cursor store"),
                ),
                restarted = wire.restarted,
                pagesRead = wire.pagesRead,
                rejectedCount = wire.rejectedCount,
                identityCount = wire.identityCount,
                unscannedKeyCount = wire.unscannedKeyCount,
                complete = wire.complete,
            )
        }

        /** 64 hex characters as their 32 bytes. */
        private fun id32(hex: String, field: String): ByteArray {
            if (hex.length != 64 || !hex.all { it in '0'..'9' || it in 'a'..'f' }) {
                throw DashSdkError.SerializationError("$field is not 32 bytes of hex: $hex")
            }
            return ByteArray(32) { hex.substring(it * 2, it * 2 + 2).toInt(16).toByte() }
        }
    }

    @Serializable
    private class WireCursor(
        val position: Long,
        @SerialName("proved_total_count") val provedTotalCount: Long? = null,
        @SerialName("key_set") val keySet: String,
        val store: String,
    )

    @Serializable
    private class WireEntry(
        val position: Long,
        @SerialName("block_height") val blockHeight: Long,
        @SerialName("block_time_ms") val blockTimeMs: Long,
        @SerialName("properties_json") val propertiesJson: String? = null,
        @SerialName("properties_refusal") val propertiesRefusal: String? = null,
        @SerialName("wallet_identity_id") val walletIdentityId: String,
        @SerialName("wallet_key_id") val walletKeyId: Long,
        @SerialName("recipient_identity_id") val recipientIdentityId: String,
        @SerialName("claimed_sender_identity_id") val claimedSenderIdentityId: String? = null,
        @SerialName("claimed_sender_key_id") val claimedSenderKeyId: Long? = null,
        val commitment: String,
        val direction: Int,
        @SerialName("auth_mode") val authMode: Int,
    ) {
        fun toEntry(): PrivateDocumentEntry = PrivateDocumentEntry(
            position = position,
            blockHeight = blockHeight,
            blockTimeMs = blockTimeMs,
            direction = when (direction) {
                0 -> PrivateDocumentDirection.RECEIVED
                1 -> PrivateDocumentDirection.SENT
                else -> throw DashSdkError.SerializationError(
                    "private document entry with an unknown direction $direction",
                )
            },
            walletIdentityId = id32(walletIdentityId, "wallet identity id"),
            walletKeyId = walletKeyId,
            recipientIdentityId = id32(recipientIdentityId, "recipient identity id"),
            authMode = when (authMode) {
                0 -> PrivateDocumentAuthMode.ANONYMOUS
                1 -> PrivateDocumentAuthMode.DENIABLE_MAC
                2 -> PrivateDocumentAuthMode.SIGNATURE
                else -> throw DashSdkError.SerializationError(
                    "private document entry with an unknown auth mode $authMode",
                )
            },
            claimedSenderIdentityId = claimedSenderIdentityId?.let {
                id32(it, "claimed sender identity id")
            },
            claimedSenderKeyId = claimedSenderKeyId,
            commitment = id32(commitment, "commitment"),
            payload = when {
                propertiesJson != null -> PrivateDocumentPayload.Properties(propertiesJson)
                propertiesRefusal != null -> PrivateDocumentPayload.Refused(propertiesRefusal)
                else -> throw DashSdkError.SerializationError(
                    "private document entry with neither properties nor a refusal",
                )
            },
        )
    }

    @Serializable
    private class WireRead(
        val entries: List<WireEntry>,
        val cursor: WireCursor,
        @SerialName("pages_read") val pagesRead: Long,
        @SerialName("rejected_count") val rejectedCount: Long,
        @SerialName("identity_count") val identityCount: Long,
        @SerialName("unscanned_key_count") val unscannedKeyCount: Long,
        val complete: Boolean,
        val restarted: Boolean,
    )
}
