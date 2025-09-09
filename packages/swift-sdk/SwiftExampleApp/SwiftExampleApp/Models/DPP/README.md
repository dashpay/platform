# DPP Models for Swift

This directory contains Swift implementations of the Dash Platform Protocol (DPP) models, providing type-safe representations of core platform data structures.

## Overview

These models are based on the official DPP specification and provide a foundation for building iOS applications that interact with Dash Platform.

## Core Types

### Basic Types
- `Identifier`: 32-byte unique identifier (Data)
- `Revision`: Version number for documents and identities (UInt64)
- `TimestampMillis`: Unix timestamp in milliseconds (UInt64)
- `Credits`: Platform credits amount (UInt64)
- `BlockHeight`: Platform chain block height (UInt64)
- `CoreBlockHeight`: Core chain block height (UInt32)

### Platform Value
- `PlatformValue`: Enum representing all possible value types in documents
  - Supports: null, bool, integer, float, string, bytes, array, map

## Identity Models

### DPPIdentity
The main identity structure containing:
- Unique identifier
- Public keys with purposes and security levels
- Credit balance
- Revision number

### IdentityPublicKey
Represents a public key with:
- **Purpose**: Authentication, Encryption, Transfer, Voting, etc.
- **Security Level**: Master, Critical, High, Medium
- **Key Type**: ECDSA, BLS12-381, etc.
- **Contract Bounds**: Optional restrictions to specific contracts

### Key Features
- Support for different identity types (User, Masternode, Evonode)
- Hierarchical security levels for keys
- Contract-specific key restrictions

## Document Models

### DPPDocument
Core document structure with:
- Unique identifier and owner
- Flexible properties using PlatformValue
- Timestamps for creation, updates, and transfers
- Block height tracking for both chains

### ExtendedDocument
Enhanced document that includes:
- Document type information
- Associated data contract
- Metadata and entropy
- Token payment information

### DocumentPatch
Partial document updates containing only changed fields

## Data Contract Models

### DPPDataContract
Complete contract definition including:
- Document type schemas
- Indices for efficient querying
- Token configurations
- Multi-party control groups
- Keywords and descriptions

### DocumentType
Defines the structure and rules for documents:
- JSON schema for validation
- Index definitions
- Security settings (insert/update/delete signatures)
- Transferability rules
- Token association

### TokenConfiguration
Comprehensive token settings:
- Basic info (name, symbol, decimals)
- Supply controls (mintable, burnable, capped)
- Trading features (transferable, tradeable, sellable)
- Security features (freezable, pausable, destructible)
- Rule-based permissions

## State Transitions

### Supported Transitions
- **Identity**: Create, Update, TopUp, CreditWithdrawal, CreditTransfer
- **DataContract**: Create, Update
- **Document**: Create, Replace, Delete, Transfer, Purchase
- **Token**: Transfer, Mint, Burn, Freeze, Unfreeze

### Common Properties
- Type identification
- Optional signatures with public key references
- Structured data for each operation

## Integration with Existing Models

The existing app models have been enhanced to support DPP:

### IdentityModel
- Added `dppIdentity` property for full DPP data
- Added `publicKeys` array for key management
- Conversion methods between simplified and DPP models

### DocumentModel
- Added `dppDocument` property
- Added `revision` tracking
- Automatic conversion from PlatformValue to simple types

### ContractModel
- Added `dppDataContract` property
- Added token configurations
- Added keywords and description support

## Usage Examples

```swift
// Create a DPP Identity
let identity = DPPIdentity.create(
    id: identifierData,
    publicKeys: [authKey, transferKey],
    balance: 1000000000
)

// Create a Document
let document = DPPDocument.create(
    ownerId: ownerIdentifier,
    properties: [
        "name": .string("Example"),
        "value": .integer(42)
    ]
)

// Convert between models
let identityModel = IdentityModel(from: dppIdentity)
let documentModel = DocumentModel(from: dppDocument, 
                                  contractId: "...", 
                                  documentType: "profile")
```

## Best Practices

1. **Use DPP models for platform interactions**: When communicating with Dash Platform, use the DPP models for accurate data representation.

2. **Use simplified models for UI**: The existing models (IdentityModel, DocumentModel, etc.) are better suited for UI binding and display.

3. **Handle conversions carefully**: When converting between PlatformValue and Swift native types, ensure proper type checking.

4. **Respect security levels**: Always check key purposes and security levels before performing operations.

5. **Track revisions**: Use revision numbers to handle concurrent updates properly.

## Future Enhancements

- Add validation methods for all models
- Implement serialization for network transport
- Add cryptographic signature verification
- Support for binary serialization formats
- Enhanced error handling for model conversions