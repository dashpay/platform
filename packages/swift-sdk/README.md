# Swift SDK for Dash Platform

This Swift SDK provides iOS-friendly bindings for the Dash Platform, wrapping the `rs-sdk-ffi` crate with idiomatic Swift interfaces.

See also: iOS Simulator MCP usage and Codex config in [IOS_SIMULATOR_MCP.md](./IOS_SIMULATOR_MCP.md).

## Features

- **Identity Management**: Create, fetch, and manage Dash Platform identities
- **Data Contracts**: Define and deploy structured data schemas
- **Document Operations**: Create, fetch, and update documents
- **Credit Transfers**: Transfer credits between identities
- **Put to Platform**: Multiple options for state transitions (instant lock, chain lock, with/without wait)

## Installation

### Requirements

- iOS 13.0+
- Xcode 12.0+
- Swift 5.3+

### Building

1. Build the Rust library:
```bash
cd packages/swift-sdk
cargo build --release
```

2. The build will generate a static library that can be linked with your iOS project.

### Integration

1. Add the generated library to your Xcode project
2. Import the Swift module:
```swift
import SwiftDashSDK
```

## API Reference

### Identity Operations
- `swift_dash_identity_fetch` - Fetch an identity by ID
- `swift_dash_identity_get_info` - Get identity information
- `swift_dash_identity_put_to_platform_with_instant_lock` - Put identity with instant lock
- `swift_dash_identity_put_to_platform_with_instant_lock_and_wait` - Put and wait for confirmation
- `swift_dash_identity_put_to_platform_with_chain_lock` - Put identity with chain lock  
- `swift_dash_identity_put_to_platform_with_chain_lock_and_wait` - Put and wait for confirmation
- `swift_dash_identity_transfer_credits` - Transfer credits between identities

### Data Contract Operations
- `swift_dash_data_contract_fetch` - Fetch a data contract by ID
- `swift_dash_data_contract_create` - Create a new data contract
- `swift_dash_data_contract_get_info` - Get contract information as JSON
- `swift_dash_data_contract_get_schema` - Get schema for a document type
- `swift_dash_data_contract_put_to_platform` - Put contract to platform
- `swift_dash_data_contract_put_to_platform_and_wait` - Put and wait for confirmation

### Document Operations
- `swift_dash_document_create` - Create a new document
- `swift_dash_document_fetch` - Fetch a document by ID
- `swift_dash_document_get_info` - Get document information
- `swift_dash_document_put_to_platform` - Put document to platform
- `swift_dash_document_put_to_platform_and_wait` - Put and wait for confirmation
- `swift_dash_document_purchase_to_platform` - Purchase document from platform
- `swift_dash_document_purchase_to_platform_and_wait` - Purchase and wait for confirmation

### SDK Management
- `swift_dash_sdk_init` - Initialize the SDK library
- `swift_dash_sdk_create` - Create an SDK instance
- `swift_dash_sdk_destroy` - Destroy an SDK instance
- `swift_dash_sdk_get_network` - Get the configured network
- `swift_dash_sdk_get_version` - Get SDK version

### Signer Operations
- `swift_dash_signer_create_test` - Create a test signer for development
- `swift_dash_signer_destroy` - Destroy a signer instance

## Usage

### SDK Initialization

```swift
// Initialize the SDK
swift_dash_sdk_init()

// Create SDK configuration
let config = swift_dash_sdk_config_testnet()  // or mainnet/local

// Create SDK instance
let sdk = swift_dash_sdk_create(config)

// Create a test signer (for development)
let signer = swift_dash_signer_create_test()

// Clean up when done
defer {
    swift_dash_signer_destroy(signer)
    swift_dash_sdk_destroy(sdk)
}
```

### Identity Operations

#### Fetch an Identity

