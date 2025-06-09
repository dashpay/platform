package com.dash.sdk.modules

import com.dash.sdk.DataContract
import com.dash.sdk.Identity
import com.dash.sdk.SDK
import com.dash.sdk.ffi.DashSDKFFI
import com.dash.sdk.utils.Converters
import com.dash.sdk.utils.Converters.checkResult
import com.dash.sdk.utils.Converters.getHandleFromResult
import com.dash.sdk.utils.base58ToByteArray
import com.dash.sdk.utils.ensureSize
import com.dash.sdk.utils.hexToByteArray
import com.sun.jna.Pointer
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import mu.KotlinLogging

private val logger = KotlinLogging.logger {}

/**
 * Module for data contract operations
 */
class Contracts internal constructor(
    private val sdk: SDK,
    private val sdkHandle: Pointer
) {
    /**
     * Fetch a data contract by ID
     * 
     * @param contractId Contract ID as byte array (32 bytes)
     * @return The fetched data contract or null if not found
     */
    suspend fun fetch(contractId: ByteArray): DataContract? = withContext(Dispatchers.IO) {
        require(contractId.size == 32) { "Contract ID must be 32 bytes" }
        
        logger.debug { "Fetching data contract" }
        
        val result = DashSDKFFI.INSTANCE.dash_sdk_data_contract_fetch(
            sdkHandle,
            contractId,
            contractId.size
        )
        
        try {
            val handle = getHandleFromResult(result, DashSDKFFI.DashSDKResultDataType.DataContractHandle)
            handle?.let {
                val contract = DataContract(it, sdk)
                contract.setId(contractId)
                logger.debug { "Data contract fetched successfully" }
                contract
            }
        } catch (e: Exception) {
            logger.debug { "Data contract not found or error occurred: ${e.message}" }
            null
        }
    }
    
    /**
     * Fetch a data contract by base58 ID
     * 
     * @param contractIdBase58 Contract ID as base58 string
     * @return The fetched data contract or null if not found
     */
    suspend fun fetchByBase58(contractIdBase58: String): DataContract? {
        val contractId = contractIdBase58.base58ToByteArray().ensureSize(32)
        return fetch(contractId)
    }
    
    /**
     * Fetch a data contract by hex ID
     * 
     * @param contractIdHex Contract ID as hex string
     * @return The fetched data contract or null if not found
     */
    suspend fun fetchByHex(contractIdHex: String): DataContract? {
        val contractId = contractIdHex.hexToByteArray().ensureSize(32)
        return fetch(contractId)
    }
    
    /**
     * Create and publish a new data contract
     * 
     * @param contractDefinition Contract definition as JSON object
     * @param identity Identity to use as the contract owner
     * @return The created data contract
     */
    suspend fun create(
        contractDefinition: JsonObject,
        identity: Identity
    ): DataContract = withContext(Dispatchers.IO) {
        logger.debug { "Creating new data contract" }
        
        val contractJson = Json.encodeToString(JsonObject.serializer(), contractDefinition)
        
        val result = DashSDKFFI.INSTANCE.dash_sdk_data_contract_put(
            sdkHandle,
            contractJson,
            identity.getHandle()
        )
        
        val handle = getHandleFromResult(result, DashSDKFFI.DashSDKResultDataType.DataContractHandle)
            ?: throw IllegalStateException("Data contract creation returned null handle")
        
        val contract = DataContract(handle, sdk)
        contract.setDefinition(contractDefinition)
        logger.info { "Data contract created successfully" }
        contract
    }
    
    /**
     * Create and publish a new data contract from JSON string
     * 
     * @param contractJson Contract definition as JSON string
     * @param identity Identity to use as the contract owner
     * @return The created data contract
     */
    suspend fun createFromJson(
        contractJson: String,
        identity: Identity
    ): DataContract = withContext(Dispatchers.IO) {
        logger.debug { "Creating new data contract from JSON" }
        
        val result = DashSDKFFI.INSTANCE.dash_sdk_data_contract_put(
            sdkHandle,
            contractJson,
            identity.getHandle()
        )
        
        val handle = getHandleFromResult(result, DashSDKFFI.DashSDKResultDataType.DataContractHandle)
            ?: throw IllegalStateException("Data contract creation returned null handle")
        
        val contract = DataContract(handle, sdk)
        val definition = Json.parseToJsonElement(contractJson) as JsonObject
        contract.setDefinition(definition)
        logger.info { "Data contract created successfully from JSON" }
        contract
    }
    
    /**
     * Update an existing data contract
     * 
     * @param contract The contract to update
     * @param updates Updates to apply as JSON object
     * @param identity Identity to use for the update (must be contract owner)
     * @return The updated data contract
     */
    suspend fun update(
        contract: DataContract,
        updates: JsonObject,
        identity: Identity
    ): DataContract = withContext(Dispatchers.IO) {
        logger.debug { "Updating data contract" }
        
        // Merge updates with existing definition
        val currentDefinition = contract.definition
        val updatedDefinition = JsonObject(currentDefinition + updates)
        
        val contractJson = Json.encodeToString(JsonObject.serializer(), updatedDefinition)
        
        val result = DashSDKFFI.INSTANCE.dash_sdk_data_contract_put(
            sdkHandle,
            contractJson,
            identity.getHandle()
        )
        
        val handle = getHandleFromResult(result, DashSDKFFI.DashSDKResultDataType.DataContractHandle)
            ?: throw IllegalStateException("Data contract update returned null handle")
        
        val updatedContract = DataContract(handle, sdk)
        updatedContract.setId(contract.id)
        updatedContract.setDefinition(updatedDefinition)
        logger.info { "Data contract updated successfully" }
        updatedContract
    }
}