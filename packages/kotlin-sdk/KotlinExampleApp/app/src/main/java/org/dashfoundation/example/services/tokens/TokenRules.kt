package org.dashfoundation.example.services.tokens

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.put
import org.dashfoundation.example.util.LenientJson

/**
 * `ChangeControlRules` — port of the Codable struct in
 * `packages/swift-sdk/Sources/SwiftDashSDK/Persistence/Types/TokenTypes.swift`
 * plus the parse logic of `DataContractParser.parseChangeControlRule`
 * (DataContractParser.swift:585-621).
 *
 * The Android token rows persist each rule as a JSON [String] column
 * ([org.dashfoundation.dashsdk.persistence.entities.TokenEntity]); this
 * type is the app-side decoded view. [toJson] emits the canonical
 * camelCase shape so a rules column written by [TokenMaterializer] always
 * round-trips through [parse].
 */
data class ChangeControlRules(
    val authorizedToMakeChange: String = AuthorizedActionTakers.NO_ONE,
    val adminActionTakers: String = AuthorizedActionTakers.NO_ONE,
    val changingAuthorizedActionTakersToNoOneAllowed: Boolean = false,
    val changingAdminActionTakersToNoOneAllowed: Boolean = false,
    val selfChangingAdminActionTakersAllowed: Boolean = false,
) {

    /**
     * `true` when this rule names someone allowed to act on it — the
     * `hasAuthorizedTakers` truth-in-UI predicate from
     * `TokenDetailsView.swift` (a shipped-but-locked `NoOne` rule reads
     * as "not available").
     */
    val hasAuthorizedTakers: Boolean
        get() = authorizedToMakeChange != AuthorizedActionTakers.NO_ONE

    /** Canonical camelCase JSON for the Room rules columns. */
    fun toJson(): String = buildJsonObject {
        put("authorizedToMakeChange", authorizedToMakeChange)
        put("adminActionTakers", adminActionTakers)
        put("changingAuthorizedActionTakersToNoOneAllowed", changingAuthorizedActionTakersToNoOneAllowed)
        put("changingAdminActionTakersToNoOneAllowed", changingAdminActionTakersToNoOneAllowed)
        put("selfChangingAdminActionTakersAllowed", selfChangingAdminActionTakersAllowed)
    }.toString()

    companion object {

        /** Decode a rules JSON column (or raw contract fragment). Null on garbage. */
        fun parse(json: String?): ChangeControlRules? {
            if (json.isNullOrBlank()) return null
            return try {
                parse(LenientJson.parseToJsonElement(json).jsonObject)
            } catch (_: Exception) {
                null
            }
        }

        /**
         * Decode a rule object from contract JSON — handles the `V0`
         * wrapper, snake_case and camelCase keys, and both the string
         * (`"MainGroup"`) and tagged-object (`{"Group": 3}` /
         * `{"Identity": "<base58>"}`) renderings of
         * `AuthorizedActionTakers`, normalized into the `Group:<n>` /
         * `Identity:<base58>` string grammar the Swift evaluator uses.
         */
        fun parse(ruleContainer: JsonObject): ChangeControlRules {
            val rule = (ruleContainer["V0"] as? JsonObject) ?: ruleContainer
            return ChangeControlRules(
                authorizedToMakeChange = rule.takerString(
                    "authorized_to_make_change", "authorizedToMakeChange",
                ) ?: AuthorizedActionTakers.NO_ONE,
                adminActionTakers = rule.takerString(
                    "admin_action_takers", "adminActionTakers",
                ) ?: AuthorizedActionTakers.NO_ONE,
                changingAuthorizedActionTakersToNoOneAllowed = rule.boolValue(
                    "changing_authorized_action_takers_to_no_one_allowed",
                    "changingAuthorizedActionTakersToNoOneAllowed",
                ) ?: false,
                changingAdminActionTakersToNoOneAllowed = rule.boolValue(
                    "changing_admin_action_takers_to_no_one_allowed",
                    "changingAdminActionTakersToNoOneAllowed",
                ) ?: false,
                selfChangingAdminActionTakersAllowed = rule.boolValue(
                    "self_changing_admin_action_takers_allowed",
                    "selfChangingAdminActionTakersAllowed",
                ) ?: false,
            )
        }

        private fun JsonObject.takerString(vararg keys: String): String? {
            for (key in keys) {
                when (val v = this[key]) {
                    is JsonPrimitive -> if (v.isString || v.content.isNotEmpty()) return v.content
                    is JsonObject -> {
                        // rs-dpp tagged-enum rendering: {"Group": 3} / {"Identity": "..."}.
                        (v["Group"] as? JsonPrimitive)?.content?.toIntOrNull()
                            ?.let { return AuthorizedActionTakers.group(it) }
                        (v["Identity"] as? JsonPrimitive)?.content
                            ?.let { return AuthorizedActionTakers.identity(it) }
                    }
                    else -> {}
                }
            }
            return null
        }

        private fun JsonObject.boolValue(vararg keys: String): Boolean? {
            for (key in keys) {
                (this[key] as? JsonPrimitive)?.content?.toBooleanStrictOrNull()?.let { return it }
            }
            return null
        }
    }
}

