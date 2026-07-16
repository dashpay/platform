package org.dashfoundation.example.ui.storage

import android.database.Cursor
import androidx.sqlite.db.SimpleSQLiteQuery
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.UInt64Value
import org.dashfoundation.dashsdk.persistence.dao.StorageCountsDao
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.formatDate
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.truncateMiddle
import java.util.Date

/**
 * Registry backing the Storage Explorer — the Android port of
 * `StorageExplorerView.swift` / `StorageModelListViews.swift` /
 * `StorageRecordDetailViews.swift`.
 *
 * iOS renders one hand-written SwiftUI list + detail view per SwiftData
 * model. Here a single generic list/detail pair renders every table
 * through this registry: each entry supplies the table name, sort order,
 * optional network scoping column, and a small per-model row renderer
 * (headline/subtitle), while the detail screen shows every column as a
 * label/value row. Rows load via raw `SELECT rowid, *` queries so no
 * per-entity DAO surface is required.
 */

/** One raw table row: SQLite rowid + column name → value. */
data class StorageRow(
    val rowId: Long,
    val columns: Map<String, Any?>,
) {
    fun text(name: String): String? = columns[name] as? String
    fun long(name: String): Long? = (columns[name] as? Long)
    fun blob(name: String): ByteArray? = columns[name] as? ByteArray
    fun bool(name: String): Boolean = (long(name) ?: 0L) != 0L
}

class StorageModel(
    /** Stable route key. */
    val name: String,
    /** List/section title (iOS display name). */
    val displayName: String,
    val tableName: String,
    /** ORDER BY clause body (without the keywords). */
    val orderBy: String,
    /** Column carrying `Network.ffiValue` when the table is network-scoped. */
    val networkColumn: String? = null,
    val countFlow: (StorageCountsDao) -> Flow<Long>,
    val headline: (StorageRow) -> String,
    val subtitle: (StorageRow) -> String? = { null },
)

private fun ByteArray.shortHex(bytes: Int = 8): String =
    copyOfRange(0, minOf(bytes, size)).toHex()

private fun StorageRow.base58(name: String): String? = blob(name)?.let { Base58.encode(it) }

private fun StorageRow.dateText(name: String): String? =
    long(name)?.let { formatDate(Date(it)) }

