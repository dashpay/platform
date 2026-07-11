# SwiftExampleApp - AI Assistant Guide

This document provides guidance for AI assistants working with the SwiftExampleApp codebase.

## Overview

SwiftExampleApp is an iOS application demonstrating the integration of both Core (SPV wallet) and Platform (identity/documents) functionality of the Dash SDK.

## Funding a testnet wallet for testing

To get testnet funds for a wallet in the app, use the built-in faucet: **Wallet → Receive** has a "request from testnet" button that funds the displayed receive address. No external faucet or pasted seed is needed — create a fresh wallet, open Wallet → Receive, and tap it. Use this when an end-to-end test needs a funded wallet (e.g. registering an identity, signing state transitions).

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

- `AppState` - Platform identity/document/contract state (SDK wrapper)
- `PlatformWalletManager` - Drives wallet creation, SPV sync, and BLAST
  balance sync via the Rust-side `platform-wallet` crate. Holds **N
  wallets concurrently** keyed by walletId — BLAST sync iterates all
  of them. Publishes `spvProgress`, `wallets` (the full map), and
  `lastError`. Look up a specific wallet with `wallet(for: walletId)`
  or grab a deterministic default via `firstWallet`. Persists data
  via SwiftData using `PlatformWalletPersistenceHandler` — UI queries
  `PersistentWallet`, `PersistentAccount`, `PersistentTransaction`,
  and `PersistentUtxo` directly with `@Query`.
- `PlatformBalanceSyncService` - Drives periodic BLAST address sync on the
  platform side.
- `ShieldedService` - Shielded pool (Orchard) operations.
- `TransitionState` - Ephemeral state (pricing, purchase eligibility) for
  state-transition flows.
- `AppUIState` - Small UI-only flags (e.g. detailed sync banner).
- `DataManager` - Handles SwiftData persistence for Platform data.
- `KeychainManager` - Manages secure key storage.

The previous `UnifiedAppState` / `WalletService` / `CoreWalletManager` /
`SPVClient` / `SPVEventHandler` stack has been removed. All wallet
operations now flow through `PlatformWalletManager`; all Core wallet data
is surfaced to views via SwiftData @Query on the Persistent* models.

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
