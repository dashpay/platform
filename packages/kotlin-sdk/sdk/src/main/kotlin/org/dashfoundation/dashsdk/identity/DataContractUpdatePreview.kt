package org.dashfoundation.dashsdk.identity

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
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.ContractUpdateNative

/**
 * What a data contract update would change, read against the contract as Platform holds it.
 * Returned by [DataContracts.previewUpdate]. Kotlin counterpart of Swift's
 * `DataContractUpdatePreview.swift`; mirrors the JSON
 * `platform_wallet_preview_data_contract_update` writes, which Rust's
 * `DataContractUpdatePreview` fills. It reports what the update changes, not whether
 * Platform accepts it: the network checks the contract update rules when it is broadcast.
 */
data class DataContractUpdatePreview(
    /** The version of the contract Platform holds. */
    val currentVersion: Long,
    /** The version the update would give it. */
    val newVersion: Long,
    /** Whether the update would change anything besides the version. Computed by Rust. */
    val hasChanges: Boolean,
    /** Document types, by name. */
    val documentTypes: SectionChanges,
    /** Tokens, by contract position (decimal strings). */
    val tokens: SectionChanges,
    /** Groups, by contract position (decimal strings). */
    val groups: SectionChanges,
    /** Whether the keywords would differ. */
    val keywordsChanged: Boolean,
    /** Whether the description would differ. */
    val descriptionChanged: Boolean,
    /** Whether the contract configuration would differ. */
    val configChanged: Boolean,
    /** The configuration members that would differ (for example `moderation`), sorted. */
    val configChangedFields: List<String>,
    /** Identities (base58, sorted) the configuration would newly name as moderators. */
    val moderatorsAdded: List<String>,
    /** Identities (base58, sorted) the configuration names as moderators and would stop naming. */
    val moderatorsRemoved: List<String>,
) {
    /** The entries of one keyed section an update adds or changes; an update never removes one. */
    data class SectionChanges(
        /** Keys the contract does not have yet. */
        val added: List<String>,
        /** Keys the contract has, with a different definition. */
        val changed: List<String>,
    )

    internal companion object {
        /** Decode the JSON `platform_wallet_preview_data_contract_update` writes. */
        fun fromJson(json: String): DataContractUpdatePreview = with(UpdateJson) {
            val o = parseObject(json, "data contract update preview")
            DataContractUpdatePreview(
                currentVersion = o.long("currentVersion"),
                newVersion = o.long("newVersion"),
                hasChanges = o.boolean("hasChanges"),
                documentTypes = o.section("documentTypes"),
                tokens = o.section("tokens"),
                groups = o.section("groups"),
                keywordsChanged = o.boolean("keywordsChanged"),
                descriptionChanged = o.boolean("descriptionChanged"),
                configChanged = o.boolean("configChanged"),
                configChangedFields = o.strings("configChangedFields"),
                moderatorsAdded = o.strings("moderatorsAdded"),
                moderatorsRemoved = o.strings("moderatorsRemoved"),
            )
        }

        private fun JsonObject.section(name: String): SectionChanges = with(UpdateJson) {
            val section = this@section[name] as? JsonObject ?: throw malformed(name)
            SectionChanges(added = section.strings("added"), changed = section.strings("changed"))
        }
    }
}

/**
 * Pasted or imported contract JSON split into the sections a contract update takes, each as
 * JSON text in source order, or null when the text does not supply it. Kotlin counterpart
 * of Swift's `DataContractUpdateSource.swift`. The split is Rust's
 * (`split_contract_update_source`): it decides which members are sections, reads a bare
 * document schemas map, and refuses text it cannot split. A section must not be
 * re-serialized: the order of a schema's members is part of the contract.
 */
data class DataContractUpdateSource(
    val documents: String?,
    val tokens: String?,
    val groups: String?,
    val keywords: String?,
    /** The description itself, not JSON. */
    val description: String?,
    val config: String?,
    /** The source has non-empty shared definitions (`schemaDefs` or `$defs`) the sections leave out. */
    val carriesSharedDefinitions: Boolean,
) {
    companion object {
        /**
         * Split [text] (`platform_wallet_split_contract_update_source`). Text that is not a
         * JSON object, or supplies no section, is refused with Rust's message, to show as is.
         *
         * @throws DashSdkError the wallet's refusal.
         */
        fun split(text: String): DataContractUpdateSource =
            fromJson(mapNativeErrors { ContractUpdateNative.splitContractUpdateSource(text) })

        /** Decode the JSON `platform_wallet_split_contract_update_source` writes. */
        internal fun fromJson(json: String): DataContractUpdateSource = with(UpdateJson) {
            val o = parseObject(json, "contract update sections")
            DataContractUpdateSource(
                documents = o.optionalString("documents"),
                tokens = o.optionalString("tokens"),
                groups = o.optionalString("groups"),
                keywords = o.optionalString("keywords"),
                description = o.optionalString("description"),
                config = o.optionalString("config"),
                carriesSharedDefinitions = o.boolean("carriesSharedDefinitions"),
            )
        }
    }
}

/** JSON access for the contract update preview and sections. */
private object UpdateJson {
    fun parseObject(json: String, what: String): JsonObject {
        val element = try {
            Json.parseToJsonElement(json)
        } catch (e: SerializationException) {
            throw DashSdkError.SerializationError("Malformed $what JSON: $json", e)
        }
        return element as? JsonObject
            ?: throw DashSdkError.SerializationError("Malformed $what JSON: $json")
    }

    fun JsonObject.long(name: String): Long =
        (this[name] as? JsonPrimitive)?.takeUnless { it.isString || it is JsonNull }?.longOrNull
            ?: throw malformed(name)

    fun JsonObject.boolean(name: String): Boolean =
        (this[name] as? JsonPrimitive)?.takeUnless { it.isString }?.booleanOrNull
            ?: throw malformed(name)

    fun JsonObject.strings(name: String): List<String> =
        (this[name] as? JsonArray ?: throw malformed(name)).map { stringOf(it, name) }

    fun JsonObject.optionalString(name: String): String? = when (val value = this[name]) {
        null, JsonNull -> null
        else -> stringOf(value, name)
    }

    private fun stringOf(element: JsonElement, name: String): String =
        (element as? JsonPrimitive)?.takeIf { it.isString }?.content ?: throw malformed(name)

    fun malformed(name: String) =
        DashSdkError.SerializationError("Contract update JSON has no valid \"$name\"")
}
