# Dash SDK FFI

FFI bindings for integrating Dash Platform SDK with cross-platform applications.

## Overview

This crate provides C-compatible FFI bindings for the Dash Platform SDK (`rs-sdk`), enabling applications on any platform that supports C interfaces to interact with Dash Platform. This includes iOS (Swift), Android (JNI), Python (ctypes/cffi), Node.js (node-ffi), and more.

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

For standard builds:
```bash
cargo build --release
```

To generate C headers:
```bash
GENERATE_BINDINGS=1 cargo build --release
```

### Platform-Specific Builds

#### iOS
```bash
./build_ios.sh
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

### Core Functions

- `dash_sdk_init()` - Initialize the FFI library
- `dash_sdk_create()` - Create an SDK instance
- `dash_sdk_destroy()` - Destroy an SDK instance
- `dash_sdk_version()` - Get the SDK version

### Identity Operations

- `dash_sdk_identity_fetch()` - Fetch an identity by ID
- `dash_sdk_identity_create()` - Create a new identity
- `dash_sdk_identity_topup()` - Top up identity with credits
- `dash_sdk_identity_register_name()` - Register a DPNS name

### Document Operations

- `dash_sdk_document_create()` - Create a new document
- `dash_sdk_document_update()` - Update an existing document
- `dash_sdk_document_delete()` - Delete a document
- `dash_sdk_document_fetch()` - Fetch documents by query

### Data Contract Operations

- `dash_sdk_data_contract_create()` - Create a new data contract
- `dash_sdk_data_contract_update()` - Update a data contract
- `dash_sdk_data_contract_fetch()` - Fetch a data contract

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