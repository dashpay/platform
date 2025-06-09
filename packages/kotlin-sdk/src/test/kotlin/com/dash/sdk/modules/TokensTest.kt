package com.dash.sdk.modules

import com.dash.sdk.SDK
import com.dash.sdk.TestConfig
import com.dash.sdk.utils.hexToByteArray
import kotlinx.coroutines.test.runTest
import org.assertj.core.api.Assertions.assertThat
import org.assertj.core.api.Assertions.assertThatThrownBy
import org.junit.jupiter.api.*
import org.junit.jupiter.api.condition.DisabledIfEnvironmentVariable

/**
 * Integration tests for the Tokens module
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
class TokensTest {
    private lateinit var sdk: SDK
    
    @BeforeAll
    fun setup() {
        sdk = SDK(TestConfig.sdkConfig)
    }
    
    @AfterAll
    fun teardown() {
        sdk.close()
    }
    
    @Test
    @DisplayName("Should fetch token balance for identity")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testGetTokenBalance() = runTest {
        // Arrange
        val contractId = TestConfig.TestContracts.DPNS_CONTRACT_ID
        val identityId = TestConfig.TestIdentities.ALICE_ID
        val tokenPosition = TestConfig.TestTokens.DEFAULT_TOKEN_POSITION
        
        // Act
        val balance = sdk.tokens.getBalance(contractId, tokenPosition, identityId)
        
        // Assert
        assertThat(balance).isGreaterThanOrEqualTo(0)
    }
    
    @Test
    @DisplayName("Should validate contract ID size for balance query")
    fun testGetBalanceInvalidContractId() {
        // Arrange
        val invalidContractId = ByteArray(16) // Should be 32 bytes
        val identityId = TestConfig.TestIdentities.ALICE_ID
        
        // Act & Assert
        assertThatThrownBy {
            kotlinx.coroutines.runBlocking {
                sdk.tokens.getBalance(invalidContractId, 0, identityId)
            }
        }.isInstanceOf(IllegalArgumentException::class.java)
            .hasMessageContaining("Contract ID must be 32 bytes")
    }
    
    @Test
    @DisplayName("Should validate identity ID size for balance query")
    fun testGetBalanceInvalidIdentityId() {
        // Arrange
        val contractId = TestConfig.TestContracts.DPNS_CONTRACT_ID
        val invalidIdentityId = ByteArray(16) // Should be 32 bytes
        
        // Act & Assert
        assertThatThrownBy {
            kotlinx.coroutines.runBlocking {
                sdk.tokens.getBalance(contractId, 0, invalidIdentityId)
            }
        }.isInstanceOf(IllegalArgumentException::class.java)
            .hasMessageContaining("Identity ID must be 32 bytes")
    }
    
    @Test
    @DisplayName("Should mint tokens")
    @Disabled("Requires identity with minting permissions")
    fun testMintTokens() = runTest {
        // Arrange
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DPNS_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        val issuer = sdk.identities.fetch(TestConfig.TestIdentities.ALICE_ID)
        assertThat(issuer).isNotNull
        
        val amount = 1000L
        val tokenPosition = TestConfig.TestTokens.DEFAULT_TOKEN_POSITION
        
        // Act & Assert (should not throw)
        sdk.tokens.mint(
            contract = contract!!,
            tokenPosition = tokenPosition,
            amount = amount,
            issuer = issuer!!
        )
    }
    
    @Test
    @DisplayName("Should mint tokens to specific recipient")
    @Disabled("Requires identity with minting permissions")
    fun testMintTokensToRecipient() = runTest {
        // Arrange
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DPNS_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        val issuer = sdk.identities.fetch(TestConfig.TestIdentities.ALICE_ID)
        assertThat(issuer).isNotNull
        
        val recipientId = TestConfig.TestIdentities.BOB_ID
        val amount = 500L
        val tokenPosition = TestConfig.TestTokens.DEFAULT_TOKEN_POSITION
        
        // Act & Assert (should not throw)
        sdk.tokens.mint(
            contract = contract!!,
            tokenPosition = tokenPosition,
            amount = amount,
            recipientId = recipientId,
            issuer = issuer!!
        )
    }
    
    @Test
    @DisplayName("Should transfer tokens between identities")
    @Disabled("Requires funded token balances")
    fun testTransferTokens() = runTest {
        // Arrange
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DPNS_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        val sender = sdk.identities.fetch(TestConfig.TestIdentities.ALICE_ID)
        assertThat(sender).isNotNull
        
        val recipientId = TestConfig.TestIdentities.BOB_ID
        val amount = 100L
        val tokenPosition = TestConfig.TestTokens.DEFAULT_TOKEN_POSITION
        
        // Act & Assert (should not throw)
        sdk.tokens.transfer(
            contract = contract!!,
            tokenPosition = tokenPosition,
            amount = amount,
            sender = sender!!,
            recipientId = recipientId
        )
    }
    
    @Test
    @DisplayName("Should burn tokens")
    @Disabled("Requires identity with token balance")
    fun testBurnTokens() = runTest {
        // Arrange
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DPNS_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        val owner = sdk.identities.fetch(TestConfig.TestIdentities.ALICE_ID)
        assertThat(owner).isNotNull
        
        val amount = 50L
        val tokenPosition = TestConfig.TestTokens.DEFAULT_TOKEN_POSITION
        
        // Act & Assert (should not throw)
        sdk.tokens.burn(
            contract = contract!!,
            tokenPosition = tokenPosition,
            amount = amount,
            owner = owner!!
        )
    }
    
    @Test
    @DisplayName("Should freeze identity tokens")
    @Disabled("Requires freeze permissions")
    fun testFreezeTokens() = runTest {
        // Arrange
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DPNS_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        val actionTaker = sdk.identities.fetch(TestConfig.TestIdentities.ALICE_ID)
        assertThat(actionTaker).isNotNull
        
        val identityToFreeze = TestConfig.TestIdentities.BOB_ID
        val tokenPosition = TestConfig.TestTokens.DEFAULT_TOKEN_POSITION
        
        // Act & Assert (should not throw)
        sdk.tokens.freeze(
            contract = contract!!,
            tokenPosition = tokenPosition,
            identityToFreeze = identityToFreeze,
            actionTaker = actionTaker!!
        )
    }
    
    @Test
    @DisplayName("Should unfreeze identity tokens")
    @Disabled("Requires unfreeze permissions and frozen tokens")
    fun testUnfreezeTokens() = runTest {
        // Arrange
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DPNS_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        val actionTaker = sdk.identities.fetch(TestConfig.TestIdentities.ALICE_ID)
        assertThat(actionTaker).isNotNull
        
        val identityToUnfreeze = TestConfig.TestIdentities.BOB_ID
        val tokenPosition = TestConfig.TestTokens.DEFAULT_TOKEN_POSITION
        
        // Act & Assert (should not throw)
        sdk.tokens.unfreeze(
            contract = contract!!,
            tokenPosition = tokenPosition,
            identityToUnfreeze = identityToUnfreeze,
            actionTaker = actionTaker!!
        )
    }
    
    @Test
    @DisplayName("Should validate transfer parameters")
    fun testTransferParamsValidation() {
        // Arrange
        val validRecipientId = TestConfig.TestIdentities.BOB_ID
        val invalidRecipientId = ByteArray(16) // Wrong size
        
        // Act & Assert - valid params should not throw
        val validParams = Tokens.TransferParams(
            contract = MockDataContract(),
            tokenPosition = 0,
            amount = 100,
            sender = MockIdentity(),
            recipientId = validRecipientId
        )
        assertThat(validParams).isNotNull
        
        // Invalid recipient ID size
        assertThatThrownBy {
            Tokens.TransferParams(
                contract = MockDataContract(),
                tokenPosition = 0,
                amount = 100,
                sender = MockIdentity(),
                recipientId = invalidRecipientId
            )
        }.isInstanceOf(IllegalArgumentException::class.java)
            .hasMessageContaining("Recipient ID must be 32 bytes")
        
        // Invalid amount
        assertThatThrownBy {
            Tokens.TransferParams(
                contract = MockDataContract(),
                tokenPosition = 0,
                amount = 0,
                sender = MockIdentity(),
                recipientId = validRecipientId
            )
        }.isInstanceOf(IllegalArgumentException::class.java)
            .hasMessageContaining("Transfer amount must be positive")
    }
    
    // Mock classes for testing parameter validation
    private class MockDataContract : com.dash.sdk.DataContract(
        com.sun.jna.Memory(8), 
        SDK(TestConfig.sdkConfig)
    )
    
    private class MockIdentity : com.dash.sdk.Identity(
        com.sun.jna.Memory(8),
        SDK(TestConfig.sdkConfig)
    )
}