# Unified SDK Architecture

## Overview

The Unified SDK combines Dash Core (Layer 1) and Dash Platform (Layer 2) functionality into a single binary, eliminating duplicate symbols and reducing binary size by 79.4% (from 143MB to 29.5MB).

## Why Unified SDK?

### Previous Architecture Problems
- **Duplicate Symbols**: Both Core and Platform SDKs included common dependencies (libsecp256k1, libc++, etc.)
- **Binary Bloat**: Combined size of separate SDKs was 143MB
- **Complex Integration**: Apps needed to manage multiple frameworks and resolve conflicts
- **Maintenance Overhead**: Separate build processes for each SDK

### Unified SDK Benefits
- **Single Binary**: One 29.5MB XCFramework contains all functionality
- **No Symbol Conflicts**: Shared dependencies included only once
- **Flexible Usage**: Apps can use Core-only, Platform-only, or both
- **Simplified Build**: One build process for all functionality
- **Better Performance**: Reduced memory footprint and faster load times

## Architecture Design

### Component Structure
```
DashUnifiedSDK.xcframework/
├── ios-arm64/
│   ├── librs_sdk_ffi.a         # Device binary
│   └── Headers/
│       ├── dash_sdk_ffi.h      # Unified header
│       └── module.modulemap
└── ios-arm64-simulator/
    ├── librs_sdk_ffi.a         # Simulator binary
    └── Headers/
        ├── dash_sdk_ffi.h      # Unified header
        └── module.modulemap
```

### Symbol Export Strategy
The unified SDK exports symbols from both layers:
- **Core Layer**: `dash_spv_ffi_*` functions for SPV wallet functionality
- **Platform Layer**: `dash_sdk_*` functions for identity and document management
- **Shared Types**: Carefully managed to avoid conflicts

## Header Merging Process

### Challenge
Both SDKs define similar types (Network, ValidationMode, etc.) causing conflicts when merged.

### Solution
The build script (`build_ios.sh`) implements intelligent header merging:

1. **Extract Core Types**: Parse `dash_spv_ffi.h` from rust-dashcore
2. **Rename Conflicts**: Transform conflicting enum values:
   - `None` → `NoValidation` (validation modes)
   - `Testnet` → `FFITestnet` (network types)
   - `Devnet` → `FFIDevnet` (network types)
3. **Remove Duplicates**: Filter out duplicate struct definitions
4. **Merge Headers**: Combine processed headers into unified output

### Example Type Resolution
```c
// Original Core SDK
typedef enum FFINetwork {
  Mainnet = 0,
  Testnet = 1,  // Conflict!
  Regtest = 2,
  Devnet = 3,   // Conflict!
} FFINetwork;

// Platform SDK
typedef enum DashSDKNetwork {
  Dash = 0,
  Testnet = 1,    // Conflict!
  Regtest = 2,  
  Devnet = 3,     // Conflict!
} DashSDKNetwork;

// Unified SDK Resolution
typedef enum FFINetwork {
  Dash = 0,
  FFITestnet = 1,   // Renamed
  Regtest = 2,
  FFIDevnet = 3,    // Renamed
} FFINetwork;

typedef enum DashSDKNetwork {
  Dash = 0,
  Testnet = 1,      // Original name
  Regtest = 2,
  Devnet = 3,       // Original name
} DashSDKNetwork;
```

## Build Process

### Prerequisites
- Rust toolchain with iOS targets
- cbindgen for header generation
- Xcode command line tools

### Build Command
```bash
cd packages/rs-sdk-ffi
./build_ios.sh [arm|x86|universal]
```

### Build Steps
1. **Compile Rust**: Build for iOS device and simulator targets
2. **Generate Headers**: Use cbindgen with iOS-specific configuration
3. **Merge Headers**: Combine Core and Platform headers
4. **Create XCFramework**: Package libraries with headers

## Integration Guide

### Swift Package Manager
```swift
.binaryTarget(
    name: "DashSDKFFI",
    path: "path/to/DashUnifiedSDK.xcframework"
)
```

### Direct Xcode Integration
1. Drag `DashUnifiedSDK.xcframework` into project
2. Ensure "Embed & Sign" is selected
3. Import modules as needed:
   ```swift
   import DashSDKFFI      // Platform functionality
   import DashSPVFFI      // Core functionality
   ```

## Type Compatibility

### Network Types
- Use `DashSDKNetwork` for Platform operations
- Use `FFINetwork` for Core operations
- Types are not interchangeable despite similar values

### Validation Modes
- Platform uses standard enum values
- Core uses renamed values (`NoValidation` instead of `None`)

### Handle Types
- `CoreSDKHandle` provides bridge between layers
- `FFIDashSpvClient` used internally by both SDKs

## Migration from Separate SDKs

### Before (Separate SDKs)
```yaml
dependencies:
  - framework: DashCore.xcframework
  - framework: DashPlatform.xcframework
# Total size: 143MB
```

### After (Unified SDK)
```yaml
dependencies:
  - framework: DashUnifiedSDK.xcframework
# Total size: 29.5MB
```

### Code Changes
No code changes required! The unified SDK maintains API compatibility with both original SDKs.

## Troubleshooting

### Common Issues

1. **Type Conflicts**
   - Symptom: "redefinition of enum" errors
   - Solution: Ensure using latest unified header with resolved conflicts

2. **Missing Symbols**
   - Symptom: Undefined symbol errors for `dash_spv_ffi_*`
   - Solution: Verify unified SDK was built with Core integration enabled

3. **Module Not Found**
   - Symptom: "No such module 'DashSPVFFI'"
   - Solution: Check module.modulemap includes both modules

## Technical Details

### Cargo Configuration
The unified SDK always includes Core dependencies:
```toml
[dependencies]
dash-sdk = { path = "../rs-sdk" }
dash-spv-ffi = { path = "../../../rust-dashcore/dash-spv-ffi" }
```

### CBind Configuration
Separate configurations for optimal code generation:
- `cbindgen.toml`: Base configuration
- `cbindgen-ios.toml`: iOS-specific type mappings
- `cbindgen-core.toml`: Core function exports

### Size Optimization
Profile-guided optimizations in release builds:
```toml
[profile.release]
lto = "fat"
codegen-units = 1
opt-level = "z"
strip = true
```

## Future Enhancements

1. **Dynamic Framework Support**: Option to build as .framework instead of static library
2. **Module Maps**: Separate module maps for Core and Platform
3. **Automated Testing**: CI/CD pipeline for unified SDK builds
4. **Version Management**: Coordinated versioning between Core and Platform

## Conclusion

The Unified SDK represents a significant architectural improvement, providing a cleaner, smaller, and more maintainable solution for iOS applications using Dash. By carefully managing type conflicts and maintaining API compatibility, we've created a drop-in replacement that "just works" while providing substantial benefits.