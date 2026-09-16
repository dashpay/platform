package org.dashfoundation.example.services.tokens

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import org.dashfoundation.dashsdk.persistence.dao.TokenDao
import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import java.nio.ByteBuffer
import java.util.Date

/**
 * Materializes [TokenEntity] rows from a stored contract's JSON — the
 * Kotlin port of the token slice of `DataContractParser.swift`
 * (`parseTokenConfiguration` + `parseChangeControlRule` +
 * `extractTokenSupply`). iOS persists `PersistentToken` children at
 * contract-parse time; the Android downloader previously stored only the
 * contract row, leaving the `tokens` table empty — this closes that gap
 * so `TokenDao`'s capability queries have rows to serve.
 *
 * Rule columns persist in the canonical [ChangeControlRules.toJson]
 * shape; each capability flag column is kept `== (rules column != null)`
 * per the [TokenEntity] writer contract.
 */
object TokenMaterializer {

    /** `contractId (32B) || position (big-endian Int)` — the Swift synthetic unique id. */
    fun tokenId(contractId: ByteArray, position: Int): ByteArray =
        ByteBuffer.allocate(36).put(contractId).putInt(position).array()

    /**
     * Parse every token on [contract] and upsert the rows through [dao].
     * Safe to re-run (upsert by the synthetic id); parse failures on one
     * token skip that token rather than failing the batch.
     */
    suspend fun materialize(contract: DataContractEntity, dao: TokenDao) {
        for (token in parse(contract)) {
            dao.upsertToken(token)
        }
    }

    /** Pure parse of [contract]'s tokens map into entity rows. */
    fun parse(contract: DataContractEntity): List<TokenEntity> {
        val root = try {
            LenientJson.parseToJsonElement(contract.serializedContract.decodeToString()).jsonObject
        } catch (_: Exception) {
            return emptyList()
        }
        val tokens = root["tokens"] as? JsonObject ?: return emptyList()
        return tokens.mapNotNull { (positionKey, element) ->
            val position = positionKey.toIntOrNull() ?: return@mapNotNull null
            val obj = element as? JsonObject ?: return@mapNotNull null
            try {
                parseToken(contract.id, position, obj)
            } catch (_: Exception) {
                null
            }
        }
    }

