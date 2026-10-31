package org.dashfoundation.dashsdk.voting

import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.longOrNull
import org.dashfoundation.dashsdk.errors.DashSdkError

// The types the decision proposal reads return, decoded from the JSON rs-sdk-ffi writes
// (`packages/rs-sdk-ffi/src/decision_proposal/json.rs`). Rust decides every value; these
// types only carry it. Names and shapes follow the Swift SDK
// (`SwiftDashSDK/Voting/DecisionProposal.swift`) and rs-dpp.

/**
 * A decision proposal (protocol version 15): a yes/no question an identity put to the
 * masternodes, a `proposal` document of the decision proposals system contract.
 *
 * The proposal passes when the yes voting power is strictly more than [passPercent] percent
 * of yes plus no, and yes plus no reaches [minimumTurnoutPercent] percent of the masternode
 * list's total voting power when voting closes, rounded up. The title, description and url
 * can be edited for a day after the proposal is created; the rule and [endDateMs] never
 * change.
 */
data class DecisionProposal(
    /** The proposal's id, base58. */
    val id: String,
    /** The proposer's identity id, base58. */
    val ownerId: String,
    /** The document revision; null when the document does not carry one. */
    val revision: Long?,
    /** When the proposal was created, in milliseconds since the Unix epoch; may be null. */
    val createdAtMs: Long?,
    /** When the proposal was last changed, in milliseconds since the Unix epoch; may be null. */
    val updatedAtMs: Long?,
    /** The question, in one line. */
    val title: String,
    /** What a yes and a no mean. */
    val description: String,
    /** Where the proposal is discussed; null when it has none. */
    val url: String?,
    /** The share of yes plus no, in percent, the yes voting power must strictly exceed. */
    val passPercent: Int,
    /** The least yes plus no, in percent of the total voting power, the proposal needs. */
    val minimumTurnoutPercent: Int,
    /** When voting closes, in milliseconds since the Unix epoch. */
    val endDateMs: Long,
) {
    internal companion object {
        /** Decode the proposal object every decision proposal read returns. */
        fun fromJson(json: String): DecisionProposal =
            fromJsonObject(DecisionProposalJson.parseObject(json, "decision proposal"))

        /** Decode a listing: a JSON array of proposal objects, in the listing's order. */
        fun listFromJson(json: String): List<DecisionProposal> {
            val array = DecisionProposalJson.parse(json, "decision proposal listing") as? JsonArray
                ?: throw DashSdkError.SerializationError("Malformed decision proposal listing: $json")
            return array.map { entry ->
                fromJsonObject(
                    entry as? JsonObject
                        ?: throw DashSdkError.SerializationError("Malformed decision proposal: $entry"),
                )
            }
        }

        private fun fromJsonObject(json: JsonObject): DecisionProposal = with(DecisionProposalJson) {
            DecisionProposal(
                id = json.string("id"),
                ownerId = json.string("ownerId"),
                revision = json.optionalLong("revision"),
                createdAtMs = json.optionalLong("createdAt"),
                updatedAtMs = json.optionalLong("updatedAt"),
                title = json.string("title"),
                description = json.string("description"),
                url = json.optionalString("url"),
                passPercent = json.int("passPercent"),
                minimumTurnoutPercent = json.int("minimumTurnoutPercent"),
                endDateMs = json.long("endDate"),
            )
        }
    }
}

/**
 * What a new decision proposal asks and the rule it is decided by, the input of
 * [DecisionProposalTransactions.create]. Kotlin mirror of the C
 * `DashSDKDecisionProposalParams` and Swift's `DecisionProposalParams`. Nothing is checked
 * here: Platform refuses a title of more than 128 characters, a description of more than
 * 4096, a [passPercent] outside 50 to 80, a [minimumTurnoutPercent] above 50 and an
 * [endDateMs] less than a week or more than four weeks after the block that creates the
 * proposal.
 */
