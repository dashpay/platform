# DPNS Name Resolution Implementation

This document describes the implementation of the `dash_sdk_identity_resolve_name` function in the rs-sdk-ffi package.

## Overview

The function resolves DPNS (Dash Platform Name Service) names to identity IDs. DPNS is similar to DNS but for Dash Platform, allowing users to register human-readable names that point to their identity IDs.

## Function Signature

```c
DashSDKResult dash_sdk_identity_resolve_name(
    const SDKHandle* sdk_handle,
    const char* name
);
```

## Parameters

- `sdk_handle`: A handle to an initialized SDK instance
- `name`: A null-terminated C string containing the name to resolve (e.g., "alice.dash" or just "alice")

## Return Value

Returns a `DashSDKResult` that contains:
- On success: Binary data containing the 32-byte identity ID
- On error: An error code and message

## Implementation Details

### Name Parsing

Names are parsed into two components:
1. **Label**: The leftmost part of the name (e.g., "alice" in "alice.dash")
2. **Parent Domain**: The domain after the last dot (e.g., "dash" in "alice.dash")

If no parent domain is specified, "dash" is used as the default.

### Normalization

Both the label and parent domain are normalized using `convert_to_homograph_safe_chars` to prevent homograph attacks and ensure consistent lookups.

### DPNS Contract

The function queries the DPNS data contract which stores domain documents. Each domain document contains:
- `normalizedLabel`: The normalized version of the label
- `normalizedParentDomainName`: The normalized parent domain name
- `records`: A map that can contain:
  - `dashUniqueIdentityId`: The primary identity ID for this name
  - `dashAliasIdentityId`: An alias identity ID for this name

### Query Process

1. Fetch the DPNS data contract using its well-known ID
2. Create a document query for the "domain" document type
3. Add where clauses to filter by normalized label and parent domain
4. Fetch the matching document
5. Extract the identity ID from the `records` field

### Priority

The function checks for identity IDs in this order:
1. `dashUniqueIdentityId` (primary)
2. `dashAliasIdentityId` (alias)

## Error Handling

The function returns appropriate error codes for:
- `InvalidParameter`: Null SDK handle, null name, or invalid UTF-8
- `InvalidState`: No tokio runtime available
- `NotFound`: DPNS contract not found, domain not found, or no identity ID in records
- `NetworkError`: Failed to fetch data from the network
- `InternalError`: Failed to create queries or other internal errors

## Example Usage

```c
// Initialize SDK
DashSDKConfig config = {
    .network = DashSDKNetwork_Testnet,
    .dapi_addresses = "https://testnet.dash.org:443",
    // ... other config
};
DashSDKResult sdk_result = dash_sdk_create(&config);
SDKHandle* sdk = (SDKHandle*)sdk_result.data;

// Resolve a name
DashSDKResult result = dash_sdk_identity_resolve_name(sdk, "alice.dash");

if (result.error == NULL) {
    // Success - result.data contains binary identity ID
    DashSDKBinaryData* binary_data = (DashSDKBinaryData*)result.data;
    // Use binary_data->data (32 bytes) and binary_data->len
    
    // Clean up
    dash_sdk_result_free(result);
} else {
    // Handle error
    printf("Error: %s\n", result.error->message);
    dash_sdk_result_free(result);
}
```

## Testing

The implementation includes unit tests for:
- Null parameter handling
- Invalid UTF-8 handling
- Name parsing logic

Integration tests would require a running Dash Platform network with registered DPNS names.