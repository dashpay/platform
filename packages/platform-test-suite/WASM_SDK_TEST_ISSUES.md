# Platform Test Suite - WASM SDK Integration Issues Documentation

## Overview
This document captures the issues encountered and solutions found while migrating the platform test suite to work with wasm-sdk.

## Key Findings

### 1. Faucet Wallet Has UTXOs
The faucet wallet DOES have UTXOs (38 found when checking directly). The "utxosList must contain at least 1 utxo" error is NOT because the faucet lacks funds.

### 2. Test Environment Setup
- Local dashmate network must be running (`yarn dashmate status`)
- Blocks must be generated to the faucet address: `yarn dashmate core cli "generatetoaddress 100 yhvXpqQjfN9S4j5mBKbxeGxiETJrrLETg5"`
- Environment variables are loaded from `.env` file via `bootstrap.js`

### 3. Critical Environment Variables
```bash
DAPI_SEED=127.0.0.1:3000
FAUCET_1_ADDRESS=yhvXpqQjfN9S4j5mBKbxeGxiETJrrLETg5
FAUCET_1_PRIVATE_KEY=cR4t6evwVZoCp1JsLk4wURK4UmBCZzZotNzn9T1mhBT19SH9JtNt
NETWORK=regtest
```

### 4. Bootstrap Process
The `lib/test/bootstrap.js` file:
- Sets `FAUCET_PRIVATE_KEY` from `FAUCET_1_PRIVATE_KEY` (or FAUCET_2 for worker 2)
- Sets `FAUCET_ADDRESS` from `FAUCET_1_ADDRESS`
- This mapping happens based on MOCHA_WORKER_ID

### 5. No Regressions from Our Changes
We verified that our changes did NOT cause the wallet sync issues:
- Adding `patchClientForTests` doesn't affect wallet sync
- The network configuration was already correct
- Reverting our changes didn't fix the UTXO issues

### 6. Current Test Status
- 6-7 tests passing (up from 0)
- 5 tests failing with various issues:
  - 3 tests: "utxosList must contain at least 1 utxo" 
  - 1 test: ES module loading error for wasm-sdk
  - 1 test: "Insufficient funds"

### 7. Root Cause of UTXO Error
The issue appears to be timing-related. The faucet wallet HAS UTXOs but `fundWallet` is being called before the wallet has fully synced. The wallet sync is asynchronous and the tests don't wait long enough.

### 8. DPP Compatibility
We successfully fixed the `dpp.identity` undefined error by:
- Adding a fallback in `createAssetLockProof.ts` when dpp is undefined
- Creating a compatibility layer in `wasm-sdk-compat.js`

### 9. ES Module Loading
Some tests fail with ES module loading errors. This is because:
- wasm-sdk is an ES module
- The build system uses CommonJS require()
- We have a dynamic loader but it's not working in all test scenarios

## Recommendations for Future Fixes

1. **Wallet Sync Issue**: Add proper waiting/retry logic in `fundWallet` to ensure the faucet wallet has synced its UTXOs before attempting to create transactions.

2. **ES Module Loading**: The wasm-sdk loader needs to be more robust. Consider using dynamic imports consistently across all test scenarios.

3. **Test Isolation**: Some tests may be interfering with each other's wallet state. Consider better test isolation.

4. **Storage Settings**: The FAUCET_WALLET_USE_STORAGE setting affects wallet persistence. When disabled, the wallet must sync from scratch each time.

## Test Commands

```bash
# Generate blocks to fund faucet
cd ../.. && yarn dashmate core cli "generatetoaddress 100 yhvXpqQjfN9S4j5mBKbxeGxiETJrrLETg5"

# Run specific test
./bin/test.sh test/functional/core/broadcastTransaction.spec.js

# Run all tests
npm test
```

## Files Modified
- `packages/js-dash-sdk/src/SDK/Client/Platform/methods/identities/internal/createAssetLockProof.ts` - Added fallback for undefined dpp
- `packages/platform-test-suite/lib/test/wasm-sdk-compat.js` - Created compatibility layer
- Various platform method files - Migrated to use wasm-sdk

## What Was NOT Changed
- Wallet sync logic
- Test environment setup
- Faucet funding mechanism