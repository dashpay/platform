# iOS SDK to Dash SDK API Mapping Plan

## Type Mappings

### Data Types
- `IOSSDKBinaryData` → `DashSDKBinaryData` (already exists in rs-sdk-ffi)
- `IOSSDKResultDataType` → `DashSDKResultDataType` (already exists in rs-sdk-ffi)
- `IOSSDKIdentityInfo` → `DashSDKIdentityInfo` (already exists in rs-sdk-ffi)
- `IOSSDKPutSettings` → `DashSDKPutSettings` (already exists in rs-sdk-ffi)
- `IOSSDKTransferCreditsResult` → `DashSDKTransferCreditsResult` (already exists in rs-sdk-ffi)

### Function Mappings

#### Identity Fetch/Get Operations
- `ios_sdk_identity_fetch()` → `dash_sdk_identity_get()` 
  - Note: The new API is called `dash_sdk_identity_fetch()`, not `dash_sdk_identity_get()`
  - Same signature and behavior

- `ios_sdk_identity_get_info()` → `dash_sdk_identity_get_info()`
  - Direct replacement, same signature

#### Identity Creation
- `ios_sdk_identity_create()` → `dash_sdk_identity_create()`
  - Direct replacement, same signature

#### Put Operations
- `ios_sdk_identity_put_to_platform_with_instant_lock()` → `dash_sdk_identity_put_to_platform_with_instant_lock()`
  - Direct replacement, same signature

- `ios_sdk_identity_put_to_platform_with_instant_lock_and_wait()` → `dash_sdk_identity_put_to_platform_with_instant_lock_and_wait()`
  - Direct replacement, same signature

- `ios_sdk_identity_put_to_platform_with_chain_lock()` → `dash_sdk_identity_put_to_platform_with_chain_lock()`
  - Direct replacement, same signature

- `ios_sdk_identity_put_to_platform_with_chain_lock_and_wait()` → `dash_sdk_identity_put_to_platform_with_chain_lock_and_wait()`
  - Direct replacement, same signature

#### Transfer Operations
- `ios_sdk_identity_transfer_credits()` → `dash_sdk_identity_transfer_credits()`
  - Direct replacement, same signature

#### Top Up Operations
- `ios_sdk_identity_topup_with_instant_lock()` → `dash_sdk_identity_topup_with_instant_lock()`
  - Direct replacement, same signature

- `ios_sdk_identity_topup_with_instant_lock_and_wait()` → `dash_sdk_identity_topup_with_instant_lock_and_wait()`
  - Direct replacement, same signature

#### Withdraw Operations
- `ios_sdk_identity_withdraw()` → `dash_sdk_identity_withdraw()`
  - Direct replacement, same signature

#### Query Operations
- `ios_sdk_identity_fetch_balance()` → `dash_sdk_identity_fetch_balance()`
  - Direct replacement, same signature

- `ios_sdk_identity_fetch_public_keys()` → `dash_sdk_identity_fetch_public_keys()`
  - Direct replacement, same signature

#### Name Operations
- `ios_sdk_identity_register_name()` → `dash_sdk_identity_register_name()`
  - Direct replacement, same signature

- `ios_sdk_identity_resolve_name()` → `dash_sdk_identity_resolve_name()`
  - Direct replacement, same signature

#### Error Handling
- `ios_sdk_error_free()` → `dash_sdk_error_free()`
  - Direct replacement, same signature

## Functions That Need Re-implementation

The following convenience wrappers need to be kept as they provide Swift-friendly interfaces:

1. **SwiftDashIdentityInfo** - Keep as wrapper around DashSDKIdentityInfo
2. **SwiftDashBinaryData** - Keep as wrapper around DashSDKBinaryData  
3. **SwiftDashTransferCreditsResult** - Keep as wrapper around DashSDKTransferCreditsResult
4. **SwiftDashPutSettings** - Keep as wrapper, needs conversion to DashSDKPutSettings

## Key Changes Required

1. Replace all `rs_sdk_ffi::ios_sdk_*` calls with `rs_sdk_ffi::dash_sdk_*`
2. Replace `IOSSDKBinaryData` with `DashSDKBinaryData`
3. Replace `IOSSDKResultDataType` with `DashSDKResultDataType`
4. Replace `IOSSDKIdentityInfo` with `DashSDKIdentityInfo`
5. Replace `IOSSDKPutSettings` with `DashSDKPutSettings`
6. Replace `IOSSDKTransferCreditsResult` with `DashSDKTransferCreditsResult`
7. Update error handling to use `dash_sdk_error_free` instead of `ios_sdk_error_free`