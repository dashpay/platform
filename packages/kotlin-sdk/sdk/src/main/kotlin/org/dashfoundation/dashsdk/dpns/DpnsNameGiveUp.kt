package org.dashfoundation.dashsdk.dpns

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.json.JSONObject

/**
 * The way the identity holding a DPNS name gives it up, from Platform
 * protocol version 15.
 *
 * Port of `DpnsNameGiveUpMethod` in
 * `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/DpnsMarketplace.swift`.
 */
enum class DpnsNameGiveUpMethod(internal val ffiValue: Int) {
    /**
     * The name's `domain` document is deleted. The name is free again:
     * anybody, someone else included, may register it.
     */
    DELETE(0),

    /**
     * A `tombstone` document takes the name's place. The name is gone and
     * nobody can register it again. This can not be undone, by anybody, the
     * identity that created it included.
     */
    TOMBSTONE(1);

    internal companion object {
        /** The method an FFI byte names, or null for a byte this build does not know. */
        fun fromFfi(value: Int): DpnsNameGiveUpMethod? = entries.firstOrNull { it.ffiValue == value }
    }
}

/**
 * Which ways the network accepts for giving up one name a wallet identity
 * holds ([DpnsMarketplace.giveUpChoices]).
 *
 * The rule behind the two flags is the protocol's, applied in Rust: a
 * contested name can only be tombstoned, any other name either way. A host
 * offers exactly the ways whose flag is `true` and decides nothing itself.
 *
 * Port of `DpnsNameGiveUpChoices` in
 * `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/DpnsMarketplace.swift`.
 */
class DpnsNameGiveUpChoices(
    /** The name's `domain` document id (32 bytes). */
    val documentId: ByteArray,
    /** Whether the network accepts [DpnsNameGiveUpMethod.DELETE] for this name. */
    val canDelete: Boolean,
    /** Whether the network accepts [DpnsNameGiveUpMethod.TOMBSTONE] for this name. */
    val canTombstone: Boolean,
) {
    override fun equals(other: Any?): Boolean =
        other is DpnsNameGiveUpChoices &&
            documentId.contentEquals(other.documentId) &&
            canDelete == other.canDelete &&
            canTombstone == other.canTombstone

    override fun hashCode(): Int =
        (documentId.contentHashCode() * 31 + canDelete.hashCode()) * 31 + canTombstone.hashCode()

    override fun toString(): String =
        "DpnsNameGiveUpChoices(canDelete=$canDelete, canTombstone=$canTombstone)"
}

/**
 * A DPNS name given up, as the network confirmed it
 * ([DpnsMarketplace.giveUpName]).
 *
 * Port of `DpnsNameGivenUp` in
 * `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/DpnsMarketplace.swift`.
 */
class DpnsNameGivenUp(
    /**
     * The `domain` document checked before signing. A delete removed exactly
     * this one. A tombstone consumed the owner's domain of the name at
     * execution, which is this one unless the name was deleted and
     * registered again in the moments between.
     */
    val documentId: ByteArray,
    /** How the name was given up. */
    val method: DpnsNameGiveUpMethod,
    /** The id of the tombstone the network stored; set exactly for [DpnsNameGiveUpMethod.TOMBSTONE]. */
    val tombstoneId: ByteArray?,
) {
    override fun equals(other: Any?): Boolean =
        other is DpnsNameGivenUp &&
            documentId.contentEquals(other.documentId) &&
            method == other.method &&
            (tombstoneId?.contentEquals(other.tombstoneId) ?: (other.tombstoneId == null))

    override fun hashCode(): Int =
        (documentId.contentHashCode() * 31 + method.hashCode()) * 31 +
            (tombstoneId?.contentHashCode() ?: 0)

    override fun toString(): String = "DpnsNameGivenUp(method=$method, tombstone=${tombstoneId != null})"
}

internal object DpnsNameGiveUpCodec {
    fun decodeChoices(json: String): DpnsNameGiveUpChoices {
        val value = JSONObject(json)
        return DpnsNameGiveUpChoices(
            documentId = value.getString("documentId").hex32("documentId"),
            canDelete = value.getBoolean("canDelete"),
            canTombstone = value.getBoolean("canTombstone"),
        )
    }

    /**
     * Copy a confirmed give-up into an owned value. A method this build does
     * not know, or a tombstone id that does not match the method (a
     * tombstone without one, a delete with one), fails closed with
     * [DashSdkError.PlatformWallet.WalletOperation], as Swift's
     * `DpnsNameGivenUp(ffi:)` returning nil does: the result is not presented
     * as something this build can not establish.
     */
    fun decodeGivenUp(json: String): DpnsNameGivenUp {
        val value = JSONObject(json)
        val method = DpnsNameGiveUpMethod.fromFfi(value.getInt("method"))
        val tombstoneId =
            if (value.isNull("tombstoneId")) null
            else value.getString("tombstoneId").hex32("tombstoneId")
        val consistent = when (method) {
            DpnsNameGiveUpMethod.DELETE -> tombstoneId == null
            DpnsNameGiveUpMethod.TOMBSTONE -> tombstoneId != null
            null -> false
        }
        if (method == null || !consistent) {
            throw DashSdkError.PlatformWallet.WalletOperation(
                "dpns_give_up_name reported success but returned an unreadable result",
            )
        }
        return DpnsNameGivenUp(
            documentId = value.getString("documentId").hex32("documentId"),
            method = method,
            tombstoneId = tombstoneId,
        )
    }

    private fun String.hex32(field: String): ByteArray {
        require(length == 64) { "$field must contain 32 bytes" }
        return ByteArray(32) { index ->
            val hi = Character.digit(this[index * 2], 16)
            val lo = Character.digit(this[index * 2 + 1], 16)
            require(hi >= 0 && lo >= 0) { "$field is not hexadecimal" }
            ((hi shl 4) or lo).toByte()
        }
    }
}
