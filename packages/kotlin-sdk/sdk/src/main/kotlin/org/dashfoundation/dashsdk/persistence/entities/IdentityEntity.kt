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
    /**
     * What Platform proved, when last asked, about the voting key stored for this masternode
     * or evonode identity: `"valid"`, `"invalid"` or `"undetermined"`; null when it was never
     * checked (no voting key, or stored before keys were checked). Swift
     * `PersistentIdentity.votingKeyCheckRaw`. Read through
     * `org.dashfoundation.dashsdk.voting.votingKeyCheck` and written only by
     * `MasternodeVotingKeyRecords.record`, with what Rust says to record.
     */
    val votingKeyCheckRaw: String? = null,
    /** The reason that came with an invalid or undetermined check, worded in Rust. */
    val votingKeyCheckMessage: String? = null,
    /**
     * The 20-byte voting address of the key the check was made of. A record outlives a key
     * replaced without one, so the address ties it to the key it describes: Rust reads a
     * record of another address as nothing recorded. Swift
     * `PersistentIdentity.votingKeyCheckAddress`.
     */
    val votingKeyCheckAddress: ByteArray? = null,
    /**
     * The 32-byte pro_tx_hash, in wire order, of the masternode or evonode this row votes for,
     * recorded when it is loaded for voting (`Voting.loadMasternodeForVoting`). Kept apart from
     * [identityId]: a masternode without an owner identity (a shared masternode) is stored
     * under its voter identity, whose id is not the pro_tx_hash. Null for a row stored before
     * it was recorded; read it through `org.dashfoundation.dashsdk.voting.votingProTxHash`.
     * Swift `PersistentIdentity.masternodeProTxHash`.
     */
    val masternodeProTxHash: ByteArray? = null,
    /**
     * Which of the masternode's identities Rust's load rule returned for this row: `"owner"` or
     * `"voter"`; null for a row not loaded through it. A voter identity holds only a voting key.
     * Read through `org.dashfoundation.dashsdk.voting.masternodeRole`. Swift
     * `PersistentIdentity.masternodeRoleRaw`.
     */
    val masternodeRoleRaw: String? = null,
) {
    override fun equals(other: Any?): Boolean =
        other is IdentityEntity && identityId.contentEquals(other.identityId)

    override fun hashCode(): Int = identityId.contentHashCode()
}
