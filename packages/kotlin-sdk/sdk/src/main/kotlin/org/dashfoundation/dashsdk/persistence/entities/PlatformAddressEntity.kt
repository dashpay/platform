package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentPlatformAddress.swift` — one DIP-17 Platform Payment
 * address (bech32m, DIP-0018).
 *
 * The row identity is the composite `(walletId, address)` — NOT the
 * address alone. Two wallets can derive the same address (the same seed
 * imported twice, watch-only duplicates, coin-type-sharing networks), and
 * a global `address` primary key would silently reassign the row to
 * whichever wallet persisted last, corrupting the loser's balance and
 * signing lookups. `addressHash` uniqueness is likewise per-wallet, and
 * every lookup/upsert path is wallet-scoped (the native store keys pool
 * entries by `(wallet_id, address)` the same way).
 *
 * [accountId] materializes the optional `account` relationship with
 * CASCADE (Swift `PersistentAccount.platformAddresses` declares
 * `.cascade`).
 */
@Entity(
    tableName = "platform_addresses",
    primaryKeys = ["walletId", "address"],
    indices = [
        Index(value = ["walletId", "addressHash"], unique = true),
        Index(value = ["accountId"]),
    ],
    foreignKeys = [
        ForeignKey(
            entity = AccountEntity::class,
            parentColumns = ["id"],
            childColumns = ["accountId"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class PlatformAddressEntity(
    /** DIP-0018 bech32m address (`dash1…` / `tdash1…`). */
    val address: String,
    /** 0 = P2PKH, 1 = P2SH; Swift `UInt8` → [Int]. */
    val addressType: Int,
    /** 20-byte address hash (unique; BLAST upsert key). */
    val addressHash: ByteArray,
    /** 33-byte compressed secp256k1 pubkey, or empty when unavailable. */
    val publicKey: ByteArray = ByteArray(0),
    /** DIP-17 account index. Swift `UInt32` → [Int]. */
    val accountIndex: Int,
    /** DIP-17 derivation index within the account. Swift `UInt32` → [Int]. */
    val addressIndex: Int,
    /** BIP32 derivation path, e.g. `m/9'/5'/17'/0'/0'/0`. */
    val derivationPath: String,
    val isUsed: Boolean = false,
    /** Credit balance (1e11 credits/DASH); Swift `UInt64` → [Long]. */
    val balance: Long = 0,
    /** Anti-replay nonce. Swift `UInt32` → [Int]. */
    val nonce: Int = 0,
    /** Platform heights; Swift `UInt32` → [Int]. */
    val firstSeenHeight: Int = 0,
    val lastSeenHeight: Int = 0,
    /** 32-byte owning wallet id (denorm). */
    val walletId: ByteArray,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
    /** Parent account rowid (Swift `account` relationship, optional). */
    val accountId: Long? = null,
)