data class DecisionProposalParams(
    /** The question, in one line. */
    val title: String,
    /** What a yes and a no mean. */
    val description: String,
    /** Where the proposal is discussed; null for none. */
    val url: String? = null,
    /** The share of yes plus no, in percent, the yes voting power must strictly exceed. */
    val passPercent: Int,
    /** The least yes plus no, in percent of the total voting power, the proposal needs. */
    val minimumTurnoutPercent: Int,
    /** When voting closes, in milliseconds since the Unix epoch. */
    val endDateMs: Long,
)

/**
 * New text for a stored decision proposal, the input of [DecisionProposalTransactions.edit].
 * A field left null keeps its value. Kotlin mirror of Swift's `DecisionProposalEdit`; rs-sdk
 * refuses an edit that changes nothing, and Platform refuses one more than a day after the
 * proposal was created.
 */
data class DecisionProposalEdit(
    /** The new title; null keeps it. */
    val title: String? = null,
    /** The new description; null keeps it. */
    val description: String? = null,
    /** What happens to the url. */
    val url: UrlChange = UrlChange.Keep,
) {
    /** What an edit does to a proposal's url. */
    sealed interface UrlChange {
        /** The url stays as it is. */
        data object Keep : UrlChange

        /** The url becomes [url]. */
        data class Set(val url: String) : UrlChange

        /** The proposal no longer has a url. */
        data object Remove : UrlChange
    }
}

/**
 * A masternode's answer on a decision proposal. [discriminant] is the `vote_choice` byte of
 * `dash_sdk_decision_proposal_cast_vote` (`0` yes, `1` no, `2` abstain); the header declares
 * no enum, so the three values are mirrored here as Swift's `YesNoAbstainVoteChoice` mirrors
 * them. [jsonName] is how the masternode answer JSON names the choice.
 */
enum class YesNoAbstainVoteChoice(val discriminant: Int, val jsonName: String) {
    YES(0, "yes"),
    NO(1, "no"),
    ABSTAIN(2, "abstain"),
    ;

    internal companion object {
        fun fromJsonName(name: String): YesNoAbstainVoteChoice =
            entries.firstOrNull { it.jsonName == name }
                ?: throw DashSdkError.SerializationError("Unknown yes/no vote choice: $name")
    }
}

/** A masternode's answer on a decision proposal's poll. */
data class MasternodeYesNoVote(
    /** The answer it gave last. */
    val choice: YesNoAbstainVoteChoice,
    /** How many times it has voted on the poll, the first vote included. */
    val timesVoted: Int,
) {
    internal companion object {
        /** Decode `{"choice": "yes" | "no" | "abstain", "timesVoted": 2}`. */
        fun fromJson(json: String): MasternodeYesNoVote = with(DecisionProposalJson) {
            val obj = parseObject(json, "masternode vote")
            MasternodeYesNoVote(
                choice = YesNoAbstainVoteChoice.fromJsonName(obj.string("choice")),
                timesVoted = obj.int("timesVoted"),
            )
        }
    }
}

/** A block, as a poll records when it opened and when it closed. */
data class PlatformBlockInfo(
    val height: Long,
    val coreHeight: Long,
    /** The block time, in milliseconds since the Unix epoch. */
    val timeMs: Long,
    val epoch: Int,
)

/** Which way a fraction of the total voting power rounds to a whole voting power (rs-dpp). */
enum class VotingPowerRounding(val jsonName: String) {
    /** Toward zero. */
    DOWN("down"),

    /** Away from zero. */
    UP("up"),

    /** Rounded down, plus one. */
    DOWN_PLUS_ONE("downPlusOne"),

    /** Rounded up, minus one, never below zero. */
    UP_MINUS_ONE("upMinusOne"),
}

/** The least yes plus no voting power a yes/no poll needs to pass (rs-dpp). */
sealed interface YesNoMinimumVotingPower {
    /** A fixed voting power. */
    data class Absolute(val votingPower: Long) : YesNoMinimumVotingPower

    /**
     * A share of the masternode list's total voting power in the block that closes the
     * poll, rounded as [rounding] says.
     */
    data class FractionOfTotal(
        val numerator: Int,
        val denominator: Int,
        val rounding: VotingPowerRounding,
    ) : YesNoMinimumVotingPower
}

