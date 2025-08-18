# Token Transition Tests

## Overview
This directory contains tests for the token state transitions in the WASM SDK.

## New Token Transitions (Implemented)
The following token transitions have been implemented and added to the SDK:

1. **tokenTransfer** - Transfer tokens between identities
2. **tokenFreeze** - Freeze tokens for a specific identity  
3. **tokenUnfreeze** - Unfreeze tokens for a specific identity
4. **tokenDestroyFrozen** - Destroy frozen tokens

## Test Files

### token-transitions.test.mjs
New test file that tests the four newly implemented token transitions:
- Tests parameter validation
- Tests error handling for invalid inputs
- Tests permission requirements
- Verifies all methods are available on the SDK instance

### state-transitions.test.mjs (Needs Update)
The existing state transitions test file contains an outdated test for `token_transfer` (line 307-325) that uses the old function signature:
```javascript
// OLD (no longer exists)
await wasmSdk.token_transfer(sdk, mnemonic, identity, contract, recipient, amount, keyIndex)

// NEW (implemented)
await sdk.tokenTransfer(contractId, position, amount, senderId, recipientId, privateKey, publicNote)
```

This test should be updated or removed since the old function no longer exists.

## Running Tests

To run the token transition tests:

1. First build the WASM SDK:
   ```bash
   ./build.sh
   ```

2. Then run the tests:
   ```bash
   node test/token-transitions.test.mjs
   ```

## Expected Results

Most tests will fail with permission/identity errors, which is expected behavior since we're testing without real funded identities. The important validations are:

1. All methods are available on the SDK instance
2. Parameter validation works correctly
3. Invalid inputs are rejected with appropriate errors
4. The methods attempt to connect to the network (even if they fail due to permissions)

## Integration with UI

The token transitions are also exposed in the HTML UI (index.html) and defined in api-definitions.json, allowing users to:
- Execute token transfers through the web interface
- Freeze and unfreeze tokens
- Destroy frozen tokens
- All with optional public notes for transparency