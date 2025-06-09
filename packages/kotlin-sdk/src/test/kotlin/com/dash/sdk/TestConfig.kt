package com.dash.sdk

import com.dash.sdk.types.Network
import com.dash.sdk.types.SDKConfig
import com.dash.sdk.utils.hexToByteArray

/**
 * Test configuration for SDK tests
 */
object TestConfig {
    /**
     * Test network configuration
     */
    val sdkConfig = SDKConfig(
        network = Network.TESTNET,
        skipAssetLockProofVerification = true,
        requestRetryCount = 3,
        requestTimeoutMs = 30000
    )
    
    /**
     * Well-known test identity IDs (from rs-sdk test vectors)
     */
    object TestIdentities {
        // These are example IDs from the test vectors
        val ALICE_ID = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF".base58ToByteArray()
        val BOB_ID = "AW97WU3tGJQvfJQKAqSqVK8qe4qZnEH1YTnJYnM3vLa6".base58ToByteArray()
        val CHARLIE_ID = "BpgCqVGYXTKhhrvF3yXQrJrPPMwrTEXBb85hkMEYRLmL".base58ToByteArray()
        
        // Non-existent identity for testing "not found" cases
        val NON_EXISTENT_ID = "1111111111111111111111111111111111111111111".base58ToByteArray()
    }
    
    /**
     * Well-known test data contract IDs
     */
    object TestContracts {
        // DPNS contract ID on testnet
        val DPNS_CONTRACT_ID = "36ez8VqoDbR8NkdXwFaf9Tp8ukBdQJLLRqbLhNbvVhXU".base58ToByteArray()
        
        // Dashpay contract ID on testnet
        val DASHPAY_CONTRACT_ID = "Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7".base58ToByteArray()
        
        // Non-existent contract for testing
        val NON_EXISTENT_CONTRACT_ID = "2222222222222222222222222222222222222222222".base58ToByteArray()
    }
    
    /**
     * Test document types
     */
    object TestDocumentTypes {
        const val DPNS_DOMAIN = "domain"
        const val DPNS_PREORDER = "preorder"
        const val DASHPAY_CONTACT_REQUEST = "contactRequest"
        const val DASHPAY_PROFILE = "profile"
    }
    
    /**
     * Test token positions
     */
    object TestTokens {
        const val DEFAULT_TOKEN_POSITION: Short = 0
    }
}

// Extension function for test data
private fun String.base58ToByteArray(): ByteArray {
    val alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
    val base = java.math.BigInteger.valueOf(58)
    
    var leadingOnes = 0
    for (char in this) {
        if (char == '1') {
            leadingOnes++
        } else {
            break
        }
    }
    
    var num = java.math.BigInteger.ZERO
    for (char in this) {
        val digit = alphabet.indexOf(char)
        require(digit >= 0) { "Invalid base58 character: $char" }
        num = num.multiply(base).add(java.math.BigInteger.valueOf(digit.toLong()))
    }
    
    val bytes = num.toByteArray()
    val result = if (bytes[0] == 0.toByte() && bytes.size > 1) {
        bytes.sliceArray(1 until bytes.size)
    } else {
        bytes
    }
    
    return ByteArray(leadingOnes) + result
}