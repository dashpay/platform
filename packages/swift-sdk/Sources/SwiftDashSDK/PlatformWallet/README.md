# Platform Wallet API Documentation

## Overview

The Platform Wallet module provides Swift bindings for managing Dash Platform identities and DashPay contacts. It wraps the Rust FFI layer to provide a memory-safe, Swift-idiomatic API.

## Quick Start

```swift
import SwiftDashSDK

// Create a Platform Wallet from mnemonic
let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
let wallet = try PlatformWallet.fromMnemonic(mnemonic)

// Get identity manager for testnet
let identityManager = try wallet.getIdentityManager(for: .testnet)

// Check identity count
let count = try identityManager.getIdentityCount()
print("Total identities: \(count)")
```

## Core Components

### PlatformWallet

The main entry point for Platform Wallet functionality.

#### Initialization

**From Mnemonic:**
```swift
static func fromMnemonic(_ mnemonic: String, passphrase: String? = nil) throws -> PlatformWallet
```

Creates a Platform Wallet from a BIP39 mnemonic phrase with optional passphrase.

Example:
```swift
let wallet = try PlatformWallet.fromMnemonic("word1 word2 ... word12")
let walletWithPassphrase = try PlatformWallet.fromMnemonic(
    "word1 word2 ... word12",
    passphrase: "my-secret-passphrase"
)
```

**From Seed:**
```swift
static func fromSeed(_ seed: Data) throws -> PlatformWallet
```

Creates a Platform Wallet from a 64-byte seed.

Example:
```swift
let seed = Data(count: 64)  // Your seed bytes
let wallet = try PlatformWallet.fromSeed(seed)
```

#### Identity Manager Access

```swift
func getIdentityManager(for network: Network) throws -> IdentityManager
```

Gets or creates an identity manager for a specific network. Results are cached per network.

Example:
```swift
let mainnetManager = try wallet.getIdentityManager(for: .mainnet)
let testnetManager = try wallet.getIdentityManager(for: .testnet)
```

```swift
func setIdentityManager(_ manager: IdentityManager, for network: Network) throws
```

Sets a specific identity manager for a network.

---

### IdentityManager

Manages a collection of identities for a specific network.

#### Identity Management

**Create Manager:**
```swift
static func create() throws -> IdentityManager
```

**Add Identity:**
```swift
func addIdentity(_ identity: ManagedIdentity) throws
```

**Get Identity:**
```swift
func getIdentity(_ identityId: Identifier) throws -> ManagedIdentity
```

**Remove Identity:**
```swift
func removeIdentity(_ identityId: Identifier) throws
```

Example:
```swift
let manager = try IdentityManager.create()

// Add an identity
let identity = try ManagedIdentity.fromIdentityBytes(identityBytes)
try manager.addIdentity(identity)

// Get it back
let retrievedIdentity = try manager.getIdentity(identityId)

// Remove it
try manager.removeIdentity(identityId)
```

#### Query Operations

**Get All Identity IDs:**
```swift
func getAllIdentityIds() throws -> [Identifier]
```

**Get Identity Count:**
```swift
func getIdentityCount() throws -> Int
```

**Primary Identity:**
```swift
func getPrimaryIdentityId() throws -> Identifier?
func setPrimaryIdentity(_ identityId: Identifier) throws
```

Example:
```swift
// List all identities
let allIds = try manager.getAllIdentityIds()
print("Found \(allIds.count) identities")

// Set primary identity
try manager.setPrimaryIdentity(allIds[0])

// Get primary identity
if let primaryId = try manager.getPrimaryIdentityId() {
    let primaryIdentity = try manager.getIdentity(primaryId)
}
```

---

### ManagedIdentity

Represents a Platform identity with DashPay contact metadata.

#### Creation

```swift
static func fromIdentityBytes(_ bytes: Data) throws -> ManagedIdentity
```

Creates a ManagedIdentity from serialized DPP identity bytes.

#### Identity Information

```swift
func getId() throws -> Identifier
func getBalance() throws -> UInt64
func getLabel() throws -> String?
func setLabel(_ label: String) throws
```

Example:
```swift
let id = try identity.getId()
let balance = try identity.getBalance()
print("Identity \(id.hexString) has \(balance) credits")

try identity.setLabel("My Main Identity")
```

#### Block Time Tracking

```swift
func getLastUpdatedBalanceBlockTime() throws -> BlockTime?
func setLastUpdatedBalanceBlockTime(_ blockTime: BlockTime) throws
func getLastSyncedKeysBlockTime() throws -> BlockTime?
```

#### Contact Requests

