package org.dashfoundation.dashsdk.dpns

import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.DpnsMarketplaceNative
import org.dashfoundation.dashsdk.wallet.TeardownGate
import org.dashfoundation.dashsdk.wallet.op
import org.json.JSONArray
import org.json.JSONObject

data class DpnsMarketplaceName(
    val documentId: ByteArray,
    val ownerId: ByteArray,
    val recordsIdentityId: ByteArray?,
    val label: String,
    val normalizedLabel: String,
    val priceCredits: ULong?,
    val createdAtMs: Long,
    val updatedAtMs: Long,
    val transferredAtMs: Long,
)

enum class DpnsNameSaleStatus(val rawValue: Int) {
    OWNED(0), SOLD(1), TRANSFERRED(2);

    companion object {
        fun fromRaw(raw: Int): DpnsNameSaleStatus =
            entries.firstOrNull { it.rawValue == raw }
                ?: throw IllegalArgumentException("unknown DPNS sale status $raw")
    }
}

data class DpnsNameState(
    val documentId: ByteArray,
    val walletIdentityId: ByteArray,
    val label: String,
    val normalizedLabel: String,
    val priceCredits: ULong?,
    val status: DpnsNameSaleStatus,
    val counterpartyId: ByteArray?,
    val createdAtMs: Long,
    val updatedAtMs: Long,
    val transferredAtMs: Long,
    val lastSyncedAtMs: Long,
)

enum class DpnsNameHistoryKind(val rawValue: Int) {
    REGISTERED(0), PRICE_SET(1), PURCHASED(2), TRANSFERRED(3);

    companion object {
        fun fromRaw(raw: Int): DpnsNameHistoryKind =
            entries.firstOrNull { it.rawValue == raw }
                ?: throw IllegalArgumentException("unknown DPNS history kind $raw")
    }
}

data class DpnsNameHistoryEvent(
    val kind: DpnsNameHistoryKind,
    val atMs: Long,
    val blockHeight: Long?,
    val priceCredits: ULong?,
    val fromId: ByteArray?,
    val toId: ByteArray?,
)

data class DpnsNameAdded(val identityId: ByteArray, val label: String)
data class DpnsNameDeparted(
    val identityId: ByteArray,
    val label: String,
    val documentId: ByteArray?,
    val status: DpnsNameSaleStatus?,
    val counterpartyId: ByteArray?,
)
data class DpnsPriceChange(
    val documentId: ByteArray,
    val label: String,
    val previousCredits: ULong?,
    val currentCredits: ULong?,
)
data class DpnsMarketplaceSyncSummary(
    val tracked: Int,
    val added: List<DpnsNameAdded>,
    val departed: List<DpnsNameDeparted>,
    val pricesChanged: List<DpnsPriceChange>,
    val syncUnixMs: Long,
)
data class DpnsManagerSyncSummary(
    val successCount: Int,
    val errorCount: Int,
    val syncUnixSeconds: Long,
)

/**
 * Typed Kotlin projection of the wallet-owned DPNS marketplace API.
 * Business decisions stay in Rust; this class validates fixed-width ids,
 * fences native-handle lifetimes, and decodes copied JNI results.
 */
