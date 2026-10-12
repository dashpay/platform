package org.dashfoundation.dashsdk.queries

import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import org.dashfoundation.dashsdk.errors.DashSdkError

/**
 * A rule of a document type's `propertyConstraints` (protocol version 14): a
 * named condition every created or replaced document's properties must meet.
 * Consensus checks every rule, in name order, and refuses a document breaking
 * one with `DocumentPropertyConstraintViolatedError` (code 10422); a refused
 * state transition is still paid for. A transfer, a purchase or a price update
 * is judged against the rules reading what it changes ([readsOwner],
 * [readsSystem]), `$revision` included.
 *
 * A rule may read the stored version of the document a write replaces
 * through the `$old.` prefix ([readsStored]): `$old.status` is the stored
 * `status`, `$old.$ownerId` the stored owner, and `$old.$updatedAt` or
 * `$old.$revision` a stored system value. A create has no stored version, so
 * there every such read is absent.
 *
 * Rust parses the rules and reports them
 * (`dash_sdk_data_contract_get_property_constraints`, through
 * [Contracts.propertyConstraints]); this type only carries what it reports.
 * The fields mirror wasm-dpp2's `DocumentPropertyConstraint` key for key, and
 * the Swift SDK's `DocumentPropertyConstraint`
 * (`SwiftDashSDK/Core/Utils/DocumentPropertyConstraints.swift`).
 */
