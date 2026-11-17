package org.dashfoundation.dashsdk.voting

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import org.dashfoundation.dashsdk.errors.DashSdkError

// The types the multiple choice proposal reads return, decoded from the JSON rs-sdk-ffi
// writes (`packages/rs-sdk-ffi/src/decision_proposal/multiple_choice/json.rs`), and the
// inputs of the writes. Rust decides every value; these types only carry it. Names and
// shapes follow the Swift SDK (`SwiftDashSDK/Voting/MultipleChoiceProposal.swift`) and
// rs-sdk.

/**
 * A multiple choice proposal (protocol version 15): a question an identity put to the
 * masternodes whose answers are the choices entered for it, a `multipleChoiceProposal`
 * document of the decision proposals system contract.
 *
 * The choices contend in a masternode vote the first choice opens; the choice with the most
 * votes is kept as an ordinary document, unless the votes for none of the choices outnumber
 * it. The title, description document and url can be edited for two hours after the
 * proposal is created; [open] never changes. A proposal has a description document, a url
 * or both.
 */
data class MultipleChoiceProposal(
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
    /** The document saying what is being decided; null when the proposal points at none. */
    val descriptionDocument: DescriptionDocument?,
    /** Where what is being decided is described or discussed; null when it has none. */
    val url: String?,
    /** Whether anyone may enter choices; only the proposal's owner may when it is false. */
    val open: Boolean,
) {
    /**
     * The document a multiple choice proposal points at for what is being decided, as the
     * reads return it. Platform refuses one that is not of a moderated type the elected
     * team of its contract may delete.
     */
    data class DescriptionDocument(
        /** The document's id, base58. */
        val id: String,
        /** The id of the contract it lives in, base58. */
        val contractId: String,
        /** Its document type. */
        val documentType: String,
    )

    internal companion object {
        /** Decode the proposal object every multiple choice proposal read returns. */
        fun fromJson(json: String): MultipleChoiceProposal = fromJsonObject(
            DecisionProposalJson.parseObject(json, "multiple choice proposal"),
        )

        /** Decode a listing: a JSON array of proposal objects, in the listing's order. */
        fun listFromJson(json: String): List<MultipleChoiceProposal> {
            val array = DecisionProposalJson.parse(json, "multiple choice proposal listing")
                as? JsonArray
                ?: throw DashSdkError.SerializationError(
                    "Malformed multiple choice proposal listing: $json",
                )
            return array.map { entry ->
                fromJsonObject(
                    entry as? JsonObject
                        ?: throw DashSdkError.SerializationError(
                            "Malformed multiple choice proposal: $entry",
                        ),
                )
            }
        }

        private fun fromJsonObject(json: JsonObject): MultipleChoiceProposal =
            with(DecisionProposalJson) {
                MultipleChoiceProposal(
                    id = json.string("id"),
                    ownerId = json.string("ownerId"),
                    revision = json.optionalLong("revision"),
                    createdAtMs = json.optionalLong("createdAt"),
                    updatedAtMs = json.optionalLong("updatedAt"),
                    title = json.string("title"),
                    descriptionDocument = json.optionalObj("descriptionDocument")?.let { described ->
                        DescriptionDocument(
                            id = described.string("id"),
                            contractId = described.string("contractId"),
                            documentType = described.string("documentType"),
                        )
                    },
                    url = json.optionalString("url"),
                    open = json.boolean("open"),
                )
            }
    }
}

/**
 * An answer entered for a multiple choice proposal, a `choice` document of the decision
 * proposals system contract. A choice is never edited. Its [id] is what a masternode vote
 * for it names ([MultipleChoiceVoteChoice.Choice]), not the id of the identity that entered
 * it.
 */
