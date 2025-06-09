# Go SDK Test Coverage

## Overview

The Go SDK includes comprehensive test coverage across all major components. Since Go is not installed in the current environment, tests cannot be run directly, but here's what's covered:

## Test Files

1. **sdk_test.go** - Core SDK functionality
   - `TestNewSDK` - SDK initialization
   - `TestNewMockSDK` - Mock SDK for testing
   - `TestSDKClose` - Proper resource cleanup
   - `TestSDKGetNetwork` - Network configuration
   - `TestVersion` - Version checking
   - `TestConfigPresets` - Configuration presets

2. **identity_test.go** - Identity management
   - `TestIdentityCreate` - Identity creation
   - `TestIdentityGet` - Fetching identities
   - `TestIdentityGetBalance` - Balance queries
   - `TestIdentityGetBalances` - Bulk balance queries
   - `TestIdentityGetByPublicKeyHash` - Key-based lookup
   - `TestIdentityPutToInstantLock` - Instant lock funding
   - `TestIdentityPutToChainLock` - Chain lock funding
   - `TestIdentityTransferCredits` - Credit transfers
   - `TestIdentityTopUp` - Identity top-up
   - `TestIdentityWithdraw` - Withdrawals
   - `TestIdentityRegisterName` - DPNS registration
   - `TestIdentityResolveName` - DPNS resolution
   - `TestIdentityClose` - Resource cleanup
   - `TestIdentityID` - ID type handling
   - `TestIdentityMethods` - General methods

3. **contract_test.go** - Data contract operations
   - `TestContractCreate` - Contract creation
   - `TestContractGet` - Fetching contracts
   - `TestContractGetMany` - Bulk fetching
   - `TestContractGetHistory` - History retrieval
   - `TestDataContractPut` - Publishing contracts
   - `TestDataContractPutAndWait` - Publishing with confirmation
   - `TestDataContractClose` - Resource cleanup
   - `TestContractID` - ID type handling
   - `TestDataContractMethods` - General methods

4. **document_test.go** - Document operations
   - `TestDocumentCreate` - Document creation
   - `TestDocumentGet` - Fetching documents
   - `TestDocumentSearch` - Document queries
   - `TestDocumentOperations` - CRUD operations
   - `TestDocumentClose` - Resource cleanup
   - `TestDocumentID` - ID type handling
   - `TestDocumentMethods` - General methods
   - `TestQueryBuilder` - Query builder functionality

5. **document_property_test.go** - Document property manipulation
   - `TestDocumentPropertyOperations` - Property updates
   - Tests for `updateHandle()` function
   - Tests for `SetProperty()` with nested paths
   - Tests for `RemoveProperty()`
   - Tests for read-only document handling

6. **token_test.go** - Token operations
   - `TestTokenMint` - Token minting
   - `TestTokenBurn` - Token burning
   - `TestTokenTransfer` - Token transfers
   - `TestTokenGetBalance` - Balance queries
   - `TestTokenGetInfo` - Token information
   - `TestTokenFreeze/Unfreeze` - Freezing operations
   - `TestTokenPurchase` - Document purchases
   - `TestTokenSetPrice` - Price management
   - `TestTokenClaim` - Distribution claims
   - `TestTokenGetAllocationInfo` - Allocation queries
   - `TestTokenDestroyFrozenFunds` - Frozen fund management
   - `TestTokenDistributionType` - Distribution types

7. **types_test.go** - Type system tests
   - `TestIdentityID` - Identity ID validation
   - `TestContractID` - Contract ID validation
   - `TestDocumentID` - Document ID validation
   - `TestPublicKeyHash` - Key hash handling
   - `TestNetworkString` - Network enumeration
   - `TestPutSettings` - Configuration types
   - `TestGasFeesPaidBy` - Payment options
   - `TestPropertySchema` - Schema definitions
   - `TestDocumentTypeDefinition` - Type definitions
   - `TestIndex` - Index handling
   - Helper function tests

8. **integration_test.go** - End-to-end tests
   - `TestIntegrationIdentityWorkflow` - Complete identity lifecycle
   - `TestIntegrationDataContractAndDocuments` - Contract to document flow
   - `TestIntegrationTokenOperations` - Token lifecycle
   - `TestIntegrationComplexQuery` - Advanced querying
   - `TestIntegrationErrorHandling` - Error scenarios

## Running Tests

To run the tests, you need:

1. **Go 1.19+** installed
2. **Rust toolchain** for building FFI library
3. **C compiler** (gcc/clang) for CGO

Then run:
```bash
./run_tests.sh
```

Or manually:
```bash
# Build FFI library
cd ../rs-sdk-ffi
cargo build --release

# Run Go tests
cd ../go-sdk
export CGO_LDFLAGS="-L../../target/release"
go test -v ./...
```

## Test Categories

### Unit Tests
- Test individual components in isolation
- Use mock SDK for offline testing
- Focus on API correctness and error handling

### Integration Tests
- Test interaction between components
- Require FFI library to be built
- Test real FFI calls and memory management

### Property Tests
- Test document property manipulation
- Verify nested path operations
- Test edge cases like read-only documents

## Coverage Areas

✅ **Well Covered:**
- Basic CRUD operations for all entities
- Error handling and edge cases
- Memory management and cleanup
- Type safety and conversions
- Query building and execution

⚠️ **Partial Coverage:**
- Network error scenarios
- Concurrent operations
- Performance benchmarks

## Mock SDK

The SDK includes a mock implementation (`NewMockSDK()`) that:
- Returns predictable test data
- Works offline without network
- Helps test application logic
- Simulates error conditions