data class DocumentPropertyConstraint(
    /** The rule's name, its key in `propertyConstraints`. */
    val name: String,
    /**
     * The rule exactly as the document type's schema declares it, as compact
     * JSON text with sorted keys (every operator object has a single key, so
     * sorting changes nothing a reader would notice). Among its operators:
     * sizes (`{ "length": path }`, `{ "byteLength": path }`,
     * `{ "count": path }`), `{ "countPresent": [path, path, ...] }`, system
     * times, heights and `$revision` as bare operands (`"$createdAt"`),
     * paths of the stored version through `$old.` (`"$old.status"`),
     * `{ "contains": [arrayPath, value] }`,
     * `{ "startsWith": [a, b] }`, `{ "endsWith": [a, b] }`,
     * `{ "notIn": [operand, [values]] }`, `{ "min": [a, b, ...] }`,
     * `{ "max": [a, b, ...] }`, `{ "abs": a }`, `{ "ifThen": [if, then] }`
     * and `{ "ifThenElse": [if, then, else] }`.
     */
    val ruleJson: String,
    /**
     * Every property the rule reads, in declared order, a property read twice
     * listed twice, every branch of an `ifThen` or `ifThenElse` included,
     * whichever one a document takes. A property of the stored version, read
     * through `$old.`, is listed under that path (`$old.status`): see
     * [readsStored]. `$ownerId`, `$old.$ownerId` and the system times,
     * heights and revisions are no properties and are not listed: see
     * [readsOwner] and [readsSystem], which cover every branch too. Nor is a
     * value of a document a reference points at: see [readsReferenced].
     */
    val reads: List<PropertyConstraintRead>,
    /**
     * Whether the rule compares the document's owner, `$ownerId`, or the
     * stored version's owner, `$old.$ownerId`, or reads a total that depends
     * on it ([readsTotals]): then a transfer or a purchase, which changes the
     * owner, is judged against it too.
     */
    val readsOwner: Boolean,
    /**
     * The system values the rule reads, by name, in declared order, one read
     * twice listed twice: `$createdAt`, `$updatedAt` and `$transferredAt` (a
     * block time in milliseconds), each also with `BlockHeight` or
     * `CoreBlockHeight` appended (the Platform or the Core block height), and
     * `$revision` (1 for a created document, one more for each later write of
     * it). The names are wasm-dpp2's `PropertyConstraintSystemProperty`. A
     * name prefixed with `$old.` is the stored version's value
     * (`$old.$updatedAt`, `$old.$revision`): see [readsStored]. A rule reads
     * only the times and heights its document type records, by listing them
     * in `required`, and `$revision` only on a document type that keeps one.
     *
     * Consensus judges a price update against the rules reading the update's
     * (`$updatedAt...`), a transfer or a purchase against those reading the
     * transfer's (`$transferredAt...`) or the owner ([readsOwner]), and each
     * of them against those reading `$revision`, of either version. This list
     * only names what a rule reads; Rust decides which writes a rule answers
     * to.
     *
     * Empty for a rule reading none, and for every rule when the native
     * library predates the field.
     */
    val readsSystem: List<String> = emptyList(),
    /**
     * The `countOf` and `sumOf` totals the rule reads, in declared order, one
     * read twice listed twice: how many documents of a type of the same
     * contract match a filter, or the total of their integer property. The
     * platform reads them from state when the document is sent; the pre-check
     * ([Contracts.checkPropertyConstraints]) reads no state and does not judge
     * a rule reading one.
     *
     * Empty for a rule reading none, and for every rule when the native
     * library predates the field.
     */
    val readsTotals: List<PropertyConstraintTotalRead> = emptyList(),
    /**
     * The values the rule reads of the documents the type's references point
     * at (`{ "referenced": [reference, path] }`), in declared order, one read
     * twice listed twice, every branch included as in [reads], which leaves
     * them out. The platform reads the referenced document when it checks the
     * reference; the pre-check ([Contracts.checkPropertyConstraints]) has no
     * referenced document and does not judge a rule reading one.
     *
     * Empty for a rule reading none, and for every rule when the native
     * library predates the field.
     */
    val readsReferenced: List<PropertyConstraintReferencedRead> = emptyList(),
    /**
     * Whether the rule reads the stored version of the document a write
     * replaces, through `$old.`: one of its properties (listed in [reads]),
     * its owner, `$old.$ownerId` ([readsOwner]), or one of its system values
     * (listed in [readsSystem]).
     *
     * A create has no stored version, so consensus reads every such value as
     * absent there: a property as one the document leaves out (0, or its
     * `ifAbsent` value; absent to `present`; equal to no string or
     * identifier), `$old.$ownerId` as equal to no identifier, and a stored
     * time, height or revision as 0. The pre-check
     * ([Contracts.checkPropertyConstraints]), which judges a document to
     * create, reads them the same way.
     *
     * `false` for a rule reading none, and for every rule when the native
     * library predates the field.
     */
    val readsStored: Boolean = false,
) {
    /** [ruleJson] indented for display, or [ruleJson] itself should it not parse back. */
    val prettyRuleJson: String
        get() = try {
            PropertyConstraintJson.pretty(Json.parseToJsonElement(ruleJson))
        } catch (_: SerializationException) {
            ruleJson
        }

    companion object {
        /**
         * Decode the JSON array `dash_sdk_data_contract_get_property_constraints`
         * returns, keeping its order (name order). A rule without
         * `readsSystem`, `readsStored`, `readsTotals` or `readsReferenced`, from a
         * native library built before the key, reads as reading none of those.
         *
         * @throws DashSdkError.SerializationError for text that is not such an array.
         */
        fun listFromJson(json: String): List<DocumentPropertyConstraint> {
            val entries = PropertyConstraintJson.parse(json) as? JsonArray
                ?: throw DashSdkError.SerializationError(
                    "propertyConstraints rules are not a JSON array",
                )
            return entries.map { entry ->
                val rule = entry as? JsonObject
                val name = rule?.get("name")?.jsonStringOrNull()
                val declaration = rule?.get("rule")
                val reads = rule?.get("reads") as? JsonArray
                val readsOwner = rule?.get("readsOwner")?.jsonBooleanOrNull()
                val readsSystem = rule?.let(::readsSystemOf)
                val readsStored = rule?.let(::readsStoredOf)
                val readsTotals = rule?.let(::readsTotalsOf)
                val readsReferenced = rule?.let(::readsReferencedOf)
                if (name == null || declaration == null || reads == null || readsOwner == null ||
                    readsSystem == null || readsStored == null || readsTotals == null ||
                    readsReferenced == null
                ) {
                    throw DashSdkError.SerializationError("Malformed propertyConstraints rule: $entry")
                }
                DocumentPropertyConstraint(
                    name = name,
                    ruleJson = PropertyConstraintJson.compact(declaration),
                    reads = reads.map(PropertyConstraintRead::fromJson),
                    readsOwner = readsOwner,
                    readsSystem = readsSystem,
                    readsStored = readsStored,
                    readsTotals = readsTotals,
                    readsReferenced = readsReferenced,
                )
            }
        }

        /**
         * The names [rule]'s `readsSystem` lists: empty when the key is
         * missing, `null` when it is anything but an array of strings.
         */
        private fun readsSystemOf(rule: JsonObject): List<String>? =
            listOrNull(rule, "readsSystem") { it.jsonStringOrNull() }

        /**
         * [rule]'s `readsStored`: `false` when the key is missing, `null` when
         * it is anything but a JSON boolean.
         */
        private fun readsStoredOf(rule: JsonObject): Boolean? {
            val value = rule["readsStored"] ?: return false
            return value.jsonBooleanOrNull()
        }

        /**
         * The totals [rule]'s `readsTotals` lists: empty when the key is
         * missing, `null` when it or an entry is malformed.
         */
        private fun readsTotalsOf(rule: JsonObject): List<PropertyConstraintTotalRead>? =
            listOrNull(rule, "readsTotals", PropertyConstraintTotalRead::fromJsonOrNull)

        /**
         * The reads [rule]'s `readsReferenced` lists: empty when the key is
         * missing, `null` when it or an entry is malformed.
         */
        private fun readsReferencedOf(rule: JsonObject): List<PropertyConstraintReferencedRead>? =
            listOrNull(rule, "readsReferenced", PropertyConstraintReferencedRead::fromJsonOrNull)

        /**
         * The entries [rule]'s optional [key] lists, each read by [read]:
         * empty when the key is missing (a native library built before it),
         * `null` when the value is not an array or [read] refuses an entry.
         */
        private fun <T> listOrNull(
            rule: JsonObject,
            key: String,
            read: (JsonElement) -> T?,
        ): List<T>? {
            val value = rule[key] ?: return emptyList()
            val entries = value as? JsonArray ?: return null
            return entries.map { read(it) ?: return null }
        }
    }
}