data class ProposalChoice(
    /** The choice's document id, base58. */
    val id: String,
    /** The identity that entered it, base58. */
    val ownerId: String,
    /** The proposal it answers, base58. */
    val proposalId: String,
    /** The document revision; null when the document does not carry one. */
    val revision: Long?,
    /** When the choice was entered, in milliseconds since the Unix epoch; may be null. */
    val createdAtMs: Long?,
    /** Null when the document does not carry it. */
    val updatedAtMs: Long?,
    /** The answer, in one line. */
    val title: String,
    /** What voting for the answer means; null when the choice has none. */
    val description: String?,
) {
    internal companion object {
        /** Decode the choice object every read of a choice returns. */
        fun fromJson(json: String): ProposalChoice =
            fromJsonObject(DecisionProposalJson.parseObject(json, "choice"))

        fun fromJsonObject(json: JsonObject): ProposalChoice = with(DecisionProposalJson) {
            ProposalChoice(
                id = json.string("id"),
                ownerId = json.string("ownerId"),
                proposalId = json.string("proposalId"),
                revision = json.optionalLong("revision"),
                createdAtMs = json.optionalLong("createdAt"),
                updatedAtMs = json.optionalLong("updatedAt"),
                title = json.string("title"),
                description = json.optionalString("description"),
            )
        }
    }
}

/**
 * The vote among a multiple choice proposal's choices
 * (`dash_sdk_multiple_choice_proposal_get_vote`): every choice with its tally while the vote
 * runs, and how it ended with every choice's final tally once it did.
 */
data class MultipleChoiceProposalVote(
    /** The choices, ordered by choice id; empty for a proposal no choice was entered for. */
    val choices: List<ChoiceInVote>,
    /** The abstaining votes; null when Platform does not report them. */
    val abstainVoteTally: Long?,
    /** The votes for none of the choices; null when Platform does not report them. */
    val noneOfTheChoicesVoteTally: Long?,
    /** How the vote ended; null while it runs. */
    val result: Result?,
) {
    /** One choice in the vote. */
    data class ChoiceInVote(
        /** The choice's document id, base58: the id a vote for it names. */
        val id: String,
        /**
         * The choice, while the vote runs. Null once it ended: the choices not kept are
         * gone, and the kept one is read with [MultipleChoiceProposals.keptChoice].
         */
        val choice: ProposalChoice?,
        /**
         * The votes towards the choice: so far while the vote runs, final once it ended.
         * Null when Platform does not report it.
         */
        val voteTally: Long?,
    )

    /** How a finished vote ended. [jsonName] is how the vote JSON names it. */
    enum class Outcome(val jsonName: String) {
        /** The choice [Result.keptChoiceId] names was kept as an ordinary document. */
        KEPT_CHOICE("keptChoice"),

        /** The votes for none of the choices outnumbered the leading choice; none was kept. */
        NONE_OF_THE_CHOICES("noneOfTheChoices"),

        /** No choice won; none was kept. */
        NO_WINNER("noWinner"),
        ;

        internal companion object {
            fun fromJsonName(name: String): Outcome =
                entries.firstOrNull { it.jsonName == name }
                    ?: throw DashSdkError.SerializationError(
                        "Unknown multiple choice vote outcome: $name",
                    )
        }
    }

    /** The record of a finished vote. */
    data class Result(
        val outcome: Outcome,
        /** The kept choice's document id, base58; null unless [outcome] is [Outcome.KEPT_CHOICE]. */
        val keptChoiceId: String?,
        /** The block that ended the vote. */
        val finalizationBlock: PlatformBlockInfo,
    )

    internal companion object {
        /** Decode the vote object. */
        fun fromJson(json: String): MultipleChoiceProposalVote = with(DecisionProposalJson) {
            val obj = parseObject(json, "multiple choice proposal vote")
            MultipleChoiceProposalVote(
                choices = obj.objects("choices").map { entry ->
                    ChoiceInVote(
                        id = entry.string("id"),
                        choice = entry.optionalObj("choice")?.let(ProposalChoice::fromJsonObject),
                        voteTally = entry.optionalLong("voteTally"),
                    )
                },
                abstainVoteTally = obj.optionalLong("abstainVoteTally"),
                noneOfTheChoicesVoteTally = obj.optionalLong("noneOfTheChoicesVoteTally"),
                result = obj.optionalObj("result")?.let { result ->
                    Result(
                        outcome = Outcome.fromJsonName(result.string("outcome")),
                        keptChoiceId = result.optionalString("keptChoiceId"),
                        finalizationBlock = block(result.obj("finalizationBlock")),
                    )
                },
            )
        }
    }
}

