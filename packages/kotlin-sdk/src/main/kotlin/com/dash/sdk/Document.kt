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
 * Represents a Dash Platform Document
 */
class Document internal constructor(
    private val handle: Pointer,
    private val sdk: SDK
) : Closeable {
    
    /**
     * Get the document ID as a byte array
     */
    val id: ByteArray by lazy {
        // Document IDs are 32 bytes
        _id ?: throw IllegalStateException("Document ID not available")
    }
    
    private var _id: ByteArray? = null
    
    internal fun setId(id: ByteArray) {
        _id = id.ensureSize(32)
    }
    
    /**
     * Get the document ID as a base58 string
     */
    val idBase58: String
        get() = id.toBase58()
    
    /**
     * Get the owner identity ID
     */
    val ownerId: ByteArray by lazy {
        _ownerId ?: throw IllegalStateException("Owner ID not available")
    }
    
    private var _ownerId: ByteArray? = null
    
    internal fun setOwnerId(ownerId: ByteArray) {
        _ownerId = ownerId.ensureSize(32)
    }
    
    /**
     * Get the owner identity ID as a base58 string
     */
    val ownerIdBase58: String
        get() = ownerId.toBase58()
    
    /**
     * Get the document type
     */
    val type: String by lazy {
        _type ?: throw IllegalStateException("Document type not available")
    }
    
    private var _type: String? = null
    
    internal fun setType(type: String) {
        _type = type
    }
    
    /**
     * Get the document properties as JSON
     */
    val properties: JsonObject by lazy {
        _properties ?: throw IllegalStateException("Document properties not available")
    }
    
    private var _properties: JsonObject? = null
    
    internal fun setProperties(properties: JsonObject) {
        _properties = properties
    }
    
    /**
     * Get the revision number
     */
    val revision: Long by lazy {
        _revision ?: 1L
    }
    
    private var _revision: Long? = null
    
    internal fun setRevision(revision: Long) {
        _revision = revision
    }
    
    /**
     * Get the internal handle (for SDK internal use)
     */
    internal fun getHandle(): Pointer = handle
    
    /**
     * Release the document handle
     */
    override fun close() {
        DashSDKFFI.INSTANCE.dash_sdk_document_handle_destroy(handle)
    }
    
    companion object {
        /**
         * Create a document ID from a base58 string
         */
        fun idFromBase58(idBase58: String): ByteArray = idBase58.base58ToByteArray().ensureSize(32)
        
        /**
         * Create a document ID from a hex string
         */
        fun idFromHex(idHex: String): ByteArray = idHex.hexToByteArray().ensureSize(32)
    }
}