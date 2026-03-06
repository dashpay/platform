# Swift SDK Migration Plan

## Problem Statement

The SwiftExampleApp contains approximately **8,000+ lines of SDK code** that should be in the `SwiftDashSDK` library. This includes:
- Platform query implementations
- State transition builders
- Service abstractions
- Domain models
- FFI wrapper extensions

The example app should only contain UI code and app-specific configuration.

---

## Current Structure Analysis

### SDK Sources (`Sources/SwiftDashSDK/`) - **34 files, ~4,500 LOC**

```
SwiftDashSDK/
├── SDK.swift                           # Core SDK class (572 lines)
├── SwiftDashSDK.swift                  # Re-exports
├── Identity.swift                      # Identity types
├── DataContract.swift                  # Contract types
├── IdentityTypes.swift                 # Identity enums/structs
├── ConcurrencyCompat.swift             # Concurrency helpers
├── KeyWallet/                          # Wallet functionality
│   ├── Wallet.swift
│   ├── KeyWallet.swift
│   ├── ManagedWallet.swift
│   ├── Account.swift
│   ├── ManagedAccount.swift
│   ├── AccountCollection.swift
│   ├── ManagedAccountCollection.swift
│   ├── KeyDerivation.swift
│   ├── Mnemonic.swift
│   ├── BIP38.swift
│   ├── Address.swift
│   ├── AddressPool.swift
│   ├── KeyWalletTypes.swift
│   ├── BLSAccount.swift
│   ├── EdDSAAccount.swift
│   └── Transaction.swift
├── PlatformWallet/                     # Platform wallet
│   ├── PlatformWallet.swift
│   ├── PlatformWalletFFI.swift
│   ├── IdentityManager.swift
│   ├── ManagedIdentity.swift
│   ├── ContactRequest.swift
│   ├── EstablishedContact.swift
│   └── PlatformWalletTypes.swift
├── SPV/
│   └── SPVClient.swift                 # SPV client wrapper
├── Tx/
│   ├── TransactionBuilder.swift
│   └── TransactionTypes.swift
└── Utils/
    └── KeyValidation.swift
```

### SwiftExampleApp - Files That Should Move

#### CRITICAL PRIORITY - SDK Extensions (~4,200 LOC)

| File | Lines | Description | Target Location |
|------|-------|-------------|-----------------|
| `SDK/StateTransitionExtensions.swift` | ~2,782 | State transition building, document/identity/contract operations | `Sources/SwiftDashSDK/StateTransition/` |
| `SDK/PlatformQueryExtensions.swift` | ~1,370 | Platform queries (identity, contract, document, DPNS) | `Sources/SwiftDashSDK/Queries/` |
| `SDK/SDKExtensions.swift` | ~23 | Minor SDK extensions | Merge into `SDK.swift` |

#### HIGH PRIORITY - Services (~800 LOC)

| File | Lines | Description | Target Location |
|------|-------|-------------|-----------------|
| `Services/DashPayService.swift` | ~292 | DashPay contact management | `Sources/SwiftDashSDK/PlatformWallet/DashPayService.swift` |
| `Services/KeychainManager.swift` | ~300 | Secure key storage | `Sources/SwiftDashSDK/Security/KeychainManager.swift` |
| `Core/Services/WalletService.swift` | ~400+ | SPV wallet service | `Sources/SwiftDashSDK/SPV/WalletService.swift` |

#### HIGH PRIORITY - Core Wallet (~600 LOC)

| File | Lines | Description | Target Location |
|------|-------|-------------|-----------------|
| `Core/Wallet/HDWallet.swift` | ~200 | HD wallet implementation | Review - may duplicate `KeyWallet/` |
| `Core/Wallet/TransactionService.swift` | ~150 | Transaction broadcasting | `Sources/SwiftDashSDK/Tx/TransactionService.swift` |
| `Core/Wallet/TransactionErrors.swift` | ~50 | Transaction error types | `Sources/SwiftDashSDK/Tx/TransactionErrors.swift` |
| `Core/Wallet/HDTransaction.swift` | ~100 | Transaction models | Review - may duplicate `Tx/` |
| `Core/Wallet/WalletManager.swift` | ~100 | Wallet lifecycle | Review - may duplicate `KeyWallet/WalletManager.swift` |