/**
 * A `countOf` or `sumOf` total a `propertyConstraints` rule reads: how many
 * documents of [documentType], a type of the same contract, match the filter,
 * or the total of their integer [property] (a `sumOf` only). The fields mirror
 * wasm-dpp2's `PropertyConstraintTotalRead` and the Swift SDK's.
 */
data class PropertyConstraintTotalRead(
    val kind: Kind,
    /** The document type the total is over. */
    val documentType: String,
    /** The summed integer property of a `sumOf`; `null` for a `countOf`. */
    val property: String?,
    /**
     * The keys the documents are matched by, properties of [documentType] or
     * `$ownerId`, in the order Rust gives; empty for a total over every
     * document of the type. The values they must take are in the rule.
     */
    val filter: List<String>,
) {
    /** What the total counts; the names are the operators'. */
    sealed interface Kind {
        /** The kind's name, as Rust reports it. */
        val name: String

        /** `countOf`: how many documents match. */
        data object CountOf : Kind {
            override val name: String get() = "countOf"
        }

        /** `sumOf`: the total of an integer property over them. */
        data object SumOf : Kind {
            override val name: String get() = "sumOf"
        }

        /** A kind this build does not know, by its name: one a later native library reports. */
        data class Other(override val name: String) : Kind

        companion object {
            /** The kind named [name], or [Other] for a name this build does not know. */
            fun fromName(name: String): Kind = when (name) {
                CountOf.name -> CountOf
                SumOf.name -> SumOf
                else -> Other(name)
            }
        }
    }

    internal companion object {
        /** The total [entry] describes, or `null` when it is malformed. */
        fun fromJsonOrNull(entry: JsonElement): PropertyConstraintTotalRead? {
            val total = entry as? JsonObject ?: return null
            val kind = total["kind"]?.jsonStringOrNull() ?: return null
            val documentType = total["documentType"]?.jsonStringOrNull() ?: return null
            val property = when (val value = total["property"]) {
                null -> null
                else -> value.jsonStringOrNull() ?: return null
            }
            val keys = total["filter"] as? JsonArray ?: return null
            val filter = keys.map { it.jsonStringOrNull() ?: return null }
            return PropertyConstraintTotalRead(Kind.fromName(kind), documentType, property, filter)
        }
    }
}

/**
 * A value a `propertyConstraints` rule reads of the document a reference of the
 * type points at, `{ "referenced": [reference, path] }`. The fields mirror
 * wasm-dpp2's `PropertyConstraintReferencedRead` and the Swift SDK's.
 */
