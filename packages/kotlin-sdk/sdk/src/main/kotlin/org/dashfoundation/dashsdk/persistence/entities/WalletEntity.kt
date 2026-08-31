package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentWallet.swift` — core wallet metadata for a single HD
 * wallet with its sync state.
 *
 * Swift `#Unique<PersistentWallet>([\.walletId])` → [walletId] is the
 * primary key. Swift `#Index([\.networkRaw], [\.walletGroupId])` → the two
 * indices below.
 *
 * Children (accounts, identities) reference this row via `walletId` foreign
 * keys declared on their side: accounts CASCADE (Swift
 * `@Relationship(deleteRule: .cascade)` on `accounts`), identities SET_NULL
 * (Swift `.nullify` on `identities`).
 */
@Entity(
    tableName = "wallets",
    indices = [
        Index(value = ["networkRaw"]),
        Index(value = ["walletGroupId"]),
    ],
)
data class WalletEntity(
    /**
     * 32-byte NETWORK-SCOPED wallet ID — globally unique because the
     * network byte is folded into the digest on the Rust side.
     */
    @PrimaryKey val walletId: ByteArray,
    /**
     * 32-byte NETWORK-INDEPENDENT group id shared by every network's wallet
     * derived from the same seed. Empty for legacy rows.
     */
    val walletGroupId: ByteArray = ByteArray(0),
    /**
     * `Network.rawValue`; Swift `UInt32?` → nullable [Int]. `null` means
     * "not yet known" (row created by a changeset before
     * `persistWalletMetadata` filled the network in).
     */
    val networkRaw: Int? = null,
    val name: String? = null,
    val walletDescription: String? = null,
    /** Swift `UInt32` → [Int] (block heights stay well under 2^31). */
    val birthHeight: Int = 0,
    /** Swift `UInt32` → [Int]. */
    val syncedHeight: Int = 0,
    /** Unix seconds; Swift `UInt64` → [Long] (unsigned semantics). */
    val lastSynced: Long = 0,
    /**
     * Bincode-serialised `dashcore` ChainLock from the previous session.
     * Opaque passthrough — decoded only by Rust; never re-encoded here.
     */
    val lastAppliedChainLockBytes: ByteArray? = null,
    /**
     * The numeric block height of the last applied chainlock, delivered
     * separately by `onWalletChangesetChainLockHeight` (the bincode blob
     * above is opaque on this side of the FFI). Monotonic max — a stale
     * round never lowers it. This is the chainlock half of the swept-
     * tombstone collection boundary `min(chainlockHeight, syncedHeight)`;
     * while NULL no finality boundary exists and the collector never
     * runs, mirroring the SQLite store's "no-op until a chainlock height
     * has been persisted".
     */
    val lastAppliedChainLockHeight: Int? = null,
    val isImported: Boolean = false,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
) {
    override fun equals(other: Any?): Boolean =
        other is WalletEntity && walletId.contentEquals(other.walletId)

    override fun hashCode(): Int = walletId.contentHashCode()
}
