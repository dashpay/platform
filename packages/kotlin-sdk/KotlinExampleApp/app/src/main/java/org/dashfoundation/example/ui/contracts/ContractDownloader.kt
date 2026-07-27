package org.dashfoundation.example.ui.contracts

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import java.util.Date

/**
 * Shared fetch + parse + persist pipeline for data contracts — port of
 * `ContractDownloader.downloadAndPersistContract` in
 * `SwiftExampleApp/Utils/ContractDownloader.swift`.
 *
 * Fidelity notes vs the Swift pipeline:
 *  - Swift fetches JSON + the binary (bincode) serialization in one round
 *    trip via `dash_sdk_data_contract_fetch_with_serialization`
 *    (ContractDownloader.swift:155) and stores the JSON bytes in
 *    `serializedContract` and the binary blob in `binarySerialization`
 *    (ContractDownloader.swift:309-315). The Kotlin queries layer only
 *    exposes `Sdk.contracts.fetchJson`, so [DataContractEntity.serializedContract]
 *    holds the same JSON bytes Swift stores, while
 *    [DataContractEntity.binarySerialization] stays null until a
 *    fetch-with-serialization JNI export lands.
 *  - Swift's `DataContractParser.parseDataContract` additionally persists
 *    child token / document-type / index / property rows. The Kotlin
 *    contract screens parse document types straight from the stored
 *    contract JSON; token rows ARE materialized (via
 *    [org.dashfoundation.example.services.tokens.TokenMaterializer])
 *    because the tokens UI reads them through `TokenDao`'s capability
 *    queries, exactly as the iOS token views read `PersistentToken` rows.
 */
object ContractDownloader {

    class ContractNotFoundException(message: String) : Exception(message)

    data class Result(
        val contract: DataContractEntity,
        val alreadyExisted: Boolean,
    )

