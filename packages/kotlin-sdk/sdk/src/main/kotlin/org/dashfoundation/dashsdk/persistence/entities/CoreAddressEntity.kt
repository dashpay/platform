package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentCoreAddress.swift` — a single on-chain address in a
 * wallet's address pool (external / internal / absent).
 *
 * Swift `@Attribute(.unique)` on `address` → primary key.
 *
 * Parent [accountId] FK CASCADEs (Swift `PersistentAccount.coreAddresses`
 * declares `.cascade`); nullable because the Swift `account` relationship
 * is optional.
 */
@Entity(
    tableName = "core_addresses",
    indices = [Index(value = ["accountId"])],
    foreignKeys = [
        ForeignKey(
            entity = AccountEntity::class,
            parentColumns = ["id"],
            childColumns = ["accountId"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class CoreAddressEntity(
    /** Base58check-encoded address. */
    @PrimaryKey val address: String,
    /** 33-byte compressed secp256k1 pubkey, or empty when unavailable. */
    val publicKey: ByteArray = ByteArray(0),
    /**
     * `AddressPoolTypeTagFFI`: 0 External, 1 Internal, 2 Absent,
     * 3 AbsentHardened. Swift `UInt8` → [Int].
     */
    val poolTypeTag: Int,
    /** Derivation index within this pool. Swift `UInt32` → [Int]. */
    val addressIndex: Int,
    /** BIP32 derivation path, e.g. `m/44'/1'/0'/0/3`. */
    val derivationPath: String,
    val isUsed: Boolean = false,
    /** Swift `UInt32` → [Int]. Zero until seen on-chain. */
    val firstSeenHeight: Int = 0,
    /** Swift `UInt32` → [Int]. */
    val lastSeenHeight: Int = 0,
    /** Cached balance in duffs; Swift `UInt64` → [Long] (unsigned semantics). */
    val balance: Long = 0,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
    /** Parent account rowid (Swift `account` relationship, optional). */
    val accountId: Long? = null,
)
