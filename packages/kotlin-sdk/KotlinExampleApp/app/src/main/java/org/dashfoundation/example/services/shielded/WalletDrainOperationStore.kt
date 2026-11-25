package org.dashfoundation.example.services.shielded

import org.dashfoundation.dashsdk.persistence.dao.WalletDrainOperationDao
import org.dashfoundation.dashsdk.persistence.entities.WalletDrainOperationEntity
import org.dashfoundation.dashsdk.wallet.toWalletDrainOperationIdBytes
import org.dashfoundation.dashsdk.wallet.walletDrainOperationIdFromBytes
import java.util.UUID

/**
 * The stored id of one Core to shielded "Max" transfer. Port of
 * `WalletDrainOperationRecord` in
 * `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Services/WalletDrainOperationStore.swift`.
 *
 * It only names the operation, the account index it swept and the recipient
 * a resume pays. Every amount, input and fee is Rust's, and so is the
 * operation's state.
 */
class WalletDrainOperationRecord(
    val operationId: UUID,
    walletId: ByteArray,
    /** `Network.ffiValue` of the network the transfer was started on. */
    val networkRaw: Int,
    val fundingAccountIndex: Int,
    /** The wallet's own shielded address at submit time. A resume pays it. */
    recipientRaw43: ByteArray,
    val createdAtMillis: Long,
) {
    private val walletIdBytes = walletId.copyOf()
    private val recipientBytes = recipientRaw43.copyOf()

    val walletId: ByteArray get() = walletIdBytes.copyOf()
    val recipientRaw43: ByteArray get() = recipientBytes.copyOf()

    fun belongsTo(walletId: ByteArray): Boolean = walletIdBytes.contentEquals(walletId)

    override fun equals(other: Any?): Boolean =
        other is WalletDrainOperationRecord &&
            operationId == other.operationId &&
            walletIdBytes.contentEquals(other.walletIdBytes) &&
            networkRaw == other.networkRaw &&
            fundingAccountIndex == other.fundingAccountIndex &&
            recipientBytes.contentEquals(other.recipientBytes) &&
            createdAtMillis == other.createdAtMillis

    override fun hashCode(): Int = operationId.hashCode()
}

/**
 * Where the stored ids live. A failed read throws instead of reading as
 * empty: an empty answer would hide an unfinished transfer. Port of
 * `WalletDrainOperationStore` (same Swift file), which keeps a JSON file;
 * here it is the SDK database's `wallet_drain_operations` table.
 */
interface WalletDrainOperationStore {
    /** Every stored id, on every network, oldest first. */
    suspend fun records(): List<WalletDrainOperationRecord>

    /** Store [record]. Throws when it cannot be written, so nothing is called. */
    suspend fun insert(record: WalletDrainOperationRecord)

    /** Remove the id; a no-op when it is already gone. */
    suspend fun remove(operationId: UUID)
}

/** [WalletDrainOperationStore] over the SDK's Room table. */
class RoomWalletDrainOperationStore(
    private val dao: WalletDrainOperationDao,
) : WalletDrainOperationStore {

    override suspend fun records(): List<WalletDrainOperationRecord> =
        dao.getAll().map { row ->
            WalletDrainOperationRecord(
                // A row whose id is not a UUID cannot be asked about; failing
                // the read keeps it from being silently skipped.
                operationId = checkNotNull(walletDrainOperationIdFromBytes(row.operationId)) {
                    "a saved Max transfer has an operation id of ${row.operationId.size} bytes"
                },
                walletId = row.walletId,
                networkRaw = row.networkRaw,
                fundingAccountIndex = row.fundingAccountIndex,
                recipientRaw43 = row.recipientRaw43,
                createdAtMillis = row.createdAtMillis,
            )
        }

    override suspend fun insert(record: WalletDrainOperationRecord) {
        dao.insert(
            WalletDrainOperationEntity(
                operationId = record.operationId.toWalletDrainOperationIdBytes(),
                walletId = record.walletId,
                networkRaw = record.networkRaw,
                fundingAccountIndex = record.fundingAccountIndex,
                recipientRaw43 = record.recipientRaw43,
                createdAtMillis = record.createdAtMillis,
            ),
        )
    }

    override suspend fun remove(operationId: UUID) {
        dao.deleteByOperationId(operationId.toWalletDrainOperationIdBytes())
    }
}
