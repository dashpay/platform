# Build Guide for AI Assistants

This guide explains how to successfully build the SwiftExampleApp with integrated Core and Platform features.

## Overview

The SwiftExampleApp combines two layers:
- **Core Layer (Layer 1)**: SPV wallet functionality from dashpay-ios
- **Platform Layer (Layer 2)**: Identity and document management from platform-ios

## Prerequisites

1. **rust-dashcore** must be cloned at: `/Users/quantum/src/rust-dashcore`
2. **dash-spv-ffi** must be built first:
   ```bash
   cd /Users/quantum/src/rust-dashcore/dash-spv-ffi
   cargo build --release --target aarch64-apple-ios
   cargo build --release --target aarch64-apple-ios-sim
   ```

## Build Process

### 1. Build the Unified iOS Framework

```bash
cd /Users/quantum/src/platform-ios/packages/rs-sdk-ffi
./build_ios.sh
```

This script:
- Builds Rust code for iOS targets
- Generates C headers using cbindgen
- Merges SPV and SDK headers into a unified header
- Creates DashUnifiedSDK.xcframework

### 2. Build SwiftExampleApp

```bash
cd /Users/quantum/src/platform-ios/packages/swift-sdk
xcodebuild -project SwiftExampleApp/SwiftExampleApp.xcodeproj \
  -scheme SwiftExampleApp \
  -sdk iphonesimulator \
  -destination 'platform=iOS Simulator,name=iPhone 16,arch=arm64' \
  -quiet clean build
```

## Common Build Issues and Solutions

### Issue 1: Missing DashSDK.xcframework
**Error**: `DashSDK.xcframework: No such file or directory`
**Solution**: The framework is actually named DashUnifiedSDK.xcframework. Either:
- Update Package.swift to reference DashUnifiedSDK.xcframework, OR
- Create a symlink: `ln -s DashUnifiedSDK.xcframework DashSDK.xcframework`

### Issue 2: Type Visibility Errors
**Error**: `'DPPIdentity' is not public`
**Solution**: Edit the DPP types to make them public:
```swift
public struct DPPIdentity: Codable, Sendable {
    public let id: Identifier
    // ... make all properties public
}
```

### Issue 3: C Header Type Definition Errors
**Error**: `unknown type name 'CoreSDKClient'` or `field has incomplete type 'FFIClientConfig'`

**Root Cause**: The header merging process combines dash_spv_ffi.h with dash_sdk_ffi.h, but:
- FFIClientConfig is an opaque type (only forward declared)
- Type aliases like CoreSDKClient/CoreSDKConfig are not properly included

**Solutions**:
1. Use pointers for opaque types:
   ```rust
   pub struct UnifiedSDKConfig {
       pub core_config: *const FFIClientConfig,  // Use pointer, not value
   }
   ```

2. Use the actual type names instead of aliases:
   ```rust
   fn get_core_client(handle: *mut UnifiedSDKHandle) -> *mut FFIDashSpvClient {
       // Return FFIDashSpvClient, not CoreSDKClient
   }
   ```

3. For undefined types, use c_void pointers:
   ```rust
   fn get_core_handle(client: *mut FFIDashSpvClient) -> *mut std::ffi::c_void {
       // Return as c_void pointer instead of undefined type
   }
   ```

### Issue 4: Duplicate Code After Merge
**Error**: Duplicate imports or implementations
**Solution**: Check these files for duplicates:
- `packages/rs-sdk/src/mock/requests.rs` - duplicate TokenContractInfo imports
- `packages/rs-dapi-client/src/transport/grpc.rs` - duplicate GetTokenContractInfoRequest implementations

### Issue 5: Clean Build Required
After merging branches or fixing header issues, always do a clean build:
```bash
# Clean Rust artifacts
cd /Users/quantum/src/platform-ios/packages/rs-sdk-ffi
cargo clean

# Rebuild
./build_ios.sh

# Clean Xcode build
cd /Users/quantum/src/platform-ios/packages/swift-sdk
xcodebuild -project SwiftExampleApp/SwiftExampleApp.xcodeproj -scheme SwiftExampleApp clean
```

## Architecture Notes

### Unified FFI Design
The rs-sdk-ffi creates a unified SDK that includes both Core and Platform functionality:
- Core SDK functions are prefixed with `dash_core_sdk_*`
- Platform SDK functions are prefixed with `dash_sdk_*`
- Unified SDK functions are prefixed with `dash_unified_sdk_*`

### Header Merging
The build_ios.sh script merges headers to create a unified interface:
1. Extracts SPV FFI content from dash_spv_ffi.h
2. Removes conflicting definitions (like duplicate CoreSDKHandle)
3. Renames conflicting enum values (None -> NoValidation, etc.)
4. Combines with generated dash_sdk_ffi.h

### State Management
SwiftExampleApp uses a unified state management approach:
- `UnifiedAppState` coordinates both Core and Platform features
- `WalletService` manages Core SDK operations
- `PlatformService` handles Platform SDK operations
- SwiftData models persist wallet data locally

## Testing the Build

After successful build, verify:
1. App bundle exists: `/Users/quantum/Library/Developer/Xcode/DerivedData/SwiftExampleApp-*/Build/Products/Debug-iphonesimulator/SwiftExampleApp.app`
2. Framework is properly linked in the app bundle
3. No runtime crashes when launching the app

## Important Files to Check

When debugging build issues, check these files:
- `/Users/quantum/src/platform-ios/packages/rs-sdk-ffi/build_ios.sh` - Build script
- `/Users/quantum/src/platform-ios/packages/rs-sdk-ffi/src/core_sdk.rs` - Core SDK bindings
- `/Users/quantum/src/platform-ios/packages/rs-sdk-ffi/src/unified.rs` - Unified SDK coordination
- `/Users/quantum/src/platform-ios/packages/rs-sdk-ffi/build/dash_sdk_ffi.h` - Generated header
- `/Users/quantum/src/platform-ios/packages/swift-sdk/Package.swift` - Swift package configuration