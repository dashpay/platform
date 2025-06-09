package com.dash.sdk

import com.dash.sdk.ffi.DashSDKFFI
import com.dash.sdk.utils.Converters
import com.dash.sdk.utils.base58ToByteArray
import com.dash.sdk.utils.ensureSize
import com.dash.sdk.utils.hexToByteArray
import com.dash.sdk.utils.toBase58
import com.sun.jna.Pointer
import java.io.Closeable

/**
 * Represents a Dash Platform Identity
 */
class Identity internal constructor(
    private val handle: Pointer,
    private val sdk: SDK
) : Closeable {
    
    /**
     * Get the identity ID as a byte array
     */
    val id: ByteArray by lazy {
        // Identity IDs are 32 bytes
        // We'll need to get this from the handle - this would typically be exposed by FFI
        // For now, we'll store it when the identity is created/fetched
        _id ?: throw IllegalStateException("Identity ID not available")
    }
    
    private var _id: ByteArray? = null
    
    internal fun setId(id: ByteArray) {
        _id = id.ensureSize(32)
    }
    
    /**
     * Get the identity ID as a base58 string
     */
    val idBase58: String
        get() = id.toBase58()
    
    /**
     * Get the current balance of this identity
     */
    suspend fun getBalance(): Long {
        val balanceRef = com.sun.jna.ptr.LongByReference()
        val result = DashSDKFFI.INSTANCE.dash_sdk_identity_fetch_balance(
            sdk.getHandle(),
            id,
            id.size,
            balanceRef
        )
        Converters.checkResult(result)
        return balanceRef.value
    }
    
    /**
     * Top up this identity with credits
     * 
     * @param assetLockProofBase64 Base64 encoded asset lock proof
     */
    suspend fun topUp(assetLockProofBase64: String) {
        val result = DashSDKFFI.INSTANCE.dash_sdk_identity_topup(
            sdk.getHandle(),
            handle,
            assetLockProofBase64
        )
        Converters.checkResult(result)
    }
    
    /**
     * Withdraw credits from this identity
     * 
     * @param amount Amount to withdraw in credits
     * @param toScript Destination script bytes
     * @param coreFeePerByte Core fee per byte
     */
    suspend fun withdraw(
        amount: Long,
        toScript: ByteArray,
        coreFeePerByte: Int = 1
    ) {
        val result = DashSDKFFI.INSTANCE.dash_sdk_identity_withdraw(
            sdk.getHandle(),
            handle,
            amount,
            toScript,
            toScript.size,
            coreFeePerByte
        )
        Converters.checkResult(result)
    }
    
    /**
     * Get the internal handle (for SDK internal use)
     */
    internal fun getHandle(): Pointer = handle
    
    /**
     * Release the identity handle
     */
    override fun close() {
        DashSDKFFI.INSTANCE.dash_sdk_identity_handle_destroy(handle)
    }
    
    companion object {
        /**
         * Create an Identity from a base58 string ID
         */
        fun fromBase58(idBase58: String): ByteArray = idBase58.base58ToByteArray().ensureSize(32)
        
        /**
         * Create an Identity from a hex string ID
         */
        fun fromHex(idHex: String): ByteArray = idHex.hexToByteArray().ensureSize(32)
    }
}