package org.dashfoundation.example.ui.contracts

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.example.util.LenientJson

/**
 * Parsed view over [DataContractEntity.serializedContract] — the stored
 * contract JSON. The iOS app materializes the same structure into
 * SwiftData child rows via `DataContractParser`; here the detail screens
 * parse it on demand from the JSON blob instead.
 */
data class ParsedContract(
    val root: JsonObject,
    /** Document type name → schema object (`documents` / `documentSchemas`). */
    val documentTypes: Map<String, JsonObject>,
    /** Token position key → token configuration object. */
    val tokens: Map<String, JsonObject>,
    /** Group position key → group definition object. */
    val groups: Map<String, JsonObject>,
) {
    companion object {
        fun from(entity: DataContractEntity): ParsedContract? = try {
            val root = LenientJson
                .parseToJsonElement(entity.serializedContract.decodeToString())
                .jsonObject
            val documents = (root["documents"] as? JsonObject)
                ?: (root["documentSchemas"] as? JsonObject)
                ?: JsonObject(emptyMap())
            ParsedContract(
                root = root,
                documentTypes = documents.mapValues { (_, v) -> v as? JsonObject ?: JsonObject(emptyMap()) },
                tokens = (root["tokens"] as? JsonObject).orEmpty()
                    .mapValues { (_, v) -> v as? JsonObject ?: JsonObject(emptyMap()) },
                groups = (root["groups"] as? JsonObject).orEmpty()
                    .mapValues { (_, v) -> v as? JsonObject ?: JsonObject(emptyMap()) },
            )
        } catch (_: Exception) {
            null
        }

        private fun JsonObject?.orEmpty(): JsonObject = this ?: JsonObject(emptyMap())
    }
}

internal fun JsonObject.stringField(key: String): String? =
    (this[key] as? JsonPrimitive)?.content

internal fun JsonObject.intField(key: String): Int? =
    (this[key] as? JsonPrimitive)?.content?.toIntOrNull()

internal fun JsonObject.boolField(key: String): Boolean? =
    (this[key] as? JsonPrimitive)?.content?.toBooleanStrictOrNull()

internal fun JsonObject.arrayField(key: String): JsonArray? = this[key] as? JsonArray

internal fun JsonObject.objectField(key: String): JsonObject? = this[key] as? JsonObject

/** Effective DPP capability flags for a document type. */
internal data class DocumentTypeCapabilities(
    val documentsMutable: Boolean,
    val canBeDeleted: Boolean,
)

/**
 * Reproduce DPP's effective document-type capability flags: the per-type
 * [schema] value when present, else the contract [config] default, else
 * DPP's ultimate default of `true`.
 *
 * `DocumentType::try_from_schema` reads `documentsMutable` / `canBeDeleted`
 * from the document schema and, when absent, falls back to the contract
 * config's `documentsMutableContractDefault` /
 * `documentsCanBeDeletedContractDefault` (both defaulting to `true`). There
 * is no `documentsCanBeDeleted` schema key. UI capability gates must match
 * this — Drive treats a submit against a disabled capability as a PAID
 * invalid transition (fees + a burned nonce for a guaranteed rejection).
 */
internal fun documentTypeCapabilities(
    schema: JsonObject?,
    config: JsonObject?,
): DocumentTypeCapabilities =
    DocumentTypeCapabilities(
        documentsMutable = schema?.boolField("documentsMutable")
            ?: config?.boolField("documentsMutableContractDefault") ?: true,
        canBeDeleted = schema?.boolField("canBeDeleted")
            ?: config?.boolField("documentsCanBeDeletedContractDefault") ?: true,
    )
