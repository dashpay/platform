package org.dashfoundation.example.services.tokens

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import org.dashfoundation.example.util.LenientJson

/**
 * Decoded group-action proposal — the Kotlin counterpart of the Swift
 * `TokenGroupAction` family, matching the JSON emitted by
 * `platform_wallet_token_pending_group_actions`
 * (rs-platform-wallet-ffi/src/tokens/group_queries.rs:174-197):
 *
 * ```json
 * {
 *   "actionId": "<base58 32-byte id>",
 *   "type": "mint" | "burn" | "freeze" | "unfreeze" | "destroyFrozenFunds"
 *         | "pause" | "resume" | "setPrice" | "directPurchase" | "<other>",
 *   "proposer": "<base58 identity id>",
 *   "tokenContractPosition": <u16>,
 *   "closed": <bool>,
 *   "params": { ...per-variant payload... }
 * }
 * ```
 *
 * Amounts arrive as decimal strings (u64-safe); [rawJson] preserves the
 * exact element so the proposal can travel through a nav route and be
 * re-decoded by the co-sign screen without loss.
 */
data class GroupActionProposal(
    val actionIdBase58: String,
    val type: String,
    val proposerBase58: String,
    val tokenContractPosition: Int,
    val closed: Boolean,
    val params: JsonObject,
    val rawJson: String,
) {

    /** Human label for the proposal type — mirror of `displayLabel`. */
    val displayLabel: String
        get() = when (type) {
            "mint" -> "Mint"
            "burn" -> "Burn"
            "freeze" -> "Freeze"
            "unfreeze" -> "Unfreeze"
            "destroyFrozenFunds" -> "Destroy Frozen Funds"
            "pause" -> "Pause"
            "resume" -> "Resume"
            "setPrice" -> "Set Price"
            "directPurchase" -> "Direct Purchase"
            else -> type.replaceFirstChar { it.uppercase() }
        }

    /** Whether the co-sign UI can replay this proposal's parameters. */
    val isCoSignSupported: Boolean
        get() = type in setOf(
            "mint", "burn", "freeze", "unfreeze", "destroyFrozenFunds",
            "pause", "resume", "setPrice",
        )

    fun paramString(key: String): String? =
        (params[key] as? JsonPrimitive)?.takeIf { it !is JsonNull }?.content

    fun paramAmount(key: String = "amount"): ULong? = paramString(key)?.toULongOrNull()

    /** One-line summary — mirror of `ProposalRow.summary`. */
    val summary: String
        get() {
            fun short(s: String?) = s?.let(TokenActionEvaluator::short) ?: "?"
            return when (type) {
                "mint" -> "Mint ${paramString("amount")} to ${short(paramString("recipient"))}"
                "burn" -> "Burn ${paramString("amount")} from ${short(paramString("burnFrom"))}"
                "freeze" -> "Freeze ${short(paramString("target"))}"
                "unfreeze" -> "Unfreeze ${short(paramString("target"))}"
                "destroyFrozenFunds" ->
                    "Destroy ${paramString("amount")} frozen from ${short(paramString("target"))}"
                "pause" -> "Pause all token operations"
                "resume" -> "Resume all token operations"
                "setPrice" -> paramString("pricePerToken")
                    ?.let { "Set direct purchase price to $it credits" }
                    ?: "Disable direct purchase"
                "directPurchase" ->
                    "Purchase ${paramString("amount")} for ${paramString("totalCost")} credits"
                else -> displayLabel
            }
        }

    companion object {

        /** Decode the JSON array from `Groups.pendingActions`. */
        fun parseList(json: String?): List<GroupActionProposal> {
            if (json.isNullOrBlank()) return emptyList()
            return try {
                (LenientJson.parseToJsonElement(json) as? JsonArray)
                    ?.mapNotNull { element -> (element as? JsonObject)?.let(::parse) }
                    ?: emptyList()
            } catch (_: Exception) {
                emptyList()
            }
        }

        /** Decode one proposal element (also used for the nav-route payload). */
        fun parse(json: String): GroupActionProposal? = try {
            parse(LenientJson.parseToJsonElement(json).jsonObject)
        } catch (_: Exception) {
            null
        }

        private fun parse(obj: JsonObject): GroupActionProposal? {
            val actionId = (obj["actionId"] as? JsonPrimitive)?.content ?: return null
            val type = (obj["type"] as? JsonPrimitive)?.content ?: return null
            val proposer = (obj["proposer"] as? JsonPrimitive)?.content ?: return null
            return GroupActionProposal(
                actionIdBase58 = actionId,
                type = type,
                proposerBase58 = proposer,
                tokenContractPosition = (obj["tokenContractPosition"] as? JsonPrimitive)
                    ?.content?.toIntOrNull() ?: 0,
                closed = (obj["closed"] as? JsonPrimitive)
                    ?.content?.toBooleanStrictOrNull() ?: false,
                params = obj["params"] as? JsonObject ?: JsonObject(emptyMap()),
                rawJson = obj.toString(),
            )
        }
    }
}

/**
 * One signer of a proposal — matches the
 * `platform_wallet_token_group_action_signers` element
 * `{"identityId": "<base58>", "power": <u32>}`.
 */
data class GroupActionSigner(
    val identityIdBase58: String,
    val power: Int,
) {
    companion object {
        fun parseList(json: String?): List<GroupActionSigner> {
            if (json.isNullOrBlank()) return emptyList()
            return try {
                LenientJson.parseToJsonElement(json).jsonArray.mapNotNull { element ->
                    val obj = element as? JsonObject ?: return@mapNotNull null
                    val id = (obj["identityId"] as? JsonPrimitive)?.content
                        ?: return@mapNotNull null
                    GroupActionSigner(
                        identityIdBase58 = id,
                        power = (obj["power"] as? JsonPrimitive)?.content?.toIntOrNull() ?: 0,
                    )
                }
            } catch (_: Exception) {
                emptyList()
            }
        }
    }
}