```swift
let identityId = "your_identity_id_here"
if let identity = swift_dash_identity_fetch(sdk, identityId) {
    // Get identity information
    if let info = swift_dash_identity_get_info(identity) {
        print("Balance: \(info.pointee.balance)")
        print("Revision: \(info.pointee.revision)")
        
        // Clean up
        swift_dash_identity_info_free(info)
    }
}
```

#### Put Identity to Platform

```swift
var settings = swift_dash_put_settings_default()
settings.timeout_ms = 60000

// Put with instant lock
if let result = swift_dash_identity_put_to_platform_with_instant_lock(
    sdk, identity, publicKeyId, signer, &settings
) {
    // Process result
    let data = Data(bytes: result.pointee.data, count: result.pointee.len)
    
    // Clean up
    swift_dash_binary_data_free(result)
}

// Put with instant lock and wait for confirmation
if let confirmedIdentity = swift_dash_identity_put_to_platform_with_instant_lock_and_wait(
    sdk, identity, publicKeyId, signer, &settings
) {
    // Identity is confirmed on platform
}
```

#### Transfer Credits

```swift
let recipientId = "recipient_identity_id"
let amount: UInt64 = 50000

if let result = swift_dash_identity_transfer_credits(
    sdk, identity, recipientId, amount, publicKeyId, signer, &settings
) {
    print("Transferred: \(result.pointee.amount) credits")
    print("To: \(String(cString: result.pointee.recipient_id))")
    
    // Clean up
    swift_dash_transfer_credits_result_free(result)
}
```

### Data Contract Operations

#### Create a Data Contract

```swift
let ownerId = "identity_that_owns_contract"
let schema = """
{
    "$format_version": "0",
    "ownerId": "\(ownerId)",
    "documents": {
        "message": {
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "maxLength": 280
                },
                "timestamp": {
                    "type": "integer"
                }
            },
            "required": ["content", "timestamp"],
            "additionalProperties": false
        }
    }
}
"""

if let contract = swift_dash_data_contract_create(sdk, ownerId, schema) {
    // Put contract to platform
    if let result = swift_dash_data_contract_put_to_platform(
        sdk, contract, publicKeyId, signer, &settings
    ) {
        // Contract deployed
        swift_dash_binary_data_free(result)
    }
}
```

#### Fetch a Data Contract

```swift
let contractId = "contract_id_here"
if let contract = swift_dash_data_contract_fetch(sdk, contractId) {
    // Get contract information
    if let info = swift_dash_data_contract_get_info(contract) {
        let infoString = String(cString: info)
        print("Contract info: \(infoString)")
        free(info)
    }
}
```

### Document Operations

#### Create a Document

```swift
let documentData = """
{
    "content": "Hello, Dash Platform!",
    "timestamp": \(Date().timeIntervalSince1970 * 1000),
    "author": "dashuser"
}
"""

if let document = swift_dash_document_create(
    sdk, contract, ownerId, "message", documentData
) {
    // Put document to platform
    if let result = swift_dash_document_put_to_platform(
        sdk, document, publicKeyId, signer, &settings
    ) {
        // Document created on platform
        swift_dash_binary_data_free(result)
    }
}
```

#### Fetch a Document

```swift
let documentType = "message"
let documentId = "document_id_here"

if let document = swift_dash_document_fetch(
    sdk, contract, documentType, documentId
) {
    // Get document information
    if let info = swift_dash_document_get_info(document) {
        print("Document ID: \(String(cString: info.pointee.id))")
        print("Owner: \(String(cString: info.pointee.owner_id))")
        print("Type: \(String(cString: info.pointee.document_type))")
        print("Revision: \(info.pointee.revision)")
        
        swift_dash_document_info_free(info)
    }
}
```

## Put Settings

Configure how state transitions are submitted:

