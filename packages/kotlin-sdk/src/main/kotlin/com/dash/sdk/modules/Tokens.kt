package com.dash.sdk.modules

import com.dash.sdk.DataContract
import com.dash.sdk.Identity
import com.dash.sdk.SDK
import com.dash.sdk.ffi.DashSDKFFI
import com.dash.sdk.utils.Converters.checkResult
import com.dash.sdk.utils.ensureSize
import com.sun.jna.Pointer
import com.sun.jna.ptr.LongByReference
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import mu.KotlinLogging

private val logger = KotlinLogging.logger {}

/**
 * Module for token operations
 */
class Tokens internal constructor(
    private val sdk: SDK,
    private val sdkHandle: Pointer
) {
    /**
     * Mint new tokens
     * 
     * @param contract The data contract containing the token
     * @param tokenPosition The position of the token within the contract
     * @param amount The amount of tokens to mint
     * @param recipientId Optional recipient identity ID (null for issuer)
     * @param issuer The identity that has minting permissions
     */
    suspend fun mint(
        contract: DataContract,
        tokenPosition: Short,
        amount: Long,
        recipientId: ByteArray? = null,
        issuer: Identity
    ) = withContext(Dispatchers.IO) {
        logger.debug { "Minting $amount tokens at position $tokenPosition" }
        
        val result = DashSDKFFI.INSTANCE.dash_sdk_token_mint(
            sdkHandle,
            contract.getHandle(),
            tokenPosition,
            amount,
            recipientId,
            recipientId?.size ?: 0,
            issuer.getHandle()
        )
        
        checkResult(result)
        logger.info { "Successfully minted $amount tokens" }
    }
    
    /**
     * Transfer tokens between identities
     * 
     * @param contract The data contract containing the token
     * @param tokenPosition The position of the token within the contract
     * @param amount The amount of tokens to transfer
     * @param sender The identity sending the tokens
     * @param recipientId The recipient identity ID
     */
    suspend fun transfer(
        contract: DataContract,
        tokenPosition: Short,
        amount: Long,
        sender: Identity,
        recipientId: ByteArray
    ) = withContext(Dispatchers.IO) {
        require(recipientId.size == 32) { "Recipient ID must be 32 bytes" }
        
        logger.debug { "Transferring $amount tokens from ${sender.idBase58} to recipient" }
        
        val result = DashSDKFFI.INSTANCE.dash_sdk_token_transfer(
            sdkHandle,
            contract.getHandle(),
            tokenPosition,
            amount,
            sender.getHandle(),
            recipientId,
            recipientId.size
        )
        
        checkResult(result)
        logger.info { "Successfully transferred $amount tokens" }
    }
    
    /**
     * Burn (destroy) tokens
     * 
     * @param contract The data contract containing the token
     * @param tokenPosition The position of the token within the contract
     * @param amount The amount of tokens to burn
     * @param owner The identity that owns the tokens
     */
    suspend fun burn(
        contract: DataContract,
        tokenPosition: Short,
        amount: Long,
        owner: Identity
    ) = withContext(Dispatchers.IO) {
        logger.debug { "Burning $amount tokens for ${owner.idBase58}" }
        
        val result = DashSDKFFI.INSTANCE.dash_sdk_token_burn(
            sdkHandle,
            contract.getHandle(),
            tokenPosition,
            amount,
            owner.getHandle()
        )
        
        checkResult(result)
        logger.info { "Successfully burned $amount tokens" }
    }
    
    /**
     * Get token balance for an identity
     * 
     * @param contractId The data contract ID containing the token
     * @param tokenPosition The position of the token within the contract
     * @param identityId The identity ID to check balance for
     * @return The token balance
     */
    suspend fun getBalance(
        contractId: ByteArray,
        tokenPosition: Short,
        identityId: ByteArray
    ): Long = withContext(Dispatchers.IO) {
        require(contractId.size == 32) { "Contract ID must be 32 bytes" }
        require(identityId.size == 32) { "Identity ID must be 32 bytes" }
        
        logger.debug { "Fetching token balance for identity" }
        
        val balanceRef = LongByReference()
        val result = DashSDKFFI.INSTANCE.dash_sdk_token_balance(
            sdkHandle,
            contractId,
            contractId.size,
            tokenPosition,
            identityId,
            identityId.size,
            balanceRef
        )
        
        checkResult(result)
        val balance = balanceRef.value
        logger.debug { "Token balance: $balance" }
        balance
    }
    
    /**
     * Freeze an identity's tokens
     * 
     * @param contract The data contract containing the token
     * @param tokenPosition The position of the token within the contract
     * @param identityToFreeze The identity ID to freeze
     * @param actionTaker The identity with freeze permissions
     */
    suspend fun freeze(
        contract: DataContract,
        tokenPosition: Short,
        identityToFreeze: ByteArray,
        actionTaker: Identity
    ) = withContext(Dispatchers.IO) {
        require(identityToFreeze.size == 32) { "Identity ID must be 32 bytes" }
        
        logger.debug { "Freezing tokens for identity" }
        
        val result = DashSDKFFI.INSTANCE.dash_sdk_token_freeze(
            sdkHandle,
            contract.getHandle(),
            tokenPosition,
            identityToFreeze,
            identityToFreeze.size,
            actionTaker.getHandle()
        )
        
        checkResult(result)
        logger.info { "Successfully froze tokens" }
    }
    
    /**
     * Unfreeze an identity's tokens
     * 
     * @param contract The data contract containing the token
     * @param tokenPosition The position of the token within the contract
     * @param identityToUnfreeze The identity ID to unfreeze
     * @param actionTaker The identity with unfreeze permissions
     */
    suspend fun unfreeze(
        contract: DataContract,
        tokenPosition: Short,
        identityToUnfreeze: ByteArray,
        actionTaker: Identity
    ) = withContext(Dispatchers.IO) {
        require(identityToUnfreeze.size == 32) { "Identity ID must be 32 bytes" }
        
        logger.debug { "Unfreezing tokens for identity" }
        
        val result = DashSDKFFI.INSTANCE.dash_sdk_token_unfreeze(
            sdkHandle,
            contract.getHandle(),
            tokenPosition,
            identityToUnfreeze,
            identityToUnfreeze.size,
            actionTaker.getHandle()
        )
        
        checkResult(result)
        logger.info { "Successfully unfroze tokens" }
    }
    
    /**
     * Data class representing token metadata
     */
    data class TokenMetadata(
        val name: String,
        val symbol: String,
        val decimals: Int,
        val totalSupply: Long,
        val maxSupply: Long?,
        val isMintable: Boolean,
        val isBurnable: Boolean,
        val isTransferable: Boolean
    )
    
    /**
     * Data class for token transfer parameters
     */
    data class TransferParams(
        val contract: DataContract,
        val tokenPosition: Short,
        val amount: Long,
        val sender: Identity,
        val recipientId: ByteArray
    ) {
        init {
            require(recipientId.size == 32) { "Recipient ID must be 32 bytes" }
            require(amount > 0) { "Transfer amount must be positive" }
        }
    }
}