/**
 * A masternode's answer in the vote among a multiple choice proposal's choices: what
 * [MultipleChoiceProposals.castVote] takes and [MultipleChoiceProposals.masternodeVote]
 * returns. Kotlin mirror of Swift's and rs-dpp's `MultipleChoiceVoteChoice`.
 */
sealed class MultipleChoiceVoteChoice {
    /**
     * A vote for the choice whose document id is [choiceId], base58 (not the id of the
     * identity that entered it).
     */
    data class Choice(val choiceId: String) : MultipleChoiceVoteChoice()

    /** A vote that no choice be kept: none is when these outnumber the leading choice. */
    data object NoneOfTheChoices : MultipleChoiceVoteChoice()

    /** An abstention, recorded but playing no part in the result. */
    data object Abstain : MultipleChoiceVoteChoice()

    internal companion object {
        /**
         * Decode `{"choice": "choice", "choiceId": "<base58>"}` or
         * `{"choice": "noneOfTheChoices" | "abstain", "choiceId": null}`.
         */
        fun fromJson(json: String): MultipleChoiceVoteChoice = with(DecisionProposalJson) {
            val obj = parseObject(json, "masternode multiple choice vote")
            when (val name = obj.string("choice")) {
                "choice" -> Choice(obj.string("choiceId"))
                "noneOfTheChoices" -> NoneOfTheChoices
                "abstain" -> Abstain
                else -> throw DashSdkError.SerializationError(
                    "Unknown multiple choice vote choice: $name",
                )
            }
        }
    }
}

/**
 * How `dash_sdk_multiple_choice_proposal_cast_vote` takes a [MultipleChoiceVoteChoice]: the
 * `vote_choice` byte ([discriminant]: `0` a choice, `1` none of the choices, `2` abstain) and
 * the `choice_id`, null for the answers that name none. The header declares no enum, so the
 * three values are mirrored here.
 */
internal data class NativeMultipleChoiceVote(val discriminant: Int, val choiceId: String?)

/** This answer as the `vote_choice` and `choice_id` of the cast vote. */
internal fun MultipleChoiceVoteChoice.toNative(): NativeMultipleChoiceVote = when (this) {
    is MultipleChoiceVoteChoice.Choice -> NativeMultipleChoiceVote(discriminant = 0, choiceId = choiceId)
    MultipleChoiceVoteChoice.NoneOfTheChoices -> NativeMultipleChoiceVote(discriminant = 1, choiceId = null)
    MultipleChoiceVoteChoice.Abstain -> NativeMultipleChoiceVote(discriminant = 2, choiceId = null)
}

/**
 * The document a new or edited multiple choice proposal points at, as the wallet writes take
 * it: the ids as their 32 bytes, like the other ids of
 * [MultipleChoiceProposalTransactions]. Kotlin mirror of the C `DescriptionDocumentFFI` and
 * Swift's `DescriptionDocumentParams`. Nothing is checked here.
 */
class DescriptionDocumentParams(
    /** The document's 32-byte id. */
    val id: ByteArray,
    /** The 32-byte id of the contract it lives in. */
    val contractId: ByteArray,
    /** Its document type. */
    val documentType: String,
) {
    override fun equals(other: Any?): Boolean =
        this === other ||
            other is DescriptionDocumentParams &&
            id.contentEquals(other.id) &&
            contractId.contentEquals(other.contractId) &&
            documentType == other.documentType

    override fun hashCode(): Int =
        31 * (31 * id.contentHashCode() + contractId.contentHashCode()) + documentType.hashCode()

    override fun toString(): String =
        "DescriptionDocumentParams(id=${id.toHexString()}, " +
            "contractId=${contractId.toHexString()}, documentType=$documentType)"

    private fun ByteArray.toHexString(): String = joinToString("") { "%02x".format(it) }
}

/**
 * What a new multiple choice proposal asks, the input of
 * [MultipleChoiceProposalTransactions.create]. Kotlin mirror of the C
 * `MultipleChoiceProposalParamsFFI` and Swift's `MultipleChoiceProposalParams`. Nothing is
 * checked here: Platform refuses a title of
 * more than 128 characters, a url of more than 512, and a proposal that gives neither a
 * description document nor a url.
 */
