package com.dash.sdk.utils

import com.dash.sdk.ffi.DashSDKFFI
import com.dash.sdk.types.DashSDKException
import com.dash.sdk.types.SDKConfig
import com.sun.jna.Memory
import com.sun.jna.Pointer
import com.sun.jna.ptr.IntByReference
import kotlinx.serialization.json.Json

/**
 * Utility functions for converting between Kotlin and FFI types
 */
object Converters {
    val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    /**
     * Convert SDK config to FFI config
     */
    fun SDKConfig.toFFI(): DashSDKFFI.DashSDKConfig {
        return DashSDKFFI.DashSDKConfig().apply {
            network = this@toFFI.network.value
            skip_asset_lock_proof_verification = this@toFFI.skipAssetLockProofVerification
            request_retry_count = this@toFFI.requestRetryCount
            request_timeout_ms = this@toFFI.requestTimeoutMs
            core_ip_address = this@toFFI.coreIpAddress
            platform_port = this@toFFI.platformPort
            dump_lookup_sessions_on_drop = this@toFFI.dumpLookupSessionsOnDrop
        }
    }

    /**
     * Check FFI result and throw exception if error
     */
    fun checkResult(result: DashSDKFFI.DashSDKResult) {
        result.error?.let { error ->
            val exception = when (error.code) {
                DashSDKFFI.DashSDKErrorCode.InvalidParameter -> 
                    DashSDKException.InvalidParameterException(error.message ?: "Invalid parameter")
                DashSDKFFI.DashSDKErrorCode.InvalidState -> 
                    DashSDKException.InvalidStateException(error.message ?: "Invalid state")
                DashSDKFFI.DashSDKErrorCode.NetworkError -> 
                    DashSDKException.NetworkException(error.message ?: "Network error")
                DashSDKFFI.DashSDKErrorCode.SerializationError -> 
                    DashSDKException.SerializationException(error.message ?: "Serialization error")
                DashSDKFFI.DashSDKErrorCode.ProtocolError -> 
                    DashSDKException.ProtocolException(error.message ?: "Protocol error")
                DashSDKFFI.DashSDKErrorCode.CryptoError -> 
                    DashSDKException.CryptoException(error.message ?: "Crypto error")
                DashSDKFFI.DashSDKErrorCode.NotFound -> 
                    DashSDKException.NotFoundException(error.message ?: "Not found")
                DashSDKFFI.DashSDKErrorCode.Timeout -> 
                    DashSDKException.TimeoutException(error.message ?: "Timeout")
                DashSDKFFI.DashSDKErrorCode.NotImplemented -> 
                    DashSDKException.NotImplementedException(error.message ?: "Not implemented")
                DashSDKFFI.DashSDKErrorCode.InternalError -> 
                    DashSDKException.InternalException(error.message ?: "Internal error")
                else -> 
                    DashSDKException.UnknownException(error.message ?: "Unknown error")
            }
            DashSDKFFI.INSTANCE.dash_sdk_error_free(error)
            throw exception
        }
    }

    /**
     * Get string from result
     */
    fun getStringFromResult(result: DashSDKFFI.DashSDKResult): String? {
        checkResult(result)
        return when (result.data_type) {
            DashSDKFFI.DashSDKResultDataType.String -> {
                val str = DashSDKFFI.INSTANCE.dash_sdk_result_get_string(result)
                str?.also { DashSDKFFI.INSTANCE.dash_sdk_string_free(it) }
            }
            else -> null
        }
    }

    /**
     * Get binary data from result
     */
    fun getBinaryDataFromResult(result: DashSDKFFI.DashSDKResult): ByteArray? {
        checkResult(result)
        return when (result.data_type) {
            DashSDKFFI.DashSDKResultDataType.BinaryData -> {
                val lengthRef = IntByReference()
                val dataPtr = DashSDKFFI.INSTANCE.dash_sdk_result_get_binary_data(result, lengthRef)
                dataPtr?.let {
                    val bytes = it.getByteArray(0, lengthRef.value)
                    DashSDKFFI.INSTANCE.dash_sdk_binary_data_free(it)
                    bytes
                }
            }
            else -> null
        }
    }

    /**
     * Get handle from result
     */
    fun getHandleFromResult(result: DashSDKFFI.DashSDKResult, expectedType: Int): Pointer? {
        checkResult(result)
        return when (result.data_type) {
            expectedType -> {
                when (expectedType) {
                    DashSDKFFI.DashSDKResultDataType.IdentityHandle -> 
                        DashSDKFFI.INSTANCE.dash_sdk_result_get_identity_handle(result)
                    DashSDKFFI.DashSDKResultDataType.DocumentHandle -> 
                        DashSDKFFI.INSTANCE.dash_sdk_result_get_document_handle(result)
                    DashSDKFFI.DashSDKResultDataType.DataContractHandle -> 
                        DashSDKFFI.INSTANCE.dash_sdk_result_get_data_contract_handle(result)
                    else -> null
                }
            }
            else -> null
        }
    }

    /**
     * Convert ByteArray to native memory
     */
    fun ByteArray.toNative(): Pointer {
        val memory = Memory(this.size.toLong())
        memory.write(0, this, 0, this.size)
        return memory
    }

    /**
     * Safe execute with cleanup
     */
    inline fun <T> safeExecute(cleanup: () -> Unit = {}, block: () -> T): T {
        return try {
            block()
        } finally {
            cleanup()
        }
    }
}