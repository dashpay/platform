package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey

/** Authoritative Core engine claim, matching Swift's PersistentCoreSpentClaim. */
@Entity(
    tableName = "core_spent_claims",
    primaryKeys = ["walletId", "txid", "vout"],
    foreignKeys = [ForeignKey(
        entity = WalletEntity::class,
        parentColumns = ["walletId"],
        childColumns = ["walletId"],
        onDelete = ForeignKey.CASCADE,
    )],
)
data class SpentClaimEntity(
    val walletId: ByteArray,
    val txid: ByteArray,
    /** Raw u32 bits, preserved through JNI's signed Int carrier. */
    val vout: Int,
    /** Null is an authoritative unknown claimant, not an absent claim. */
    val claimant: ByteArray?,
)
