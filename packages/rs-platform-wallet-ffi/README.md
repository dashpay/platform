# Platform Wallet FFI

C-compatible FFI (Foreign Function Interface) bindings for the `platform-wallet` crate.

## Overview

This library provides C-compatible bindings for the Platform Wallet, enabling integration with other languages such as Swift, Kotlin, C++, and any language that can call C functions.

## Features

- **Wallet Management**: Create and manage platform wallets from seed or mnemonic
- **Identity Management**: Manage multiple Platform identities per network
- **Contact System**: Handle contact requests and established contacts (DashPay integration)
- **Serialization**: JSON serialization/deserialization support
- **Memory Safe**: Proper handle-based resource management
- **Thread Safe**: Uses thread-safe handle storage

## Building

### As a static library

```bash
cargo build --release
```

The static library will be available at `target/release/libplatform_wallet_ffi.a` (Unix) or `platform_wallet_ffi.lib` (Windows).

### As a dynamic library

```bash
cargo build --release --crate-type=cdylib
```

The dynamic library will be available at:
- Linux: `target/release/libplatform_wallet_ffi.so`
- macOS: `target/release/libplatform_wallet_ffi.dylib`
- Windows: `target/release/platform_wallet_ffi.dll`

## Usage

Include the header file in your C/C++ project:

```c
#include "platform_wallet_ffi.h"
```

### Example

```c
#include <stdio.h>
#include "platform_wallet_ffi.h"

int main() {
    // Initialize library
    platform_wallet_ffi_init();

    // Create wallet from mnemonic
    Handle wallet_handle = NULL_HANDLE;
    PlatformWalletFFIError error = {0};

    PlatformWalletFFIResult result = platform_wallet_info_create_from_mnemonic(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        NULL,
        &wallet_handle,
        &error
    );

    if (result != PLATFORM_WALLET_FFI_SUCCESS) {
        printf("Error: %s\n", error.message);
        platform_wallet_ffi_error_free(error);
        return 1;
    }

    // Create identity manager
    Handle manager_handle = NULL_HANDLE;
    result = identity_manager_create(&manager_handle, &error);

    if (result != PLATFORM_WALLET_FFI_SUCCESS) {
        printf("Error: %s\n", error.message);
        platform_wallet_ffi_error_free(error);
        platform_wallet_info_destroy(wallet_handle);
        return 1;
    }

    // Set identity manager for testnet
    result = platform_wallet_info_set_identity_manager(
        wallet_handle,
        NETWORK_TYPE_TESTNET,
        manager_handle,
        &error
    );

    // Cleanup
    identity_manager_destroy(manager_handle);
    platform_wallet_info_destroy(wallet_handle);

    return 0;
}
```

## API Overview

### Wallet Management

- `platform_wallet_info_create_from_seed()` - Create wallet from seed bytes
- `platform_wallet_info_create_from_mnemonic()` - Create wallet from BIP39 mnemonic
- `platform_wallet_info_get_identity_manager()` - Get identity manager for network
- `platform_wallet_info_set_identity_manager()` - Set identity manager for network
- `platform_wallet_info_to_json()` - Serialize wallet to JSON
- `platform_wallet_info_destroy()` - Free wallet resources

### Identity Management

- `identity_manager_create()` - Create new identity manager
- `identity_manager_add_identity()` - Add identity to manager
- `identity_manager_remove_identity()` - Remove identity from manager
- `identity_manager_get_identity()` - Get identity by ID
- `identity_manager_get_all_identity_ids()` - Get all identity IDs
- `identity_manager_set_primary_identity()` - Set primary identity
- `identity_manager_get_primary_identity_id()` - Get primary identity ID
- `identity_manager_destroy()` - Free manager resources

### Managed Identity

- `managed_identity_create_from_identity_bytes()` - Create from DPP identity
- `managed_identity_get_id()` - Get identity ID
- `managed_identity_get_balance()` - Get identity balance
- `managed_identity_get_label()` - Get identity label
- `managed_identity_set_label()` - Set identity label
- `managed_identity_get_last_updated_balance_block_time()` - Get sync status
- `managed_identity_set_last_updated_balance_block_time()` - Update sync status
- `managed_identity_to_json()` - Serialize to JSON
- `managed_identity_destroy()` - Free identity resources