class DpnsMarketplace internal constructor(
    private val gate: TeardownGate? = null,
) {
    suspend fun search(
        walletHandle: Long,
        prefix: String = "",
        limit: Int = 0,
        startAfter: ByteArray? = null,
    ): List<DpnsMarketplaceName> = gate.op {
        require(limit >= 0) { "limit must be non-negative" }
        require(startAfter == null || startAfter.size == 32) { "startAfter must be 32 bytes" }
        decodeNames(mapNativeErrors {
            DpnsMarketplaceNative.search(walletHandle, prefix, limit, startAfter)
        })
    }

    suspend fun nameState(walletHandle: Long, name: String): DpnsMarketplaceName? = gate.op {
        mapNativeErrors { DpnsMarketplaceNative.nameState(walletHandle, name) }?.let(::decodeName)
    }

    suspend fun myNames(walletHandle: Long, identityId: ByteArray? = null): List<DpnsNameState> = gate.op {
        require(identityId == null || identityId.size == 32) { "identityId must be 32 bytes" }
        decodeStates(mapNativeErrors { DpnsMarketplaceNative.myNames(walletHandle, identityId) })
    }

    suspend fun history(walletHandle: Long, name: String): List<DpnsNameHistoryEvent> = gate.op {
        decodeHistory(mapNativeErrors { DpnsMarketplaceNative.history(walletHandle, name) })
    }

    suspend fun setPrice(
        walletHandle: Long,
        ownerIdentityId: ByteArray,
        name: String,
        priceCredits: ULong,
        signerHandle: Long,
    ): DpnsMarketplaceName = trade(ownerIdentityId, signerHandle) {
        DpnsMarketplaceNative.setPrice(
            walletHandle, ownerIdentityId, name, priceCredits.toLong(), signerHandle,
        )
    }

    suspend fun delist(
        walletHandle: Long,
        ownerIdentityId: ByteArray,
        name: String,
        signerHandle: Long,
    ): DpnsMarketplaceName = trade(ownerIdentityId, signerHandle) {
        DpnsMarketplaceNative.delist(walletHandle, ownerIdentityId, name, signerHandle)
    }

    suspend fun transfer(
        walletHandle: Long,
        ownerIdentityId: ByteArray,
        name: String,
        recipientIdentityId: ByteArray,
        signerHandle: Long,
    ): DpnsMarketplaceName = trade(ownerIdentityId, signerHandle) {
        require(recipientIdentityId.size == 32) { "recipientIdentityId must be 32 bytes" }
        DpnsMarketplaceNative.transfer(
            walletHandle, ownerIdentityId, name, recipientIdentityId, signerHandle,
        )
    }

    suspend fun purchase(
        walletHandle: Long,
        purchaserIdentityId: ByteArray,
        name: String,
        expectedPriceCredits: ULong,
        signerHandle: Long,
    ): DpnsMarketplaceName = trade(purchaserIdentityId, signerHandle) {
        DpnsMarketplaceNative.purchase(
            walletHandle, purchaserIdentityId, name, expectedPriceCredits.toLong(), signerHandle,
        )
    }

    suspend fun sync(walletHandle: Long): DpnsMarketplaceSyncSummary = gate.op {
        decodeSyncSummary(mapNativeErrors { DpnsMarketplaceNative.sync(walletHandle) })
    }

    private suspend fun trade(
        identityId: ByteArray,
        signerHandle: Long,
        native: () -> String,
    ): DpnsMarketplaceName = gate.op {
        require(identityId.size == 32) { "identityId must be 32 bytes" }
        require(signerHandle != 0L) { "signerHandle must not be 0" }
        decodeName(mapNativeErrors(native))
    }

    internal companion object {
        fun decodeName(json: String): DpnsMarketplaceName = JSONObject(json).toMarketplaceName()
        fun decodeNames(json: String): List<DpnsMarketplaceName> {
            val values = JSONArray(json)
            return List(values.length()) { values.getJSONObject(it).toMarketplaceName() }
        }
        fun decodeStates(json: String): List<DpnsNameState> {
            val values = JSONArray(json)
            return List(values.length()) { values.getJSONObject(it).toState() }
        }
        fun decodeHistory(json: String): List<DpnsNameHistoryEvent> {
            val values = JSONArray(json)
            return List(values.length()) { values.getJSONObject(it).toHistory() }
        }
        fun decodeSyncSummary(json: String): DpnsMarketplaceSyncSummary =
            JSONObject(json).toSyncSummary()
    }
}

private fun JSONObject.optionalLong(name: String): Long? =
    if (isNull(name)) null else getLong(name)

