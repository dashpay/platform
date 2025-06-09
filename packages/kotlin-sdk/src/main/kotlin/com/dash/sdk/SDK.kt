package com.dash.sdk

import com.dash.sdk.ffi.DashSDKFFI
import com.dash.sdk.modules.*
import com.dash.sdk.types.SDKConfig
import com.dash.sdk.utils.Converters
import com.dash.sdk.utils.Converters.checkResult
import com.dash.sdk.utils.Converters.toFFI
import com.sun.jna.Pointer
import mu.KotlinLogging
import java.io.Closeable

private val logger = KotlinLogging.logger {}

/**
 * Main entry point for the Dash Platform SDK
 * 
 * This class provides access to all Dash Platform functionality including:
 * - Identity management
 * - Data contracts
 * - Documents
 * - Tokens
 */
class SDK(config: SDKConfig) : Closeable {
    private val handle: Pointer
    
    /**
     * Module for identity-related operations
     */
    val identities: Identities
    
    /**
     * Module for data contract operations
     */
    val contracts: Contracts
    
    /**
     * Module for document operations
     */
    val documents: Documents
    
    /**
     * Module for token operations
     */
    val tokens: Tokens

    init {
        logger.info { "Initializing Dash SDK with network: ${config.network}" }
        
        // Initialize the FFI library
        DashSDKFFI.INSTANCE.dash_sdk_init()
        
        // Create SDK instance
        val ffiConfig = config.toFFI()
        val result = DashSDKFFI.INSTANCE.dash_sdk_create(ffiConfig)
        checkResult(result)
        
        handle = result.data 
            ?: throw IllegalStateException("SDK creation returned null handle")
        
        logger.debug { "SDK handle created successfully" }
        
        // Initialize modules
        identities = Identities(this, handle)
        contracts = Contracts(this, handle)
        documents = Documents(this, handle)
        tokens = Tokens(this, handle)
        
        logger.info { "Dash SDK initialized successfully" }
    }

    /**
     * Get the internal SDK handle (for module use)
     */
    internal fun getHandle(): Pointer = handle

    /**
     * Close the SDK and release resources
     */
    override fun close() {
        logger.info { "Closing Dash SDK" }
        DashSDKFFI.INSTANCE.dash_sdk_destroy(handle)
        logger.debug { "SDK handle destroyed" }
    }

    companion object {
        /**
         * Get the SDK version
         */
        fun getVersion(): String = DashSDKFFI.INSTANCE.dash_sdk_version()

        /**
         * Get current time in milliseconds (platform time)
         */
        fun getCurrentTimeMs(): Long = DashSDKFFI.INSTANCE.dash_sdk_current_time_ms()

        /**
         * Get mainnet core chains configuration
         */
        fun getMainnetCoreChains(): String = DashSDKFFI.INSTANCE.dash_sdk_mainnet_core_chains_json()

        /**
         * Get testnet core chains configuration
         */
        fun getTestnetCoreChains(): String = DashSDKFFI.INSTANCE.dash_sdk_testnet_core_chains_json()

        /**
         * Get devnet core chains configuration
         */
        fun getDevnetCoreChains(): String = DashSDKFFI.INSTANCE.dash_sdk_devnet_core_chains_json()
    }
}