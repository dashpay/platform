package com.dash.sdk.modules

import com.dash.sdk.DataContract
import com.dash.sdk.Document
import com.dash.sdk.Identity
import com.dash.sdk.SDK
import com.dash.sdk.ffi.DashSDKFFI
import com.dash.sdk.utils.Converters
import com.dash.sdk.utils.Converters.checkResult
import com.dash.sdk.utils.Converters.getHandleFromResult
import com.dash.sdk.utils.Converters.getStringFromResult
import com.dash.sdk.utils.ensureSize
import com.sun.jna.Pointer
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.*
import mu.KotlinLogging

private val logger = KotlinLogging.logger {}

/**
 * Module for document operations
 */
class Documents internal constructor(
    private val sdk: SDK,
    private val sdkHandle: Pointer
) {
    /**
     * Create a new document
     * 
     * @param dataContract The data contract that defines the document type
     * @param documentType The type of document to create
     * @param properties The document properties as a JSON object
     * @param owner The identity that will own the document
     * @return The created document
     */
    suspend fun create(
        dataContract: DataContract,
        documentType: String,
        properties: JsonObject,
        owner: Identity
    ): Document = withContext(Dispatchers.IO) {
        logger.debug { "Creating new document of type: $documentType" }
        
        val params = DashSDKFFI.DashSDKDocumentCreateParams().apply {
            data_contract_handle = dataContract.getHandle()
            document_type = documentType
            owner_identity_handle = owner.getHandle()
            properties_json = Json.encodeToString(JsonObject.serializer(), properties)
        }
        
        val result = DashSDKFFI.INSTANCE.dash_sdk_document_create(sdkHandle, params)
        
        val handle = getHandleFromResult(result, DashSDKFFI.DashSDKResultDataType.DocumentHandle)
            ?: throw IllegalStateException("Document creation returned null handle")
        
        val document = Document(handle, sdk)
        document.setType(documentType)
        document.setOwnerId(owner.id)
        document.setProperties(properties)
        logger.info { "Document created successfully" }
        document
    }
    
    /**
     * Query documents
     * 
     * @param dataContract The data contract to query against
     * @param documentType The type of documents to query
     * @param query The query as a JSON object (MongoDB-style query)
     * @param limit Maximum number of documents to return
     * @return List of matching documents
     */
    suspend fun search(
        dataContract: DataContract,
        documentType: String,
        query: JsonObject = JsonObject(emptyMap()),
        limit: Int = 100
    ): List<Document> = withContext(Dispatchers.IO) {
        logger.debug { "Searching documents of type: $documentType" }
        
        val queryJson = Json.encodeToString(JsonObject.serializer(), query)
        
        val result = DashSDKFFI.INSTANCE.dash_sdk_document_search(
            sdkHandle,
            dataContract.getHandle(),
            documentType,
            queryJson,
            limit
        )
        
        val jsonString = getStringFromResult(result) ?: return@withContext emptyList()
        
        // Parse the JSON array of documents
        val jsonArray = Json.parseToJsonElement(jsonString).jsonArray
        
        jsonArray.map { element ->
            val docJson = element.jsonObject
            // This is a simplified version - in reality we'd need to get handles for each document
            // or the FFI would return a list of handles
            // For now, we'll create placeholder documents
            val handle = Pointer(0) // Placeholder
            val document = Document(handle, sdk)
            
            // Extract document properties from JSON
            docJson["id"]?.jsonPrimitive?.content?.let { 
                document.setId(it.hexToByteArray().ensureSize(32))
            }
            docJson["ownerId"]?.jsonPrimitive?.content?.let {
                document.setOwnerId(it.hexToByteArray().ensureSize(32))
            }
            docJson["revision"]?.jsonPrimitive?.long?.let {
                document.setRevision(it)
            }
            document.setType(documentType)
            document.setProperties(docJson["properties"]?.jsonObject ?: JsonObject(emptyMap()))
            
            document
        }
    }
    
    /**
     * Transfer ownership of a document
     * 
     * @param document The document to transfer
     * @param recipientId The new owner's identity ID
     */
    suspend fun transfer(
        document: Document,
        recipientId: ByteArray
    ) = withContext(Dispatchers.IO) {
        require(recipientId.size == 32) { "Recipient ID must be 32 bytes" }
        
        logger.debug { "Transferring document ownership" }
        
        val result = DashSDKFFI.INSTANCE.dash_sdk_document_transfer(
            sdkHandle,
            document.getHandle(),
            recipientId,
            recipientId.size
        )
        
        checkResult(result)
        document.setOwnerId(recipientId)
        logger.info { "Document transferred successfully" }
    }
    
    /**
     * Update a document
     * 
     * @param document The document to update
     * @param updates The updates to apply as a JSON object
     */
    suspend fun update(
        document: Document,
        updates: JsonObject
    ) = withContext(Dispatchers.IO) {
        logger.debug { "Updating document" }
        
        val updatesJson = Json.encodeToString(JsonObject.serializer(), updates)
        
        val result = DashSDKFFI.INSTANCE.dash_sdk_document_update(
            sdkHandle,
            document.getHandle(),
            updatesJson
        )
        
        checkResult(result)
        
        // Update the document properties
        val currentProps = document.properties
        val updatedProps = JsonObject(currentProps + updates)
        document.setProperties(updatedProps)
        document.setRevision(document.revision + 1)
        
        logger.info { "Document updated successfully" }
    }
    
    /**
     * Delete a document
     * 
     * @param document The document to delete
     */
    suspend fun delete(
        document: Document
    ) = withContext(Dispatchers.IO) {
        logger.debug { "Deleting document" }
        
        val result = DashSDKFFI.INSTANCE.dash_sdk_document_delete(
            sdkHandle,
            document.getHandle()
        )
        
        checkResult(result)
        logger.info { "Document deleted successfully" }
    }
    
    /**
     * Helper function to build MongoDB-style queries
     */
    class QueryBuilder {
        private val query = mutableMapOf<String, JsonElement>()
        
        /**
         * Add an equality condition
         */
        fun where(field: String, value: Any): QueryBuilder {
            query[field] = when (value) {
                is String -> JsonPrimitive(value)
                is Number -> JsonPrimitive(value)
                is Boolean -> JsonPrimitive(value)
                else -> throw IllegalArgumentException("Unsupported value type")
            }
            return this
        }
        
        /**
         * Add a greater than condition
         */
        fun whereGreaterThan(field: String, value: Number): QueryBuilder {
            query[field] = buildJsonObject {
                put("\$gt", JsonPrimitive(value))
            }
            return this
        }
        
        /**
         * Add a less than condition
         */
        fun whereLessThan(field: String, value: Number): QueryBuilder {
            query[field] = buildJsonObject {
                put("\$lt", JsonPrimitive(value))
            }
            return this
        }
        
        /**
         * Add an "in" condition
         */
        fun whereIn(field: String, values: List<Any>): QueryBuilder {
            val jsonArray = buildJsonArray {
                values.forEach { value ->
                    when (value) {
                        is String -> add(JsonPrimitive(value))
                        is Number -> add(JsonPrimitive(value))
                        is Boolean -> add(JsonPrimitive(value))
                        else -> throw IllegalArgumentException("Unsupported value type")
                    }
                }
            }
            query[field] = buildJsonObject {
                put("\$in", jsonArray)
            }
            return this
        }
        
        /**
         * Build the query
         */
        fun build(): JsonObject = JsonObject(query)
    }
}

// Extension function to convert hex string to ByteArray
private fun String.hexToByteArray(): ByteArray {
    require(length % 2 == 0) { "Hex string must have even length" }
    return chunked(2)
        .map { it.toInt(16).toByte() }
        .toByteArray()
}