**Send Contact Request:**
```swift
func sendContactRequest(
    recipientId: Identifier,
    senderKeyIndex: UInt32,
    recipientKeyIndex: UInt32,
    accountReference: UInt32,
    encryptedPublicKey: Data
) throws
```

Example:
```swift
let recipientId = try Identifier(hexString: "abcd...")
let encryptedKey = // ... ECDH encrypted public key

try identity.sendContactRequest(
    recipientId: recipientId,
    senderKeyIndex: 0,
    recipientKeyIndex: 0,
    accountReference: 0,
    encryptedPublicKey: encryptedKey
)
```

**Accept/Reject Requests:**
```swift
func acceptContactRequest(senderId: Identifier) throws
func rejectContactRequest(senderId: Identifier) throws
```

**Query Contact Requests:**
```swift
func getSentContactRequestIds() throws -> [Identifier]
func getIncomingContactRequestIds() throws -> [Identifier]
func getSentContactRequest(recipientId: Identifier) throws -> ContactRequest?
func getIncomingContactRequest(senderId: Identifier) throws -> ContactRequest?
```

Example:
```swift
// Get all incoming requests
let incomingIds = try identity.getIncomingContactRequestIds()
for senderId in incomingIds {
    if let request = try identity.getIncomingContactRequest(senderId: senderId) {
        let sender = try request.getSenderId()
        print("Request from \(sender.hexString)")

        // Accept or reject
        try identity.acceptContactRequest(senderId: senderId)
    }
}
```

#### Established Contacts

```swift
func getEstablishedContactIds() throws -> [Identifier]
func getEstablishedContact(contactId: Identifier) throws -> EstablishedContact?
func isContactEstablished(contactId: Identifier) throws -> Bool
```

Example:
```swift
// List all contacts
let contactIds = try identity.getEstablishedContactIds()

for contactId in contactIds {
    if let contact = try identity.getEstablishedContact(contactId: contactId) {
        let alias = try contact.getAlias()
        print("Contact: \(alias ?? contactId.hexString)")
    }
}
```

---

### ContactRequest

Represents a contact request between two identities.

#### Creation

```swift
static func create(
    senderId: Identifier,
    recipientId: Identifier,
    senderKeyIndex: UInt32,
    recipientKeyIndex: UInt32,
    accountReference: UInt32,
    encryptedPublicKey: Data,
    createdAt: UInt64
) throws -> ContactRequest
```

#### Properties

```swift
func getSenderId() throws -> Identifier
func getRecipientId() throws -> Identifier
func getSenderKeyIndex() throws -> UInt32
func getRecipientKeyIndex() throws -> UInt32
func getAccountReference() throws -> UInt32
func getEncryptedPublicKey() throws -> Data
func getCreatedAt() throws -> UInt64
```

Example:
```swift
let senderId = try request.getSenderId()
let recipientId = try request.getRecipientId()
let encryptedKey = try request.getEncryptedPublicKey()
let timestamp = try request.getCreatedAt()

print("Request from \(senderId.hexString) to \(recipientId.hexString)")
print("Created at: \(Date(timeIntervalSince1970: Double(timestamp) / 1000))")
```

---

### EstablishedContact

Represents a bidirectional friendship in DashPay.

#### Contact Information

```swift
func getContactIdentityId() throws -> Identifier
```

#### Alias Management

```swift
func getAlias() throws -> String?
func setAlias(_ alias: String) throws
func clearAlias() throws
```

Example:
```swift
// Set a friendly name
try contact.setAlias("Alice")

// Get the alias
if let alias = try contact.getAlias() {
    print("Contact name: \(alias)")
}

// Clear it
try contact.clearAlias()
```

#### Notes

```swift
func getNote() throws -> String?
func setNote(_ note: String) throws
func clearNote() throws
```

Example:
```swift
try contact.setNote("Met at conference 2024")
let note = try contact.getNote()
try contact.clearNote()
```

#### Visibility

```swift
func isHidden() throws -> Bool
func hide() throws
func unhide() throws
```

Example:
```swift
// Hide contact
try contact.hide()
print("Is hidden: \(try contact.isHidden())")

// Show contact again
try contact.unhide()
```

---

## Supporting Types

### Identifier

32-byte identifier for identities and documents.

```swift
struct Identifier {
    let bytes: [UInt8]
    var hexString: String

    init(bytes: [UInt8]) throws
    init(hexString: String) throws
    static func random() throws -> Identifier
}
```

Example:
```swift
// From hex string
let id = try Identifier(hexString: "abcd1234...")

// From bytes
let bytes: [UInt8] = [0x01, 0x02, ...]
let id2 = try Identifier(bytes: bytes)

// Generate random
let randomId = try Identifier.random()

// Convert to hex
print(randomId.hexString)
```

### BlockTime

Platform block information.

