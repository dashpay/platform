package org.dashfoundation.example.ui.contracts

import java.math.BigInteger
import java.util.Base64
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.doubleOrNull
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.toHex

/**
 * What every element of a typed array is (protocol version 14). Mirrors the
 * `DocumentTypedArrayItem` union wasm-dpp2 exposes, read off the property's
 * `items` schema.
 */
internal sealed interface TypedArrayItem {
    /** Short name for captions: the schema keyword, or byteArray / identifier. */
    val label: String

    data class IntegerItem(
        val minimum: BigInteger? = null,
        val maximum: BigInteger? = null,
        val allowedValues: List<BigInteger>? = null,
    ) : TypedArrayItem {
        override val label: String get() = "integer"
    }

    data class NumberItem(
        val minimum: Double? = null,
        val maximum: Double? = null,
        val allowedValues: List<Double>? = null,
    ) : TypedArrayItem {
        override val label: String get() = "number"
    }

    data class BooleanItem(val allowedValues: List<Boolean>? = null) : TypedArrayItem {
        override val label: String get() = "boolean"
    }

    /**
     * A string element. `pattern` is kept for display only: the form does not
     * check it, since JSON Schema and Java regular expressions differ, and
     * consensus judges it anyway.
     */
    data class StringItem(
        val minLength: Int? = null,
        val maxLength: Int? = null,
        val pattern: String? = null,
        val allowedValues: List<String>? = null,
    ) : TypedArrayItem {
        override val label: String get() = "string"
    }

    data class ByteArrayItem(val minSize: Int? = null, val maxSize: Int? = null) : TypedArrayItem {
        override val label: String get() = "byteArray"
    }

    data object IdentifierItem : TypedArrayItem {
        override val label: String get() = "identifier"
    }
}

/**
 * One typed array property of a document type: a `type: "array"` property
 * declared by an `items` schema rather than `byteArray: true`. [path] is the
 * dotted path within the document type, for example `reasons`, or
 * `team.leads` for one nested in an object property.
 */
internal data class DocumentTypedArray(
    val path: String,
    val items: TypedArrayItem,
    val minItems: Int?,
    val maxItems: Int?,
    val uniqueItems: Boolean,
) {
    /** One-line summary for captions and the type details screen. */
    val summary: String
        get() = buildList {
            add("List of ${items.label}")
            when {
                minItems != null && maxItems != null -> add("$minItems to $maxItems items")
                maxItems != null -> add("up to $maxItems items")
                minItems != null -> add("at least $minItems items")
            }
            if (uniqueItems) add("unique")
        }.joinToString(" · ")
}

/**
 * Read [property] as a typed array at [path], or null when it is not one: no
 * `items` schema, `byteArray: true` on the property itself (a plain byte
 * array), or an element schema this form does not recognise.
 */
internal fun documentTypedArray(path: String, property: JsonObject): DocumentTypedArray? {
    if (property.stringField("type") != "array") return null
    if (property.boolField("byteArray") == true) return null
    val items = property.objectField("items") ?: return null
    val item = typedArrayItem(items) ?: return null
    return DocumentTypedArray(
        path = path,
        items = item,
        minItems = property.intField("minItems"),
        maxItems = property.intField("maxItems"),
        uniqueItems = property.boolField("uniqueItems") == true,
    )
}

/**
 * Every typed array a document type [schema] declares, including those nested
 * in object properties (dotted paths), sorted by path. Empty for a pre-v14
 * type and for one declaring none.
 */
internal fun documentTypedArrays(schema: JsonObject?): List<DocumentTypedArray> {
    val found = mutableListOf<DocumentTypedArray>()
    fun walk(properties: JsonObject?, prefix: String) {
        properties ?: return
        for ((name, element) in properties) {
            val property = element as? JsonObject ?: continue
            val path = if (prefix.isEmpty()) name else "$prefix.$name"
            documentTypedArray(path, property)?.let { found += it }
            if (property.stringField("type") == "object") {
                walk(property.objectField("properties"), path)
            }
        }
    }
    walk(schema?.objectField("properties"), "")
    return found.sortedBy { it.path }
}

