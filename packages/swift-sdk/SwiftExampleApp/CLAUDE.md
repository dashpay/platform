# SwiftExampleApp - AI Assistant Guide

This document provides guidance for AI assistants working with the SwiftExampleApp codebase.

## Overview

SwiftExampleApp is an iOS application demonstrating the integration of both Core (SPV wallet) and Platform (identity/documents) functionality of the Dash SDK.

## Key Architecture Patterns

### Unified SDK Integration
- Core SDK functions: `dash_core_sdk_*` prefix
- Platform SDK functions: `dash_sdk_*` prefix  
- Unified SDK functions: `dash_unified_sdk_*` prefix

### Data Persistence with SwiftData
The app uses SwiftData for local persistence with the following key models:
- `PersistentIdentity` - Stores identity information
- `PersistentDocument` - Stores documents
- `PersistentContract` - Stores data contracts
- `PersistentToken` - Stores token configurations
- `PersistentTokenBalance` - Stores token balances
- `PersistentPublicKey` - Stores public keys with optional private key references

### Token Querying System

The `PersistentToken` model includes an advanced querying system for finding tokens with specific control rules:

#### Indexed Properties
```swift
// Boolean properties for easy filtering
token.canManuallyMint      // Has manual minting rules
token.canManuallyBurn      // Has manual burning rules
token.canFreeze            // Has freeze rules
token.hasDistribution      // Has distribution mechanisms
token.isPaused             // Token is paused
```

#### Query Predicates
```swift
// Find all mintable tokens
@Query(filter: PersistentToken.mintableTokensPredicate())
private var mintableTokens: [PersistentToken]

// Find tokens with specific control rules
let descriptor = FetchDescriptor<PersistentToken>(
    predicate: PersistentToken.tokensWithControlRulePredicate(rule: .manualMinting)
)
```

#### Available Predicates
- `mintableTokensPredicate()` - Tokens that allow manual minting
- `burnableTokensPredicate()` - Tokens that allow manual burning
- `freezableTokensPredicate()` - Tokens that can be frozen
- `distributionTokensPredicate()` - Tokens with distribution mechanisms
- `pausedTokensPredicate()` - Paused tokens
- `tokensByContractPredicate(contractId:)` - Tokens by contract
- `tokensWithControlRulePredicate(rule:)` - Tokens with specific control rule

### Key Storage Architecture

Private keys are stored separately from identities:
- Private keys belong to public keys, not identities
- Uses iOS Keychain for secure storage
- Cryptographic validation ensures correct key matching

### Service Architecture

- `UnifiedAppState` - Coordinates Core and Platform features
- `WalletService` - Manages SPV wallet operations
- `PlatformService` - Handles identity and document operations
- `DataManager` - Handles SwiftData persistence
- `KeychainManager` - Manages secure key storage

## Common Development Tasks

### Adding New Token Control Rules
1. Add the rule to `PersistentToken` model
2. Create a computed property for easy access
3. Add a predicate method for querying
4. Update `DataContractParser` to parse the rule

### Working with Private Keys
- Always validate private keys match their public keys using `KeyValidation.validatePrivateKeyForPublicKey`
- Store in Keychain using `KeychainManager`
- Link to `PersistentPublicKey`, not `PersistentIdentity`

### Loading Data Contracts
1. Use `LocalDataContractsView` to load contracts from network
2. `DataContractParser` automatically parses tokens and document types
3. Relationships are automatically linked via `dataContract` property

## Testing Guidelines

- Mock data creation helpers exist in test files
- Use `TestSigner` for transaction signing in tests
- Check `KeyValidation` for cryptographic validation logic

## UI Patterns

- Use SwiftUI with `@Query` for reactive data
- Break complex views into smaller components to avoid compiler timeouts
- Use `NavigationLink` for drill-down navigation
- Implement proper loading and error states

## Important Notes

- Always clean and rebuild after merging branches
- Token models support full rs-dpp specification
- All Codable types must be Equatable for SwiftData predicates
- Use English plural forms for token display names