```swift
struct BlockTime {
    let height: UInt32
    let coreHeight: UInt32
    let timestamp: UInt64

    init(height: UInt32, coreHeight: UInt32, timestamp: UInt64)
}
```

### Network

Available network types.

```swift
enum Network: UInt32 {
    case mainnet = 0
    case testnet = 1
    case devnet = 2
    case local = 3
}
```

### PlatformWalletError

Error types thrown by Platform Wallet operations.

```swift
enum PlatformWalletError: Error {
    case nullPointer
    case invalidHandle
    case invalidParameter
    case invalidIdentifier
    case invalidNetwork
    case walletOperation(String)
    case identityNotFound
    case contactNotFound
    case utf8Conversion
    case serialization
    case deserialization
    case unknown(String)
}
```

---

## Usage Patterns

### Complete Contact Request Flow

```swift
// Alice sends request to Bob
let aliceIdentity = try ManagedIdentity.fromIdentityBytes(aliceBytes)
let bobId = try Identifier(hexString: "bob-id-hex")

try aliceIdentity.sendContactRequest(
    recipientId: bobId,
    senderKeyIndex: 0,
    recipientKeyIndex: 0,
    accountReference: 0,
    encryptedPublicKey: encryptedKey
)

// Bob receives and accepts
let bobIdentity = try ManagedIdentity.fromIdentityBytes(bobBytes)
let aliceId = try Identifier(hexString: "alice-id-hex")

// Check for request
if let request = try bobIdentity.getIncomingContactRequest(senderId: aliceId) {
    // Accept it
    try bobIdentity.acceptContactRequest(senderId: aliceId)

    // Now they're contacts!
    let isEstablished = try bobIdentity.isContactEstablished(contactId: aliceId)
    print("Contact established: \(isEstablished)")
}
```

### Managing Contact Metadata

```swift
let contacts = try identity.getEstablishedContactIds()

for contactId in contacts {
    if let contact = try identity.getEstablishedContact(contactId: contactId) {
        // Set alias and note
        try contact.setAlias("Alice Smith")
        try contact.setNote("Friend from university")

        // Later, hide temporarily
        try contact.hide()

        // Check visibility
        let isVisible = !(try contact.isHidden())
    }
}
```

### Multi-Network Identity Management

```swift
let wallet = try PlatformWallet.fromMnemonic(mnemonic)

// Separate managers for each network
let mainnetManager = try wallet.getIdentityManager(for: .mainnet)
let testnetManager = try wallet.getIdentityManager(for: .testnet)

// Add identities to appropriate networks
try testnetManager.addIdentity(testIdentity)
try mainnetManager.addIdentity(mainnetIdentity)

// Set primary identity per network
try testnetManager.setPrimaryIdentity(testIdentityId)
try mainnetManager.setPrimaryIdentity(mainnetIdentityId)
```

---

## Memory Management

All classes (PlatformWallet, IdentityManager, ManagedIdentity, ContactRequest, EstablishedContact) automatically manage their FFI handles through Swift's `deinit`. You don't need to manually free resources.

```swift
do {
    let wallet = try PlatformWallet.fromMnemonic(mnemonic)
    let manager = try wallet.getIdentityManager(for: .testnet)
    // Use manager...
} // wallet and manager are automatically freed here
```

---

## Thread Safety

Most operations are synchronous and not inherently thread-safe. Use appropriate synchronization when accessing from multiple threads:

```swift
actor PlatformWalletActor {
    let wallet: PlatformWallet

    init(mnemonic: String) throws {
        self.wallet = try PlatformWallet.fromMnemonic(mnemonic)
    }

    func getManager(for network: Network) throws -> IdentityManager {
        try wallet.getIdentityManager(for: network)
    }
}
```

---

## Error Handling

All throwing functions use Swift's error handling. Always wrap in `do-catch`:

```swift
do {
    let wallet = try PlatformWallet.fromMnemonic(mnemonic)
    let manager = try wallet.getIdentityManager(for: .testnet)
    let count = try manager.getIdentityCount()
} catch PlatformWalletError.invalidParameter {
    print("Invalid input")
} catch PlatformWalletError.identityNotFound {
    print("Identity not found")
} catch {
    print("Other error: \(error)")
}
```

---

## See Also

- [SwiftExampleApp Integration](../../../SwiftExampleApp/SwiftExampleApp/Services/DashPayService.swift) - Real-world usage example
- [Unit Tests](../../../SwiftTests/Tests/SwiftDashSDKTests/PlatformWalletTests.swift) - Comprehensive test examples
- [Integration Tests](../../../SwiftTests/Tests/SwiftDashSDKTests/PlatformWalletIntegrationTests.swift) - Full workflow examples
