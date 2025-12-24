# iOS Build Troubleshooting Guide

## Common Build Issues and Solutions

### 1. "Could not build Objective-C module 'DashSDKFFI'" Error

This error occurs when the FFI header file is missing or not properly linked.

**Solution:**
```bash
# Run the setup script from the swift-sdk directory
cd packages/swift-sdk
./setup_ios_build.sh
```

### 2. Manual Setup Steps (if the script fails)

#### Step 1: Build the Rust FFI
```bash
cd packages/rs-sdk-ffi
./build_ios.sh
```

#### Step 2: Create the header symlink
```bash
cd packages/swift-sdk
mkdir -p Sources/CDashSDKFFI

# Create symlink to the FFI header
ln -sf ../../rs-sdk-ffi/build/DashUnifiedSDK.xcframework/ios-arm64/Headers/dash_sdk_ffi.h Sources/CDashSDKFFI/dash_sdk_ffi.h
```

#### Step 3: Clean and rebuild
```bash
cd SwiftExampleApp
rm -rf DerivedData
xcodebuild clean -project SwiftExampleApp.xcodeproj -scheme SwiftExampleApp

# Build
xcodebuild -project SwiftExampleApp.xcodeproj \
  -scheme SwiftExampleApp \
  -sdk iphonesimulator \
  -destination 'platform=iOS Simulator,name=iPhone 16' \
  build
```

### 3. Enum Redefinition Errors

If you see errors like "redefinition of enumerator 'Regtest'", this means there are conflicting enum definitions in the FFI headers.

**Solution:**
Make sure you have the latest changes from the feat/ios-2 branch:
```bash
git fetch origin
git checkout feat/ios-2
git pull origin feat/ios-2
```

Then rebuild the FFI:
```bash
cd packages/rs-sdk-ffi
./build_ios.sh
```

### 4. Missing Dependencies

If the Rust build fails, ensure you have the required iOS targets:
```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```

### 5. Architecture Mismatch

If you're on an Apple Silicon Mac and see architecture-related errors:
```bash
# Use the arm64 architecture explicitly
xcodebuild -project SwiftExampleApp.xcodeproj \
  -scheme SwiftExampleApp \
  -sdk iphonesimulator \
  -destination 'platform=iOS Simulator,name=iPhone 16,arch=arm64' \
  build
```

## Verification Steps

1. **Check FFI build output:**
   ```bash
   ls -la packages/rs-sdk-ffi/build/DashUnifiedSDK.xcframework
   ```

2. **Check header symlink:**
   ```bash
   ls -la packages/swift-sdk/Sources/CDashSDKFFI/dash_sdk_ffi.h
   ```

3. **Verify header content:**
   ```bash
   # Should show the unified FFI header with both Core and Platform functions
   head -50 packages/swift-sdk/Sources/CDashSDKFFI/dash_sdk_ffi.h
   ```

## Clean Build

For a completely clean build:
```bash
# Clean all build artifacts
cd packages/rs-sdk-ffi
rm -rf build/

cd ../swift-sdk
rm -rf SwiftExampleApp/DerivedData
rm -rf ~/Library/Developer/Xcode/DerivedData/SwiftExampleApp-*

# Then run setup
./setup_ios_build.sh
```