data class MultipleChoiceProposalParams(
    /** The question, in one line. */
    val title: String,
    /** The document saying what is being decided; null for none. */
    val descriptionDocument: DescriptionDocumentParams? = null,
    /**
     * Where what is being decided is described, for a proposal that points at no description
     * document, or where it is discussed, for one that does; null for none.
     */
    val url: String? = null,
    /**
     * Whether anyone may enter choices; only the proposal's owner may when it is false. It
     * never changes.
     */
    val open: Boolean,
)

/**
 * What a stored multiple choice proposal changes to, the input of
 * [MultipleChoiceProposalTransactions.edit]. A field left as it is here keeps its value.
 * Kotlin mirror of the C `MultipleChoiceProposalEditFFI` and Swift's
 * `MultipleChoiceProposalEdit`; rs-sdk refuses an edit that changes nothing, and Platform
 * one more than two hours after the proposal was created and one that leaves it with
 * neither a description document nor a url.
 */
data class MultipleChoiceProposalEdit(
    /** The new title; null keeps it. */
    val title: String? = null,
    /** What happens to the description document. */
    val descriptionDocument: Change<DescriptionDocumentParams> = Change.Keep,
    /** What happens to the url. */
    val url: Change<String> = Change.Keep,
) {
    /** What an edit does to a field a proposal may not have. */
    sealed interface Change<out T> {
        /** The field stays as it is. */
        data object Keep : Change<Nothing>

        /** The field becomes [value]. */
        data class Set<T>(val value: T) : Change<T>

        /** The proposal no longer has the field. */
        data object Remove : Change<Nothing>
    }
}

/**
 * How `MultipleChoiceProposalEditFFI` says a [MultipleChoiceProposalEdit.Change]: the new
 * [value], or null with [remove] set to remove the field, or null without it to keep it.
 */
internal data class NativeChange<T>(val value: T?, val remove: Boolean)

/** This change as a field and its `remove_` flag of `MultipleChoiceProposalEditFFI`. */
internal fun <T> MultipleChoiceProposalEdit.Change<T>.toNative(): NativeChange<T> = when (this) {
    MultipleChoiceProposalEdit.Change.Keep -> NativeChange(value = null, remove = false)
    is MultipleChoiceProposalEdit.Change.Set -> NativeChange(value = value, remove = false)
    MultipleChoiceProposalEdit.Change.Remove -> NativeChange(value = null, remove = true)
}

/**
 * An answer to enter for a multiple choice proposal, the input of
 * [MultipleChoiceProposalTransactions.enterChoice]. Kotlin mirror of the C `ChoiceParamsFFI`
 * and Swift's `ProposalChoiceParams`. Nothing is checked here: Platform refuses a title of
 * more than 128 characters and a description of more than 4096.
 */
data class ProposalChoiceParams(
    /** The answer, in one line. */
    val title: String,
    /** What voting for the answer means; null for none. */
    val description: String? = null,
)

/**
 * A multiple choice proposal Platform confirmed creating, as
 * [MultipleChoiceProposalTransactions.create] returns it.
 */
data class CreatedMultipleChoiceProposal(
    /** The proposal's id, base58: the id the reads of `Sdk.multipleChoiceProposals` take. */
    val id: String,
    /**
     * The confirmed document's canonical JSON, the shape the platform wallet's document
     * create returns.
     */
    val documentJson: String,
)

/**
 * A choice Platform confirmed entering, as [MultipleChoiceProposalTransactions.enterChoice]
 * returns it.
 */
data class EnteredProposalChoice(
    /** The choice's document id, base58: the id a masternode vote for it names. */
    val id: String,
    /**
     * The confirmed document's canonical JSON, the shape the platform wallet's document
     * create returns.
     */
    val documentJson: String,
)

/**
 * The credits a multiple choice proposal amount read returns: rs-sdk-ffi writes them as a
 * decimal string, not JSON. Anything else is a [DashSdkError.SerializationError].
 */
internal fun creditsFromDecimal(decimal: String): ULong = decimal.toULongOrNull()
    ?: throw DashSdkError.SerializationError("Malformed credit amount: $decimal")