private fun typedArrayItem(items: JsonObject): TypedArrayItem? = when (items.stringField("type")) {
    "integer" -> TypedArrayItem.IntegerItem(
        minimum = items.bigIntegerField("minimum"),
        maximum = items.bigIntegerField("maximum"),
        allowedValues = items.arrayField("enum")
            ?.mapNotNull { (it as? JsonPrimitive)?.takeUnless { p -> p.isString }?.content?.toBigIntegerOrNull() },
    )
    "number" -> TypedArrayItem.NumberItem(
        minimum = (items["minimum"] as? JsonPrimitive)?.doubleOrNull,
        maximum = (items["maximum"] as? JsonPrimitive)?.doubleOrNull,
        allowedValues = items.arrayField("enum")
            ?.mapNotNull { (it as? JsonPrimitive)?.takeUnless { p -> p.isString }?.doubleOrNull },
    )
    "boolean" -> TypedArrayItem.BooleanItem(
        allowedValues = items.arrayField("enum")
            ?.mapNotNull { (it as? JsonPrimitive)?.takeUnless { p -> p.isString }?.booleanOrNull },
    )
    "string" -> TypedArrayItem.StringItem(
        minLength = items.intField("minLength"),
        maxLength = items.intField("maxLength"),
        pattern = items.stringField("pattern"),
        allowedValues = items.arrayField("enum")
            ?.mapNotNull { (it as? JsonPrimitive)?.takeIf { p -> p.isString }?.content },
    )
    "array" -> when {
        items.boolField("byteArray") != true -> null
        items.stringField("contentMediaType")?.contains("identifier") == true ->
            TypedArrayItem.IdentifierItem
        else -> TypedArrayItem.ByteArrayItem(
            minSize = items.intField("minItems"),
            maxSize = items.intField("maxItems"),
        )
    }
    else -> null
}

private fun JsonObject.bigIntegerField(key: String): BigInteger? =
    (this[key] as? JsonPrimitive)?.takeUnless { it.isString }?.content?.toBigIntegerOrNull()

/** The outcome of converting one element's form text into its JSON value. */
internal sealed interface TypedArrayElement {
    /** The JSON value to send for the element. */
    data class Valid(val json: JsonPrimitive) : TypedArrayElement

    /** Why the text is not an acceptable element, for the row's error line. */
    data class Invalid(val reason: String) : TypedArrayElement
}

/**
 * Convert the form text [raw] for one element of kind [item] into the JSON
 * value the Rust sanitize step accepts: a JSON number for integer and number
 * elements, a JSON boolean for boolean ones (sanitize does not turn the
 * string "5" into an integer), the string for string elements, the base58 (or
 * 64-character hex) string for an identifier, and a lowercase hex string for
 * a byte array. The declared bounds and `enum` are checked as a courtesy, so
 * the user does not pay for a transition consensus is sure to refuse;
 * consensus stays the authority.
 */