/** What a yes/no poll decides (rs-dpp `YesNoVotePollPurpose`). */
sealed interface YesNoVotePollPurpose {
    /** The poll a decision proposal opens; [documentId] is the proposal's id, base58. */
    data class DecisionProposal(val documentId: String) : YesNoVotePollPurpose
}

/** A yes/no poll as it was opened, rule included (rs-dpp `YesNoVotePoll`). */
data class YesNoVotePoll(
    val purpose: YesNoVotePollPurpose,
    /** With [supermajorityDenominator], the share of yes plus no the yes power must exceed. */
    val supermajorityNumerator: Int,
    val supermajorityDenominator: Int,
    val minimumVotingPower: YesNoMinimumVotingPower,
)

/** The record a finished yes/no poll keeps (rs-dpp `YesNoVotePollResult`). */
data class YesNoVotePollResult(
    val passed: Boolean,
    val yesVotingPower: Long,
    val noVotingPower: Long,
    val abstainVotingPower: Long,
    /** The yes plus no voting power the poll needed, resolved in the block that closed it. */
    val requiredVotingPower: Long,
    val startBlock: PlatformBlockInfo,
    val finalizationBlock: PlatformBlockInfo,
)

/** Whether a yes/no poll is open or has finished. */
sealed interface YesNoVotePollStatus {
    /** Voting is open; the poll opened in [startBlock]. */
    data class Started(val startBlock: PlatformBlockInfo) : YesNoVotePollStatus

    /** The poll has finished, with its record. */
    data class Finished(val result: YesNoVotePollResult) : YesNoVotePollStatus
}

/**
 * The status and tallies of a decision proposal's yes/no poll
 * (`dash_sdk_decision_proposal_get_vote_poll_state`). Voting powers count a masternode 1
 * and an evonode 4.
 */
data class YesNoVotePollState(
    /** The poll as it was opened, rule included. */
    val votePoll: YesNoVotePoll,
    val status: YesNoVotePollStatus,
    /** The yes voting power so far; 0 once the poll finished (read its result then). */
    val yesVotingPower: Long,
    /** The no voting power so far; 0 once the poll finished. */
    val noVotingPower: Long,
    /** The abstaining voting power so far; 0 once the poll finished. */
    val abstainVotingPower: Long,
) {
    /** The poll's record once it finished; null while it is open. */
    val result: YesNoVotePollResult?
        get() = (status as? YesNoVotePollStatus.Finished)?.result

    internal companion object {
        /** Decode the poll state object. */
        fun fromJson(json: String): YesNoVotePollState = with(DecisionProposalJson) {
            val obj = parseObject(json, "yes/no vote poll state")
            val status = when (val name = obj.string("status")) {
                "started" -> YesNoVotePollStatus.Started(block(obj.obj("startBlock")))
                "finished" -> YesNoVotePollStatus.Finished(result(obj.obj("result")))
                else -> throw DashSdkError.SerializationError("Unknown yes/no vote poll status: $name")
            }
            YesNoVotePollState(
                votePoll = votePoll(obj.obj("votePoll")),
                status = status,
                yesVotingPower = obj.long("yesVotingPower"),
                noVotingPower = obj.long("noVotingPower"),
                abstainVotingPower = obj.long("abstainVotingPower"),
            )
        }

        private fun votePoll(obj: JsonObject): YesNoVotePoll = with(DecisionProposalJson) {
            YesNoVotePoll(
                purpose = purpose(obj.obj("purpose")),
                supermajorityNumerator = obj.int("supermajorityNumerator"),
                supermajorityDenominator = obj.int("supermajorityDenominator"),
                minimumVotingPower = minimumVotingPower(obj.obj("minimumVotingPower")),
            )
        }

        private fun purpose(obj: JsonObject): YesNoVotePollPurpose = with(DecisionProposalJson) {
            when (val type = obj.string("\$type")) {
                "decisionProposal" -> YesNoVotePollPurpose.DecisionProposal(obj.string("documentId"))
                else -> throw DashSdkError.SerializationError("Unknown yes/no vote poll purpose: $type")
            }
        }

        private fun minimumVotingPower(obj: JsonObject): YesNoMinimumVotingPower =
            with(DecisionProposalJson) {
                when (val type = obj.string("\$type")) {
                    "absolute" -> YesNoMinimumVotingPower.Absolute(obj.long("votingPower"))
                    "fractionOfTotal" -> YesNoMinimumVotingPower.FractionOfTotal(
                        numerator = obj.int("numerator"),
                        denominator = obj.int("denominator"),
                        rounding = obj.string("rounding").let { name ->
                            VotingPowerRounding.entries.firstOrNull { it.jsonName == name }
                                ?: throw DashSdkError.SerializationError(
                                    "Unknown voting power rounding: $name",
                                )
                        },
                    )
                    else -> throw DashSdkError.SerializationError(
                        "Unknown minimum voting power: $type",
                    )
                }
            }

        private fun result(obj: JsonObject): YesNoVotePollResult = with(DecisionProposalJson) {
            YesNoVotePollResult(
                passed = obj.boolean("passed"),
                yesVotingPower = obj.long("yesVotingPower"),
                noVotingPower = obj.long("noVotingPower"),
                abstainVotingPower = obj.long("abstainVotingPower"),
                requiredVotingPower = obj.long("requiredVotingPower"),
                startBlock = block(obj.obj("startBlock")),
                finalizationBlock = block(obj.obj("finalizationBlock")),
            )
        }

        private fun block(obj: JsonObject): PlatformBlockInfo = with(DecisionProposalJson) {
            PlatformBlockInfo(
                height = obj.long("height"),
                coreHeight = obj.long("coreHeight"),
                timeMs = obj.long("timeMs"),
                epoch = obj.int("epoch"),
            )
        }
    }
}

