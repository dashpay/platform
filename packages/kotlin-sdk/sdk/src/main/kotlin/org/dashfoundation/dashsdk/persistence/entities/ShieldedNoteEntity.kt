package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentShieldedNote.swift` — one decrypted shielded
 * (Orchard) note owned by a subwallet.
 *
 * Swift `@Attribute(.unique)` on `nullifier` → primary key (Orchard
 * nullifiers are globally unique — the upsert key that prevents
 * double-counts on re-sync).
 * Swift `#Index([\.walletId, \.accountIndex])` → index below.
 *
 * No FK: the Swift model declares no relationship; rows are removed
 * manually on wallet teardown (`ShieldedDao.deleteNotesByWallet`).
 */
@Entity(
    tableName = "shielded_notes",
    indices = [Index(value = ["walletId", "accountIndex"])],
)
data class ShieldedNoteEntity(
    /** Spending nullifier (32 bytes). */
    @PrimaryKey val nullifier: ByteArray,
    /** 32-byte wallet id. */
    val walletId: ByteArray,
    /** ZIP-32 account index; Swift `UInt32` → [Int]. */
    val accountIndex: Int,
    /** Global commitment-tree position; Swift `UInt64` → [Long]. */
    val position: Long,
    /** Note commitment (32 bytes). */
    val cmx: ByteArray,
    /** Swift `UInt64` → [Long]. */
    val blockHeight: Long,
    val isSpent: Boolean = false,
    /** Note value in credits; Swift `UInt64` → [Long] (unsigned semantics). */
    val value: Long,
    /**
     * Serialized `orchard::Note` bytes (115 bytes) — opaque passthrough
     * decoded only by Rust.
     */
    val noteData: ByteArray,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
) {
    override fun equals(other: Any?): Boolean =
        other is ShieldedNoteEntity && nullifier.contentEquals(other.nullifier)

    override fun hashCode(): Int = nullifier.contentHashCode()
}