#### MEDIUM PRIORITY - Models (~1,000 LOC)

| File | Lines | Description | Target Location |
|------|-------|-------------|-----------------|
| `Core/Models/UTXO.swift` | ~60 | UTXO model | `Sources/SwiftDashSDK/SPV/UTXO.swift` |
| `Core/Models/Balance.swift` | ~40 | Balance model | `Sources/SwiftDashSDK/Wallet/Balance.swift` |
| `Core/Models/Transaction.swift` | ~80 | Transaction display model | `Sources/SwiftDashSDK/Tx/TransactionModel.swift` |
| `Core/Models/HDWalletModels.swift` | ~150 | Wallet state models | `Sources/SwiftDashSDK/Wallet/WalletModels.swift` |
| `Core/Utils/DataContractParser.swift` | ~300 | Contract parsing | `Sources/SwiftDashSDK/DataContract/Parser.swift` |
| `Models/Network.swift` | ~50 | Network enum | `Sources/SwiftDashSDK/Network.swift` |
| `Models/TestnetNodes.swift` | ~100 | Node configuration | `Sources/SwiftDashSDK/Config/TestnetNodes.swift` |

#### MEDIUM PRIORITY - DPP Types (~400 LOC)

| File | Lines | Description | Target Location |
|------|-------|-------------|-----------------|
| `Models/DPP/DPPCoreTypes.swift` | ~100 | Core DPP types | Review - may duplicate `Identity.swift`/`DataContract.swift` |
| `Models/DPP/Identity.swift` | ~100 | Identity types | Review - may duplicate SDK types |
| `Models/DPP/DataContract.swift` | ~100 | Contract types | Review - may duplicate SDK types |
| `Models/DPP/Document.swift` | ~50 | Document types | `Sources/SwiftDashSDK/Document.swift` |
| `Models/DPP/StateTransition.swift` | ~50 | State transition types | `Sources/SwiftDashSDK/StateTransition/Types.swift` |

#### LOW PRIORITY - Helpers (~200 LOC)

| File | Lines | Description | Target Location |
|------|-------|-------------|-----------------|
| `Helpers/WIFParser.swift` | ~50 | WIF key parsing | `Sources/SwiftDashSDK/Utils/WIFParser.swift` |
| `Utils/TestKeyGenerator.swift` | ~50 | Key generation for tests | `Sources/SwiftDashSDK/Utils/TestKeyGenerator.swift` |
| `SDK/TestSigner.swift` | ~51 | Test signing | `Sources/SwiftDashSDK/Testing/TestSigner.swift` |

---

## Files That Should STAY in SwiftExampleApp

### UI Code (~35+ view files)
- All files in `Views/` directory
- All files in `Core/Views/` directory
- `ContentView.swift`
- `SwiftExampleAppApp.swift`
- `Version.swift`

### App-Specific State
- `AppState.swift` - Needs refactoring (split SDK manager from UI state)
- `UnifiedAppState.swift` - App-specific coordination

### SwiftData Persistence (App-Specific)
- All files in `Models/SwiftData/` - These are tied to iOS app persistence
- `Services/DataManager.swift` - SwiftData operations
- `Core/Utils/ModelContainerHelper.swift`

### App-Specific Services
- `Core/Services/FilterMatchService.swift` - Filter matching UI logic
- `Core/Models/FilterMatch.swift` - App-specific model
- `Core/Models/CoreTypes.swift` - App display types
- `Core/Wallet/WalletStorage.swift` - SwiftData wallet persistence
- `Core/Wallet/WalletViewModel.swift` - UI view model

### App Configuration
- `Utils/EnvLoader.swift` - Environment loading

---

## Migration Phases

### Phase 1: Critical SDK Extensions (Highest Impact)

**Goal**: Move 4,000+ lines of SDK functionality

