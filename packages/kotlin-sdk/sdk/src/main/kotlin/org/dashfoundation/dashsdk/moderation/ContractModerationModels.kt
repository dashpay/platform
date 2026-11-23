package org.dashfoundation.dashsdk.moderation

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

// The values the contract moderation and fee pot reads return (protocol version 14), decoded
// from the JSON rs-sdk-ffi writes (`packages/rs-sdk-ffi/src/contract_moderation/json.rs`,
// whose field names follow the wasm bindings). Names and shapes follow the Swift SDK
// (`SwiftDashSDK/ContractModeration/ContractModeration.swift`). Identifiers are base58
// strings, times are milliseconds since the Unix epoch, credits are whole credits. Nothing
// here decides anything about moderation: a field absent in the JSON is null here.

/**
 * One of the moderation lists a moderated contract may keep.
 *
 * [jsonName] is the name the JSON writes; [ffiTag] is the byte the FFI takes, which must
 * agree with `ContractModerationListFFI` in `packages/rs-sdk-ffi/src/contract_moderation/mod.rs`.
 */
enum class ContractModerationList(val jsonName: String, internal val ffiTag: Int) {
    BANLIST("banlist", 0),
    SUSPENSIONS("suspensions", 1),
    WARNINGS("warnings", 2),
    ;

    internal companion object {
        fun fromJsonName(name: String): ContractModerationList =
            entries.firstOrNull { it.jsonName == name }
                ?: throw DashSdkError.SerializationError("Unknown moderation list \"$name\"")
    }
}

/**
 * A team action's status: active while its approvals fall short of its rule, closed once
 * they met it and the action ran. [ffiTag] must agree with `ContractTeamActionStatusFFI` in
 * `packages/rs-sdk-ffi/src/contract_moderation/mod.rs`.
 */
enum class ContractTeamActionStatus(val jsonName: String, internal val ffiTag: Int) {
    ACTIVE("active", 0),
    CLOSED("closed", 1),
}

/** A document a moderation reason is about. */
data class ContractModerationDocument(
    /** The document type of the document, on the moderated contract. */
    val documentTypeName: String,
    /** The document's id, base58. */
    val documentId: String,
)

/**
 * Why a moderator banned, suspended or warned an identity, or deleted a document, as the
 * moderator wrote it. Nothing checks what it says.
 */
data class ContractModerationReason(
    /** Reserved for ban codes; null when there is none. */
    val code: Int?,
    /** Free text; may be empty. */
    val text: String,
    /** The documents the reason is about; null when it names none. */
    val documents: List<ContractModerationDocument>?,
    /** The `reason` document of the moderation charters contract, base58; null when none. */
    val reasonDocumentId: String?,
)

/** One warning an identity carries. */
data class ContractWarning(
    /** The time of the block that issued it, milliseconds. */
    val warnedAtMs: Long,
    /** Why, as the moderator wrote it. */
    val reason: ContractModerationReason,
)

/**
 * One identity's status on the lists of a moderated contract that were read. A list that was
 * not read says nothing: its fields are null, which is not "not banned".
 */
data class ContractModerationStatus(
    /** The lists this status covers. */
    val lists: List<ContractModerationList>,
    /** Set when [lists] includes the banlist. */
    val banned: Boolean?,
    /** Why the identity is banned, when it is. */
    val banReason: ContractModerationReason?,
    /**
     * When [lists] includes the suspension list and the identity is suspended: the block time
     * until which it is, milliseconds. A lapsed suspension stays until it is swept.
     */
    val suspendedUntilMs: Long?,
    /** Why the identity is suspended, when it is. */
    val suspensionReason: ContractModerationReason?,
    /** When [lists] includes the warning list: its warnings, oldest first, empty for none. */
    val warnings: List<ContractWarning>?,
)

/** One entry of a moderation list. */
data class ContractModerationEntry(
    /** The identity on the list, base58. */
    val identityId: String,
    /** For a suspension list entry: when it lapses, milliseconds. */
    val untilMs: Long?,
    /**
     * Why the identity is on the list: the ban's reason, the suspension's, or for a warning
     * list entry the latest warning's.
     */
    val reason: ContractModerationReason,
    /** For a warning list entry: every warning, oldest first. */
    val warnings: List<ContractWarning>?,
)

