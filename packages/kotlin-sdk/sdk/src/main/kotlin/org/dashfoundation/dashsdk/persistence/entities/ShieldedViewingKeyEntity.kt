package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.Index
import java.util.Date

/**
 * One persisted Orchard full viewing key per `(walletId, accountIndex)`.
 *
 * The key is viewing-grade only: it can recognize and decrypt notes but
 * cannot authorize a spend. It is the raw 96-byte Orchard
 * `FullViewingKey` encoding (`ak ‖ nk ‖ rivk`) emitted by
 * `ShieldedChangeSet.viewing_keys`.
 *
 * No wallet FK, matching the other shielded tables and SwiftData model;
 * wallet teardown removes these rows explicitly.
 */
@Entity(
    tableName = "shielded_viewing_keys",
    primaryKeys = ["walletId", "accountIndex"],
    indices = [Index(value = ["walletId"])],
)
data class ShieldedViewingKeyEntity(
    val walletId: ByteArray,
    /** ZIP-32 `u32` account index carried in Kotlin as its raw [Int] bits. */
    val accountIndex: Int,
    val fvkBytes: ByteArray,
    val lastUpdated: Date = Date(),
) {
    init {
        require(walletId.size == WALLET_ID_SIZE) {
            "walletId must be exactly $WALLET_ID_SIZE bytes, got ${walletId.size}"
        }
        require(fvkBytes.size == FVK_SIZE) {
            "Orchard full viewing key must be exactly $FVK_SIZE bytes, got ${fvkBytes.size}"
        }
    }

    companion object {
        const val WALLET_ID_SIZE: Int = 32
        const val FVK_SIZE: Int = 96
    }
}