1. **Create new SDK directories**:
   ```
   Sources/SwiftDashSDK/
   ├── StateTransition/
   │   ├── StateTransitionBuilder.swift
   │   ├── IdentityTransitions.swift
   │   ├── ContractTransitions.swift
   │   ├── DocumentTransitions.swift
   │   └── TokenTransitions.swift
   ├── Queries/
   │   ├── IdentityQueries.swift
   │   ├── ContractQueries.swift
   │   ├── DocumentQueries.swift
   │   └── DPNSQueries.swift
   └── Document.swift
   ```

2. **Move `StateTransitionExtensions.swift`**:
   - Split into logical files by domain
   - Keep as SDK extensions
   - Update access control (`public`)
   - Remove app-specific dependencies

3. **Move `PlatformQueryExtensions.swift`**:
   - Split into logical files by entity type
   - Keep as SDK extensions
   - Ensure all helper methods are available

4. **Update SwiftExampleApp**:
   - Remove moved files
   - Update imports

**Estimated Effort**: Large - these files have many dependencies

### Phase 2: Services Migration

**Goal**: Move reusable services to SDK

1. **Move `KeychainManager.swift`**:
   - Pure utility, no app dependencies
   - Add to `Sources/SwiftDashSDK/Security/`
   - Make public

2. **Move `DashPayService.swift`**:
   - Already uses SDK PlatformWallet types
   - Add to `Sources/SwiftDashSDK/PlatformWallet/`
   - Extract UI-specific parts (`DashPayContact`, `DashPayContactRequest`) - keep in app

3. **Move `WalletService.swift`** (if not duplicating):
   - Review overlap with existing SDK wallet code
   - May need significant refactoring

**Estimated Effort**: Medium

### Phase 3: Models and Types

**Goal**: Consolidate domain models

1. **Review DPP types for duplication**:
   - Compare `Models/DPP/*` with SDK's `Identity.swift`, `DataContract.swift`
   - Consolidate into single source of truth

2. **Move core models**:
   - `UTXO.swift` → SDK
   - `Balance.swift` → SDK
   - `Transaction.swift` (display model) → SDK or keep in app
   - `DataContractParser.swift` → SDK

3. **Move configuration**:
   - `Network.swift` → SDK
   - `TestnetNodes.swift` → SDK config

**Estimated Effort**: Medium - requires careful deduplication

### Phase 4: Wallet Code Cleanup

**Goal**: Eliminate duplication

1. **Audit `Core/Wallet/` vs `KeyWallet/`**:
   - `HDWallet.swift` - likely duplicates `KeyWallet.swift`
   - `WalletManager.swift` - likely duplicates `KeyWallet/WalletManager.swift`
   - `HDTransaction.swift` - likely duplicates `KeyWallet/Transaction.swift`

2. **Consolidate or remove duplicates**

3. **Update app to use SDK wallet code**

**Estimated Effort**: Medium - requires understanding both implementations

### Phase 5: AppState Refactoring

**Goal**: Separate SDK manager from UI state

1. **Create `SDKManager` in SDK**:
   - SDK lifecycle (init, network switch)
   - Contract loading
   - Status monitoring
   - Move from `AppState.swift`

2. **Reduce `AppState.swift` to UI concerns**:
   - View navigation state
   - Selected items
   - UI-specific preferences
   - Use `SDKManager` for SDK operations

**Estimated Effort**: Large - `AppState.swift` is 25,000+ bytes with many dependencies

---

## Proposed SDK Structure After Migration

