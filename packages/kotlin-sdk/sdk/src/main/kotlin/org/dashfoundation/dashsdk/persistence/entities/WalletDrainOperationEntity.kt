package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

/**
 * The id a host stored for one Core to shielded "Max" transfer (a wallet
 * drain), written BEFORE the drain is called so that a relaunch can ask
 * Rust what became of it. Port of `WalletDrainOperationRecord` in
 * `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Services/WalletDrainOperationStore.swift`,
 * which keeps the same fields in a JSON file.
 *
 * It names the operation and nothing else: every amount, input and fee is
 * Rust's, and so is the operation's state
 * ([org.dashfoundation.dashsdk.wallet.PlatformWalletManager.shieldedWalletDrainStatus]).
 * A row here is therefore not by itself an unfinished transfer. It is
 * removed when Rust reports the operation needs nothing more
 * ([org.dashfoundation.dashsdk.wallet.WalletDrainRules.keepsStoredId]) and
 * when its wallet is deleted.
 *
 * Not written by a persistence callback: Rust never sees this table. The
 * operation id Rust stores with the asset lock it builds lives on
 * [AssetLockEntity.operationId].
 *
 * No FK to `wallets`, like [AssetLockEntity]: rows are removed by the
 * wallet-deletion pass.
 */
@Entity(
    tableName = "wallet_drain_operations",
    indices = [Index(value = ["walletId"])],
)
class WalletDrainOperationEntity(
    /** The operation's UUID as its 16 bytes, most significant first. */
    @PrimaryKey val operationId: ByteArray,
    /** 32-byte owning wallet id. */
    val walletId: ByteArray,
    /** `Network.ffiValue` of the network the transfer was started on. */
    val networkRaw: Int,
    /** The Core account index the drain swept. */
    val fundingAccountIndex: Int,
    /**
     * The wallet's own 43-byte raw Orchard address at submit time. A resume
     * pays it.
     */
    val recipientRaw43: ByteArray,
    /** Epoch millis the row was written, for a stable list order. */
    val createdAtMillis: Long,
) {
    override fun equals(other: Any?): Boolean =
        other is WalletDrainOperationEntity &&
            operationId.contentEquals(other.operationId) &&
            walletId.contentEquals(other.walletId) &&
            networkRaw == other.networkRaw &&
            fundingAccountIndex == other.fundingAccountIndex &&
            recipientRaw43.contentEquals(other.recipientRaw43) &&
            createdAtMillis == other.createdAtMillis

    override fun hashCode(): Int = operationId.contentHashCode()
}
