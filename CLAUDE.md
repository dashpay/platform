# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 📖 Essential Reading

**IMPORTANT**: Before building any packages or working with this repository, you MUST read the [JavaScript-Developer-Guide.md](JavaScript-Developer-Guide.md) first. This comprehensive guide contains:

- **Critical build procedures** and workspace setup
- **Yarn workspace architecture** and dependency management
- **JavaScript-only build optimization** (much faster than full platform builds)
- **Package-specific development workflows**
- **Troubleshooting and debugging strategies**
- **Example script creation guidelines**

The JavaScript Developer Guide provides the foundation for understanding how to work efficiently with this monorepo's 21 packages and complex dependency chains.

<<<<<<< HEAD
## IMPORTANT: Tool Usage Rules

**ALWAYS use the swift-rust-ffi-engineer agent for:**
- Any Swift/Rust FFI integration work
- Swift wrapper implementations over FFI functions
- Debugging Swift/FFI type compatibility issues
- iOS SDK and SwiftExampleApp development
- Memory management across Swift/Rust boundaries
- Refactoring Swift code to properly wrap FFI functions

## Commands
=======
## Quick Command Reference
>>>>>>> 12c4d0494 (refactor(docs): streamline CLAUDE.md to concise quick reference)

### Essential Commands

**For comprehensive build procedures and optimization strategies**, see [JavaScript-Developer-Guide.md](JavaScript-Developer-Guide.md#5-build-and-packaging).

```bash
# Quick setup
yarn setup                    # Full setup (15-20 min)
yarn install && ultra --recursive --filter "packages/@(js-dash-sdk|wallet-lib|js-dapi-client|wasm-dpp|js-grpc-common|dapi-grpc)" --build  # JS-only (3-5 min)

# Development environment  
yarn start                    # Start local environment
yarn stop                     # Stop environment
yarn build                    # Rebuild all packages
yarn reset                    # Nuclear reset
```

### Testing

**For comprehensive testing strategies**, see [JavaScript-Developer-Guide.md#4-testing-strategy](JavaScript-Developer-Guide.md#4-testing-strategy).

```bash
# Essential test commands
yarn test                     # All tests (requires yarn start)
yarn test:sdk                 # JavaScript SDK tests
yarn test:wallet-lib          # Wallet library tests
yarn workspace <pkg> test     # Specific package tests
```

### Rust Development

```bash
# Run Rust tests for a specific package
cargo test -p <package_name>

# Run all Rust tests
cargo test --workspace

# Check Rust code
cargo check --workspace

# Run clippy linter
cargo clippy --workspace

# Format Rust code
cargo fmt --all
```

### Utility Commands

```bash
yarn lint                     # Run linters
yarn dashmate                 # Node management CLI
yarn configure:tests:network  # Configure test network
```

## Architecture

### Technology Stack

- **Rust**: Core platform components (Drive, DAPI server, DPP implementation)
- **JavaScript/TypeScript**: Client SDKs, developer tools, test suite
- **WebAssembly**: Bridge between Rust and JavaScript implementations
- **gRPC**: Service communication protocol
- **Docker**: Local development environment

### Key Components

**Drive** (`packages/rs-drive`): Platform's decentralized storage component, implementing a replicated state machine for storing and retrieving application data.

**DAPI** (`packages/dapi`): Decentralized API server that provides a unified interface for interacting with the Dash network and Platform.

**DPP** (`packages/rs-dpp`, `packages/wasm-dpp`): Dash Platform Protocol implementation that defines data structures and validation rules.

**SDK** (`packages/js-dash-sdk`, `packages/rs-sdk`): Client libraries providing high-level interfaces for building applications on Dash Platform.

**WASM SDK** (`packages/wasm-sdk`): WebAssembly bindings for browser-based applications. See [AI_REFERENCE.md](packages/wasm-sdk/AI_REFERENCE.md) for comprehensive API documentation.

**Dashmate** (`packages/dashmate`): Node management tool for setting up and managing Dash Platform nodes.

### Data Contracts

Platform uses data contracts to define application data schemas:
- `dpns-contract`: Dash Platform Naming Service
- `dashpay-contract`: Social payments functionality
- `feature-flags-contract`: System feature toggles
- `masternode-reward-shares-contract`: Masternode reward distribution
- `withdrawals-contract`: Platform credit withdrawals

### Development Workflow

**For step-by-step workflows and best practices**, see [JavaScript-Developer-Guide.md#3-development-workflow-best-practices](JavaScript-Developer-Guide.md#3-development-workflow-best-practices).

**Key concepts:**
- **Yarn workspaces**: 21 packages managed as a unified dependency graph
- **WASM bridge**: Rust ↔ JavaScript integration via WebAssembly
- **Docker environment**: Local development via dashmate
- **Dependency order**: Build lower-level packages (wasm-dpp) before higher-level (js-dash-sdk)

## Quick Reference

### Package Build Times (JavaScript development)
- **Full platform**: `yarn setup` (15-20 minutes, all 21 packages)  
- **JavaScript core**: `yarn install && ultra --filter "packages/@(js-dash-sdk|wallet-lib|js-dapi-client|wasm-dpp|js-grpc-common|dapi-grpc)" --build` (3-5 minutes)
- **Individual WASM**: `cd packages/wasm-dpp && yarn build` (~1-2 minutes)

### Key Architecture Patterns
- **State Transitions**: All document/contract updates use state transitions
- **WASM Integration**: Rust core → WASM bindings → JavaScript SDKs  
- **Proof Verification**: All operations include cryptographic proofs
- **Workspace Dependencies**: Changes cascade through dependency chain
- **Platform Versioning**: Uses `rs-platform-version` for protocol versioning
- **Serialization**: Custom serialization with `rs-platform-serialization`
- **Value Handling**: `rs-platform-value` for cross-language data representation

## iOS Development

### Building iOS SDK and SwiftExampleApp

See [packages/swift-sdk/BUILD_GUIDE_FOR_AI.md](packages/swift-sdk/BUILD_GUIDE_FOR_AI.md) for detailed instructions on building the iOS components.

For SwiftExampleApp-specific guidance including token querying and data models, see [packages/swift-sdk/SwiftExampleApp/CLAUDE.md](packages/swift-sdk/SwiftExampleApp/CLAUDE.md).

Quick build commands:
```bash
# Build unified iOS framework (includes Core + Platform)
cd packages/rs-sdk-ffi
./build_ios.sh

# Build SwiftExampleApp
cd packages/swift-sdk
xcodebuild -project SwiftExampleApp/SwiftExampleApp.xcodeproj \
  -scheme SwiftExampleApp \
  -sdk iphonesimulator \
  -destination 'platform=iOS Simulator,name=iPhone 16,arch=arm64' \
  -quiet clean build
```

### iOS Architecture

**Unified SDK**: The iOS SDK combines both Core (SPV wallet) and Platform (identity/documents) functionality:
- Core SDK functions: `dash_core_sdk_*` prefix
- Platform SDK functions: `dash_sdk_*` prefix  
- Unified SDK functions: `dash_unified_sdk_*` prefix

**SwiftExampleApp**: Demonstrates integration of both layers:
- Uses SwiftUI for UI and SwiftData for persistence
- `UnifiedAppState` coordinates Core and Platform features
- `WalletService` manages SPV wallet operations
- `PlatformService` handles identity and document operations

**Common iOS Build Issues**:
- Missing xcframework: Create symlink or update Package.swift
- Type visibility: Make DPP types public in Swift
- C header issues: Use pointers for opaque FFI types
- After merges: Always clean and rebuild from scratch
