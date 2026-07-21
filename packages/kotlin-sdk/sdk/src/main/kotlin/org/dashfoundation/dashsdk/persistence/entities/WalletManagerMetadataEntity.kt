package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentWalletManagerMetadata.swift` — wallet-manager-level
 * metadata, singleton per network.
 *
 * Swift `@Attribute(.unique)` on `networkRaw` → primary key.
 */
@Entity(tableName = "wallet_manager_metadata")
data class WalletManagerMetadataEntity(
    /** `Network.rawValue`; Swift `UInt32` → [Int]. */
    @PrimaryKey val networkRaw: Int,
    /** Combined sync height across all wallets. Swift `UInt32` → [Int]. */
    val combinedSyncHeight: Int = 0,
    /** Combined sync block hash (32 bytes). */
    val combinedSyncBlockHash: ByteArray? = null,
    /** Number of wallets managed. Swift `Int` → [Int]. */
    val walletCount: Int = 0,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
) {
    override fun equals(other: Any?): Boolean =
        other is WalletManagerMetadataEntity && networkRaw == other.networkRaw

    override fun hashCode(): Int = networkRaw
}
