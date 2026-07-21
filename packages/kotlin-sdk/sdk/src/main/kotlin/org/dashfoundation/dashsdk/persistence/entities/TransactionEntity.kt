package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentTransaction.swift` — one wallet transaction record.
 *
 * Deliberately NOT scoped to a wallet or account (per the Swift doc): the
 * same on-chain tx can pay into several accounts/wallets. Funds membership
 * is recoverable through TXOs; payload-only provider membership is recorded
 * explicitly by [TransactionAccountInvolvementEntity].
 *
 * Swift `@Attribute(.unique)` on `txid` → primary key.
 * Swift `#Index([\.firstSeen])` → index below.
 */
@Entity(
    tableName = "transactions",
    indices = [Index(value = ["firstSeen"])],
)
data class TransactionEntity(
    /** 32-byte txid, raw little-endian wire bytes (NOT display-flipped). */
    @PrimaryKey val txid: ByteArray,
    /** Consensus-encoded raw transaction bytes — opaque passthrough to Rust. */
    val transactionData: ByteArray,
    /** 0=mempool, 1=instantSend, 2=inBlock, 3=inChainLockedBlock. Swift `UInt32`. */
    val context: Int = 0,
    /** Swift `UInt32` → [Int]. 0 for mempool. */
    val blockHeight: Int = 0,
    val blockHash: ByteArray? = null,
    /** Swift `UInt32` → [Int]. */
    val blockTimestamp: Int = 0,
    /** Transaction index within its block; meaningful iff [hasBlockPosition]. */
    val blockPosition: Int = 0,
    /** False for unconfirmed and pre-v7 rows. */
    val hasBlockPosition: Boolean = false,
    /** 0=incoming, 1=outgoing, 2=internal, 3=coinJoin. Swift `UInt32`. */
    val direction: Int = 0,
    /** Human-readable only — NOT a stable discriminant (see Swift doc). */
    val transactionType: String = "Standard",
    /**
     * Typed discriminant of Rust `TransactionType`; Swift `UInt8` → [Int].
     * `0xFF` (255) is the "not yet populated" sentinel.
     */
    val transactionTypeKind: Int = 0xFF,
    /** Signed net amount in duffs (positive=received, negative=sent). */
    val netAmount: Long = 0,
    /** Fee in duffs; Swift `UInt64?` → nullable [Long] (unsigned semantics). */
    val fee: Long? = null,
    val label: String = "",
    /** Unix seconds first observed; Swift `UInt64` → [Long]. */
    val firstSeen: Long = 0,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
) {
    override fun equals(other: Any?): Boolean =
        other is TransactionEntity && txid.contentEquals(other.txid)

    override fun hashCode(): Int = txid.contentHashCode()
}