/** One page of a moderation list, in identity id order. */
data class ContractModerationEntriesPage(
    val entries: List<ContractModerationEntry>,
    /**
     * The cursor of the next page, to pass back as `startAfter`; null on the last page. Null
     * ends paging, but a cursor does not promise another row: a full final page carries one,
     * and the page after it is empty.
     */
    val nextStartAfter: String?,
)

/**
 * A value a removal record keeps of its document, as rs-sdk decoded it, in the tagged form
 * the FFI writes (`{"type": ..., "value": ...}`). The tag carries the platform value's kind,
 * so nothing is inferred from a JSON number: a signed and an unsigned integer of the same
 * magnitude stay apart, and `1.0` and `-0.0` stay doubles, sign included. Kotlin reads the
 * tag; it never decodes a kept value's bytes itself.
 */
sealed class ContractKeptFieldValue {
    data object Null : ContractKeptFieldValue()
    data class Bool(val value: Boolean) : ContractKeptFieldValue()

    /** A signed integer up to 64 bits. */
    data class Int(val value: Long) : ContractKeptFieldValue()

    /** An unsigned integer up to 64 bits. */
    data class UInt(val value: ULong) : ContractKeptFieldValue()

    /** A signed 128-bit value outside `Long`, in decimal. */
    data class Int128(val value: String) : ContractKeptFieldValue()

    /** An unsigned 128-bit value outside `ULong`, in decimal. */
    data class UInt128(val value: String) : ContractKeptFieldValue()
    data class Double(val value: kotlin.Double) : ContractKeptFieldValue()
    data class Text(val value: String) : ContractKeptFieldValue()

    /** An identifier, base58. */
    data class Identifier(val value: String) : ContractKeptFieldValue()

    /** Bytes, base64. */
    data class Bytes(val value: String) : ContractKeptFieldValue()
    data class Array(val values: List<ContractKeptFieldValue>) : ContractKeptFieldValue()
    data class Object(val members: Map<String, ContractKeptFieldValue>) : ContractKeptFieldValue()
}

/** The record a contract keeps of one document a moderator deleted. */
data class ContractDocumentRemovalEntry(
    /** The removed document's id, base58. */
    val documentId: String,
    /** The identity that owned it, base58. */
    val documentOwnerId: String,
    /** The contract owner or moderator that removed it, base58. */
    val moderatorId: String,
    /** Why, as the moderator wrote it. */
    val reason: ContractModerationReason,
    /** The time of the block that removed it, milliseconds. */
    val removedAtMs: Long,
    /** A double SHA-256 of the document as it was, 64 hex characters. */
    val documentHash: String,
    /** The moderator that restored the document, base58; null while the removal stands. */
    val restoredBy: String?,
    /** The time of the block that restored it, milliseconds; null while the removal stands. */
    val restoredAtMs: Long?,
    /**
     * The values the record keeps of the document, by property path, in path order; empty
     * when its type keeps none. They are read under the document type as the contract holds
     * it now: a record stores a kept object's members by position, not by name, so after a
     * protocol 14 update that reordered them (refused from protocol 15) an older record reads
     * under the new order's names.
     */
    val keptFields: Map<String, ContractKeptFieldValue>,
)

/** Removal records of one document type, in document id order. */
data class ContractDocumentRemovalsPage(
    val removals: List<ContractDocumentRemovalEntry>,
    /**
     * The cursor of the next page, to pass back as `startAfter`; null on the last page and
     * for a read by ids. A cursor does not promise another row.
     */
    val nextStartAfter: String?,
)

/** What a team action does once approved. */
sealed class ContractTeamActionEvent {
    /** The deletion of a settled document, which the seated team decides together. */
    data class DeleteSettledDocument(
        /** The document's type. */
        val documentTypeName: String,
        /** The document, base58. */
        val documentId: String,
        /** The document's last modification when proposed, milliseconds. */
        val documentLastModifiedAtMs: Long,
        /** The document's revision when proposed; null on a type whose documents carry none. */
        val documentRevision: Long?,
        /** Why, as the proposer gave it. */
        val reason: ContractModerationReason,
    ) : ContractTeamActionEvent()
}