internal fun typedArrayElement(item: TypedArrayItem, raw: String): TypedArrayElement {
    val text = raw.trim()
    return when (item) {
        is TypedArrayItem.IntegerItem -> {
            val value = text.toBigIntegerOrNull()
                ?: return TypedArrayElement.Invalid("not a whole number")
            item.minimum?.let { if (value < it) return TypedArrayElement.Invalid("below the minimum $it") }
            item.maximum?.let { if (value > it) return TypedArrayElement.Invalid("above the maximum $it") }
            if (item.allowedValues != null && value !in item.allowedValues) {
                return TypedArrayElement.Invalid("not one of ${item.allowedValues.joinToString(", ")}")
            }
            TypedArrayElement.Valid(JsonPrimitive(value))
        }

        is TypedArrayItem.NumberItem -> {
            val value = text.toDoubleOrNull()?.takeIf { it.isFinite() }
                ?: return TypedArrayElement.Invalid("not a number")
            item.minimum?.let { if (value < it) return TypedArrayElement.Invalid("below the minimum $it") }
            item.maximum?.let { if (value > it) return TypedArrayElement.Invalid("above the maximum $it") }
            if (item.allowedValues != null && value !in item.allowedValues) {
                return TypedArrayElement.Invalid("not one of ${item.allowedValues.joinToString(", ")}")
            }
            TypedArrayElement.Valid(JsonPrimitive(value))
        }

        is TypedArrayItem.BooleanItem -> {
            val value = text.toBooleanStrictOrNull()
                ?: return TypedArrayElement.Invalid("not true or false")
            if (item.allowedValues != null && value !in item.allowedValues) {
                return TypedArrayElement.Invalid("not one of ${item.allowedValues.joinToString(", ")}")
            }
            TypedArrayElement.Valid(JsonPrimitive(value))
        }

        is TypedArrayItem.StringItem -> {
            // Strings keep their spaces: only the other kinds are trimmed.
            val length = raw.codePointCount(0, raw.length)
            item.minLength?.let {
                if (length < it) return TypedArrayElement.Invalid("shorter than $it characters")
            }
            item.maxLength?.let {
                if (length > it) return TypedArrayElement.Invalid("longer than $it characters")
            }
            if (item.allowedValues != null && raw !in item.allowedValues) {
                return TypedArrayElement.Invalid("not one of ${item.allowedValues.joinToString(", ")}")
            }
            TypedArrayElement.Valid(JsonPrimitive(raw))
        }

        TypedArrayItem.IdentifierItem -> {
            if (Base58.decodeIdentifier(text) == null) {
                TypedArrayElement.Invalid("not a base58 identifier")
            } else {
                TypedArrayElement.Valid(JsonPrimitive(text))
            }
        }

        is TypedArrayItem.ByteArrayItem -> {
            val hex = text.removePrefix("0x").lowercase()
            if (hex.length % 2 != 0 || hex.any { it !in '0'..'9' && it !in 'a'..'f' }) {
                return TypedArrayElement.Invalid("not hex bytes")
            }
            val size = hex.length / 2
            item.minSize?.let { if (size < it) return TypedArrayElement.Invalid("fewer than $it bytes") }
            item.maxSize?.let { if (size > it) return TypedArrayElement.Invalid("more than $it bytes") }
            TypedArrayElement.Valid(JsonPrimitive(hex))
        }
    }
}

/**
 * Convert every row of a typed array into its JSON array, in row order.
 * Throws [IllegalArgumentException] naming the first bad row (`name[index]`),
 * or a count outside `minItems` / `maxItems`, or a repeated element when
 * `uniqueItems` is set. The form surfaces the message instead of broadcasting
 * a transition consensus would refuse, which would still be paid for.
 */
internal fun typedArrayJson(name: String, typedArray: DocumentTypedArray, rows: List<String>): JsonArray {
    typedArray.minItems?.let {
        require(rows.size >= it) { "$name needs at least $it items, has ${rows.size}" }
    }
    typedArray.maxItems?.let {
        require(rows.size <= it) { "$name allows at most $it items, has ${rows.size}" }
    }
    val elements = rows.mapIndexed { index, raw ->
        when (val element = typedArrayElement(typedArray.items, raw)) {
            is TypedArrayElement.Valid -> element.json
            is TypedArrayElement.Invalid -> throw IllegalArgumentException("$name[$index]: ${element.reason}")
        }
    }
    if (typedArray.uniqueItems) {
        val repeated = elements.groupBy { it.content }.filterValues { it.size > 1 }.keys
        require(repeated.isEmpty()) { "$name must not repeat an element: ${repeated.joinToString(", ")}" }
    }
    return JsonArray(elements)
}

/**
 * The form rows for a stored typed array [value] taken from a document's
 * canonical JSON, or null when the value is not an array of scalars. Byte
 * array elements arrive as base64 and are shown as hex: the Rust sanitize
 * step tries hex before base64, so sending back base64 made only of hex
 * digits would decode to different bytes.
 */
internal fun typedArraySeedRows(item: TypedArrayItem, value: JsonElement?): List<String>? {
    val array = value as? JsonArray ?: return null
    return array.map { element ->
        val primitive = element as? JsonPrimitive ?: return null
        if (item is TypedArrayItem.ByteArrayItem && primitive.isString) {
            base64ToHex(primitive.content) ?: primitive.content
        } else {
            primitive.content
        }
    }
}

private fun base64ToHex(text: String): String? = try {
    Base64.getDecoder().decode(text).toHex()
} catch (_: IllegalArgumentException) {
    null
}