/** Explorer order matches `StorageExplorerView.swift`'s row order. */
val STORAGE_MODELS: List<StorageModel> = listOf(
    StorageModel(
        name = "identities", displayName = "Identities", tableName = "identities",
        orderBy = "lastUpdated DESC", networkColumn = "networkRaw",
        countFlow = { it.countIdentities() },
        headline = { row ->
            row.text("dpnsName") ?: row.text("alias") ?: row.base58("identityId") ?: "(identity)"
        },
        subtitle = { row -> "${row.long("balance") ?: 0} credits" },
    ),
    StorageModel(
        name = "dpnsNames", displayName = "DPNS Names", tableName = "dpns_names",
        orderBy = "acquiredAt DESC", networkColumn = "networkRaw",
        countFlow = { it.countDpnsNames() },
        headline = { row ->
            "${row.text("label") ?: "?"}.${row.text("parentDomainName") ?: "dash"}"
        },
        subtitle = { row -> row.base58("identityId")?.let { truncateMiddle(it) } },
    ),
    StorageModel(
        name = "dashpayProfiles", displayName = "DashPay Profiles",
        tableName = "dashpay_profiles",
        orderBy = "lastUpdated DESC", networkColumn = "networkRaw",
        countFlow = { it.countDashpayProfiles() },
        headline = { row -> row.text("displayName") ?: "(no display name)" },
        subtitle = { row -> row.base58("identityId")?.let { truncateMiddle(it) } },
    ),
    StorageModel(
        name = "contactRequests", displayName = "Contact Requests",
        tableName = "dashpay_contact_requests",
        orderBy = "createdAtMillis DESC", networkColumn = "networkRaw",
        countFlow = { it.countDashpayContactRequests() },
        headline = { row -> row.blob("contactIdentityId")?.shortHex() ?: "(contact)" },
        subtitle = { row ->
            val direction = if (row.bool("isOutgoing")) "outgoing" else "incoming"
            "$direction · from ${row.blob("ownerIdentityId")?.shortHex()}"
        },
    ),
    StorageModel(
        name = "documents", displayName = "Documents", tableName = "documents",
        orderBy = "localUpdatedAt DESC", networkColumn = "networkRaw",
        countFlow = { it.countDocuments() },
        headline = { row -> row.text("documentType") ?: "(document)" },
        subtitle = { row -> row.text("documentId")?.let { truncateMiddle(it) } },
    ),
    StorageModel(
        name = "dataContracts", displayName = "Data Contracts", tableName = "data_contracts",
        orderBy = "lastAccessedAt DESC", networkColumn = "networkRaw",
        countFlow = { it.countDataContracts() },
        headline = { row -> row.text("name") ?: "(contract)" },
        subtitle = { row -> row.base58("id")?.let { truncateMiddle(it) } },
    ),
    StorageModel(
        name = "publicKeys", displayName = "Public Keys", tableName = "public_keys",
        orderBy = "identityId, keyId",
        countFlow = { it.countPublicKeys() },
        headline = { row -> "Key ${row.long("keyId") ?: 0}" },
        subtitle = { row -> "${row.text("purpose")} / ${row.text("securityLevel")}" },
    ),
    StorageModel(
        name = "tokens", displayName = "Tokens", tableName = "tokens",
        orderBy = "name",
        countFlow = { it.countTokens() },
        headline = { row -> row.text("name") ?: "(token)" },
        subtitle = { row -> "base supply ${row.text("baseSupply") ?: "?"}" },
    ),
    StorageModel(
        name = "tokenBalances", displayName = "Token Balances", tableName = "token_balances",
        orderBy = "lastUpdated DESC", networkColumn = "networkRaw",
        countFlow = { it.countTokenBalances() },
        headline = { row -> row.text("tokenName") ?: row.text("tokenId") ?: "(balance)" },
        subtitle = { row ->
            row.blob("balance")
                ?.let(UInt64Value::fromBigEndianBytes)
                ?.value
                ?.toString() ?: "?"
        },
    ),
    StorageModel(
        name = "tokenHistory", displayName = "Token History",
        tableName = "token_history_events",
        orderBy = "createdAt DESC",
        countFlow = { it.countTokenHistoryEvents() },
        headline = { row -> row.text("eventType") ?: "(event)" },
        subtitle = { row -> row.dateText("eventTimestamp") },
    ),
    StorageModel(
        name = "documentTypes", displayName = "Document Types", tableName = "document_types",
        orderBy = "name",
        countFlow = { it.countDocumentTypes() },
        headline = { row -> row.text("name") ?: "(type)" },
        subtitle = { row -> row.base58("contractId")?.let { truncateMiddle(it) } },
    ),
    StorageModel(
        name = "indices", displayName = "Indices", tableName = "indices",
        orderBy = "name",
        countFlow = { it.countIndices() },
        headline = { row -> row.text("name") ?: "(index)" },
        subtitle = { row -> row.text("documentTypeName") },
    ),
    StorageModel(
        name = "properties", displayName = "Properties", tableName = "properties",
        orderBy = "name",
        countFlow = { it.countProperties() },
        headline = { row -> row.text("name") ?: "(property)" },
        subtitle = { row -> row.text("type") },
    ),
    StorageModel(
        name = "keywords", displayName = "Keywords", tableName = "keywords",
        orderBy = "keyword",
        countFlow = { it.countKeywords() },
        headline = { row -> row.text("keyword") ?: "(keyword)" },
    ),
    StorageModel(
        name = "platformAddresses", displayName = "Platform Addresses",
        tableName = "platform_addresses",
        orderBy = "accountIndex, addressIndex",
        countFlow = { it.countPlatformAddresses() },
        headline = { row -> truncateMiddle(row.text("address") ?: "(address)", 14, 8) },
        subtitle = { row ->
            buildString {
                append("#${row.long("addressIndex") ?: 0}")
                if (row.bool("isUsed")) append(" • used")
                row.long("balance")?.takeIf { it > 0 }?.let { append(" • $it") }
            }
        },
    ),
    StorageModel(
        name = "platformAddressesSyncStates",
        displayName = "Platform Addresses Sync State",
        tableName = "platform_addresses_sync_states",
        orderBy = "lastUpdated DESC", networkColumn = "networkRaw",
        countFlow = { it.countPlatformAddressesSyncStates() },
        headline = { row ->
            networkDisplayName(row.long("networkRaw"))
        },
        subtitle = { row -> "Height ${row.long("syncHeight") ?: 0}" },
    ),
    StorageModel(
        name = "wallets", displayName = "Wallets", tableName = "wallets",
        orderBy = "lastUpdated DESC", networkColumn = "networkRaw",
        countFlow = { it.countWallets() },
        headline = { row ->
            row.text("name") ?: row.blob("walletId")?.shortHex() ?: "(wallet)"
        },
        subtitle = { row ->
            "${networkDisplayName(row.long("networkRaw"))} · height ${row.long("syncedHeight") ?: 0}"
        },
    ),
    StorageModel(
        name = "accounts", displayName = "Accounts", tableName = "accounts",
        orderBy = "walletId, accountType, accountIndex",
        countFlow = { it.countAccounts() },
        headline = { row -> row.text("accountTypeName") ?: "(account)" },
        subtitle = { row ->
            "Index ${row.long("accountIndex") ?: 0} · wallet ${row.blob("walletId")?.shortHex(4)}"
        },
    ),
    StorageModel(
        name = "coreAddresses", displayName = "Core Addresses", tableName = "core_addresses",
        orderBy = "poolTypeTag, addressIndex",
        countFlow = { it.countCoreAddresses() },
        headline = { row -> truncateMiddle(row.text("address") ?: "(address)", 14, 8) },
        subtitle = { row ->
            buildString {
                append("pool ${row.long("poolTypeTag") ?: 0} • #${row.long("addressIndex") ?: 0}")
                if (row.bool("isUsed")) append(" • used")
                row.long("balance")?.takeIf { it > 0 }?.let { append(" • $it") }
            }
        },
    ),
    StorageModel(
        name = "transactions", displayName = "Transactions", tableName = "transactions",
        orderBy = "firstSeen DESC",
        countFlow = { it.countTransactions() },
        headline = { row ->
            row.blob("txid")?.let { truncateMiddle(it.toHex(), 10, 8) } ?: "(tx)"
        },
        subtitle = { row ->
            "${row.text("transactionType") ?: "Standard"} · net ${row.long("netAmount") ?: 0}"
        },
    ),
    StorageModel(
        name = "transactionAccountInvolvements",
        displayName = "Transaction Account Involvements",
        tableName = "transaction_account_involvements",
        orderBy = "accountId",
        countFlow = { it.countTransactionAccountInvolvements() },
        headline = { row ->
            row.blob("transactionTxid")?.let { truncateMiddle(it.toHex(), 10, 8) } ?: "(tx)"
        },
        subtitle = { row -> "account ${row.long("accountId") ?: 0}" },
    ),
    StorageModel(
        name = "txos", displayName = "TXOs", tableName = "txos",
        orderBy = "height DESC",
        countFlow = { it.countTxos() },
        headline = { row ->
            row.blob("outpoint")?.let { truncateMiddle(it.toHex(), 10, 8) } ?: "(txo)"
        },
        subtitle = { row ->
            buildString {
                append("${row.long("amount") ?: 0} duffs")
                val height = row.long("height") ?: 0
                append(if (height == 0L) " · mempool" else " · h $height")
                append(if (row.bool("isSpent")) " · spent" else " · unspent")
            }
        },
    ),
    StorageModel(
        name = "pendingInputs", displayName = "Pending Inputs", tableName = "pending_inputs",
        orderBy = "createdAt DESC",
        countFlow = { it.countPendingInputs() },
        headline = { row ->
            row.blob("outpoint")?.let { truncateMiddle(it.toHex(), 10, 8) } ?: "(input)"
        },
        subtitle = { row -> "input ${row.long("inputIndex") ?: 0}" },
    ),
    StorageModel(
        name = "assetLocks", displayName = "Asset Locks", tableName = "asset_locks",
        orderBy = "updatedAt DESC",
        countFlow = { it.countAssetLocks() },
        headline = { row -> truncateMiddle(row.text("outPointHex") ?: "(lock)", 10, 8) },
        subtitle = { row ->
            "status ${row.long("statusRaw") ?: 0} · identity #${row.long("identityIndexRaw") ?: 0}"
        },
    ),
    StorageModel(
        name = "walletManagerMetadata", displayName = "Manager Metadata",
        tableName = "wallet_manager_metadata",
        orderBy = "lastUpdated DESC", networkColumn = "networkRaw",
        countFlow = { it.countWalletManagerMetadata() },
        headline = { row -> networkDisplayName(row.long("networkRaw")) },
        subtitle = { row ->
            "Height ${row.long("combinedSyncHeight") ?: 0} · " +
                "${row.long("walletCount") ?: 0} wallets"
        },
    ),
    StorageModel(
        name = "shieldedNotes", displayName = "Shielded Notes", tableName = "shielded_notes",
        orderBy = "blockHeight DESC, position",
        countFlow = { it.countShieldedNotes() },
        headline = { row -> "${row.long("value") ?: 0} credits" },
        subtitle = { row ->
            buildString {
                append("acct ${row.long("accountIndex") ?: 0}")
                append(" · pos ${row.long("position") ?: 0}")
                append(" · h ${row.long("blockHeight") ?: 0}")
                append(if (row.bool("isSpent")) " · spent" else " · unspent")
            }
        },
    ),
    StorageModel(
        name = "shieldedOutgoingNotes", displayName = "Shielded Sent Notes",
        tableName = "shielded_outgoing_notes",
        orderBy = "blockHeight DESC, accountIndex",
        countFlow = { it.countShieldedOutgoingNotes() },
        headline = { row -> "${row.long("value") ?: 0} credits" },
        subtitle = { row ->
            "acct ${row.long("accountIndex") ?: 0} · h ${row.long("blockHeight") ?: 0}"
        },
    ),
    StorageModel(
        name = "shieldedSyncStates", displayName = "Shielded Sync State",
        tableName = "shielded_sync_states",
        orderBy = "accountIndex",
        countFlow = { it.countShieldedSyncStates() },
        headline = { row -> "Wallet ${row.blob("walletId")?.shortHex(4) ?: "?"}" },
        subtitle = { row ->
            "acct ${row.long("accountIndex") ?: 0} · " +
                "synced index ${row.long("lastSyncedIndex") ?: 0}"
        },
    ),
    StorageModel(
        name = "shieldedViewingKeys", displayName = "Shielded Viewing Keys",
        tableName = "shielded_viewing_keys", orderBy = "accountIndex",
        countFlow = { it.countShieldedViewingKeys() },
        headline = { row -> "Wallet ${row.blob("walletId")?.shortHex(4) ?: "?"}" },
        subtitle = { row ->
            "acct ${row.long("accountIndex") ?: 0} · " +
                "${row.blob("fvkBytes")?.size ?: 0}-byte FVK"
        },
    ),
    StorageModel(
        name = "shieldedActivities", displayName = "Shielded Activity",
        tableName = "shielded_activities",
        orderBy = "blockHeight DESC, accountIndex",
        countFlow = { it.countShieldedActivities() },
        headline = { row -> "${row.long("amount") ?: 0} credits" },
        subtitle = { row ->
            "kind ${row.long("kindTag") ?: 0} · status ${row.long("status") ?: 0} · " +
                "h ${row.long("blockHeight") ?: 0}"
        },
    ),
)

