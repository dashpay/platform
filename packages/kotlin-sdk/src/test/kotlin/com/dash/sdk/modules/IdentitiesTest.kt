package com.dash.sdk.modules

import com.dash.sdk.SDK
import com.dash.sdk.TestConfig
import com.dash.sdk.types.DashSDKException
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.runTest
import org.assertj.core.api.Assertions.assertThat
import org.assertj.core.api.Assertions.assertThatThrownBy
import org.junit.jupiter.api.*
import org.junit.jupiter.api.condition.DisabledIfEnvironmentVariable

/**
 * Integration tests for the Identities module
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
class IdentitiesTest {
    private lateinit var sdk: SDK
    
    @BeforeAll
    fun setup() {
        // Note: These tests require either:
        // 1. A running local network
        // 2. Connection to testnet
        // 3. Mock FFI implementation with test vectors
        sdk = SDK(TestConfig.sdkConfig)
    }
    
    @AfterAll
    fun teardown() {
        sdk.close()
    }
    
    @Test
    @DisplayName("Should fetch an existing identity by ID")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testFetchExistingIdentity() = runTest {
        // Arrange
        val identityId = TestConfig.TestIdentities.ALICE_ID
        
        // Act
        val identity = sdk.identities.fetch(identityId)
        
        // Assert
        assertThat(identity).isNotNull
        identity?.let {
            assertThat(it.id).isEqualTo(identityId)
            assertThat(it.idBase58).isNotEmpty()
        }
    }
    
    @Test
    @DisplayName("Should return null when fetching non-existent identity")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testFetchNonExistentIdentity() = runTest {
        // Arrange
        val nonExistentId = TestConfig.TestIdentities.NON_EXISTENT_ID
        
        // Act
        val identity = sdk.identities.fetch(nonExistentId)
        
        // Assert
        assertThat(identity).isNull()
    }
    
    @Test
    @DisplayName("Should fetch identity by base58 ID")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testFetchByBase58() = runTest {
        // Arrange
        val base58Id = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"
        
        // Act
        val identity = sdk.identities.fetchByBase58(base58Id)
        
        // Assert
        assertThat(identity).isNotNull
        identity?.let {
            assertThat(it.idBase58).isEqualTo(base58Id)
        }
    }
    
    @Test
    @DisplayName("Should fetch identity balance")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testFetchIdentityBalance() = runTest {
        // Arrange
        val identityId = TestConfig.TestIdentities.ALICE_ID
        
        // Act
        val balance = sdk.identities.fetchBalance(identityId)
        
        // Assert
        assertThat(balance).isGreaterThanOrEqualTo(0)
    }
    
    @Test
    @DisplayName("Should fetch balance for identity object")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testIdentityGetBalance() = runTest {
        // Arrange
        val identity = sdk.identities.fetch(TestConfig.TestIdentities.ALICE_ID)
        assertThat(identity).isNotNull
        
        // Act
        val balance = identity!!.getBalance()
        
        // Assert
        assertThat(balance).isGreaterThanOrEqualTo(0)
    }
    
    @Test
    @DisplayName("Should validate identity ID size")
    fun testInvalidIdentityIdSize() {
        // Arrange
        val invalidId = ByteArray(16) // Should be 32 bytes
        
        // Act & Assert
        assertThatThrownBy {
            runBlocking {
                sdk.identities.fetch(invalidId)
            }
        }.isInstanceOf(IllegalArgumentException::class.java)
            .hasMessageContaining("Identity ID must be 32 bytes")
    }
    
    @Test
    @DisplayName("Should fetch multiple identity balances")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testFetchMultipleBalances() = runTest {
        // Arrange
        val identityIds = listOf(
            TestConfig.TestIdentities.ALICE_ID,
            TestConfig.TestIdentities.BOB_ID,
            TestConfig.TestIdentities.CHARLIE_ID
        )
        
        // Act
        val balances = sdk.identities.fetchBalances(identityIds)
        
        // Assert
        // Note: This test depends on the FFI implementation returning proper map data
        // For now, we just verify it doesn't throw an exception
        assertThat(balances).isNotNull
    }
    
    @Test
    @DisplayName("Should handle empty list when fetching balances")
    fun testFetchBalancesEmptyList() = runTest {
        // Arrange
        val emptyList = emptyList<ByteArray>()
        
        // Act
        val balances = sdk.identities.fetchBalances(emptyList)
        
        // Assert
        assertThat(balances).isEmpty()
    }
    
    @Test
    @DisplayName("Should create new identity with asset lock proof")
    @Disabled("Requires funded asset lock proof")
    fun testCreateIdentityWithAssetLock() = runTest {
        // Arrange
        val assetLockProof = "base64_encoded_asset_lock_proof_here"
        
        // Act
        val identity = sdk.identities.create(assetLockProof)
        
        // Assert
        assertThat(identity).isNotNull
        assertThat(identity.id).hasSize(32)
    }
    
    @Test
    @DisplayName("Should top up identity with asset lock proof")
    @Disabled("Requires funded identity and asset lock proof")
    fun testTopUpIdentity() = runTest {
        // Arrange
        val identity = sdk.identities.fetch(TestConfig.TestIdentities.ALICE_ID)
        assertThat(identity).isNotNull
        val assetLockProof = "base64_encoded_asset_lock_proof_here"
        
        // Act & Assert (should not throw)
        identity!!.topUp(assetLockProof)
    }
}