/**
 * Strict field readers for the decision proposal JSON: a missing or mistyped field is a
 * [DashSdkError.SerializationError] naming it, never a default value standing in for what
 * Rust wrote.
 */
internal object DecisionProposalJson {
    fun parse(json: String, what: String): JsonElement = try {
        Json.parseToJsonElement(json)
    } catch (e: SerializationException) {
        throw DashSdkError.SerializationError("Malformed $what JSON: $json", e)
    }

    fun parseObject(json: String, what: String): JsonObject =
        parse(json, what) as? JsonObject
            ?: throw DashSdkError.SerializationError("Malformed $what JSON: $json")

    fun JsonObject.obj(name: String): JsonObject =
        this[name] as? JsonObject ?: throw malformed(name)

    fun JsonObject.string(name: String): String =
        (this[name] as? JsonPrimitive)?.takeIf { it.isString }?.content ?: throw malformed(name)

    fun JsonObject.optionalString(name: String): String? = when (val value = this[name]) {
        null, JsonNull -> null
        is JsonPrimitive -> if (value.isString) value.content else throw malformed(name)
        else -> throw malformed(name)
    }

    fun JsonObject.long(name: String): Long = number(this[name]) ?: throw malformed(name)

    fun JsonObject.optionalLong(name: String): Long? = when (val value = this[name]) {
        null, JsonNull -> null
        else -> number(value) ?: throw malformed(name)
    }

    fun JsonObject.int(name: String): Int {
        val value = long(name)
        if (value < Int.MIN_VALUE || value > Int.MAX_VALUE) throw malformed(name)
        return value.toInt()
    }

    fun JsonObject.boolean(name: String): Boolean =
        (this[name] as? JsonPrimitive)?.takeUnless { it.isString }?.booleanOrNull
            ?: throw malformed(name)

    private fun number(value: JsonElement?): Long? =
        (value as? JsonPrimitive)?.takeUnless { it.isString || it is JsonNull }?.longOrNull

    private fun malformed(name: String) =
        DashSdkError.SerializationError("Decision proposal JSON has no valid \"$name\"")
}
