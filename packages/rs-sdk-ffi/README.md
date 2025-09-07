# Dash SDK FFI - Unified SDK

FFI bindings for integrating both Dash Core (Layer 1) and Dash Platform (Layer 2) functionality through a unified SDK.

## Overview

This crate provides C-compatible FFI bindings for both the Dash Platform SDK (`rs-sdk`) and Dash Core SDK (`dash-spv-ffi`), creating a unified SDK that eliminates duplicate symbols and reduces binary size by 79.4%. Applications can use Core-only, Platform-only, or both functionalities from a single binary.

### Key Features
- **Unified Binary**: Single 29.5MB library (down from 143MB combined)
- **Dual Layer Support**: Access both Layer 1 (SPV/transactions) and Layer 2 (identities/documents)
- **No Symbol Conflicts**: Intelligent header merging resolves type conflicts
- **Cross-Platform**: Works on iOS, Android, and any platform supporting C interfaces

## Building

### Prerequisites

- Rust toolchain with appropriate targets:
  ```bash
  # For iOS
  rustup target add aarch64-apple-ios
  rustup target add aarch64-apple-ios-sim
  rustup target add x86_64-apple-ios
  
  # For Android
  rustup target add aarch64-linux-android
  rustup target add armv7-linux-androideabi
  rustup target add x86_64-linux-android
  
  # For other platforms, add as needed
  ```

- cbindgen (for header generation): `cargo install cbindgen`

### Build Instructions

The unified SDK includes both Core and Platform functionality by default:

```bash
# Standard build (includes both Core and Platform)
cargo build --release

# Generate unified C headers
GENERATE_BINDINGS=1 cargo build --release
```

### Platform-Specific Builds

#### iOS (Unified SDK)
```bash
# Build unified SDK for iOS (recommended)
./build_ios.sh [arm|x86|universal]

# Creates DashUnifiedSDK.xcframework with:
# - Both dash_sdk_* (Platform) and dash_spv_ffi_* (Core) symbols
# - Unified header with resolved type conflicts
# - Support for device and simulator architectures
```

#### Android
```bash
cargo build --target aarch64-linux-android --release
```

#### Other Platforms
Build for your target platform using the appropriate Rust target.

## Integration

### C/C++ Usage

```c
#include "dash_sdk_ffi.h"

// Initialize the SDK
dash_sdk_init();

// Create SDK configuration
DashSDKConfig config = {
    .network = DASH_SDK_NETWORK_TESTNET,
    .dapi_addresses = "seed-1.testnet.networks.dash.org",
    .request_retry_count = 3,
    .request_timeout_ms = 30000
};

// Create SDK instance
DashSDKResult result = dash_sdk_create(&config);
if (result.error) {
    // Handle error
    dash_sdk_error_free(result.error);
    return;
}

void* sdk = result.data;

// Use the SDK...

// Clean up
dash_sdk_destroy(sdk);
```

### Swift Usage Example

```swift
// Initialize the SDK
dash_sdk_init()

// Create SDK configuration
var config = DashSDKConfig(
    network: DashSDKNetwork.testnet,
    dapi_addresses: "seed-1.testnet.networks.dash.org".cString(using: .utf8),
    request_retry_count: 3,
    request_timeout_ms: 30000
)

// Create SDK instance
let result = dash_sdk_create(&config)
if let error = result.error {
    // Handle error
    dash_sdk_error_free(error)
    return
}

let sdk = result.data

// Use the SDK...

// Clean up
dash_sdk_destroy(sdk)
```

### Python Usage Example

```python
import ctypes
from ctypes import *

# Load the library
lib = cdll.LoadLibrary('./target/release/librs_sdk_ffi.so')

# Initialize
lib.dash_sdk_init()

# Create configuration
class DashSDKConfig(Structure):
    _fields_ = [
        ("network", c_int),
        ("dapi_addresses", c_char_p),
        ("request_retry_count", c_uint32),
        ("request_timeout_ms", c_uint64)
    ]

config = DashSDKConfig(
    network=1,  # Testnet
    dapi_addresses=b"seed-1.testnet.networks.dash.org",
    request_retry_count=3,
    request_timeout_ms=30000
)

# Create SDK instance
result = lib.dash_sdk_create(byref(config))
# ... handle result and use SDK
```

## API Reference

### Platform SDK Functions (Layer 2)

#### Core Functions
- `dash_sdk_init()` - Initialize the FFI library
- `dash_sdk_create()` - Create an SDK instance
- `dash_sdk_destroy()` - Destroy an SDK instance
- `dash_sdk_version()` - Get the SDK version

#### Identity Operations
- `dash_sdk_identity_fetch()` - Fetch an identity by ID
- `dash_sdk_identity_create()` - Create a new identity
- `dash_sdk_identity_topup()` - Top up identity with credits
- `dash_sdk_identity_register_name()` - Register a DPNS name

#### Document Operations
- `dash_sdk_document_create()` - Create a new document
- `dash_sdk_document_update()` - Update an existing document
- `dash_sdk_document_delete()` - Delete a document
- `dash_sdk_document_fetch()` - Fetch documents by query

#### Data Contract Operations
- `dash_sdk_data_contract_create()` - Create a new data contract
- `dash_sdk_data_contract_update()` - Update a data contract
- `dash_sdk_data_contract_fetch()` - Fetch a data contract

### Core SDK Functions (Layer 1)

#### SPV Client Operations
- `dash_spv_ffi_client_new()` - Create SPV client instance
- `dash_spv_ffi_client_start()` - Start SPV synchronization
- `dash_spv_ffi_client_stop()` - Stop SPV client
- `dash_spv_ffi_client_sync_to_tip()` - Sync blockchain to latest block

#### Wallet Operations
- `dash_spv_ffi_client_get_balance()` - Get wallet balance
- `dash_spv_ffi_client_watch_address()` - Watch address for transactions
- `dash_spv_ffi_client_broadcast_transaction()` - Broadcast transaction
- `dash_spv_ffi_client_get_transaction()` - Get transaction details

#### HD Wallet Functions
- `key_wallet_ffi_mnemonic_generate()` - Generate HD wallet mnemonic
- `key_wallet_ffi_derive_address()` - Derive addresses from HD wallet
- `key_wallet_ffi_sign_transaction()` - Sign transactions with HD keys

## Architecture

The FFI layer follows these principles:

1. **Opaque Handles**: Complex Rust types are exposed as opaque pointers
2. **C-Compatible Types**: All data crossing the FFI boundary uses C-compatible types
3. **Error Handling**: Functions return error codes with optional error messages
4. **Memory Management**: Clear ownership rules with dedicated free functions
5. **Cross-Platform**: Works on any platform that can interface with C

## Development

### Adding New Functions

1. Add the Rust implementation in the appropriate module
2. Ensure the function is marked with `#[no_mangle]` and `extern "C"`
3. Update cbindgen.toml if needed
4. Regenerate headers by running: `GENERATE_BINDINGS=1 cargo build --release`

### Testing

Run tests with:
```bash
cargo test
```

For platform-specific testing, create test applications on each target platform.

## License

MIT