package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index

/**
 * Explicit transaction↔typed-account membership.
 *
 * Funds transactions also have TXO-derived membership, but provider special
 * transactions may match only through their payload and create no TXO. The
 * account FK points at the row carrying the complete typed account identity.
 */
@Entity(
    tableName = "transaction_account_involvements",
    primaryKeys = ["transactionTxid", "accountId"],
    indices = [Index(value = ["accountId"])],
    foreignKeys = [
        ForeignKey(
            entity = TransactionEntity::class,
            parentColumns = ["txid"],
            childColumns = ["transactionTxid"],
            onDelete = ForeignKey.CASCADE,
        ),
        ForeignKey(
            entity = AccountEntity::class,
            parentColumns = ["id"],
            childColumns = ["accountId"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class TransactionAccountInvolvementEntity(
    val transactionTxid: ByteArray,
    val accountId: Long,
)
