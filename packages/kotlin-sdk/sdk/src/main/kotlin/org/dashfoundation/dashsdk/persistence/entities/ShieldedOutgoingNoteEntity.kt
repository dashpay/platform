package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.Index
import java.util.Date

/**
 * Port of `PersistentShieldedOutgoingNote.swift` — one outgoing (sent)
 * shielded note recovered via the outgoing viewing key. Append-only send
 * history (no nullifier / position / spend state).
 *
 * Swift `#Unique([\.walletId, \.accountIndex, \.cmx])` → composite
 * primary key. Swift `#Index([\.walletId, \.accountIndex])` → index.
 */
@Entity(
    tableName = "shielded_outgoing_notes",
    primaryKeys = ["walletId", "accountIndex", "cmx"],
    indices = [Index(value = ["walletId", "accountIndex"])],
)
data class ShieldedOutgoingNoteEntity(
    /** 32-byte wallet id. */
    val walletId: ByteArray,
    /** ZIP-32 account index; Swift `UInt32` → [Int]. */
    val accountIndex: Int,
    /** Note commitment of the sent note (32 bytes) — natural key. */
    val cmx: ByteArray,
    /** Recipient's raw Orchard address (43 bytes). */
    val recipient: ByteArray,
    /** Value sent in credits; Swift `UInt64` → [Long] (unsigned semantics). */
    val value: Long,
    /** Raw Dash memo bytes (≤36 bytes). */
    val memo: ByteArray,
    /** Swift `UInt64` → [Long]. */
    val blockHeight: Long,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
)