/** One action an elected contract's seated moderation team votes on. */
data class ContractTeamActionEntry(
    /** The action's id, base58: what the other members approve it by. */
    val actionId: String,
    /** The member that proposed it, base58. */
    val proposerId: String,
    /** The time of the block of the proposal, milliseconds. */
    val proposedAtMs: Long,
    /** What it does once approved. */
    val event: ContractTeamActionEvent,
    /** How many approvals it holds, the proposer's among them; an active action's is an upper bound. */
    val approvalCount: Long,
)

/** One page of team actions, in action id order. */
data class ContractTeamActionsPage(
    val actions: List<ContractTeamActionEntry>,
    /** The cursor of the next page, to pass back as `startAtActionId`; null on the last page. */
    val nextStartAtActionId: String?,
    /**
     * Whether the next page includes the [nextStartAtActionId] action, to pass back as
     * `startAtActionIdIncluded`; set with [nextStartAtActionId].
     */
    val nextStartAtActionIdIncluded: Boolean?,
)

/**
 * The members that approved a team action, in identity id order; empty when the contract
 * holds no action of that id with the status asked.
 */
data class ContractTeamActionSigners(val signerIds: List<String>)

/** How many counted moderation actions one member signed since the last claim or settlement. */
data class ContractModerationActionCount(
    /** The member, base58. */
    val identityId: String,
    /** Its bans, suspensions, warnings and document deletions. */
    val count: Long,
)

/**
 * The moderation action counts of an elected contract's seated team since the moderators pot
 * was last claimed or settled, in identity id order.
 */
data class ContractModerationActionCounts(val counts: List<ContractModerationActionCount>)

/** What one fee pot of a contract holds. */
data class ContractFeePotState(
    /**
     * The credits in the pot: its balance, not a promise of what a claim will distribute.
     * The whole `u64` range, as Rust writes it.
     */
    val credits: ULong,
    /**
     * The epoch of the pot's last explicit claim; null when it was never claimed. Not the
     * last payout: a membership change of an elected team can also pay out the moderators
     * pot (a moderators pot settlement), which leaves these fields as they were.
     */
    val lastClaimEpoch: Int?,
    /** The time of the block of the last explicit claim, milliseconds; set with [lastClaimEpoch]. */
    val lastClaimTimeMs: Long?,
    /** The identity that signed the last claim, base58; set with [lastClaimEpoch]. */
    val lastClaimantId: String?,
)

/** The fee pots of a contract. */
data class ContractFeePots(
    /** The pot the contract owner claims. */
    val owner: ContractFeePotState,
    /** The pot the contract's moderation team shares. */
    val moderators: ContractFeePotState,
)

/**
 * Decodes the JSON the contract moderation FFI functions return. Shared by the wrapper and
 * the JVM decoder tests, so both read the same way. A malformed value throws
 * [DashSdkError.SerializationError].
 */
internal object ContractModerationJson {

    fun status(json: String): ContractModerationStatus = parseObject(json).let { o ->
        ContractModerationStatus(
            lists = o.array("lists").map { element ->
                ContractModerationList.fromJsonName(stringOf(element, "lists"))
            },
            banned = o.optionalBoolean("banned"),
            banReason = o.optionalObj("banReason")?.let(::reason),
            suspendedUntilMs = o.optionalLong("suspendedUntil"),
            suspensionReason = o.optionalObj("suspensionReason")?.let(::reason),
            warnings = o.optionalArray("warnings")?.map { warning(objectOf(it, "warnings")) },
        )
    }

    fun entriesPage(json: String): ContractModerationEntriesPage = parseObject(json).let { o ->
        ContractModerationEntriesPage(
            entries = o.array("entries").map { element ->
                val entry = objectOf(element, "entries")
                ContractModerationEntry(
                    identityId = entry.string("identityId"),
                    untilMs = entry.optionalLong("until"),
                    reason = reason(entry.obj("reason")),
                    warnings = entry.optionalArray("warnings")
                        ?.map { warning(objectOf(it, "warnings")) },
                )
            },
            nextStartAfter = o.optionalString("nextStartAfter"),
        )
    }

