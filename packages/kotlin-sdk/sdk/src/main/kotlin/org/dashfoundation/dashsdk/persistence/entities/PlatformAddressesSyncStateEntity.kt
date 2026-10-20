package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentPlatformAddressesSyncState.swift` — BLAST
 * incremental-sync watermark, one row per network.
 *
 * Swift `@Attribute(.unique)` on `walletId` → primary key. NOTE: despite
 * the name, the stored value is a network-scoped pseudo key, not a real
 * wallet id (kept as `walletId` for schema compatibility per the Swift
 * doc) — hence deliberately NO foreign key to `wallets`.
 */
@Entity(
    tableName = "platform_addresses_sync_states",
    indices = [Index(value = ["networkRaw"])],
)
data class PlatformAddressesSyncStateEntity(
    /** Stable 32-byte scope key (network-scoped pseudo wallet id). */
    @PrimaryKey val walletId: ByteArray,
    /** `Network.rawValue`; Swift `UInt32` → [Int]. */
    val networkRaw: Int,
    /** Swift `UInt64` → [Long] (unsigned semantics). */
    val syncHeight: Long,
    /** Unix seconds; Swift `UInt64` → [Long]. */
    val syncTimestamp: Long,
    /** Compaction marker; Swift `UInt64` → [Long]. */
    val lastKnownRecentBlock: Long,
    val lastUpdated: Date = Date(),
) {
    override fun equals(other: Any?): Boolean =
        other is PlatformAddressesSyncStateEntity && walletId.contentEquals(other.walletId)

    override fun hashCode(): Int = walletId.contentHashCode()
}
