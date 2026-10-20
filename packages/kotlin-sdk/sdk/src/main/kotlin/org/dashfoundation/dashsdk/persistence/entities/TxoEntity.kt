package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentTxo.swift` — one transaction output (spent or
 * unspent). Spent rows are kept (never deleted) so history stays whole.
 *
 * Swift `@Attribute(.unique)` on `outpoint` → primary key
 * (36 bytes: 32-byte txid wire-order + 4-byte vout little-endian).
 * Swift `#Index([\.walletId])` → index below.
 *
 * Relationship materialization (SwiftData relationships → FK columns):
 * - `transaction` → [txid], CASCADE (Swift `PersistentTransaction.outputs`
 *   declares `.cascade`). Nullable only for the brief insert window, same
 *   as the Swift optional.
 * - `spendingTransaction` → [spendingTxid], SET_NULL (Swift `.nullify` on
 *   `PersistentTransaction.inputs` — deleting a spending tx flips this TXO
 *   back to unspent, never deletes it).
 * - `account` → [accountId], SET_NULL (plain pointer, default nullify).
 * - `coreAddress` → [coreAddressId], CASCADE (Swift
 *   `PersistentCoreAddress.txos` declares `.cascade`).
 */
@Entity(
    tableName = "txos",
    indices = [
        Index(value = ["walletId"]),
        Index(value = ["txid"]),
        Index(value = ["spendingTxid"]),
        Index(value = ["accountId"]),
        Index(value = ["coreAddressId"]),
    ],
    foreignKeys = [
        ForeignKey(
            entity = TransactionEntity::class,
            parentColumns = ["txid"],
            childColumns = ["txid"],
            onDelete = ForeignKey.CASCADE,
        ),
        ForeignKey(
            entity = TransactionEntity::class,
            parentColumns = ["txid"],
            childColumns = ["spendingTxid"],
            onDelete = ForeignKey.SET_NULL,
        ),
        ForeignKey(
            entity = AccountEntity::class,
            parentColumns = ["id"],
            childColumns = ["accountId"],
            onDelete = ForeignKey.SET_NULL,
        ),
        ForeignKey(
            entity = CoreAddressEntity::class,
            parentColumns = ["address"],
            childColumns = ["coreAddressId"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class TxoEntity(
    /** 36-byte outpoint: txid (wire order) + vout LE — the Swift unique key. */
    @PrimaryKey val outpoint: ByteArray,
    /** Output index within the transaction. Swift `UInt32` → [Int]. */
    val vout: Int,
    /** Value in duffs; Swift `UInt64` → [Long] (unsigned semantics). */
    val amount: Long,
    /** Owning address (Base58Check) — authoritative string identifier. */
    val address: String,
    val scriptPubKey: ByteArray = ByteArray(0),
    /** Block height where created. Swift `UInt32` → [Int]. */
    val height: Int = 0,
    val isCoinbase: Boolean = false,
    val isConfirmed: Boolean = false,
    val isInstantLocked: Boolean = false,
    val isLocked: Boolean = false,
    /**
     * Whether this coin is out of the restore set; kept explicit (hot
     * filter path). Monotonic on the record and `utxos_spent` channels
     * (`isSpent = existing || spender in-block || stamped`), so it is
     * true for a spender that reached a block, for every stamped hold
     * ([supersededByTxid] set), and for a healed asset-lock spend — and
     * false for a coin whose only claim is a mempool/IS-locked link. Only
     * a sweep release or an unlinked re-delivery of the coin lowers it.
     */
    val isSpent: Boolean = false,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
    /** 32-byte wallet id denorm — the per-wallet scan column. */
    val walletId: ByteArray = ByteArray(0),
    /** FK to the creating transaction (Swift `transaction` relationship). */
    val txid: ByteArray? = null,
    /** FK to the spending transaction (Swift `spendingTransaction`), null = unspent. */
    val spendingTxid: ByteArray? = null,
    /**
     * Vin index within the spending transaction; Swift `UInt32?` → [Int]?.
     * `null` when unspent or migrated from a pre-feature row.
     */
    val spendingInputIndex: Int? = null,
    /** Fallback parent-account pointer (Swift `account` relationship). */
    val accountId: Long? = null,
    /**
     * Owning `core_addresses.address` (Swift `coreAddress` relationship).
     * The [address] string is the authoritative identifier; this FK is the
     * navigation pointer.
     */
    val coreAddressId: String? = null,
    /**
     * Port of Swift `PersistentTxo.supersededByTxid` — the winner a sweep
     * attributed this coin's consumption to, mirroring the SQLite store's
     * `spent_in_txid`. Two writers set it: the sweep pass
     * (`PlatformWalletPersistenceHandler.applySweptTransaction`), for
     * every held input of a swept loser that has a row, keyed by the
     * loser's decoded input outpoints rather than by this row's link; and
     * `onWalletChangesetUtxoAdded` draining a `pending_inputs` tombstone —
     * the funding output arrived only after the loser that spent it was
     * swept and deleted. Deliberately NOT an FK: the winner named here
     * need not have its own `transactions` row (it can be
     * wallet-irrelevant), so this column has to hold a bare txid that
     * `transactions(txid)` may never contain.
     *
     * The hold is the stamp, not the link. A stamped row keeps
     * `isSpent = true` whatever later happens to [spendingTxid] — a new
     * spender may adopt the link (attribution for `walletFundedTransaction`)
     * without lowering the flag — and a stamped row that is UNLINKED is
     * what the sweep release pass frees. Cleared by exactly two events:
     * a sweep release of this outpoint (a later sweep proved the coin came
     * free after all, and no stored network-final spender vetoes it), and
     * the wallet re-delivering the coin unspent while the row is unlinked
     * (`onWalletChangesetUtxoAdded`: the wallet knows the coin, so any
     * network-final spender of it is re-discovered by its own scan; holding
     * the row would lock a real coin out forever after a reorg of the
     * winner).
     */
    val supersededByTxid: ByteArray? = null,
) {
    override fun equals(other: Any?): Boolean =
        other is TxoEntity && outpoint.contentEquals(other.outpoint)

    override fun hashCode(): Int = outpoint.contentHashCode()
}