```
Sources/SwiftDashSDK/
├── SDK.swift                           # Core SDK class
├── SwiftDashSDK.swift                  # Re-exports
├── SDKManager.swift                    # NEW: High-level SDK lifecycle
├── Network.swift                       # NEW: Network enum
│
├── Identity/                           # Consolidated identity
│   ├── Identity.swift
│   ├── IdentityTypes.swift
│   └── IdentityQueries.swift           # NEW: From PlatformQueryExtensions
│
├── DataContract/                       # Consolidated contracts
│   ├── DataContract.swift
│   ├── ContractQueries.swift           # NEW: From PlatformQueryExtensions
│   └── Parser.swift                    # NEW: From DataContractParser
│
├── Document/                           # NEW: Document module
│   ├── Document.swift
│   └── DocumentQueries.swift           # NEW: From PlatformQueryExtensions
│
├── StateTransition/                    # NEW: State transitions
│   ├── Builder.swift                   # From StateTransitionExtensions
│   ├── IdentityTransitions.swift
│   ├── ContractTransitions.swift
│   ├── DocumentTransitions.swift
│   └── TokenTransitions.swift
│
├── DPNS/                               # NEW: DPNS module
│   └── DPNSQueries.swift               # From PlatformQueryExtensions
│
├── KeyWallet/                          # Existing - keep
│   └── ...
│
├── PlatformWallet/                     # Existing - expand
│   ├── ...
│   └── DashPayService.swift            # NEW: From Services
│
├── SPV/                                # Existing - expand
│   ├── SPVClient.swift
│   ├── WalletService.swift             # NEW: Core service logic
│   └── UTXO.swift                      # NEW: From Core/Models
│
├── Tx/                                 # Existing - expand
│   ├── TransactionBuilder.swift
│   ├── TransactionTypes.swift
│   ├── TransactionService.swift        # NEW: From Core/Wallet
│   └── TransactionErrors.swift         # NEW: From Core/Wallet
│
├── Wallet/                             # NEW: Common wallet types
│   ├── Balance.swift                   # From Core/Models
│   └── WalletModels.swift              # From Core/Models
│
├── Security/                           # NEW: Security utilities
│   └── KeychainManager.swift           # From Services
│
├── Config/                             # NEW: Configuration
│   └── TestnetNodes.swift              # From Models
│
├── Utils/                              # Existing - expand
│   ├── KeyValidation.swift
│   ├── WIFParser.swift                 # NEW: From Helpers
│   └── TestKeyGenerator.swift          # NEW: From Utils
│
└── Testing/                            # NEW: Test utilities
    └── TestSigner.swift                # From SDK
```

---

## Risk Assessment

### High Risk
- **StateTransitionExtensions.swift**: 2,782 lines, many FFI calls, complex dependencies
- **AppState.swift**: 25,000+ bytes, central to the app, many consumers
- **Wallet code duplication**: Two parallel implementations may have subtle differences

### Medium Risk
- **PlatformQueryExtensions.swift**: 1,370 lines, cleaner structure
- **Services**: Well-encapsulated, fewer dependencies
- **DPP types**: May have evolved differently in app vs SDK

### Low Risk
- **Utility files**: Self-contained, minimal dependencies
- **Configuration files**: Simple data structures
- **Helper functions**: Isolated functionality

---

## Recommended Order of Execution

1. **KeychainManager.swift** - Low risk, self-contained
2. **Network.swift** & **TestnetNodes.swift** - Simple data
3. **UTXO.swift** & **Balance.swift** - Simple models
4. **DataContractParser.swift** - Useful for SDK
5. **DashPayService.swift** - Already uses SDK types
6. **PlatformQueryExtensions.swift** - Split into modules
7. **StateTransitionExtensions.swift** - Split into modules
8. **WalletService.swift** - After audit for duplication
9. **Wallet code cleanup** - Eliminate duplicates
10. **AppState refactoring** - Create SDKManager

---

## Validation Steps

After each phase:
1. Build SwiftDashSDK library
2. Build SwiftExampleApp
3. Run app on simulator
4. Test affected functionality
5. Run any existing tests

---

## Files Summary

### To Move: ~40 files, ~8,000+ LOC
### To Stay: ~50 files (UI, SwiftData, app-specific)
### To Review for Duplication: ~10 files

---

## Notes

- All moved code needs `public` access modifiers for external use
- Consider backwards compatibility - existing app code should work after migration
- Some files may need splitting (e.g., StateTransitionExtensions by domain)
- DPP types need careful review to avoid breaking existing serialization
