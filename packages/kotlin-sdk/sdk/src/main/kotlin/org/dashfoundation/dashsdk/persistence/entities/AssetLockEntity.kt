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
 *
 * The three sidecar columns ([operationId], [shieldedRecipientRaw43],
 * [identityRecipientId]) are written by their own persistence callbacks,
 * which native fires after the row's upsert in the same changeset (see the
 * `PersistAssetLock*Fn` typedefs in
 * `packages/rs-platform-wallet-ffi/src/persistence.rs`). Each is immutable
 * once stored: the handler refuses a different value, and the
 * `asset_locks_*_immutable` triggers [DashDatabase] installs refuse one at
 * the SQL layer too, as the SQLite store's V019, V020 and V023 triggers do. A later
 * upsert of the row carries them forward rather than clearing them.
 * `(walletId, operationId)` is unique, as SQLite's
 * `asset_locks_wallet_operation_id` index is (NULLs never collide).
 */
@Entity(
    tableName = "asset_locks",
    indices = [
        Index(value = ["walletId"]),
        Index(value = ["walletId", "operationId"], unique = true),
    ],
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
     * 5 RecoveredFromChain, 6 DrainCompleted.
     *
     * `4` and `6` are terminal (`6`, a proven wallet-drain consumption, is
     * the stronger: only another `6` overwrites it); `5` is NOT, its higher discriminant
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
    /**
     * 16-byte operation UUID native correlates a wallet drain with
     * (`PersistenceCapabilities::ASSET_LOCK_OPERATION_IDS`, bit 13; SQLite
     * `V019__asset_lock_operation_id`, Swift
     * `PersistentAssetLock.operationId`). Null for a lock built without one.
     */
    val operationId: ByteArray? = null,
    /**
     * Raw 43-byte Orchard address a shielded funding (`fundingTypeRaw == 5`)
     * was built to pay, so a resume after restart pays the recipient the user
     * confirmed (`ASSET_LOCK_SHIELDED_RECIPIENTS`, bit 14; SQLite
     * `V020__asset_lock_shielded_recipient`, Swift
     * `PersistentAssetLock.shieldedRecipientRaw43`).
     */
    val shieldedRecipientRaw43: ByteArray? = null,
    /**
     * 32-byte id of the identity an unbound identity top-up
     * (`fundingTypeRaw == 2`) was built or assigned to pay, so a resume after
     * restart pays that identity and refuses another
     * (`ASSET_LOCK_IDENTITY_RECIPIENTS`, bit 17; SQLite
     * `V023__asset_lock_identity_recipient`). Null for every other lock, and
     * for an unbound lock rebuilt from the chain until the user assigns one.
     */
    val identityRecipientId: ByteArray? = null,
)