### Contact Management

- `managed_identity_add_sent_contact_request()` - Add outgoing contact request
- `managed_identity_add_incoming_contact_request()` - Add incoming contact request
- `managed_identity_remove_sent_contact_request()` - Remove outgoing request
- `managed_identity_remove_incoming_contact_request()` - Remove incoming request
- `managed_identity_get_sent_contact_request_ids()` - Get all sent requests
- `managed_identity_get_incoming_contact_request_ids()` - Get all incoming requests
- `managed_identity_get_established_contact_ids()` - Get all established contacts
- `managed_identity_is_contact_established()` - Check if contact is established
- `managed_identity_remove_established_contact()` - Remove established contact

### Utilities

- `platform_wallet_generate_random_identifier()` - Generate random ID
- `platform_wallet_identifier_to_hex()` - Convert ID to hex string
- `platform_wallet_identifier_from_hex()` - Parse ID from hex string
- `platform_wallet_serialize_to_json_bytes()` - Serialize JSON to bytes
- `platform_wallet_deserialize_from_json_bytes()` - Deserialize bytes to JSON

### Memory Management

Always free resources when done:

- `platform_wallet_string_free()` - Free C strings
- `platform_wallet_bytes_free()` - Free byte arrays
- `platform_wallet_identifier_array_free()` - Free identifier arrays
- `platform_wallet_ffi_error_free()` - Free error messages

## Error Handling

All functions return a `PlatformWalletFFIResult` status code. Check for `PLATFORM_WALLET_FFI_SUCCESS` and handle errors appropriately.

Error codes:
- `PLATFORM_WALLET_FFI_SUCCESS` - Operation succeeded
- `PLATFORM_WALLET_FFI_ERROR_INVALID_HANDLE` - Invalid handle provided
- `PLATFORM_WALLET_FFI_ERROR_NULL_POINTER` - Null pointer provided
- `PLATFORM_WALLET_FFI_ERROR_SERIALIZATION` - Serialization failed
- `PLATFORM_WALLET_FFI_ERROR_DESERIALIZATION` - Deserialization failed
- `PLATFORM_WALLET_FFI_ERROR_IDENTITY_NOT_FOUND` - Identity not found
- `PLATFORM_WALLET_FFI_ERROR_CONTACT_NOT_FOUND` - Contact not found
- And more...

## Testing

Run the test suite:

```bash
cargo test
```

Run integration tests:

```bash
cargo test --test integration_tests
```

## Thread Safety

The library uses thread-safe storage for handles, making it safe to use from multiple threads. However, you should not use the same handle from multiple threads simultaneously.

## Memory Management

The library uses a handle-based system to manage resources. Always call the appropriate `_destroy()` function to free resources when done.

Strings and arrays returned by the library must be freed using the provided free functions:
- `platform_wallet_string_free()`
- `platform_wallet_bytes_free()`
- `platform_wallet_identifier_array_free()`

## ABI stability / Release notes

This crate exposes a C ABI. Changes that alter an exported function's
signature or the numeric value of a result code are **breaking** for C/Swift/JNI
consumers and must be called out here.

- **C-ABI break:** `platform_wallet_manager_shielded_fund_from_asset_lock`
  gained a trailing `funding_path_ptr: *const u8, funding_path_len: usize`
  parameter — a UTF-8 BIP32 derivation path selecting the single source
  account (a null pointer / zero length = the unmixed BIP44 account). Callers
  linking the old symbol must be recompiled against the regenerated header;
  a null path reproduces the previous single-account behaviour. Adds result
  code `ERROR_ASSET_LOCK_INSUFFICIENT_FUNDS = 29`.

## License

MIT

## See Also

- [platform-wallet](../rs-platform-wallet) - Core Rust implementation
- [rs-sdk-ffi](../rs-sdk-ffi) - Platform SDK FFI bindings
