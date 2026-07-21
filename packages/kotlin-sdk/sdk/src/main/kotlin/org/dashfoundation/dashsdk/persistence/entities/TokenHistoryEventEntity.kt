package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentTokenHistoryEvent.swift` — one token history event.
 *
 * Swift `@Attribute(.unique)` on `id: UUID` → primary key as the UUID's
 * canonical string form.
 *
 * [tokenRef] materializes the optional `token` relationship with CASCADE
 * (Swift `PersistentToken.historyEvents` declares `.cascade`).
 */
@Entity(
    tableName = "token_history_events",
    indices = [Index(value = ["tokenRef"])],
    foreignKeys = [
        ForeignKey(
            entity = TokenEntity::class,
            parentColumns = ["id"],
            childColumns = ["tokenRef"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class TokenHistoryEventEntity(
    /** Swift `UUID` → canonical UUID string. */
    @PrimaryKey val id: String,
    /** `TokenEventType.rawValue` ("Mint", "Burn", "Transfer", …). */
    val eventType: String,
    val transactionId: ByteArray? = null,
    val blockHeight: Long? = null,
    val coreBlockHeight: Long? = null,
    /** 32-byte identity ids. */
    val fromIdentity: ByteArray? = null,
    val toIdentity: ByteArray? = null,
    val performedByIdentity: ByteArray,
    /** Decimal strings (u64-ranged token amounts). */
    val amount: String? = null,
    val balanceBefore: String? = null,
    val balanceAfter: String? = null,
    /** JSON `[String: Any]` blob — passthrough. */
    val additionalDataJSON: ByteArray? = null,
    val eventDescription: String? = null,
    val createdAt: Date = Date(),
    val eventTimestamp: Date = Date(),
    /** FK materialization of the Swift `token` relationship (`tokens.id`). */
    val tokenRef: ByteArray? = null,
)
