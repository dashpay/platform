package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.Index
import java.util.Date

/**
 * Port of `PersistentShieldedSyncState.swift` — per-subwallet shielded
 * sync watermark, one row per `(walletId, accountIndex)`.
 *
 * Swift `#Unique([\.walletId, \.accountIndex])` → composite primary key.
 * Swift `#Index([\.walletId])` → index.
 */
@Entity(
    tableName = "shielded_sync_states",
    primaryKeys = ["walletId", "accountIndex"],
    indices = [Index(value = ["walletId"])],
)
data class ShieldedSyncStateEntity(
    /** 32-byte wallet id. */
    val walletId: ByteArray,
    /** ZIP-32 account index; Swift `UInt32` → [Int]. */
    val accountIndex: Int,
    /**
     * Count of note positions scanned = next global commitment-tree index
     * to scan (exclusive); 0 = nothing scanned. Swift `UInt64` → [Long].
     */
    val lastSyncedIndex: Long = 0,
    val lastUpdated: Date = Date(),
)
