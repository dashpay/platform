package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentToken.swift` — one token configuration.
 *
 * Swift `@Attribute(.unique)` on `id` (`contractId (32B) || position
 * (big-endian Int)`) → primary key.
 *
 * The FK is declared on the [contractId] scalar with CASCADE (Swift
 * `PersistentDataContract.tokens` declares `.cascade`; tokens are only
 * created after the contract row exists in `DataContractParser`).
 *
 * ## Control-rule / distribution payloads
 *
 * Swift stores the `ChangeControlRules` / `TokenPerpetualDistribution` /
 * `TokenPreProgrammedDistribution` / `TokenDistributionChangeRules` /
 * `TokenLocalization` Codable structs (TokenTypes.swift) inline. They are
 * UI-only payloads (parsed out of the contract JSON, never sent to Rust),
 * so they persist here as JSON [String] columns.
 *
 * SwiftData predicates query capability as `rules != nil`
 * (PersistentToken.swift:274-345). Since the payloads are opaque JSON
 * here, the capability set derived from the Swift computed properties
 * (PersistentToken.swift:138-210) is ALSO persisted as [Boolean] columns
 * so DAO queries are plain SQL. Writers must keep each flag ==
 * `(corresponding rules column != null)` (and [hasDistribution] ==
 * `perpetualDistribution != null || preProgrammedDistribution != null`).
 */
@Entity(
    tableName = "tokens",
    indices = [Index(value = ["contractId"])],
    foreignKeys = [
        ForeignKey(
            entity = DataContractEntity::class,
            parentColumns = ["id"],
            childColumns = ["contractId"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class TokenEntity(
    /** `contractId || position.bigEndian` — the Swift synthetic unique id. */
    @PrimaryKey val id: ByteArray,
    /** 32-byte parent contract id (also the FK column). */
    val contractId: ByteArray,
    /** Token position within the contract. Swift `Int` → [Int]. */
    val position: Int,
    val name: String,
    /** Decimal string (u64-ranged values overflow Long, hence String in Swift). */
    val baseSupply: String,
    val maxSupply: String? = null,
    val decimals: Int = 8,
    /** JSON `{lang: TokenLocalization}` map (UI-only Codable → JSON String). */
    val localizations: String? = null,
    val isPaused: Boolean = false,
    val allowTransferToFrozenBalance: Boolean = true,
    val keepsTransferHistory: Boolean = true,
    val keepsFreezingHistory: Boolean = true,
    val keepsMintingHistory: Boolean = true,
    val keepsBurningHistory: Boolean = true,
    val keepsDirectPricingHistory: Boolean = true,
    val keepsDirectPurchaseHistory: Boolean = true,
    /** JSON `ChangeControlRules` (UI-only) — null ⇔ rule absent. */
    val conventionsChangeRules: String? = null,
    val maxSupplyChangeRules: String? = null,
    val manualMintingRules: String? = null,
    val manualBurningRules: String? = null,
    val freezeRules: String? = null,
    val unfreezeRules: String? = null,
    val destroyFrozenFundsRules: String? = null,
    val emergencyActionRules: String? = null,
    /** JSON `TokenPerpetualDistribution` (UI-only). */
    val perpetualDistribution: String? = null,
    /** JSON `TokenPreProgrammedDistribution` (UI-only). */
    val preProgrammedDistribution: String? = null,
    /** 32-byte destination identity id. */
    val newTokensDestinationIdentity: ByteArray? = null,
    val mintingAllowChoosingDestination: Boolean = true,
    /** JSON `TokenDistributionChangeRules` (UI-only). */
    val distributionChangeRules: String? = null,
    /** `TokenTradeMode.rawValue` ("NotTradeable"). */
    val tradeMode: String = "NotTradeable",
    val tradeModeChangeRules: String? = null,
    val mainControlGroupPosition: Int? = null,
    val mainControlGroupCanBeModified: String? = null,
    val tokenDescription: String? = null,
    val createdAt: Date = Date(),
    /** Swift names this `lastUpdatedAt` (not `lastUpdated`). */
    val lastUpdatedAt: Date = Date(),

    // Capability columns — persisted mirrors of the Swift computed
    // properties so `mintableTokensPredicate` & co. compile to plain SQL.
    /** Mirror of Swift `canManuallyMint` (= manualMintingRules != nil). */
    val canManuallyMint: Boolean = false,
    /** Mirror of Swift `canManuallyBurn` (= manualBurningRules != nil). */
    val canManuallyBurn: Boolean = false,
    /** Mirror of Swift `canFreeze` (= freezeRules != nil). */
    val canFreeze: Boolean = false,
    /** Mirror of Swift `canUnfreeze` (= unfreezeRules != nil). */
    val canUnfreeze: Boolean = false,
    /** Mirror of Swift `canDestroyFrozenFunds` (= destroyFrozenFundsRules != nil). */
    val canDestroyFrozenFunds: Boolean = false,
    /** Mirror of Swift `hasEmergencyActions` (= emergencyActionRules != nil). */
    val hasEmergencyActions: Boolean = false,
    /** Mirror of Swift `canChangeMaxSupply` (= maxSupplyChangeRules != nil). */
    val canChangeMaxSupply: Boolean = false,
    /** Mirror of Swift `canChangeConventions` (= conventionsChangeRules != nil). */
    val canChangeConventions: Boolean = false,
    /** Mirror of Swift `canChangeTradeMode` (= tradeModeChangeRules != nil). */
    val canChangeTradeMode: Boolean = false,
    /** Mirror of Swift `hasDistribution` (= perpetual != nil || preProgrammed != nil). */
    val hasDistribution: Boolean = false,
) {
    override fun equals(other: Any?): Boolean =
        other is TokenEntity && id.contentEquals(other.id)

    override fun hashCode(): Int = id.contentHashCode()
}
