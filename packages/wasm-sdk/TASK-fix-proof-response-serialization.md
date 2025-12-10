# Task: Fix ProofMetadataResponse Serialization Issues

## Problem Summary

`ProofMetadataResponse.toJSON()` fails to serialize properly when the response contains BigInt values that exceed JavaScript's `Number.MAX_SAFE_INTEGER` (9007199254740991).

## Reproduction

1. Use the evo-sdk-website at `/Users/ivanshumkov/Projects/dashpay/evo-sdk-website`
2. Run `yarn && yarn serve` to start the website
3. Open browser to `http://localhost:8081`
4. Select "Queries" → "System & Utility" → "Get Total Credits in Platform"
5. Check "With Proof Info" checkbox
6. Click "Execute"

**Expected:** JSON result with `{data, metadata, proof}` structure
**Actual:** Either `{}` (empty object) or error: `"Error: 23522425453263151 can't be represented as a JavaScript number"`

## Root Cause Analysis

The `ProofMetadataResponse` class in wasm-sdk has:
- `toJSON(): any` method
- `readonly data: any`
- `readonly metadata: ResponseMetadata`
- `readonly proof: ProofInfo`

When `toJSON()` is called and the `data` field contains a BigInt value (like total credits = 23522425453263151), the Rust-to-JS conversion fails because:
1. BigInt values > `Number.MAX_SAFE_INTEGER` cannot be safely represented as JavaScript numbers
2. The WASM binding tries to convert these to JS numbers, causing the error

## Location of Issue

Check the Rust implementation of `ProofMetadataResponse::to_json()` in:
- `packages/wasm-sdk/src/` (look for ProofMetadataResponse struct and its toJSON/to_json implementation)

## Suggested Fix

Modify the `toJSON()` implementation to convert BigInt values to strings instead of numbers. This is the standard approach for handling large integers in JSON serialization.

For example, in the Rust WASM binding:
```rust
// Instead of returning large numbers directly, convert to string
// if value > Number.MAX_SAFE_INTEGER, serialize as string
```

Or use a custom serializer that handles BigInt → String conversion.

## Test Cases

After fixing, verify these queries work with "With Proof Info" enabled:
1. `getTotalCreditsInPlatform` - returns large credit balance
2. `getDataContract` - returns contract with proof
3. `getCurrentEpoch` - returns epoch info with proof
4. `getEpochsInfo` - returns multiple epochs with proof

## Related Files

- WASM SDK types: `/Users/ivanshumkov/Projects/dashevo/platform.workspaces/methods/packages/wasm-sdk/dist/sdk.d.ts` (lines 2072-2085 for ProofMetadataResponse)
- Consumer code experiencing issue: `/Users/ivanshumkov/Projects/dashpay/evo-sdk-website/public/app.js` (formatResult function)

## Alternative Workaround (if SDK fix takes time)

The consumer (`app.js`) could manually extract properties using getters instead of relying on `toJSON()`:
```javascript
if ('data' in val && 'metadata' in val && 'proof' in val) {
    return {
        data: val.data,
        metadata: val.metadata,
        proof: val.proof
    };
}
```

But the proper fix should be in the wasm-sdk to ensure `toJSON()` works correctly.
