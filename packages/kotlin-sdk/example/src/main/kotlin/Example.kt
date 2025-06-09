import com.dash.sdk.SDK
import com.dash.sdk.modules.Documents
import com.dash.sdk.types.Network
import com.dash.sdk.types.SDKConfig
import com.dash.sdk.utils.toBase58
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/**
 * Example application demonstrating the Dash Platform Kotlin SDK
 */
fun main() = runBlocking {
    println("=== Dash Platform Kotlin SDK Example ===\n")
    
    // 1. Initialize SDK
    println("1. Initializing SDK...")
    val config = SDKConfig(
        network = Network.TESTNET,
        skipAssetLockProofVerification = true
    )
    
    val sdk = SDK(config)
    println("   ✓ SDK initialized with network: ${config.network}")
    
    try {
        // 2. Fetch an identity
        println("\n2. Fetching identity...")
        val identityIdBase58 = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"
        val identity = sdk.identities.fetchByBase58(identityIdBase58)
        
        if (identity != null) {
            println("   ✓ Identity found: ${identity.idBase58}")
            
            // Get balance
            val balance = identity.getBalance()
            println("   ✓ Balance: $balance credits")
        } else {
            println("   ✗ Identity not found")
        }
        
        // 3. Fetch a data contract
        println("\n3. Fetching DPNS data contract...")
        val dpnsContractIdBase58 = "36ez8VqoDbR8NkdXwFaf9Tp8ukBdQJLLRqbLhNbvVhXU"
        val dpnsContract = sdk.contracts.fetchByBase58(dpnsContractIdBase58)
        
        if (dpnsContract != null) {
            println("   ✓ DPNS contract found: ${dpnsContract.idBase58}")
        } else {
            println("   ✗ DPNS contract not found")
        }
        
        // 4. Search for DPNS domains
        println("\n4. Searching for DPNS domains...")
        if (dpnsContract != null) {
            val query = Documents.QueryBuilder()
                .where("normalizedParentDomainName", "dash")
                .build()
                
            val domains = sdk.documents.search(
                dataContract = dpnsContract,
                documentType = "domain",
                query = query,
                limit = 5
            )
            
            println("   ✓ Found ${domains.size} domains")
            domains.forEachIndexed { index, domain ->
                val label = domain.properties["label"]?.toString()?.trim('"') ?: "unknown"
                println("   ${index + 1}. $label.dash")
            }
        }
        
        // 5. Fetch Dashpay profiles
        println("\n5. Fetching Dashpay profiles...")
        val dashpayContractIdBase58 = "Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7"
        val dashpayContract = sdk.contracts.fetchByBase58(dashpayContractIdBase58)
        
        if (dashpayContract != null) {
            println("   ✓ Dashpay contract found")
            
            val profiles = sdk.documents.search(
                dataContract = dashpayContract,
                documentType = "profile",
                limit = 3
            )
            
            println("   ✓ Found ${profiles.size} profiles")
            profiles.forEachIndexed { index, profile ->
                val displayName = profile.properties["displayName"]?.toString()?.trim('"') ?: "Anonymous"
                println("   ${index + 1}. $displayName")
            }
        } else {
            println("   ✗ Dashpay contract not found")
        }
        
        // 6. Example: Creating a document (disabled - requires funded identity)
        println("\n6. Document creation example (code only):")
        println("""
        // Create a new DPNS domain
        val newDomain = sdk.documents.create(
            dataContract = dpnsContract,
            documentType = "domain",
            properties = buildJsonObject {
                put("label", "myname")
                put("normalizedLabel", "myname")
                put("normalizedParentDomainName", "dash")
                put("records", buildJsonObject {
                    put("dashUniqueIdentityId", identity.idBase58)
                })
            },
            owner = identity
        )
        """.trimIndent())
        
        // 7. Example: Token operations (disabled - requires proper setup)
        println("\n7. Token operations example (code only):")
        println("""
        // Get token balance
        val tokenBalance = sdk.tokens.getBalance(
            contractId = someContract.id,
            tokenPosition = 0,
            identityId = identity.id
        )
        
        // Transfer tokens
        sdk.tokens.transfer(
            contract = someContract,
            tokenPosition = 0,
            amount = 100,
            sender = senderIdentity,
            recipientId = recipientIdentity.id
        )
        """.trimIndent())
        
        // 8. SDK utilities
        println("\n8. SDK Utilities:")
        println("   SDK Version: ${SDK.getVersion()}")
        println("   Current Time (ms): ${SDK.getCurrentTimeMs()}")
        
    } catch (e: Exception) {
        println("\nError: ${e.message}")
        e.printStackTrace()
    } finally {
        // Clean up
        sdk.close()
        println("\n✓ SDK closed")
    }
}

/**
 * Additional example: Advanced document queries
 */
suspend fun advancedDocumentQueries(sdk: SDK) {
    println("\n=== Advanced Document Queries ===")
    
    // Complex query with multiple conditions
    val complexQuery = Documents.QueryBuilder()
        .where("status", "active")
        .whereGreaterThan("createdAt", 1640995200000) // Jan 1, 2022
        .whereLessThan("updatedAt", System.currentTimeMillis())
        .whereIn("type", listOf("profile", "contact"))
        .build()
    
    println("Query: $complexQuery")
}

/**
 * Example: Error handling
 */
suspend fun errorHandlingExample(sdk: SDK) {
    println("\n=== Error Handling Example ===")
    
    try {
        // Try to fetch non-existent identity
        val nonExistent = sdk.identities.fetchByBase58("1111111111111111111111111111111111111111111")
        if (nonExistent == null) {
            println("Identity not found (expected)")
        }
    } catch (e: com.dash.sdk.types.DashSDKException) {
        when (e) {
            is com.dash.sdk.types.DashSDKException.NetworkException -> 
                println("Network error: ${e.message}")
            is com.dash.sdk.types.DashSDKException.NotFoundException -> 
                println("Not found: ${e.message}")
            else -> 
                println("SDK error: ${e.message}")
        }
    }
}