    fun removalsPage(json: String): ContractDocumentRemovalsPage = parseObject(json).let { o ->
        ContractDocumentRemovalsPage(
            removals = o.array("removals").map { element ->
                val entry = objectOf(element, "removals")
                ContractDocumentRemovalEntry(
                    documentId = entry.string("documentId"),
                    documentOwnerId = entry.string("documentOwnerId"),
                    moderatorId = entry.string("moderatorId"),
                    reason = reason(entry.obj("reason")),
                    removedAtMs = entry.long("removedAt"),
                    documentHash = entry.string("documentHash"),
                    restoredBy = entry.optionalString("restoredBy"),
                    restoredAtMs = entry.optionalLong("restoredAt"),
                    keptFields = entry.obj("keptFields").mapValues { (_, value) -> keptValue(value) },
                )
            },
            nextStartAfter = o.optionalString("nextStartAfter"),
        )
    }

    fun teamActionsPage(json: String): ContractTeamActionsPage = parseObject(json).let { o ->
        ContractTeamActionsPage(
            actions = o.array("actions").map { element ->
                val entry = objectOf(element, "actions")
                ContractTeamActionEntry(
                    actionId = entry.string("actionId"),
                    proposerId = entry.string("proposerId"),
                    proposedAtMs = entry.long("proposedAt"),
                    event = teamActionEvent(entry.obj("event")),
                    approvalCount = entry.long("approvalCount"),
                )
            },
            nextStartAtActionId = o.optionalString("nextStartAtActionId"),
            nextStartAtActionIdIncluded = o.optionalBoolean("nextStartAtActionIdIncluded"),
        )
    }

    fun teamActionSigners(json: String): ContractTeamActionSigners =
        ContractTeamActionSigners(
            parseObject(json).array("signerIds").map { stringOf(it, "signerIds") },
        )

    fun actionCounts(json: String): ContractModerationActionCounts =
        ContractModerationActionCounts(
            parseObject(json).array("counts").map { element ->
                val count = objectOf(element, "counts")
                ContractModerationActionCount(
                    identityId = count.string("identityId"),
                    count = count.long("count"),
                )
            },
        )

    fun feePots(json: String): ContractFeePots = parseObject(json).let { o ->
        ContractFeePots(owner = feePot(o.obj("owner")), moderators = feePot(o.obj("moderators")))
    }

    // ── Pieces ─────────────────────────────────────────────────────────

    private fun reason(o: JsonObject) = ContractModerationReason(
        code = o.optionalLong("code")?.toIntChecked("code"),
        text = o.string("text"),
        documents = o.optionalArray("documents")?.map { element ->
            val document = objectOf(element, "documents")
            ContractModerationDocument(
                documentTypeName = document.string("documentTypeName"),
                documentId = document.string("documentId"),
            )
        },
        reasonDocumentId = o.optionalString("reasonDocumentId"),
    )

    private fun warning(o: JsonObject) =
        ContractWarning(warnedAtMs = o.long("warnedAt"), reason = reason(o.obj("reason")))

    private fun teamActionEvent(o: JsonObject): ContractTeamActionEvent =
        when (val type = o.string("type")) {
            "deleteSettledDocument" -> ContractTeamActionEvent.DeleteSettledDocument(
                documentTypeName = o.string("documentTypeName"),
                documentId = o.string("documentId"),
                documentLastModifiedAtMs = o.long("documentLastModifiedAt"),
                documentRevision = o.optionalLong("documentRevision"),
                reason = reason(o.obj("reason")),
            )
            else -> throw DashSdkError.SerializationError("Unknown team action event type $type")
        }

    private fun feePot(o: JsonObject) = ContractFeePotState(
        credits = numberContent(o["credits"]).toULongOrNull() ?: throw malformed("credits"),
        lastClaimEpoch = o.optionalLong("lastClaimEpoch")?.toIntChecked("lastClaimEpoch"),
        lastClaimTimeMs = o.optionalLong("lastClaimTimeMs"),
        lastClaimantId = o.optionalString("lastClaimantId"),
    )

