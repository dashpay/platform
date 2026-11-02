package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentAssetLock.swift` — one tracked asset-lock credit
 * output (DIP-0027), one row per `(walletId, outpoint)`.
 *
 * Swift `@Attribute(.unique)` on `outPointHex` → primary key
 * (`<txid display hex>:<vout>`). Swift `#Index([\.walletId])` → index.
 *
 * No FK to `wallets`: the Swift model declares no relationship — rows are
 * deleted manually by wallet teardown (`deleteWalletData`), which the
 * `AssetLockDao.deleteByWallet` mirror covers.
 */
@Entity(
    tableName = "asset_locks",
    indices = [Index(value = ["walletId"])],
)
data class AssetLockEntity(
    @PrimaryKey val outPointHex: String,
    /** 32-byte owning wallet id. */
    val walletId: ByteArray,
    /** Consensus-encoded asset-lock transaction — opaque passthrough to Rust. */
    val transactionBytes: ByteArray,
    /**
     * `AssetLockFundingType` discriminant: 0 IdentityRegistration,
     * 1 IdentityTopUp, 2 IdentityTopUpNotBound, 3 IdentityInvitation,
     * 4 AssetLockAddressTopUp, 5 AssetLockShieldedAddressTopUp.
     */
    val fundingTypeRaw: Int,
    /** Identity index slot consumed by this lock. Swift `Int32`. */
    val identityIndexRaw: Int,
    /** BIP44 account index the funding tx was built from. Swift `Int32`. */
    val accountIndexRaw: Int = 0,
    /** Locked amount in duffs. Swift `Int64`. */
    val amountDuffs: Long,
    /**
     * `AssetLockStatus` discriminant: 0 Built, 1 Broadcast,
     * 2 InstantSendLocked, 3 ChainLocked, 4 Consumed,
     * 5 RecoveredFromChain.
     *
     * `4` is terminal; `5` is NOT, its higher discriminant
     * notwithstanding. `5` means Core finality is proven while
     * Platform-side consumption is unknown — what the restore scan and the
     * chainlock-promotion path write — so it belongs in every "still
     * recoverable" predicate alongside `1..3`, and a contiguous `1..3`
     * range silently drops it.
     */
    val statusRaw: Int,
    /**
     * Bincode-encoded `AssetLockProof` (dpp standard config) — opaque
     * passthrough decoded only by Rust. `null` until IS/Chain-locked.
     */
    val proofBytes: ByteArray? = null,
    /** 20-byte recipient platform-address hash (fundingTypeRaw == 4 only). */
    val recipientPlatformAddressHash: ByteArray? = null,
    /** 0 = P2PKH, 1 = P2SH; Swift `UInt8?` → [Int]?. Null iff hash is null. */
    val recipientPlatformAddressType: Int? = null,
    val createdAt: Date = Date(),
    /** Swift names this `updatedAt` (not `lastUpdated`). */
    val updatedAt: Date = Date(),
)
