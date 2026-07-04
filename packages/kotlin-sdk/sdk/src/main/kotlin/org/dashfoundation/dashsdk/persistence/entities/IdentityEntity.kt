package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentIdentity.swift` — one Dash Platform identity.
 *
 * Swift `@Attribute(.unique)` on `identityId` → primary key.
 * Swift `#Index([\.networkRaw])` → index below.
 *
 * [walletId] materializes the optional `wallet` relationship with
 * SET_NULL (Swift `PersistentWallet.identities` declares `.nullify` — an
 * identity survives a wallet delete as an orphaned row).
 *
 * Children reference this row with FKs on their side: `public_keys`,
 * `documents`, `dpns_names`, `dashpay_profiles`,
 * `dashpay_contact_requests` all CASCADE; `token_balances` and
 * `data_contracts.ownerIdentityId` SET_NULL — matching each Swift
 * `@Relationship(deleteRule:)`.
 */
@Entity(
    tableName = "identities",
    indices = [
        Index(value = ["networkRaw"]),
        Index(value = ["walletId"]),
    ],
    foreignKeys = [
        ForeignKey(
            entity = WalletEntity::class,
            parentColumns = ["walletId"],
            childColumns = ["walletId"],
            onDelete = ForeignKey.SET_NULL,
        ),
    ],
)
data class IdentityEntity(
    /** 32-byte identity id. */
    @PrimaryKey val identityId: ByteArray,
    /** Credits balance; Swift stores `Int64` (bit-pattern of the u64). */
    val balance: Long = 0,
    val revision: Long = 0,
    val isLocal: Boolean = true,
    val alias: String? = null,
    /** "Show this one in the cell" hint; full labels live in `dpns_names`. */
    val dpnsName: String? = null,
    val mainDpnsName: String? = null,
    /** `IdentityType.rawValue` ("User" / "Masternode" / "Evonode"). */
    val identityType: String = "User",
    val votingPrivateKeyIdentifier: String? = null,
    val ownerPrivateKeyIdentifier: String? = null,
    val payoutPrivateKeyIdentifier: String? = null,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
    val lastSyncedAt: Date? = null,
    /** `Network.rawValue`; Swift `UInt32` → [Int]. */
    val networkRaw: Int,
    /** FK materialization of the Swift `wallet` relationship (0..1). */
    val walletId: ByteArray? = null,
    /** DIP-9 identity index within the owning wallet. Swift `UInt32`. */
    val identityIndex: Int = 0,
) {
    override fun equals(other: Any?): Boolean =
        other is IdentityEntity && identityId.contentEquals(other.identityId)

    override fun hashCode(): Int = identityId.contentHashCode()
}
