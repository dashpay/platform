# iOS SDK FFI

FFI bindings for integrating Dash Platform SDK with iOS applications.

## Overview

This crate provides C-compatible FFI bindings for the Dash Platform SDK (`rs-sdk`), enabling iOS applications to interact with Dash Platform through Swift.

## Building

### Prerequisites

- Rust toolchain with iOS targets:
  ```bash
  rustup target add aarch64-apple-ios
  rustup target add aarch64-apple-ios-sim
  rustup target add x86_64-apple-ios
  ```

- Xcode and command line tools
- cbindgen (for header generation): `cargo install cbindgen`

### Build Instructions

1. Run the build script:
   ```bash
   ./build_ios.sh
   ```

2. The script will:
   - Build static libraries for all iOS targets
   - Generate C headers using cbindgen
   - Create an XCFramework at `build/DashSDK.xcframework`

### Manual Build

To build for a specific target:
```bash
cargo build --target aarch64-apple-ios --release
```

To generate headers:
```bash
GENERATE_BINDINGS=1 cargo build --release
```

## Integration

### Xcode Project

1. Drag `DashSDK.xcframework` into your Xcode project
2. Make sure it's added to your target's frameworks
3. Import the module in Swift:
   ```swift
   import DashSDKFFI
   ```

### Swift Usage Example

```swift
// Initialize the SDK
ios_sdk_init()

// Create SDK configuration
var config = IOSSDKConfig(
    network: IOSSDKNetwork.testnet,
    wallet_mnemonic: "your mnemonic here".cString(using: .utf8),
    wallet_passphrase: nil,
    skip_asset_lock_proof_verification: false,
    request_retry_count: 3,
    request_timeout_ms: 30000
)

// Create SDK instance
let result = ios_sdk_create(&config)
if let error = result.error {
    // Handle error
    ios_sdk_error_free(error)
    return
}

let sdk = result.data

// Use the SDK...

// Clean up
ios_sdk_destroy(sdk)
```

## API Reference

### Core Functions

- `ios_sdk_init()` - Initialize the FFI library
- `ios_sdk_create()` - Create an SDK instance
- `ios_sdk_destroy()` - Destroy an SDK instance

### Identity Operations

- `ios_sdk_identity_fetch()` - Fetch an identity by ID
- `ios_sdk_identity_create()` - Create a new identity
- `ios_sdk_identity_topup()` - Top up identity with credits
- `ios_sdk_identity_register_name()` - Register a DPNS name

### Wallet Operations

- `ios_sdk_get_wallet_address()` - Get wallet address
- `ios_sdk_get_wallet_balance()` - Get wallet balance
- `ios_sdk_refresh_wallet()` - Sync wallet with network

## Architecture

The FFI layer follows these principles:

1. **Opaque Handles**: Complex Rust types are exposed as opaque pointers
2. **C-Compatible Types**: All data crossing the FFI boundary uses C-compatible types
3. **Error Handling**: Functions return error codes with optional error messages
4. **Memory Management**: Clear ownership rules with dedicated free functions

## Development

### Adding New Functions

1. Add the Rust implementation in the appropriate module
2. Ensure the function is marked with `#[no_mangle]` and `extern "C"`
3. Update cbindgen.toml if needed
4. Regenerate headers by running the build script

### Testing

Run tests with:
```bash
cargo test
```

For iOS-specific testing, create a test iOS app that links against the framework.

## License

MIT