fun storageModel(name: String): StorageModel? = STORAGE_MODELS.firstOrNull { it.name == name }

private fun networkDisplayName(raw: Long?): String =
    raw?.let { value ->
        Network.entries.firstOrNull { it.ffiValue == value.toInt() }?.displayName
    } ?: "Unknown"

/**
 * Load rows for [model] via raw SQL, network-scoped when the table carries
 * a network column (matching the per-network `@Query` predicates the iOS
 * list views use).
 */
suspend fun loadStorageRows(
    database: DashDatabase,
    model: StorageModel,
    network: Network,
): List<StorageRow> = withContext(Dispatchers.IO) {
    val where = model.networkColumn?.let { "WHERE $it = ${network.ffiValue} " } ?: ""
    val sql = "SELECT rowid AS _storage_rowid_, * FROM ${model.tableName} " +
        where + "ORDER BY ${model.orderBy}"
    database.query(SimpleSQLiteQuery(sql)).use { cursor ->
        buildList {
            while (cursor.moveToNext()) {
                add(cursor.toStorageRow())
            }
        }
    }
}

/** Load one row by SQLite rowid for the generic detail screen. */
suspend fun loadStorageRow(
    database: DashDatabase,
    model: StorageModel,
    rowId: Long,
): StorageRow? = withContext(Dispatchers.IO) {
    val sql = "SELECT rowid AS _storage_rowid_, * FROM ${model.tableName} WHERE rowid = ?"
    database.query(SimpleSQLiteQuery(sql, arrayOf(rowId))).use { cursor ->
        if (cursor.moveToNext()) cursor.toStorageRow() else null
    }
}

private fun Cursor.toStorageRow(): StorageRow {
    var rowId = 0L
    val columns = LinkedHashMap<String, Any?>()
    for (i in 0 until columnCount) {
        val name = getColumnName(i)
        val value: Any? = when (getType(i)) {
            Cursor.FIELD_TYPE_NULL -> null
            Cursor.FIELD_TYPE_INTEGER -> getLong(i)
            Cursor.FIELD_TYPE_FLOAT -> getDouble(i)
            Cursor.FIELD_TYPE_STRING -> getString(i)
            Cursor.FIELD_TYPE_BLOB -> getBlob(i)
            else -> null
        }
        if (name == "_storage_rowid_") {
            rowId = (value as? Long) ?: 0L
        } else {
            columns[name] = value
        }
    }
    return StorageRow(rowId = rowId, columns = columns)
}
