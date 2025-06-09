package com.dash.sdk.modules

import com.dash.sdk.SDK
import com.dash.sdk.TestConfig
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import org.assertj.core.api.Assertions.assertThat
import org.junit.jupiter.api.*
import org.junit.jupiter.api.condition.DisabledIfEnvironmentVariable

/**
 * Integration tests for the Documents module
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
class DocumentsTest {
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
    @DisplayName("Should create a new document")
    @Disabled("Requires funded identity and valid data contract")
    fun testCreateDocument() = runTest {
        // Arrange
        val identity = sdk.identities.fetch(TestConfig.TestIdentities.ALICE_ID)
        assertThat(identity).isNotNull
        
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DPNS_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        val properties = buildJsonObject {
            put("label", "test-document")
            put("normalizedLabel", "test-document")
            put("normalizedParentDomainName", "dash")
            put("records", buildJsonObject {
                put("dashUniqueIdentityId", identity!!.idBase58)
            })
        }
        
        // Act
        val document = sdk.documents.create(
            dataContract = contract!!,
            documentType = TestConfig.TestDocumentTypes.DPNS_DOMAIN,
            properties = properties,
            owner = identity!!
        )
        
        // Assert
        assertThat(document).isNotNull
        assertThat(document.type).isEqualTo(TestConfig.TestDocumentTypes.DPNS_DOMAIN)
        assertThat(document.ownerId).isEqualTo(identity.id)
        assertThat(document.properties).isEqualTo(properties)
    }
    
    @Test
    @DisplayName("Should search documents with empty query")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testSearchDocumentsEmptyQuery() = runTest {
        // Arrange
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DPNS_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        // Act
        val documents = sdk.documents.search(
            dataContract = contract!!,
            documentType = TestConfig.TestDocumentTypes.DPNS_DOMAIN,
            limit = 10
        )
        
        // Assert
        assertThat(documents).isNotNull
        // Note: Results depend on actual data in the network
    }
    
    @Test
    @DisplayName("Should search documents with where clause")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testSearchDocumentsWithQuery() = runTest {
        // Arrange
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DPNS_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        val query = Documents.QueryBuilder()
            .where("normalizedParentDomainName", "dash")
            .build()
        
        // Act
        val documents = sdk.documents.search(
            dataContract = contract!!,
            documentType = TestConfig.TestDocumentTypes.DPNS_DOMAIN,
            query = query,
            limit = 5
        )
        
        // Assert
        assertThat(documents).isNotNull
    }
    
    @Test
    @DisplayName("Should search documents with complex query")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testSearchDocumentsComplexQuery() = runTest {
        // Arrange
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DASHPAY_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        val query = Documents.QueryBuilder()
            .whereGreaterThan("revision", 0)
            .whereLessThan("revision", 100)
            .build()
        
        // Act
        val documents = sdk.documents.search(
            dataContract = contract!!,
            documentType = TestConfig.TestDocumentTypes.DASHPAY_PROFILE,
            query = query,
            limit = 3
        )
        
        // Assert
        assertThat(documents).isNotNull
    }
    
    @Test
    @DisplayName("Should update document properties")
    @Disabled("Requires existing document ownership")
    fun testUpdateDocument() = runTest {
        // Arrange
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DPNS_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        val documents = sdk.documents.search(
            dataContract = contract!!,
            documentType = TestConfig.TestDocumentTypes.DPNS_DOMAIN,
            limit = 1
        )
        assertThat(documents).isNotEmpty
        
        val document = documents.first()
        val updates = buildJsonObject {
            put("label", "updated-label")
        }
        
        // Act
        sdk.documents.update(document, updates)
        
        // Assert
        assertThat(document.properties["label"]?.toString()).isEqualTo("\"updated-label\"")
        assertThat(document.revision).isGreaterThan(1)
    }
    
    @Test
    @DisplayName("Should transfer document ownership")
    @Disabled("Requires document ownership and recipient identity")
    fun testTransferDocument() = runTest {
        // Arrange
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DPNS_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        val documents = sdk.documents.search(
            dataContract = contract!!,
            documentType = TestConfig.TestDocumentTypes.DPNS_DOMAIN,
            limit = 1
        )
        assertThat(documents).isNotEmpty
        
        val document = documents.first()
        val recipientId = TestConfig.TestIdentities.BOB_ID
        
        // Act
        sdk.documents.transfer(document, recipientId)
        
        // Assert
        assertThat(document.ownerId).isEqualTo(recipientId)
    }
    
    @Test
    @DisplayName("Should delete document")
    @Disabled("Requires document ownership")
    fun testDeleteDocument() = runTest {
        // Arrange
        val identity = sdk.identities.fetch(TestConfig.TestIdentities.ALICE_ID)
        assertThat(identity).isNotNull
        
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DPNS_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        // First create a document to delete
        val properties = buildJsonObject {
            put("label", "test-to-delete")
            put("normalizedLabel", "test-to-delete")
            put("normalizedParentDomainName", "dash")
        }
        
        val document = sdk.documents.create(
            dataContract = contract!!,
            documentType = TestConfig.TestDocumentTypes.DPNS_DOMAIN,
            properties = properties,
            owner = identity!!
        )
        
        // Act & Assert (should not throw)
        sdk.documents.delete(document)
    }
    
    @Test
    @DisplayName("Should build query with 'in' operator")
    fun testQueryBuilderWithInOperator() {
        // Arrange & Act
        val query = Documents.QueryBuilder()
            .whereIn("status", listOf("active", "pending", "approved"))
            .where("type", "profile")
            .build()
        
        // Assert
        assertThat(query).isNotNull
        assertThat(query["status"]).isNotNull
        assertThat(query["type"]?.toString()).contains("profile")
    }
    
    @Test
    @DisplayName("Should search Dashpay contact requests")
    @DisabledIfEnvironmentVariable(named = "SKIP_NETWORK_TESTS", matches = "true")
    fun testSearchDashpayContactRequests() = runTest {
        // Arrange
        val contract = sdk.contracts.fetch(TestConfig.TestContracts.DASHPAY_CONTRACT_ID)
        assertThat(contract).isNotNull
        
        // Act
        val documents = sdk.documents.search(
            dataContract = contract!!,
            documentType = TestConfig.TestDocumentTypes.DASHPAY_CONTACT_REQUEST,
            limit = 5
        )
        
        // Assert
        assertThat(documents).isNotNull
        documents.forEach { doc ->
            assertThat(doc.type).isEqualTo(TestConfig.TestDocumentTypes.DASHPAY_CONTACT_REQUEST)
        }
    }
}