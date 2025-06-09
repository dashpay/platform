# GetBalances Implementation

## Overview

The `GetBalances()` function in the Go SDK has been properly implemented to fetch balances for multiple identities in a single call. This replaces the previous stub implementation that returned an empty map.

## Implementation Details

### C Structure Mapping

The implementation correctly handles the C structures defined in the FFI header:

```c
typedef struct DashSDKIdentityBalanceEntry {
  uint8_t identity_id[32];  // 32-byte identity ID
  uint64_t balance;         // Balance in credits (uint64 max = not found)
} DashSDKIdentityBalanceEntry;

typedef struct DashSDKIdentityBalanceMap {
  struct DashSDKIdentityBalanceEntry *entries;  // Array of entries
  uintptr_t count;                               // Number of entries
} DashSDKIdentityBalanceMap;
```

### Key Features

1. **Identity ID Format Support**:
   - Accepts both hex-encoded (64 characters) and Base58-encoded (44 characters) identity IDs
   - Internally converts all IDs to 32-byte arrays for the FFI call
   - Returns results with hex-encoded identity IDs as map keys

2. **Proper FFI Call**:
   - Calls `dash_sdk_identities_fetch_balances` with an array of 32-byte identity IDs
   - Correctly passes the array pointer and count to the C function

3. **Memory Management**:
   - Properly frees the returned `DashSDKIdentityBalanceMap` using `dash_sdk_identity_balance_map_free`
   - Safe conversion of C array to Go slice using unsafe pointer arithmetic

4. **Error Handling**:
   - Validates identity ID formats before making the FFI call
   - Returns descriptive errors for invalid IDs
   - Handles empty input gracefully

5. **Not Found Handling**:
   - The FFI returns `uint64::MAX` (18446744073709551615) for identities that don't exist
   - These are filtered out and not included in the returned map

## Usage Example

```go
// Fetch balances for multiple identities
ids := []string{
    "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF",  // Base58
    "2d1a6de6c01d4b8b8c0f6b1e0a2b5a6f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f",  // Hex
}

balances, err := sdk.Identities().GetBalances(ctx, ids)
if err != nil {
    log.Fatal(err)
}

// Results are keyed by hex-encoded identity IDs
for idHex, balance := range balances {
    fmt.Printf("Identity %s has balance: %d\n", idHex, balance)
}
```

## Testing

Comprehensive tests have been added to verify:
- Correct handling of both hex and Base58 identity IDs
- Empty input handling
- Invalid ID format detection
- Result format validation (hex keys, valid balances)
- Mixed valid/invalid ID error handling

## Technical Notes

1. The implementation uses unsafe pointer casting to convert the C array to a Go slice:
   ```go
   entries := (*[1 << 30]C.DashSDKIdentityBalanceEntry)(unsafe.Pointer(balanceMap.entries))[:balanceMap.count:balanceMap.count]
   ```

2. Identity IDs in the result map are always hex-encoded for consistency, regardless of input format

3. The special value `^uint64(0)` (bitwise NOT of 0) is used to check for "not found" identities

This implementation provides efficient batch balance queries with proper type safety and memory management.