    private fun parseToken(contractId: ByteArray, position: Int, dict: JsonObject): TokenEntity {
        val conventions = dict.obj("conventions")
        val localizations = conventions?.obj("localizations")
        val english = localizations?.obj("en")
        val singular = english?.str("singularForm") ?: english?.str("singular")

        // Rules (V0-unwrapped, taker strings normalized).
        val conventionsChange = dict.rule("conventionsChangeRules")
        val maxSupplyChange = dict.rule("maxSupplyChangeRules")
        val manualMinting = dict.rule("manualMintingRules")
        val manualBurning = dict.rule("manualBurningRules")
        val freeze = dict.rule("freezeRules")
        val unfreeze = dict.rule("unfreezeRules")
        val destroyFrozenFunds = dict.rule("destroyFrozenFundsRules")
        val emergencyAction = dict.rule("emergencyActionRules")

        // Distribution rules block (DataContractParser.swift:495-563).
        val distributionRules = dict.obj("distributionRules")
        val perpetual = distributionRules?.obj("perpetualDistribution")
        val preProgrammed = distributionRules?.obj("preProgrammedDistribution")
        val newTokensDestination = distributionRules?.str("newTokensDestinationIdentity")
            ?.let { Base58.decodeIdentifier(it) }
        val mintingAllowChoosing = distributionRules
            ?.boolean("mintingAllowChoosingDestination") ?: true
        val distributionChange = TokenDistributionChangeRules(
            perpetualDistributionRules = distributionRules?.rule("perpetualDistributionRules"),
            newTokensDestinationIdentityRules =
                distributionRules?.rule("newTokensDestinationIdentityRules"),
            mintingAllowChoosingDestinationRules =
                distributionRules?.rule("mintingAllowChoosingDestinationRules"),
            changeDirectPurchasePricingRules =
                distributionRules?.rule("changeDirectPurchasePricingRules"),
        ).takeIf { !it.isEmpty }

        // Marketplace rules.
        val marketplace = dict.obj("marketplaceRules")
        val tradeMode = marketplace?.str("tradeMode") ?: "NotTradeable"
        val tradeModeChange = marketplace?.rule("tradeModeChangeRules")

        // History-keeping: nested object or a single boolean for all.
        val keepsHistoryObj = dict.obj("keepsHistory")
        val keepsHistoryAll = dict.boolean("keepsHistory")
        fun keeps(key: String): Boolean =
            keepsHistoryObj?.boolean(key) ?: keepsHistoryAll ?: true

        val now = Date()
        val hasDistribution = perpetual != null || preProgrammed != null
        return TokenEntity(
            id = tokenId(contractId, position),
            contractId = contractId,
            position = position,
            name = singular ?: dict.str("description") ?: "Token $position",
            baseSupply = dict.supply("baseSupply") ?: "0",
            maxSupply = dict.supply("maxSupply")?.takeIf { it != "0" },
            decimals = conventions?.int("decimals") ?: dict.int("decimals") ?: 8,
            localizations = localizations
                ?.filterKeys { it != "\$formatVersion" }
                ?.takeIf { it.isNotEmpty() }
                ?.let { JsonObject(it).toString() },
            isPaused = dict.boolean("startAsPaused") ?: false,
            allowTransferToFrozenBalance =
                dict.boolean("allowTransferToFrozenBalance") ?: true,
            keepsTransferHistory = keeps("keepsTransferHistory"),
            keepsFreezingHistory = keeps("keepsFreezingHistory"),
            keepsMintingHistory = keeps("keepsMintingHistory"),
            keepsBurningHistory = keeps("keepsBurningHistory"),
            keepsDirectPricingHistory = keeps("keepsDirectPricingHistory"),
            keepsDirectPurchaseHistory = keeps("keepsDirectPurchaseHistory"),
            conventionsChangeRules = conventionsChange?.toJson(),
            maxSupplyChangeRules = maxSupplyChange?.toJson(),
            manualMintingRules = manualMinting?.toJson(),
            manualBurningRules = manualBurning?.toJson(),
            freezeRules = freeze?.toJson(),
            unfreezeRules = unfreeze?.toJson(),
            destroyFrozenFundsRules = destroyFrozenFunds?.toJson(),
            emergencyActionRules = emergencyAction?.toJson(),
            perpetualDistribution = perpetual?.toString(),
            preProgrammedDistribution = preProgrammed?.toString(),
            newTokensDestinationIdentity = newTokensDestination,
            mintingAllowChoosingDestination = mintingAllowChoosing,
            distributionChangeRules = distributionChange?.toJson(),
            tradeMode = tradeMode,
            tradeModeChangeRules = tradeModeChange?.toJson(),
            mainControlGroupPosition = dict.int("mainControlGroup"),
            mainControlGroupCanBeModified = dict.str("mainControlGroupCanBeModified"),
            tokenDescription = dict.str("description"),
            createdAt = now,
            lastUpdatedAt = now,
            // Capability columns — MUST stay == (rules column != null),
            // and hasDistribution == (perpetual || preProgrammed).
            canManuallyMint = manualMinting != null,
            canManuallyBurn = manualBurning != null,
            canFreeze = freeze != null,
            canUnfreeze = unfreeze != null,
            canDestroyFrozenFunds = destroyFrozenFunds != null,
            hasEmergencyActions = emergencyAction != null,
            canChangeMaxSupply = maxSupplyChange != null,
            canChangeConventions = conventionsChange != null,
            canChangeTradeMode = tradeModeChange != null,
            hasDistribution = hasDistribution,
        )
    }

    // ── JSON helpers ──────────────────────────────────────────────────

    private fun JsonObject.obj(key: String): JsonObject? = this[key] as? JsonObject

    private fun JsonObject.str(key: String): String? =
        (this[key] as? JsonPrimitive)?.takeIf { it.isString }?.content

    private fun JsonObject.int(key: String): Int? =
        (this[key] as? JsonPrimitive)?.content?.toIntOrNull()

    private fun JsonObject.boolean(key: String): Boolean? =
        (this[key] as? JsonPrimitive)?.content?.toBooleanStrictOrNull()

    private fun JsonObject.rule(key: String): ChangeControlRules? =
        (this[key] as? JsonObject)?.let(ChangeControlRules::parse)

    /**
     * u64-safe supply extraction — port of `extractTokenSupply`
     * (numbers or strings; decimal string out).
     */
    private fun JsonObject.supply(key: String): String? {
        val primitive = this[key] as? JsonPrimitive ?: return null
        val content = primitive.content
        if (content.isEmpty() || content == "null") return null
        // Integer-looking content passes through; scientific/decimal
        // renderings are normalized via BigDecimal.
        return content.toBigIntegerOrNull()?.toString()
            ?: content.toBigDecimalOrNull()?.toBigInteger()?.toString()
    }

    private fun String.toBigIntegerOrNull() = try {
        java.math.BigInteger(this)
    } catch (_: NumberFormatException) {
        null
    }

    private fun String.toBigDecimalOrNull() = try {
        java.math.BigDecimal(this)
    } catch (_: NumberFormatException) {
        null
    }
}
