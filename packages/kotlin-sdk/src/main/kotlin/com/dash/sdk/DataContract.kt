package com.dash.sdk

import com.dash.sdk.ffi.DashSDKFFI
import com.dash.sdk.utils.base58ToByteArray
import com.dash.sdk.utils.ensureSize
import com.dash.sdk.utils.hexToByteArray
import com.dash.sdk.utils.toBase58
import com.sun.jna.Pointer
import kotlinx.serialization.json.JsonObject
import java.io.Closeable

/**
 * Represents a Dash Platform Data Contract
 */
class DataContract internal constructor(
    private val handle: Pointer,
    private val sdk: SDK
) : Closeable {
    
    /**
     * Get the contract ID as a byte array
     */
    val id: ByteArray by lazy {
        // Contract IDs are 32 bytes
        _id ?: throw IllegalStateException("Contract ID not available")
    }
    
    private var _id: ByteArray? = null
    
    internal fun setId(id: ByteArray) {
        _id = id.ensureSize(32)
    }
    
    /**
     * Get the contract ID as a base58 string
     */
    val idBase58: String
        get() = id.toBase58()
    
    /**
     * Get the contract definition as JSON
     */
    val definition: JsonObject by lazy {
        _definition ?: throw IllegalStateException("Contract definition not available")
    }
    
    private var _definition: JsonObject? = null
    
    internal fun setDefinition(definition: JsonObject) {
        _definition = definition
    }
    
    /**
     * Get the internal handle (for SDK internal use)
     */
    internal fun getHandle(): Pointer = handle
    
    /**
     * Release the data contract handle
     */
    override fun close() {
        DashSDKFFI.INSTANCE.dash_sdk_data_contract_handle_destroy(handle)
    }
    
    companion object {
        /**
         * Create a contract ID from a base58 string
         */
        fun idFromBase58(idBase58: String): ByteArray = idBase58.base58ToByteArray().ensureSize(32)
        
        /**
         * Create a contract ID from a hex string
         */
        fun idFromHex(idHex: String): ByteArray = idHex.hexToByteArray().ensureSize(32)
    }
}