data class PropertyConstraintReferencedRead(
    /** The top-level property of the document type holding the reference. */
    val reference: String,
    /**
     * The field of the referenced document: a dotted property path of its
     * type, `$id`, `$ownerId`, `$creatorId`, or a time or height it records
     * (`$createdAt`, ...).
     */
    val path: String,
    /**
     * How the rule reads the field, by the kinds [PropertyConstraintRead] uses:
     * an identifier (`$id`, `$ownerId`, `$creatorId`) as
     * [PropertyConstraintRead.Kind.Identifier], a time or height as
     * [PropertyConstraintRead.Kind.Value].
     */
    val kind: PropertyConstraintRead.Kind,
) {
    internal companion object {
        /** The read [entry] describes, or `null` when it is malformed. */
        fun fromJsonOrNull(entry: JsonElement): PropertyConstraintReferencedRead? {
            val read = entry as? JsonObject ?: return null
            val reference = read["reference"]?.jsonStringOrNull() ?: return null
            val path = read["path"]?.jsonStringOrNull() ?: return null
            val kind = read["kind"]?.jsonStringOrNull() ?: return null
            return PropertyConstraintReferencedRead(reference, path, PropertyConstraintRead.Kind.fromName(kind))
        }
    }
}

/** A property a `propertyConstraints` rule reads, and how it reads it. */
data class PropertyConstraintRead(
    /** The property's dotted path. */
    val path: String,
    val kind: Kind,
) {
    /** How a rule reads a property; the names are wasm-dpp2's `PropertyConstraintReadKind`. */
    sealed interface Kind {
        /** The kind's name, as Rust reports it. */
        val name: String

        /** By its value, as an integer operand: an integer or boolean property. */
        data object Value : Kind {
            override val name: String get() = "value"
        }

        /**
         * Only whether the document holds it, in `present`, `absent` or
         * `countPresent`.
         */
        data object Presence : Kind {
            override val name: String get() = "presence"
        }

        /** By its value, compared with strings: a string property. */
        data object Text : Kind {
            override val name: String get() = "text"
        }

        /** By its value, compared with identifiers: an identifier property. */
        data object Identifier : Kind {
            override val name: String get() = "identifier"
        }

        /**
         * By its size, in a `length` operand (its characters) or a
         * `byteLength` operand (its UTF-8 bytes): a string property.
         */
        data object Length : Kind {
            override val name: String get() = "length"
        }

        /**
         * By its size, in a `count` operand: an array property's items, or a
         * byte array property's bytes.
         */
        data object Count : Kind {
            override val name: String get() = "count"
        }

        /** By its elements, which a `contains` looks among: a typed array property. */
        data object Elements : Kind {
            override val name: String get() = "elements"
        }

        /**
         * By its bytes, in a `byteAt` operand or a `startsWith` or `endsWith`
         * of byte arrays: a byte array property.
         */
        data object Bytes : Kind {
            override val name: String get() = "bytes"
        }

        /** A kind this build does not know, by its name: one a later native library reports. */
        data class Other(override val name: String) : Kind

        companion object {
            /** The kind named [name], or [Other] for a name this build does not know. */
            fun fromName(name: String): Kind = when (name) {
                Value.name -> Value
                Presence.name -> Presence
                Text.name -> Text
                Identifier.name -> Identifier
                Length.name -> Length
                Count.name -> Count
                Elements.name -> Elements
                Bytes.name -> Bytes
                else -> Other(name)
            }
        }
    }

    internal companion object {
        fun fromJson(entry: JsonElement): PropertyConstraintRead {
            val read = entry as? JsonObject
            val path = read?.get("path")?.jsonStringOrNull()
            val kind = read?.get("kind")?.jsonStringOrNull()
            if (path == null || kind == null) {
                throw DashSdkError.SerializationError("Malformed propertyConstraints read: $entry")
            }
            return PropertyConstraintRead(path, Kind.fromName(kind))
        }
    }
}

/**
 * The first `propertyConstraints` rule a document breaks, as consensus would
 * report it in `DocumentPropertyConstraintViolatedError` (code 10422).
 *
 * Rust judges the document (`dash_sdk_data_contract_check_property_constraints`,
 * through [Contracts.checkPropertyConstraints]) with the check consensus runs,
 * the device clock standing in for the times the create records, a stored
 * time, height or revision read through `$old.` as 0, and a rule reading a
 * block height the create records, a `countOf` or `sumOf` total or a value of
 * a referenced document left unjudged; this type only carries the verdict.
 * The fields mirror wasm-dpp2's `DocumentPropertyConstraintViolation` and the
 * Swift SDK's `PropertyConstraintViolation`.
 */
