package org.dashfoundation.example.services.tokens

import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.contentOrNull
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
        get() = authorizedToMakeChange != AuthorizedActionTakers.NO_ONE && !isUnknown

    /** `true` when this build could not read who is authorized. */
    val isUnknown: Boolean
        get() = authorizedToMakeChange.startsWith(AuthorizedActionTakers.UNKNOWN_PREFIX)

    /** "Yes" / "No" / "Unknown" for the Token Details capability rows. */
    val capabilityLabel: String
        get() = when {
            isUnknown -> "Unknown"
            hasAuthorizedTakers -> "Yes"
            else -> "No"
        }

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
         * Decode a rule object from contract JSON or from a rules column.
         * Handles the `V0` wrapper and snake_case and camelCase keys. Each
         * action-taker field goes through [AuthorizedActionTakers.parse],
         * so a value it does not recognise is kept as an
         * [AuthorizedActionTakers.UNKNOWN_PREFIX] string instead of being
         * read as "no one".
         */
        fun parse(ruleContainer: JsonObject): ChangeControlRules {
            val rule = (ruleContainer["V0"] as? JsonObject) ?: ruleContainer
            return ChangeControlRules(
                authorizedToMakeChange = rule.taker(
                    "authorized_to_make_change", "authorizedToMakeChange",
                ),
                adminActionTakers = rule.taker(
                    "admin_action_takers", "adminActionTakers",
                ),
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

        /**
         * The first present key's taker. A rule object that carries
         * neither key is not something rs-dpp writes, so it reads as
         * unknown rather than as "no one".
         */
        private fun JsonObject.taker(vararg keys: String): String {
            for (key in keys) {
                val value = this[key] ?: continue
                return AuthorizedActionTakers.parse(value)
            }
            return AuthorizedActionTakers.unknown("missing")
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
 * `Identity:<base58>` / `Group:<position>`), plus `Unknown:<what>` for a
 * value this build cannot read.
 */
object AuthorizedActionTakers {
    const val NO_ONE = "NoOne"
    const val CONTRACT_OWNER = "ContractOwner"
    const val MAIN_GROUP = "MainGroup"
    const val GROUP_PREFIX = "Group:"
    const val IDENTITY_PREFIX = "Identity:"

    /**
     * A taker this build could not read: a `$type` added by a newer
     * protocol version, a tagged variant missing its payload, or a shape
     * rs-dpp does not write. Kept visible so the action row explains the
     * gap instead of reporting "no one is authorized".
     */
    const val UNKNOWN_PREFIX = "Unknown:"

    fun group(position: Int): String = "$GROUP_PREFIX$position"
    fun identity(base58: String): String = "$IDENTITY_PREFIX$base58"
    fun unknown(what: String): String = "$UNKNOWN_PREFIX$what"

    /**
     * Read one action-taker value into the string grammar. Accepts:
     * - the flat `$type` map rs-dpp has written since 4.0.0-beta.4
     *   (`AuthorizedActionTakers`'s `Serialize` impl in
     *   `packages/rs-dpp/src/data_contract/change_control_rules/authorized_action_takers.rs`):
     *   `{"$type":"noOne"}`, `{"$type":"contractOwner"}`,
     *   `{"$type":"identity","identity":"<base58>"}`, `{"$type":"mainGroup"}`,
     *   `{"$type":"group","position":<n>}`;
     * - the string grammar above, which is how the rules columns store it;
     * - the older externally tagged `{"Group": n}` / `{"Identity": "<base58>"}`.
     *
     * Anything else comes back as [unknown], never as [NO_ONE].
     */
    fun parse(value: JsonElement): String {
        if (value is JsonPrimitive) {
            val text = value.contentOrNull?.takeIf { value.isString }
                ?: return unknown(value.toString())
            return if (isGrammar(text)) text else unknown(text)
        }
        val map = value as? JsonObject ?: return unknown(value.toString())
        val wireType = (map["\$type"] as? JsonPrimitive)?.takeIf { it.isString }?.content
        if (wireType != null) {
            return when (wireType) {
                "noOne" -> NO_ONE
                "contractOwner" -> CONTRACT_OWNER
                "mainGroup" -> MAIN_GROUP
                "identity" -> (map["identity"] as? JsonPrimitive)
                    ?.takeIf { it.isString }?.content?.let(::identity)
                    ?: unknown(wireType)
                "group" -> (map["position"] as? JsonPrimitive)
                    ?.content?.toIntOrNull()?.let(::group)
                    ?: unknown(wireType)
                else -> unknown(wireType)
            }
        }
        (map["Group"] as? JsonPrimitive)?.content?.toIntOrNull()?.let { return group(it) }
        (map["Identity"] as? JsonPrimitive)?.takeIf { it.isString }?.content
            ?.let { return identity(it) }
        return unknown(map.toString())
    }

    private fun isGrammar(text: String): Boolean = when {
        text == NO_ONE || text == CONTRACT_OWNER || text == MAIN_GROUP -> true
        text.startsWith(GROUP_PREFIX) -> text.removePrefix(GROUP_PREFIX).toIntOrNull() != null
        text.startsWith(IDENTITY_PREFIX) -> text.length > IDENTITY_PREFIX.length
        text.startsWith(UNKNOWN_PREFIX) -> true
        else -> false
    }
}

/**
 * Who a perpetual distribution pays: rs-dpp's `TokenDistributionRecipient`
 * (`packages/rs-dpp/src/data_contract/associated_token/token_perpetual_distribution/distribution_recipient.rs`).
 * Drive lets only that recipient claim it (the wrong-claimant check in
 * `token_claim_transition_action/v0/transformer.rs`); the minting
 * destination (`newTokensDestinationIdentity`) plays no part.
 */
sealed interface TokenDistributionRecipient {
    data object ContractOwner : TokenDistributionRecipient
    data class Identity(val base58: String) : TokenDistributionRecipient
    data object EvonodesByParticipation : TokenDistributionRecipient

    /** A recipient this build cannot read; [what] names it for the UI. */
    data class Unknown(val what: String) : TokenDistributionRecipient

    companion object {

        /**
         * Read the `distributionRecipient` of a stored perpetual
         * distribution block (`{"$formatVersion":"0","distributionType":…,
         * "distributionRecipient":{"$type":…}}`, optionally `V0`-wrapped).
         */
        fun fromPerpetualDistribution(json: String?): TokenDistributionRecipient {
            if (json.isNullOrBlank()) return Unknown("no perpetual distribution")
            val root = try {
                LenientJson.parseToJsonElement(json) as? JsonObject
            } catch (_: Exception) {
                null
            } ?: return Unknown("unreadable perpetual distribution")
            val body = (root["V0"] as? JsonObject) ?: root
            val recipient = body["distributionRecipient"]
                ?: return Unknown("missing distributionRecipient")
            return parse(recipient)
        }

        /** The flat `$type` map rs-dpp writes; anything else is [Unknown]. */
        fun parse(value: JsonElement): TokenDistributionRecipient {
            val map = value as? JsonObject ?: return Unknown(value.toString())
            val wireType = (map["\$type"] as? JsonPrimitive)?.takeIf { it.isString }?.content
                ?: return Unknown(map.toString())
            return when (wireType) {
                "contractOwner" -> ContractOwner
                "identity" -> (map["identity"] as? JsonPrimitive)
                    ?.takeIf { it.isString }?.content?.let(::Identity)
                    ?: Unknown(wireType)
                "evonodesByParticipation" -> EvonodesByParticipation
                else -> Unknown(wireType)
            }
        }
    }
}

/**
 * `oncePerIdentityDistribution` (protocol version 14): a fixed [amount]
 * every identity may claim exactly once. Decoded view of the raw block
 * [TokenMaterializer] persists in
 * [org.dashfoundation.dashsdk.persistence.entities.TokenEntity.oncePerIdentityDistribution]
 * (`{"$formatVersion":"0","amount":<u64>}`, the amount a JSON number or a
 * decimal string). [amount] is a decimal string like every other raw token
 * amount in the app.
 */
data class TokenOncePerIdentityDistribution(val amount: String) {

    companion object {

        fun parse(json: String?): TokenOncePerIdentityDistribution? {
            if (json.isNullOrBlank()) return null
            return try {
                parse(LenientJson.parseToJsonElement(json).jsonObject)
            } catch (_: Exception) {
                null
            }
        }

        /**
         * Null when the block carries no `amount` that is a raw u64, the
         * type the protocol gives token amounts. The range rs-dpp admits for
         * this field (1 to `i64::MAX`) is validated in Rust at registration
         * and is deliberately not mirrored here.
         */
        fun parse(obj: JsonObject): TokenOncePerIdentityDistribution? {
            // Tolerate the enum-wrapped rendering the pre-programmed
            // resolver also accepts; rs-dpp itself emits the flat shape.
            val body = (obj["V0"] as? JsonObject) ?: obj
            val content = (body["amount"] as? JsonPrimitive)?.content ?: return null
            return TokenAmounts.parseRaw(content)?.let(::TokenOncePerIdentityDistribution)
        }
    }
}