```swift
var settings = swift_dash_put_settings_default()

// Timeouts
settings.connect_timeout_ms = 30000      // Connection timeout
settings.timeout_ms = 60000              // Request timeout
settings.wait_timeout_ms = 120000        // Wait for confirmation timeout

// Retry behavior
settings.retries = 3                     // Number of retries
settings.ban_failed_address = true       // Ban addresses that fail

// Fee management
settings.user_fee_increase = 10          // Increase fee by 10%

// Security
settings.allow_signing_with_any_security_level = false
settings.allow_signing_with_any_purpose = false
```

## Memory Management

The SDK uses manual memory management. Always free allocated resources:

```swift
// Free binary data
swift_dash_binary_data_free(binaryData)

// Free info structures
swift_dash_identity_info_free(identityInfo)
swift_dash_document_info_free(documentInfo)
swift_dash_transfer_credits_result_free(transferResult)

// Free strings
free(cString)

// Destroy handles
swift_dash_sdk_destroy(sdk)
swift_dash_signer_destroy(signer)
```

## Error Handling

All functions that can fail return optional values. Always check for nil:

```swift
guard let sdk = swift_dash_sdk_create(config) else {
    print("Failed to create SDK")
    return
}

guard let identity = swift_dash_identity_fetch(sdk, identityId) else {
    print("Failed to fetch identity")
    return
}
```

## Testing

The Swift SDK uses compilation verification and Swift integration testing:

```bash
# Verify compilation
cargo build -p swift-sdk

# Run unit tests
cargo test -p swift-sdk --lib

# Check symbol exports
nm -g target/debug/libswift_sdk.a | grep swift_dash_
```

For comprehensive testing, integrate the compiled library into an iOS project with XCTest suites.

## Example App

Here's a complete example:

```swift
import SwiftDashSDK

class DashPlatformService {
    private var sdk: OpaquePointer?
    private var signer: OpaquePointer?
    
    init() {
        swift_dash_sdk_init()
        
        let config = swift_dash_sdk_config_testnet()
        sdk = swift_dash_sdk_create(config)
        signer = swift_dash_signer_create_test()
    }
    
    deinit {
        if let signer = signer {
            swift_dash_signer_destroy(signer)
        }
        if let sdk = sdk {
            swift_dash_sdk_destroy(sdk)
        }
    }
    
    func createMessage(content: String, authorId: String) async throws {
        guard let sdk = sdk, let signer = signer else {
            throw DashError.notInitialized
        }
        
        // Fetch contract
        let contractId = "your_contract_id"
        guard let contract = swift_dash_data_contract_fetch(sdk, contractId) else {
            throw DashError.contractNotFound
        }
        
        // Create document
        let timestamp = Int(Date().timeIntervalSince1970 * 1000)
        let documentData = """
        {
            "content": "\(content)",
            "timestamp": \(timestamp),
            "author": "\(authorId)"
        }
        """
        
        guard let document = swift_dash_document_create(
            sdk, contract, authorId, "message", documentData
        ) else {
            throw DashError.documentCreationFailed
        }
        
        // Put to platform
        var settings = swift_dash_put_settings_default()
        settings.timeout_ms = 60000
        
        guard let result = swift_dash_document_put_to_platform(
            sdk, document, 0, signer, &settings
        ) else {
            throw DashError.platformSubmissionFailed
        }
        
        defer { swift_dash_binary_data_free(result) }
        
        // Success!
        print("Message created successfully")
    }
}

enum DashError: Error {
    case notInitialized
    case contractNotFound
    case documentCreationFailed
    case platformSubmissionFailed
}
```

## Building the Library

To build the library:

```bash
cargo build --release -p swift-sdk
```

This will generate both static and dynamic libraries that can be linked with iOS applications.

## Integration with iOS Projects

1. Build the library using the command above
2. Include the generated header file in your Xcode project
3. Link against the generated library
4. Use the C functions directly from Swift

## Thread Safety

The underlying FFI is thread-safe, but individual handles should not be shared across threads without proper synchronization.

## License

This SDK follows the same license as the Dash Platform project.