/**
 * `TokenDistributionChangeRules` — port of the Codable bundle in
 * `TokenTypes.swift` that groups the four distribution-related sub-rules.
 * Persisted as one JSON column ([TokenEntity.distributionChangeRules])
 * with each present sub-rule in the canonical [ChangeControlRules.toJson]
 * shape.
 */
data class TokenDistributionChangeRules(
    val perpetualDistributionRules: ChangeControlRules? = null,
    val newTokensDestinationIdentityRules: ChangeControlRules? = null,
    val mintingAllowChoosingDestinationRules: ChangeControlRules? = null,
    val changeDirectPurchasePricingRules: ChangeControlRules? = null,
) {

    val isEmpty: Boolean
        get() = perpetualDistributionRules == null &&
            newTokensDestinationIdentityRules == null &&
            mintingAllowChoosingDestinationRules == null &&
            changeDirectPurchasePricingRules == null

    fun toJson(): String = buildJsonObject {
        perpetualDistributionRules?.let {
            put("perpetualDistributionRules", LenientJson.parseToJsonElement(it.toJson()))
        }
        newTokensDestinationIdentityRules?.let {
            put("newTokensDestinationIdentityRules", LenientJson.parseToJsonElement(it.toJson()))
        }
        mintingAllowChoosingDestinationRules?.let {
            put("mintingAllowChoosingDestinationRules", LenientJson.parseToJsonElement(it.toJson()))
        }
        changeDirectPurchasePricingRules?.let {
            put("changeDirectPurchasePricingRules", LenientJson.parseToJsonElement(it.toJson()))
        }
    }.toString()

    companion object {

        fun parse(json: String?): TokenDistributionChangeRules? {
            if (json.isNullOrBlank()) return null
            return try {
                parse(LenientJson.parseToJsonElement(json).jsonObject)
            } catch (_: Exception) {
                null
            }
        }

        fun parse(obj: JsonObject): TokenDistributionChangeRules = TokenDistributionChangeRules(
            perpetualDistributionRules = (obj["perpetualDistributionRules"] as? JsonObject)
                ?.let(ChangeControlRules::parse),
            newTokensDestinationIdentityRules = (obj["newTokensDestinationIdentityRules"] as? JsonObject)
                ?.let(ChangeControlRules::parse),
            mintingAllowChoosingDestinationRules = (obj["mintingAllowChoosingDestinationRules"] as? JsonObject)
                ?.let(ChangeControlRules::parse),
            changeDirectPurchasePricingRules = (obj["changeDirectPurchasePricingRules"] as? JsonObject)
                ?.let(ChangeControlRules::parse),
        )
    }
}

/**
 * The `AuthorizedActionTakers` string grammar — port of the enum in
 * `TokenTypes.swift` (`NoOne` / `ContractOwner` / `MainGroup` /
 * `Identity:<base58>` / `Group:<position>`).
 */
object AuthorizedActionTakers {
    const val NO_ONE = "NoOne"
    const val CONTRACT_OWNER = "ContractOwner"
    const val MAIN_GROUP = "MainGroup"
    const val GROUP_PREFIX = "Group:"
    const val IDENTITY_PREFIX = "Identity:"

    fun group(position: Int): String = "$GROUP_PREFIX$position"
    fun identity(base58: String): String = "$IDENTITY_PREFIX$base58"
}
