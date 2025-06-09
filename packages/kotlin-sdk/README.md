# Dash Platform Kotlin SDK

A Kotlin SDK for interacting with the Dash Platform, built on top of the Rust SDK FFI bindings.

## Features

- **Identity Management**: Create, fetch, and manage Dash Platform identities
- **Data Contracts**: Deploy and interact with data contracts
- **Documents**: Create, update, query, and delete documents
- **Tokens**: Mint, transfer, burn, and manage fungible tokens
- **Type-safe API**: Leverages Kotlin's type system for safer interactions
- **Coroutine Support**: All operations are suspend functions for async execution

## Installation

### Gradle (Kotlin DSL)

```kotlin
dependencies {
    implementation("com.dash:kotlin-sdk:1.0.0")
}
```

### Gradle (Groovy)

```groovy
dependencies {
    implementation 'com.dash:kotlin-sdk:1.0.0'
}
```

### Maven

```xml
<dependency>
    <groupId>com.dash</groupId>
    <artifactId>kotlin-sdk</artifactId>
    <version>1.0.0</version>
</dependency>
```

## Quick Start

```kotlin
import com.dash.sdk.SDK
import com.dash.sdk.types.Network
import com.dash.sdk.types.SDKConfig
import kotlinx.coroutines.runBlocking

fun main() = runBlocking {
    // Initialize SDK
    val config = SDKConfig(
        network = Network.TESTNET,
        skipAssetLockProofVerification = true
    )
    
    val sdk = SDK(config)
    
    // Fetch an identity
    val identity = sdk.identities.fetchByBase58("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF")
    println("Identity balance: ${identity?.getBalance()} credits")
    
    // Don't forget to close the SDK
    sdk.close()
}
```

## Usage Examples

### Identity Operations

```kotlin
// Fetch identity by different ID formats
val identityByBase58 = sdk.identities.fetchByBase58("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF")
val identityByHex = sdk.identities.fetchByHex("abcd1234...")
val identityByBytes = sdk.identities.fetch(byteArrayOf(...))

// Get identity balance
val balance = identity.getBalance()

// Create new identity (requires asset lock proof)
val newIdentity = sdk.identities.create(assetLockProofBase64)

// Top up identity
identity.topUp(assetLockProofBase64)
```

### Data Contract Operations

```kotlin
// Fetch existing contract
val contract = sdk.contracts.fetchByBase58("36ez8VqoDbR8NkdXwFaf9Tp8ukBdQJLLRqbLhNbvVhXU")

// Create new contract
val contractDefinition = buildJsonObject {
    put("protocolVersion", 1)
    putJsonObject("documents") {
        putJsonObject("note") {
            put("type", "object")
            putJsonObject("properties") {
                putJsonObject("message") {
                    put("type", "string")
                    put("maxLength", 256)
                }
            }
        }
    }
}

val newContract = sdk.contracts.create(contractDefinition, ownerIdentity)
```

### Document Operations

```kotlin
// Create document
val document = sdk.documents.create(
    dataContract = contract,
    documentType = "note",
    properties = buildJsonObject {
        put("message", "Hello, Dash Platform!")
        put("author", "Alice")
    },
    owner = identity
)

// Query documents
val query = Documents.QueryBuilder()
    .where("author", "Alice")
    .whereGreaterThan("timestamp", 1640995200000)
    .build()

val documents = sdk.documents.search(
    dataContract = contract,
    documentType = "note",
    query = query,
    limit = 10
)

// Update document
sdk.documents.update(document, buildJsonObject {
    put("message", "Updated message")
})

// Transfer ownership
sdk.documents.transfer(document, newOwnerIdentityId)

// Delete document
sdk.documents.delete(document)
```

### Token Operations

```kotlin
// Get token balance
val balance = sdk.tokens.getBalance(contractId, tokenPosition, identityId)

// Mint tokens (requires minting permissions)
sdk.tokens.mint(
    contract = contract,
    tokenPosition = 0,
    amount = 1000,
    recipientId = recipientIdentityId,
    issuer = issuerIdentity
)

// Transfer tokens
sdk.tokens.transfer(
    contract = contract,
    tokenPosition = 0,
    amount = 100,
    sender = senderIdentity,
    recipientId = recipientIdentityId
)

// Burn tokens
sdk.tokens.burn(
    contract = contract,
    tokenPosition = 0,
    amount = 50,
    owner = ownerIdentity
)
```

## Advanced Configuration

```kotlin
val config = SDKConfig(
    network = Network.LOCAL,
    skipAssetLockProofVerification = false,
    requestRetryCount = 5,
    requestTimeoutMs = 60000,
    coreIpAddress = "127.0.0.1",
    platformPort = 3000,
    dumpLookupSessionsOnDrop = true
)
```

## Error Handling

The SDK throws specific exceptions for different error scenarios:

```kotlin
try {
    val identity = sdk.identities.fetch(identityId)
} catch (e: DashSDKException) {
    when (e) {
        is DashSDKException.NetworkException -> println("Network error: ${e.message}")
        is DashSDKException.NotFoundException -> println("Identity not found")
        is DashSDKException.InvalidParameterException -> println("Invalid parameter: ${e.message}")
        else -> println("Unexpected error: ${e.message}")
    }
}
```

## Testing

The SDK includes comprehensive test suites. To run tests:

```bash
./gradlew test
```

To skip network tests (for offline testing):

```bash
SKIP_NETWORK_TESTS=true ./gradlew test
```

## Building from Source

1. Clone the repository
2. Build the Rust FFI library:
   ```bash
   cd packages/rs-sdk-ffi
   cargo build --release
   ```
3. Build the Kotlin SDK:
   ```bash
   cd packages/kotlin-sdk
   ./gradlew build
   ```

## Native Library Requirements

The SDK requires the `dash_sdk_ffi` native library. The library is automatically loaded from:
- Linux: `libdash_sdk_ffi.so`
- macOS: `libdash_sdk_ffi.dylib`
- Windows: `dash_sdk_ffi.dll`

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License.