    /** One tagged kept value, as rs-sdk-ffi wrote it. */
    internal fun keptValue(element: JsonElement): ContractKeptFieldValue {
        val o = objectOf(element, "keptFields")
        val value = o["value"]
        return when (val type = o.string("type")) {
            "null" -> ContractKeptFieldValue.Null
            "bool" -> ContractKeptFieldValue.Bool(
                (value as? JsonPrimitive)?.takeUnless { it.isString }?.booleanOrNull
                    ?: throw malformed("value"),
            )
            "int" -> ContractKeptFieldValue.Int(numberContent(value).toLongOrNull() ?: throw malformed("value"))
            "uint" -> ContractKeptFieldValue.UInt(numberContent(value).toULongOrNull() ?: throw malformed("value"))
            "int128" -> ContractKeptFieldValue.Int128(o.string("value"))
            "uint128" -> ContractKeptFieldValue.UInt128(o.string("value"))
            "double" -> ContractKeptFieldValue.Double(numberContent(value).toDoubleOrNull() ?: throw malformed("value"))
            "string" -> ContractKeptFieldValue.Text(o.string("value"))
            "identifier" -> ContractKeptFieldValue.Identifier(o.string("value"))
            "bytes" -> ContractKeptFieldValue.Bytes(o.string("value"))
            "array" -> ContractKeptFieldValue.Array(
                (value as? JsonArray ?: throw malformed("value")).map(::keptValue),
            )
            "object" -> ContractKeptFieldValue.Object(
                (value as? JsonObject ?: throw malformed("value")).mapValues { (_, member) -> keptValue(member) },
            )
            else -> throw DashSdkError.SerializationError("Unknown kept field value type $type")
        }
    }

    // ── JSON access ────────────────────────────────────────────────────

    private fun parseObject(json: String): JsonObject {
        val element = try {
            Json.parseToJsonElement(json)
        } catch (e: SerializationException) {
            throw DashSdkError.SerializationError("Malformed contract moderation JSON: $json", e)
        }
        return element as? JsonObject
            ?: throw DashSdkError.SerializationError("Malformed contract moderation JSON: $json")
    }

    private fun objectOf(element: JsonElement, name: String): JsonObject =
        element as? JsonObject ?: throw malformed(name)

    private fun stringOf(element: JsonElement, name: String): String =
        (element as? JsonPrimitive)?.takeIf { it.isString }?.content ?: throw malformed(name)

    /** The literal of a JSON number, as written: never rounded through a Double. */
    private fun numberContent(element: JsonElement?): String =
        (element as? JsonPrimitive)?.takeUnless { it.isString || it is JsonNull }?.content
            ?: throw malformed("value")

    private fun JsonObject.obj(name: String): JsonObject = this[name] as? JsonObject ?: throw malformed(name)

    private fun JsonObject.optionalObj(name: String): JsonObject? = when (val value = this[name]) {
        null, JsonNull -> null
        is JsonObject -> value
        else -> throw malformed(name)
    }

    private fun JsonObject.array(name: String): JsonArray = this[name] as? JsonArray ?: throw malformed(name)

    private fun JsonObject.optionalArray(name: String): JsonArray? = when (val value = this[name]) {
        null, JsonNull -> null
        is JsonArray -> value
        else -> throw malformed(name)
    }

    private fun JsonObject.string(name: String): String =
        this[name]?.let { stringOf(it, name) } ?: throw malformed(name)

    private fun JsonObject.optionalString(name: String): String? = when (val value = this[name]) {
        null, JsonNull -> null
        else -> stringOf(value, name)
    }

    private fun JsonObject.long(name: String): Long = number(this[name]) ?: throw malformed(name)

    private fun JsonObject.optionalLong(name: String): Long? = when (val value = this[name]) {
        null, JsonNull -> null
        else -> number(value) ?: throw malformed(name)
    }

    private fun JsonObject.optionalBoolean(name: String): Boolean? = when (val value = this[name]) {
        null, JsonNull -> null
        else -> (value as? JsonPrimitive)?.takeUnless { it.isString }?.booleanOrNull ?: throw malformed(name)
    }

    private fun number(value: JsonElement?): Long? =
        (value as? JsonPrimitive)?.takeUnless { it.isString || it is JsonNull }?.longOrNull

    private fun Long.toIntChecked(name: String): Int {
        if (this < Int.MIN_VALUE || this > Int.MAX_VALUE) throw malformed(name)
        return toInt()
    }

    private fun malformed(name: String) =
        DashSdkError.SerializationError("Contract moderation JSON has no valid \"$name\"")
}
