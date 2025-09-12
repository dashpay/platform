# Expected Test Failures

This document categorizes and explains expected test failures in the WASM SDK test suite.

## Categories of Expected Failures

### 1. Network-Required Tests ❌

These tests require an active network connection to Dash Platform nodes:

#### Query Functions
- All identity queries (`identity_fetch`, `get_identity_balance`, etc.)
- All document queries (`get_documents`, `get_document`)
- All data contract queries (`data_contract_fetch`, `get_data_contract_history`)
- All token queries (`get_token_statuses`, `get_token_total_supply`, etc.)
- All epoch queries (`get_epochs_info`, `get_current_epoch`, etc.)
- All voting/contested resource queries
- All group queries

**Reason**: These functions need to communicate with Dash Platform nodes via gRPC.

#### State Transitions
- State transition functions work but may fail in test environments

**Updated Status (Post-Breakthrough Discovery)**:
✅ All platform state transition functions are implemented and working:
- `createDocument`, `updateDocument`, `deleteDocument`
- `createDataContract`, `updateDataContract`

**Current Test Challenges**:
1. Require valid funded identity with sufficient testnet credits
2. Need proper authentication with real private keys/mnemonics
3. Testnet proof verification may have quorum cache issues
4. Recommendation: Test with `proofs: false` for development

### 2. Testnet Configuration Issues 🌐

These are actual bugs that should be fixed:

#### testSerialization Returns Undefined
- **Test**: `testSerialization method availability`
- **Issue**: Method exists but returns `undefined` instead of test data
- **Expected**: Should return serialized test objects

#### Path Helper Functions
- **Tests**: `derivation_path_bip44_mainnet/testnet`, `derivation_path_dip9_mainnet/testnet`
- **Issue**: Return structure missing 'path' property
- **Expected**: Should return object with `{ path: "m/44'/5'/0'/0/0", ... }`

#### DPNS Homograph Conversion
- **Tests**: `dpns_convert_to_homograph_safe - special characters`, `dpns_convert_to_homograph_safe - unicode homographs`
- **Issue**: Doesn't remove special characters or handle unicode homographs
- **Expected**: Should convert/remove problematic characters

### 3. Test Environment Issues ⚠️

These fail due to test environment limitations:

#### Address Validation
- **Test**: `Can validate addresses`
- **Issue**: Test uses invalid example addresses
- **Fix**: Use actual valid Dash addresses for testing

#### Functions Causing Panics
- **`identity_put`**: Invalid secret key error
- **`epoch_testing`**: Connection pool configuration error
- **`start` (called twice)**: Trace dispatcher already set

**Note**: These are internal test functions and may not be intended for public use.

### 4. Not Yet Implemented 🚧

These functions are stubs or not fully implemented:

#### Extended Key Functions
- `derive_child_public_key`
- `xprv_to_xpub`

**Status**: Return "not yet implemented" error (expected behavior)

## Test Suite Status

| Category | Total | Pass | Fail | Notes |
|----------|-------|------|------|-------|
| SDK Init | 10 | 9 | 1 | Address validation needs fix |
| Key Gen | 53 | 49 | 4 | Path helpers return wrong structure |
| DPNS | 34 | 31 | 3 | Homograph handling incomplete |
| Utilities | 14 | 13 | 1 | testSerialization bug |
| **Total** | **111** | **102** | **9** | **91.9% pass rate** |

## Recommendations

### High Priority Fixes
1. Fix path helper functions to return correct structure
2. Fix testSerialization to return proper test data
3. Update address validation test with valid addresses

### Medium Priority
1. Implement DPNS homograph protection
2. Handle panics in test functions gracefully
3. Document which functions are internal/test-only

### Low Priority
1. Implement child key derivation functions
2. Add comprehensive network connectivity tests

## Running Tests

### Run All Tests
```bash
node test/run-all-tests.mjs
```

### Run Individual Test Suites
```bash
node test/sdk-init-simple.test.mjs
node test/key-generation.test.mjs
node test/dpns.test.mjs
node test/utilities-simple.test.mjs
```

### View Test Report
After running all tests, open `test/test-report.html` in a browser for detailed results.

## Network Testing

To test network-dependent functions:
1. Ensure you have internet connectivity
2. Dash Platform testnet nodes must be accessible
3. For state transitions, you need funded test identities

## Contributing

When adding new tests:
1. Categorize as network-dependent or standalone
2. Document expected failures in this file
3. Use try/catch to handle expected errors gracefully
4. Update the test runner if adding new test files