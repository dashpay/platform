# Migration Guide: Separate SDKs to Unified SDK

This guide helps you migrate from using separate Core and Platform SDKs to the new Unified SDK architecture.

## Overview of Changes

### Before (Separate SDKs)
- **Core SDK**: `libdash_spv_ffi.a` + `libkey_wallet_ffi.a` (114MB)
- **Platform SDK**: `DashSDK.xcframework` (29MB)
- **Total Size**: 143MB
- **Symbol Conflicts**: Duplicate symbols between SDKs
- **Complex Integration**: Manual symbol resolution required

### After (Unified SDK)
- **Single Framework**: `DashUnifiedSDK.xcframework` (29.5MB)
- **No Conflicts**: All symbols unified
- **Simple Integration**: Drop-in replacement
- **79.4% Size Reduction**: Optimized single binary

## Migration Steps

### Step 1: Update Build Scripts

If you have custom build scripts, update them to use the unified build:

```bash
# Old approach (separate builds)
cd rust-dashcore/dash-spv-ffi
cargo build --release
cd ../../platform-ios/packages/rs-sdk-ffi
cargo build --release --no-default-features

# New approach (unified build)
cd platform-ios/packages/rs-sdk-ffi
./build_ios.sh
```

### Step 2: Update Xcode Project

#### Remove Old Frameworks
1. Remove from "Frameworks, Libraries, and Embedded Content":
   - `DashCore.xcframework`
   - `DashPlatform.xcframework`
   - `libdash_spv_ffi.a`
   - `libkey_wallet_ffi.a`

#### Add Unified Framework
1. Drag `DashUnifiedSDK.xcframework` into your project
2. Select "Embed & Sign" in the frameworks list
3. Update Framework Search Paths:
   ```
   $(PROJECT_DIR)/Libraries/DashUnifiedSDK.xcframework
   ```

### Step 3: Update project.yml (if using XcodeGen)

```yaml
# Old configuration
dependencies:
  - framework: Libraries/DashCore.xcframework
  - framework: Libraries/DashPlatform.xcframework
  - library: Libraries/libdash_spv_ffi.a
  - library: Libraries/libkey_wallet_ffi.a

# New configuration
dependencies:
  - package: SwiftDashCoreSDK
  - package: SwiftDashSDK
  # Framework is linked automatically through packages
```

### Step 4: Update Import Statements

No changes needed! The unified SDK maintains the same module names:

```swift
// These imports remain the same
import DashSDKFFI      // Platform functionality
import DashSPVFFI      // Core functionality
import SwiftDashCoreSDK
import SwiftDashSDK
```

### Step 5: Update Header References

If you have direct C/Objective-C imports:

```objc
// Old (separate headers)
#import "dash_spv_ffi.h"
#import "dash_sdk_ffi.h"

// New (unified header)
#import "dash_sdk_ffi.h"  // Now includes both sets of functions
```

### Step 6: Handle Type Changes

Some enum values were renamed to avoid conflicts:

```swift
// Core SDK enums (if using raw FFI)
// Old
FFINetwork.Testnet
FFINetwork.Devnet
FFIValidationMode.None

// New
FFINetwork.FFITestnet
FFINetwork.FFIDevnet
FFIValidationMode.NoValidation

// Platform SDK enums (unchanged)
DashSDKNetwork.Testnet  // Still works
DashSDKNetwork.Devnet   // Still works
```

## Troubleshooting

### Issue: "Module 'DashSPVFFI' not found"

**Solution**: Ensure the unified SDK's module map includes both modules:
```
module DashSDKFFI {
    header "dash_sdk_ffi.h"
    export *
}
```

### Issue: "Undefined symbol: _dash_spv_ffi_*"

**Solution**: Verify the unified SDK was built with Core integration:
```bash
# Check symbols in the library
nm DashUnifiedSDK.xcframework/ios-arm64/librs_sdk_ffi.a | grep dash_spv_ffi
```

### Issue: "Duplicate symbol" errors

**Solution**: You're likely still linking the old separate libraries. Remove all references to:
- `libdash_spv_ffi.a`
- `libkey_wallet_ffi.a`
- Old XCFrameworks

### Issue: Type mismatch errors

**Solution**: Update your code to use the new enum values:
```swift
// Update type references
let network: FFINetwork = .FFITestnet  // Note the FFI prefix
```

## Rollback Plan

If you need to temporarily rollback:

1. Keep the old frameworks in a backup directory
2. Revert your project.yml or Xcode project changes
3. Rebuild with the old separate SDK approach

However, we recommend completing the migration as the unified SDK provides significant benefits.

## Benefits After Migration

1. **Smaller App Size**: 79.4% reduction in SDK size
2. **Faster Build Times**: Single framework to link
3. **Better Performance**: Reduced memory usage
4. **Simpler Maintenance**: One SDK to update
5. **No Symbol Conflicts**: Unified symbol namespace

## Getting Help

If you encounter issues during migration:

1. Check the [UNIFIED_SDK_ARCHITECTURE.md](UNIFIED_SDK_ARCHITECTURE.md) for technical details
2. Review the example projects that use the unified SDK
3. File an issue with migration problems you encounter

## Version Compatibility

- Unified SDK v1.0+ requires:
  - SwiftDashCoreSDK v0.1.0+
  - SwiftDashSDK v0.1.0+
  - iOS 17.0+ deployment target