/** Prices cross JSON as decimal strings so the full protocol u64 range is lossless. */
private fun JSONObject.optionalULong(name: String): ULong? =
    if (isNull(name)) null else get(name).toString().toULong()

private fun JSONObject.optionalId(name: String): ByteArray? =
    if (isNull(name)) null else getString(name).decodeHex32(name)

private fun JSONObject.toMarketplaceName() = DpnsMarketplaceName(
    documentId = getString("documentId").decodeHex32("documentId"),
    ownerId = getString("ownerId").decodeHex32("ownerId"),
    recordsIdentityId = optionalId("recordsIdentityId"),
    label = getString("label"),
    normalizedLabel = getString("normalizedLabel"),
    priceCredits = optionalULong("priceCredits"),
    createdAtMs = getLong("createdAtMs"),
    updatedAtMs = getLong("updatedAtMs"),
    transferredAtMs = getLong("transferredAtMs"),
)

private fun JSONObject.toState() = DpnsNameState(
    documentId = getString("documentId").decodeHex32("documentId"),
    walletIdentityId = getString("walletIdentityId").decodeHex32("walletIdentityId"),
    label = getString("label"),
    normalizedLabel = getString("normalizedLabel"),
    priceCredits = optionalULong("priceCredits"),
    status = DpnsNameSaleStatus.fromRaw(getInt("status")),
    counterpartyId = optionalId("counterpartyId"),
    createdAtMs = getLong("createdAtMs"),
    updatedAtMs = getLong("updatedAtMs"),
    transferredAtMs = getLong("transferredAtMs"),
    lastSyncedAtMs = getLong("lastSyncedAtMs"),
)

private fun JSONObject.toHistory() = DpnsNameHistoryEvent(
    kind = DpnsNameHistoryKind.fromRaw(getInt("kind")),
    atMs = getLong("atMs"),
    blockHeight = optionalLong("blockHeight"),
    priceCredits = optionalULong("priceCredits"),
    fromId = optionalId("fromId"),
    toId = optionalId("toId"),
)

private fun JSONObject.toSyncSummary(): DpnsMarketplaceSyncSummary {
    val addedJson = getJSONArray("added")
    val departedJson = getJSONArray("departed")
    val pricesJson = getJSONArray("pricesChanged")
    return DpnsMarketplaceSyncSummary(
        tracked = getInt("tracked"),
        added = List(addedJson.length()) { i -> addedJson.getJSONObject(i).let {
            DpnsNameAdded(it.getString("identityId").decodeHex32("identityId"), it.getString("label"))
        } },
        departed = List(departedJson.length()) { i -> departedJson.getJSONObject(i).let {
            DpnsNameDeparted(
                identityId = it.getString("identityId").decodeHex32("identityId"),
                label = it.getString("label"),
                documentId = it.optionalId("documentId"),
                status = if (it.isNull("status")) null else DpnsNameSaleStatus.fromRaw(it.getInt("status")),
                counterpartyId = it.optionalId("counterpartyId"),
            )
        } },
        pricesChanged = List(pricesJson.length()) { i -> pricesJson.getJSONObject(i).let {
            DpnsPriceChange(
                documentId = it.getString("documentId").decodeHex32("documentId"),
                label = it.getString("label"),
                previousCredits = it.optionalULong("previousCredits"),
                currentCredits = it.optionalULong("currentCredits"),
            )
        } },
        syncUnixMs = getLong("syncUnixMs"),
    )
}

private fun String.decodeHex32(field: String): ByteArray {
    require(length == 64) { "$field must contain 32 bytes" }
    return ByteArray(32) { index ->
        val hi = Character.digit(this[index * 2], 16)
        val lo = Character.digit(this[index * 2 + 1], 16)
        require(hi >= 0 && lo >= 0) { "$field is not hexadecimal" }
        ((hi shl 4) or lo).toByte()
    }
}