data class PropertyConstraintViolation(
    /** The broken rule's name. */
    val rule: String,
    val violation: Kind,
    /** A readable reason, as in the consensus error's message. */
    val message: String,
) {
    /** Why the rule is broken; the names are wasm-dpp2's `PropertyConstraintViolationKind`. */
    sealed interface Kind {
        /** The reason's name, as Rust reports it. */
        val name: String

        /** The rule evaluates without a fault but does not hold. */
        data object NotMet : Kind {
            override val name: String get() = "NotMet"
        }

        /** A value the rule reads or computes does not fit a 128-bit signed integer. */
        data object Overflow : Kind {
            override val name: String get() = "Overflow"
        }

        /** A `divide` or `modulo` by zero. */
        data object DivisionByZero : Kind {
            override val name: String get() = "DivisionByZero"
        }

        /** A `power` with a negative exponent. */
        data object NegativeExponent : Kind {
            override val name: String get() = "NegativeExponent"
        }

        /** A value the rule reads is not an integer. */
        data object NotAnInteger : Kind {
            override val name: String get() = "NotAnInteger"
        }

        /** A reason this build does not know, by its name. */
        data class Other(override val name: String) : Kind

        companion object {
            /** The reason named [name], or [Other] for a name this build does not know. */
            fun fromName(name: String): Kind = when (name) {
                NotMet.name -> NotMet
                Overflow.name -> Overflow
                DivisionByZero.name -> DivisionByZero
                NegativeExponent.name -> NegativeExponent
                NotAnInteger.name -> NotAnInteger
                else -> Other(name)
            }
        }
    }

    /** One sentence naming the rule, the reason and the message (Swift's `errorDescription`). */
    val description: String
        get() = "The document breaks the propertyConstraints rule \"$rule\" " +
            "(${violation.name}): $message."

    companion object {
        /**
         * Decode the JSON `dash_sdk_data_contract_check_property_constraints`
         * returns: `null` for JSON `null`, when the document meets every rule.
         *
         * @throws DashSdkError.SerializationError for text that is neither
         *   `null` nor a violation object.
         */
        fun fromJson(json: String): PropertyConstraintViolation? {
            val value = PropertyConstraintJson.parse(json)
            if (value is JsonNull) return null
            val violation = value as? JsonObject
            val rule = violation?.get("rule")?.jsonStringOrNull()
            val kind = violation?.get("violation")?.jsonStringOrNull()
            val message = violation?.get("message")?.jsonStringOrNull()
            if (rule == null || kind == null || message == null) {
                throw DashSdkError.SerializationError("Malformed propertyConstraints violation: $json")
            }
            return PropertyConstraintViolation(rule, Kind.fromName(kind), message)
        }
    }
}

/** JSON readers for the two `propertyConstraints` payloads. */
internal object PropertyConstraintJson {
    private val prettyPrinter = Json { prettyPrint = true }

    /** The JSON value [text] holds, [JsonNull] for `null`. */
    fun parse(text: String): JsonElement = try {
        Json.parseToJsonElement(text)
    } catch (e: SerializationException) {
        throw DashSdkError.SerializationError("Not JSON: $text", e)
    }

    /** [value] as compact JSON text with sorted keys. */
    fun compact(value: JsonElement): String = sortedKeys(value).toString()

    /** [value] as indented JSON text with sorted keys. */
    fun pretty(value: JsonElement): String =
        prettyPrinter.encodeToString(JsonElement.serializer(), sortedKeys(value))

    private fun sortedKeys(value: JsonElement): JsonElement = when (value) {
        is JsonObject -> JsonObject(
            value.entries
                .sortedBy { it.key }
                .associateTo(LinkedHashMap()) { (key, element) -> key to sortedKeys(element) },
        )
        is JsonArray -> JsonArray(value.map(::sortedKeys))
        is JsonPrimitive -> value
    }
}

/** The string a JSON string holds; `null` for any other JSON value. */
private fun JsonElement.jsonStringOrNull(): String? =
    (this as? JsonPrimitive)?.takeIf { it.isString }?.content

/**
 * The boolean a JSON `true` or `false` holds; `null` for any other JSON value,
 * the string `"true"` included.
 */
private fun JsonElement.jsonBooleanOrNull(): Boolean? =
    (this as? JsonPrimitive)?.takeIf { !it.isString }?.booleanOrNull
