package com.dash.sdk.modules

import com.dash.sdk.SDK
import com.dash.sdk.TestConfig
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonObject
import org.assertj.core.api.Assertions.assertThat
import org.assertj.core.api.Assertions.assertThatThrownBy
import org.junit.jupiter.api.*
import org.junit.jupiter.api.condition.DisabledIfEnvironmentVariable

/**
 * Integration tests for the Contracts module
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
class ContractsTest {
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
    @DisplayName("Should fetch existing data contract by ID")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testFetchExistingContract() = runTest {
        // Arrange
        val contractId = TestConfig.TestContracts.DPNS_CONTRACT_ID
        
        // Act
        val contract = sdk.contracts.fetch(contractId)
        
        // Assert
        assertThat(contract).isNotNull
        contract?.let {
            assertThat(it.id).isEqualTo(contractId)
            assertThat(it.idBase58).isNotEmpty()
        }
    }
    
    @Test
    @DisplayName("Should return null when fetching non-existent contract")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testFetchNonExistentContract() = runTest {
        // Arrange
        val nonExistentId = TestConfig.TestContracts.NON_EXISTENT_CONTRACT_ID
        
        // Act
        val contract = sdk.contracts.fetch(nonExistentId)
        
        // Assert
        assertThat(contract).isNull()
    }
    
    @Test
    @DisplayName("Should fetch contract by base58 ID")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testFetchByBase58() = runTest {
        // Arrange
        val base58Id = "36ez8VqoDbR8NkdXwFaf9Tp8ukBdQJLLRqbLhNbvVhXU" // DPNS contract
        
        // Act
        val contract = sdk.contracts.fetchByBase58(base58Id)
        
        // Assert
        assertThat(contract).isNotNull
        contract?.let {
            assertThat(it.idBase58).isEqualTo(base58Id)
        }
    }
    
    @Test
    @DisplayName("Should validate contract ID size")
    fun testInvalidContractIdSize() = runTest {
        // Arrange
        val invalidId = ByteArray(16) // Should be 32 bytes
        
        // Act & Assert
        assertThatThrownBy {
            kotlinx.coroutines.runBlocking {
                sdk.contracts.fetch(invalidId)
            }
        }.isInstanceOf(IllegalArgumentException::class.java)
            .hasMessageContaining("Contract ID must be 32 bytes")
    }
    
    @Test
    @DisplayName("Should create data contract from JSON object")
    @Disabled("Requires funded identity and proper contract definition")
    fun testCreateContractFromJsonObject() = runTest {
        // Arrange
        val identity = sdk.identities.fetch(TestConfig.TestIdentities.ALICE_ID)
        assertThat(identity).isNotNull
        
        val contractDefinition = buildJsonObject {
            put("protocolVersion", 1)
            put("${"$"}schema", "https://schema.dash.org/dpp-0-4-0/meta/data-contract")
            putJsonObject("documents") {
                putJsonObject("note") {
                    put("type", "object")
                    putJsonObject("properties") {
                        putJsonObject("message") {
                            put("type", "string")
                            put("maxLength", 256)
                        }
                        putJsonObject("author") {
                            put("type", "string")
                            put("maxLength", 64)
                        }
                    }
                    put("required", kotlinx.serialization.json.buildJsonArray {
                        add("message")
                        add("author")
                    })
                    put("additionalProperties", false)
                }
            }
        }
        
        // Act
        val contract = sdk.contracts.create(contractDefinition, identity!!)
        
        // Assert
        assertThat(contract).isNotNull
        assertThat(contract.id).hasSize(32)
        assertThat(contract.definition).isEqualTo(contractDefinition)
    }
    
    @Test
    @DisplayName("Should create data contract from JSON string")
    @Disabled("Requires funded identity")
    fun testCreateContractFromJsonString() = runTest {
        // Arrange
        val identity = sdk.identities.fetch(TestConfig.TestIdentities.ALICE_ID)
        assertThat(identity).isNotNull
        
        val contractJson = """
        {
            "protocolVersion": 1,
            "${"$"}schema": "https://schema.dash.org/dpp-0-4-0/meta/data-contract",
            "documents": {
                "note": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "maxLength": 256
                        },
                        "author": {
                            "type": "string",
                            "maxLength": 64
                        }
                    },
                    "required": ["message", "author"],
                    "additionalProperties": false
                }
            }
        }
        """.trimIndent()
        
        // Act
        val contract = sdk.contracts.createFromJson(contractJson, identity!!)
        
        // Assert
        assertThat(contract).isNotNull
        assertThat(contract.id).hasSize(32)
    }
    
    @Test
    @DisplayName("Should update existing contract")
    @Disabled("Requires funded identity and existing contract ownership")
    fun testUpdateContract() = runTest {
        // Arrange
        val identity = sdk.identities.fetch(TestConfig.TestIdentities.ALICE_ID)
        assertThat(identity).isNotNull
        
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DPNS_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        val updates = buildJsonObject {
            putJsonObject("documents") {
                putJsonObject("note") {
                    putJsonObject("properties") {
                        putJsonObject("timestamp") {
                            put("type", "integer")
                            put("minimum", 0)
                        }
                    }
                }
            }
        }
        
        // Act
        val updatedContract = sdk.contracts.update(contract!!, updates, identity!!)
        
        // Assert
        assertThat(updatedContract).isNotNull
        assertThat(updatedContract.id).isEqualTo(contract.id)
    }
    
    @Test
    @DisplayName("Should fetch Dashpay contract")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testFetchDashpayContract() = runTest {
        // Arrange
        val contractId = TestConfig.TestContracts.DASHPAY_CONTRACT_ID
        
        // Act
        val contract = sdk.contracts.fetch(contractId)
        
        // Assert
        assertThat(contract).isNotNull
        contract?.let {
            assertThat(it.id).isEqualTo(contractId)
            // Verify it has expected document types
            val documents = it.definition["documents"]
            assertThat(documents).isNotNull
        }
    }
}