    /**
     * Fetch [contractIdBase58] from the network, persist it into Room, and
     * return the row. When the contract id is already stored, touches
     * `lastAccessedAt` and returns the existing row with
     * `alreadyExisted == true` (← ContractDownloader.swift:254-270).
     */
    suspend fun downloadAndPersistContract(
        contractIdBase58: String,
        suggestedName: String?,
        sdk: Sdk,
        database: DashDatabase,
        network: Network,
    ): Result {
        val trimmedId = contractIdBase58.trim()
        require(trimmedId.isNotEmpty()) { "Please enter a contract ID" }

        // Fetch JSON + the versioned binary serialization in one round trip
        // (← Swift's `dataContractGetWithSerialization`). The binary blob is
        // what the trusted context provider deserializes, so persisting it
        // lets `AppContainer.loadKnownContractsIntoSdk` preload this contract
        // for proof verification without a network fetch.
        val fetched = try {
            sdk.contracts.fetchWithSerialization(trimmedId)
        } catch (e: Exception) {
            val message = e.message ?: "Unknown error"
            if (message.contains("not found", ignoreCase = true)) {
                throw ContractNotFoundException(message)
            }
            throw Exception("Failed to fetch data contract: $message", e)
        } ?: throw ContractNotFoundException("Data contract not found")
        val json = fetched.json
        val binarySerialization = fetched.binarySerialization

        val root = LenientJson.parseToJsonElement(json).jsonObject

        // Resolve the canonical 32-byte contract id: prefer the id in the
        // JSON, fall back to the user's input (← ContractDownloader.swift:229-239).
        val idFromJson = (root["id"] as? JsonPrimitive)?.content
        val contractIdBytes = idFromJson?.let { Base58.decodeIdentifier(it) }
            ?: Base58.decodeIdentifier(trimmedId)
            ?: throw Exception("Could not extract contract ID from response")

        val dao = database.dataContractDao()
        val preExisting = dao.getById(contractIdBytes)
        if (preExisting != null) {
            // Backfill the binary serialization for rows persisted before the
            // with-serialization fetch, so the startup preload can pick them
            // up. Keep any existing value when the node didn't return one.
            dao.upsert(
                preExisting.copy(
                    lastAccessedAt = Date(),
                    binarySerialization = binarySerialization ?: preExisting.binarySerialization,
                ),
            )
            // Re-runs are how rows stored before the token materializer
            // landed pick up their token children.
            org.dashfoundation.example.services.tokens.TokenMaterializer
                .materialize(preExisting, database.tokenDao())
            return Result(contract = preExisting, alreadyExisted = true)
        }

        val documents = (root["documents"] as? JsonObject)
            ?: (root["documentSchemas"] as? JsonObject)
            ?: JsonObject(emptyMap())
        val tokens = (root["tokens"] as? JsonObject) ?: JsonObject(emptyMap())

        val entity = DataContractEntity(
            id = contractIdBytes,
            name = resolveName(suggestedName, trimmedId, documents, tokens),
            serializedContract = json.toByteArray(Charsets.UTF_8),
            binarySerialization = binarySerialization,
            version = (root["version"] as? JsonPrimitive)?.content?.toIntOrNull() ?: 1,
            ownerId = (root["ownerId"] as? JsonPrimitive)?.content
                ?.let { Base58.decodeIdentifier(it) },
            contractDescription = (root["description"] as? JsonPrimitive)?.content,
            schemaData = documents.toString().toByteArray(Charsets.UTF_8),
            documentTypesData = JsonArray(documents.keys.map { JsonPrimitive(it) })
                .toString().toByteArray(Charsets.UTF_8),
            networkRaw = network.ffiValue,
            canBeDeleted = root.booleanFlag("canBeDeleted", default = false),
            readonly = root.booleanFlag("readonly", default = false),
            keepsHistory = root.booleanFlag("keepsHistory", default = false),
            documentsKeepHistoryContractDefault =
                root.booleanFlag("documentsKeepHistoryContractDefault", default = false),
            documentsMutableContractDefault =
                root.booleanFlag("documentsMutableContractDefault", default = true),
            documentsCanBeDeletedContractDefault =
                root.booleanFlag("documentsCanBeDeletedContractDefault", default = true),
            hasTokens = tokens.isNotEmpty(),
            tokensData = tokens.takeIf { it.isNotEmpty() }
                ?.toString()?.toByteArray(Charsets.UTF_8),
            groupsData = (root["groups"] as? JsonObject)?.takeIf { it.isNotEmpty() }
                ?.toString()?.toByteArray(Charsets.UTF_8),
        )
        dao.upsert(entity)
        // Materialize token child rows (← DataContractParser's
        // PersistentToken pass) so the tokens UI has rows to serve.
        org.dashfoundation.example.services.tokens.TokenMaterializer
            .materialize(entity, database.tokenDao())
        return Result(contract = entity, alreadyExisted = false)
    }

    private fun JsonObject.booleanFlag(key: String, default: Boolean): Boolean =
        (this[key] as? JsonPrimitive)?.content?.toBooleanStrictOrNull() ?: default

    /**
     * Display-name heuristics matching ContractDownloader.swift:277-307:
     * token-only contracts get "<TokenName> Token Contract", contracts with
     * documents get "Contract with <firstDocType>", anything else a
     * truncated-id fallback.
     */
    private fun resolveName(
        suggestedName: String?,
        trimmedId: String,
        documents: JsonObject,
        tokens: JsonObject,
    ): String {
        suggestedName?.trim()?.takeIf { it.isNotEmpty() }?.let { return it }

        if (documents.isEmpty() && tokens.size == 1) {
            val tokenData = tokens.values.first() as? JsonObject
            val singular = ((tokenData?.get("conventions") as? JsonObject)
                ?.get("localizations") as? JsonObject)
                ?.let { it["en"] as? JsonObject }
                ?.let { (it["singularForm"] as? JsonPrimitive)?.content }
            if (singular != null) return "$singular Token Contract"
            (tokenData?.get("description") as? JsonPrimitive)?.content
                ?.let { return "$it Token Contract" }
            (tokenData?.get("name") as? JsonPrimitive)?.content
                ?.let { return "$it Token Contract" }
            return "Token Contract"
        }
        documents.keys.firstOrNull()?.let { return "Contract with $it" }
        return "Contract ${trimmedId.take(8)}..."
    }
}
