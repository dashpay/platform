package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey
import org.dashfoundation.dashsdk.persistence.UInt64Value
import java.util.Date

/**
 * Port of `PersistentTokenBalance.swift` — one identity's balance of one
 * token.
 *
 * The Swift model declares no unique attribute; the persister upserts by
 * `(tokenId, identityId)` (PlatformWalletPersistenceHandler.swift:1527) —
 * mirrored here as a surrogate rowid PK plus a `(tokenId, identityId)`
 * index (non-unique, matching the Swift schema exactly).
 * Swift `#Index([\.networkRaw])` → index below.
 *
 * Relationship materialization:
 * - `identity` → [identityRef], SET_NULL (Swift
 *   `PersistentIdentity.tokenBalances` declares `.nullify`; the non-null
 *   [identityId] scalar survives regardless, as in Swift).
 * - `token` → [tokenRef], CASCADE (Swift `PersistentToken.balances`
 *   declares `.cascade`).
 */
@Entity(
    tableName = "token_balances",
    indices = [
        Index(value = ["networkRaw"]),
        Index(value = ["tokenId", "identityId"]),
        Index(value = ["identityId"]),
        Index(value = ["identityRef"]),
        Index(value = ["tokenRef"]),
    ],
    foreignKeys = [
        ForeignKey(
            entity = IdentityEntity::class,
            parentColumns = ["identityId"],
            childColumns = ["identityRef"],
            onDelete = ForeignKey.SET_NULL,
        ),
        ForeignKey(
            entity = TokenEntity::class,
            parentColumns = ["id"],
            childColumns = ["tokenRef"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class TokenBalanceEntity(
    /** Surrogate rowid — Swift has no unique attribute on this model. */
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    /** Token id as base58 String (verbatim Swift storage shape). */
    val tokenId: String,
    /** 32-byte identity id (denorm scalar, always set). */
    val identityId: ByteArray,
    /** Full protocol u64; Room stores it as an order-preserving 8-byte BLOB. */
    val balance: UInt64Value = UInt64Value.ZERO,
    val frozen: Boolean = false,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
    val lastSyncedAt: Date? = null,
    val tokenName: String? = null,
    val tokenSymbol: String? = null,
    /** Swift `Int32?`. */
    val tokenDecimals: Int? = null,
    /** `Network.rawValue`; Swift `UInt32` → [Int]. */
    val networkRaw: Int,
    /** FK materialization of the Swift `identity` relationship. */
    val identityRef: ByteArray? = null,
    /** FK materialization of the Swift `token` relationship (`tokens.id`). */
    val tokenRef: ByteArray? = null,
)
