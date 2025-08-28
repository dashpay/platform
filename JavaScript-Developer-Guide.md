# JavaScript Developer's Guide to Dash Platform Repository

## Table of Contents
1. [Repository Structure Analysis](#1-repository-structure-analysis)
2. [Understanding Yarn Workspaces](#2-understanding-yarn-workspaces)
3. [Development Workflow Best Practices](#3-development-workflow-best-practices)
4. [Testing Strategy](#4-testing-strategy)
5. [Build and Packaging](#5-build-and-packaging)
6. [Debugging and Troubleshooting](#6-debugging-and-troubleshooting)
7. [Contribution Guidelines](#7-contribution-guidelines)
8. [Creating Example Scripts](#8-creating-example-scripts)

---

## 1. Repository Structure Analysis

### 1.1 Monorepo Architecture

The Dash Platform uses Yarn workspaces to manage a complex monorepo with both JavaScript and Rust packages. The JavaScript packages are organized as follows:

```
packages/
├── js-dash-sdk/          # Main JavaScript SDK (depends on most JS packages)
├── wallet-lib/           # Core wallet functionality
├── js-dapi-client/       # DAPI client library
├── wasm-dpp/             # WebAssembly DPP bindings
├── wasm-sdk/             # WebAssembly SDK for browsers
├── js-grpc-common/       # gRPC utilities
├── dapi-grpc/            # gRPC protocol definitions
├── dapi/                 # DAPI server implementation
└── *-contract/           # Various data contract packages
```

### 1.2 Package Dependencies and Build Chain

The dependency chain flows as follows (higher packages depend on lower ones):

```
┌─────────────────┐
│   js-dash-sdk   │ (Main entry point)
└─────┬───────────┘
      │
      ├── wallet-lib
      ├── js-dapi-client
      ├── wasm-dpp
      ├── js-grpc-common
      ├── dapi-grpc
      └── various contracts
```

**Critical Insight**: Changes to lower-level packages (like `wasm-dpp`, `js-dapi-client`, `wallet-lib`) will affect packages that depend on them. Always test the entire dependency chain when making changes.

### 1.3 WASM Bridge Architecture

The repository uses WebAssembly to bridge Rust and JavaScript code:

- **Rust Packages** (`rs-*`): Core platform logic written in Rust
- **WASM Packages** (`wasm-*`): WebAssembly bindings for JavaScript consumption
- **JS Packages** (`js-*`): JavaScript wrappers and higher-level APIs

**Build Process Flow**:
```
Rust Code → WASM Compilation → JavaScript Bindings → Package Distribution
```

---

## 2. Understanding Yarn Workspaces

### 2.1 What Are Yarn Workspaces?

Think of yarn workspaces as **"rooms in a house that share utilities"**. Instead of each room having its own separate plumbing, electricity, and internet connection, they all share these resources efficiently from a central source.

In the Dash Platform repository:
- Each **"room"** is a package (wallet-lib, js-dash-sdk, etc.)
- The **"shared utilities"** are dependencies (lodash, crypto libraries, etc.)
- The **"house"** is the root directory that manages everything

**Traditional approach (without workspaces)**:
```
project-root/
├── package-a/
│   ├── node_modules/ (5000+ files)
│   └── package.json
├── package-b/
│   ├── node_modules/ (5000+ files) ← DUPLICATION!
│   └── package.json
└── package-c/
    ├── node_modules/ (5000+ files) ← MORE DUPLICATION!
    └── package.json
```

**Yarn workspaces approach**:
```
project-root/
├── node_modules/ (shared dependencies)
├── packages/
│   ├── package-a/ (no local node_modules)
│   ├── package-b/ (no local node_modules)
│   └── package-c/ (no local node_modules)
└── package.json (workspace configuration)
```

### 2.2 Why Workspaces Are Essential for Dash Platform

The Dash Platform repository has **21 different packages** that depend on each other:

```
Current Workspaces in Dash Platform:
├── @dashevo/platform (root)
├── @dashevo/js-dash-sdk (main SDK)
├── @dashevo/wallet-lib
├── @dashevo/dapi-client  
├── @dashevo/wasm-dpp
├── @dashevo/grpc-common
├── ... 16 more packages
```

**Without workspaces**, you'd face these problems:
- **Massive duplication**: Each package would have its own copy of shared dependencies
- **Version conflicts**: Package A uses lodash v4.17.20, Package B uses v4.17.21
- **Slow development**: Changes to wallet-lib wouldn't immediately affect js-dash-sdk
- **Complex testing**: You'd have to manually rebuild and reinstall packages to test changes

**With workspaces**, you get:
- **Single source of truth**: One `node_modules` folder for the entire project
- **Automatic linking**: Changes to wallet-lib are instantly available to js-dash-sdk
- **Unified dependency management**: All packages share compatible versions
- **Fast iteration**: No need to reinstall when making local changes

### 2.3 How Workspace Dependencies Work

Look at this real example from `packages/wallet-lib/package.json`:

```json
{
  "dependencies": {
    "@dashevo/dapi-client": "workspace:*",
    "@dashevo/grpc-common": "workspace:*", 
    "@dashevo/wasm-dpp": "workspace:*",
    "@dashevo/dashcore-lib": "~0.22.0",
    "lodash": "^4.17.21"
  }
}
```

**Key insights**:
- `workspace:*` means "use the local version from this repository"
- `~0.22.0` means "use this specific version from npm"
- When you make changes to `@dashevo/dapi-client`, `wallet-lib` sees them immediately
- No need to publish or reinstall - the packages are **symlinked**

### 2.4 The Dependency Chain in Action

Here's how workspace linking enables the complex dependency relationships:

```
js-dash-sdk (depends on)
├── wallet-lib (depends on)  
│   ├── dapi-client (depends on)
│   │   └── grpc-common
│   └── wasm-dpp
├── dapi-client (also directly)
└── wasm-dpp (also directly)
```

**What happens when you change `grpc-common`**:
1. Your changes are immediately available to `dapi-client` (workspace link)
2. `dapi-client` sees the new `grpc-common` automatically
3. `wallet-lib` sees the updated `dapi-client` automatically  
4. `js-dash-sdk` sees all the updates cascading through

**Without workspaces**, you'd need to:
1. Change `grpc-common` and publish it to npm
2. Update `dapi-client` to use the new version and publish it
3. Update `wallet-lib` to use the new `dapi-client` and publish it
4. Finally update `js-dash-sdk`

### 2.5 Essential Workspace Commands

#### 2.5.1 Working with Individual Packages

```bash
# Run a command in a specific workspace
yarn workspace @dashevo/wallet-lib test

# Install a dependency in a specific workspace  
yarn workspace @dashevo/wallet-lib add lodash

# Build a specific workspace
yarn workspace @dashevo/wallet-lib build:web
```

#### 2.5.2 Working with Multiple Packages

```bash
# List all workspaces
yarn workspaces list

# Run a command in ALL workspaces
yarn workspaces foreach run build

# Run tests in specific groups (defined in root package.json)
yarn test:wallet-lib  # Tests wallet-lib + all packages that depend on it
```

#### 2.5.3 Dependency Management

```bash
# See why a package is installed (useful for debugging)
yarn why @dashevo/wallet-lib

# Check for dependency issues across workspaces
yarn install

# Focus on specific workspace dependencies only (for faster installs)
yarn workspaces focus @dashevo/wallet-lib
```

### 2.6 Common Workspace Gotchas and Solutions

#### 2.6.1 "Cannot find module" Errors

**Symptom**: `Cannot find module '@dashevo/wallet-lib'`

**Cause**: Workspace linking hasn't happened or is broken

**Solution**:
```bash
# From repository root
yarn install  # Recreates workspace links
```

#### 2.6.2 Stale Dependencies

**Symptom**: Changes to package A don't appear in package B

**Cause**: Build artifacts might be cached

**Solution**:
```bash
yarn workspace <package-name> build  # Rebuild the changed package
# or
yarn build  # Rebuild everything
```

#### 2.6.3 Version Mismatches

**Symptom**: Different packages using incompatible versions

**Cause**: Workspace resolution conflicts

**Solution**: Use the `resolutions` field in root `package.json`:
```json
{
  "resolutions": {
    "lodash": "^4.17.21"  // Forces all packages to use this version
  }
}
```

### 2.7 Workspace Benefits for Development

#### 2.7.1 Instant Feedback Loop

**Traditional monorepo** (without workspaces):
```bash
# Change wallet-lib
vim packages/wallet-lib/src/wallet.js

# Build and publish wallet-lib
cd packages/wallet-lib
npm run build
npm publish

# Update js-dash-sdk to use new version
cd ../js-dash-sdk  
npm install @dashevo/wallet-lib@latest
npm run build
npm test
```

**With workspaces**:
```bash
# Change wallet-lib
vim packages/wallet-lib/src/wallet.js

# Test immediately (js-dash-sdk sees changes automatically)
yarn test:sdk
```

#### 2.7.2 Unified Tooling

All packages share the same tooling configuration:
- **ESLint config**: Defined once at the root
- **Build tools**: Shared webpack configurations
- **Test runners**: Consistent test setup across packages

#### 2.7.3 Simplified CI/CD

```bash
# Install all dependencies for all packages in one command
yarn install

# Build all packages in correct dependency order
yarn build

# Test everything
yarn test
```

### 2.8 Best Practices for Workspace Development

#### 2.8.1 Always Work from Repository Root

```bash
# ✅ Good - run commands from project root
yarn workspace @dashevo/wallet-lib test

# ❌ Avoid - running commands from inside package directories
cd packages/wallet-lib && yarn test  # May miss workspace context
```

#### 2.8.2 Use Workspace-Aware Test Commands

```bash
# ✅ Good - tests package + all dependents
yarn test:wallet-lib  

# ❌ Less useful - only tests wallet-lib in isolation  
yarn workspace @dashevo/wallet-lib test
```

#### 2.8.3 Understand the Build Order

Packages must be built in dependency order:
1. **Base packages**: `wasm-dpp`, `grpc-common`
2. **Client packages**: `dapi-client`
3. **Higher-level**: `wallet-lib`
4. **Main SDK**: `js-dash-sdk`

The monorepo handles this automatically, but when debugging:
```bash
yarn workspaces list  # See all packages
yarn workspaces foreach --topological run build  # Build in dependency order
```

---

## 3. Development Workflow Best Practices

### 3.1 Initial Setup

Always start with the complete setup process:

```bash
# Clone and setup the repository
git clone <repository-url>
cd platform
yarn setup  # Installs deps, builds, and configures everything
```

**Important**: The `yarn setup` command builds all 21 packages, which takes 15-20 minutes. **For JavaScript development only**, use the optimized setup:

```bash
# JavaScript-optimized setup (much faster)
yarn install  # Creates workspace links
ultra --recursive --filter "packages/@(js-dash-sdk|wallet-lib|js-dapi-client|wasm-dpp|js-grpc-common|dapi-grpc)" --build
```

**Full setup** (if you need the complete platform):
- Installs all workspace dependencies in one pass
- Creates workspace symlinks between packages  
- Builds all dependencies in the correct dependency order

### 3.2 Starting the Development Environment

For most JavaScript development work:

```bash
# Start the local development environment (required for many tests)
yarn start

# In another terminal, you can now run tests and make changes
```

### 3.3 Package-Specific Development Workflows

#### 3.3.1 Working on `wallet-lib`

**Scenario**: You want to add a new method to the wallet functionality.

**Step-by-step workflow**:

1. **Make your changes** to `packages/wallet-lib/src/`

2. **Test your changes locally**:
```bash
# Test only wallet-lib
yarn workspace @dashevo/wallet-lib test:unit

# Test wallet-lib and all packages that depend on it
yarn test:wallet-lib
```

3. **Verify dependent packages** still work:
```bash
# Test the main SDK (which depends on wallet-lib)
yarn test:sdk

# Or test everything
yarn test
```

4. **Build for distribution** (if needed):
```bash
yarn workspace @dashevo/wallet-lib build:web
```

**⚡ Performance Tip**: If you're only working on JavaScript packages, use the optimized build commands from [Section 5.5](#55-javascript-only-build-optimization) to avoid building unnecessary infrastructure packages.

#### 3.3.2 Working on `wasm-dpp` (WASM Package)

**Scenario**: You need to expose new Rust functionality through WASM.

**Step-by-step workflow**:

1. **Understand the dual build process**:
   - Rust code must be compiled to WASM
   - WASM must be wrapped with JavaScript bindings
   - TypeScript definitions must be generated

2. **Make changes** to `packages/wasm-dpp/src/` (Rust code)

3. **Build the WASM**:
```bash
cd packages/wasm-dpp
yarn build:wasm  # Compiles Rust → WASM
yarn build:js    # Generates JS bindings and TypeScript definitions
# Or simply:
yarn build       # Does both steps
```

4. **Test your changes**:
```bash
yarn test:node     # Node.js tests
yarn test:browsers # Browser tests
yarn test:types    # TypeScript type checking
```

5. **Test dependent packages**:
```bash
# From repository root
yarn test:dpp  # Tests all DPP-related packages
```

#### 3.3.3 Working on `js-dapi-client`

**Scenario**: You want to add a new DAPI method or fix a client issue.

**Step-by-step workflow**:

1. **Make your changes** to `packages/js-dapi-client/lib/`

2. **Test with high coverage requirements**:
```bash
cd packages/js-dapi-client
yarn test:coverage  # Has strict coverage requirements (98%+ in most areas)
```

3. **Test browser compatibility**:
```bash
yarn test:browsers
```

4. **Test dependent packages**:
```bash
# From repository root
yarn test:dapi-client  # Tests js-dapi-client and all dependents
```

#### 3.3.4 Working on `wasm-sdk` (Browser-Focused)

**Scenario**: You want to add new functionality to the browser SDK.

**Step-by-step workflow**:

1. **Make changes** to `packages/wasm-sdk/src/` (Rust code)

2. **Build using the optimized script**:
```bash
cd packages/wasm-sdk
./build.sh              # Development build
./build-optimized.sh    # Production build (slower but smaller)
```

3. **Test in browser**:
```bash
# Open packages/wasm-sdk/index.html in a browser
# The tests run in the browser environment
```

4. **Run comprehensive test suite**:
```bash
cd packages/wasm-sdk/test
node run-all-tests.mjs  # Runs all automated tests
```

### 3.4 Cross-Package Development

**Scenario**: You're making changes that affect multiple packages.

**Recommended approach**:

1. **Identify the dependency order** using the chain above
2. **Start with the lowest-level package** (e.g., `wasm-dpp`)
3. **Build and test each level incrementally**:

```bash
# Example: Changes affecting wasm-dpp → js-dapi-client → js-dash-sdk

# Step 1: Build and test wasm-dpp
cd packages/wasm-dpp
yarn build && yarn test

# Step 2: Test packages that depend on wasm-dpp
yarn test:dapi-client  # This will rebuild and test js-dapi-client

# Step 3: Test the main SDK
yarn test:sdk

# Step 4: Full integration test
yarn test
```

**⚡ Performance Tip**: For faster iteration during JavaScript development, use filtered builds:
```bash
# Instead of yarn build (all packages)
ultra --recursive --filter "packages/@(js-dash-sdk|wallet-lib|js-dapi-client|wasm-dpp|js-grpc-common|dapi-grpc)" --build
```

---

## 4. Testing Strategy

### 4.1 Testing Architecture Overview

The repository uses different testing approaches for different package types:

- **Unit Tests**: `*.spec.js` files alongside source code
- **Integration Tests**: `tests/integration/` directories
- **Functional Tests**: `tests/functional/` directories
- **Browser Tests**: Karma-based browser testing
- **E2E Tests**: Platform-wide integration tests

### 4.2 Testing Commands by Package Type

#### 4.2.1 JavaScript Packages (wallet-lib, js-dapi-client, etc.)

```bash
# Unit tests
yarn test:unit

# Integration tests
yarn test:integration

# Browser tests
yarn test:browsers

# All tests
yarn test
```

#### 4.2.2 TypeScript Packages (js-dash-sdk)

```bash
# Type checking
yarn test:types  # Uses tsd for TypeScript definition testing

# Unit tests (compiled from TypeScript)
yarn test:unit

# Browser tests
yarn test:browsers

# All tests
yarn test
```

#### 4.2.3 WASM Packages (wasm-dpp, wasm-sdk)

```bash
# Node.js tests
yarn test:node

# Browser tests
yarn test:browsers

# TypeScript definition tests
yarn test:types

# All tests
yarn test
```

### 4.3 Test Dependencies

**Critical**: Many tests require the local development environment to be running:

```bash
# Terminal 1: Start the environment
yarn start

# Terminal 2: Run tests
yarn test
```

**Common test failures** when the environment isn't running:
- Connection timeouts to DAPI
- Failed blockchain queries
- Integration test failures

### 4.4 Testing Cross-Package Changes

When changes span multiple packages, use the repository-level test commands that understand dependencies:

```bash
# Test specific package groups
yarn test:wallet-lib     # Tests wallet-lib + dependents
yarn test:dapi-client    # Tests js-dapi-client + dependents
yarn test:sdk           # Tests js-dash-sdk + dependents

# Test by component
yarn test:dpp           # Tests all DPP-related packages
yarn test:dapi          # Tests all DAPI-related packages
```

---

## 5. Build and Packaging

### 5.1 Build System Overview

The repository uses `ultra-runner` for parallel builds across the monorepo:

```bash
yarn build  # Builds all packages in dependency order
```

### 5.2 Package-Specific Build Processes

#### 5.2.1 WASM Packages Build Process

WASM packages have the most complex build process:

**wasm-dpp**:
```bash
cd packages/wasm-dpp
yarn build:wasm  # Rust → WASM + bindings
yarn build:js    # WASM → JavaScript/TypeScript
```

**wasm-sdk**:
```bash
cd packages/wasm-sdk
./build.sh                    # Development build
./build-optimized.sh         # Production build
```

**Build artifacts**:
- `wasm/` or `pkg/`: WASM binaries and JavaScript bindings
- `dist/`: Transpiled JavaScript for distribution
- `*.d.ts`: TypeScript definitions

#### 5.2.2 JavaScript Package Build Process

Most JavaScript packages use simple build processes:

**For packages with browser builds**:
```bash
yarn build:web  # Webpack-based browser bundle
```

**For TypeScript packages**:
```bash
yarn build:ts   # TypeScript compilation
yarn build      # Full build (TS + webpack if applicable)
```

### 5.3 Build Order and Dependencies

**Critical concept**: Packages must be built in dependency order. The monorepo tools handle this automatically, but when building manually:

1. **WASM packages first** (wasm-dpp, wasm-sdk)
2. **Core JS packages** (js-grpc-common, js-dapi-client)
3. **Higher-level packages** (wallet-lib, js-dash-sdk)

### 5.4 WASM Compilation Details

#### 5.4.1 Development vs Production Builds

**Development builds** (faster compilation):
```bash
CARGO_BUILD_PROFILE=dev ./build.sh
```

**Production builds** (smaller, optimized):
```bash
CARGO_BUILD_PROFILE=release ./build-optimized.sh
```

#### 5.4.2 WASM Optimization

The build process includes multiple optimization steps:

1. **Rust compilation** with release optimizations
2. **wasm-bindgen** for JavaScript bindings
3. **wasm-opt** for binary optimization
4. **Base64 encoding** for embedding (some packages)

### 5.5 JavaScript-Only Build Optimization

**Critical for JavaScript developers**: You don't need to build all 21 packages in the monorepo! The JavaScript SDK ecosystem only requires **6 core packages**, dramatically reducing build times and complexity.

#### 5.5.1 Core JavaScript Packages

For JavaScript SDK development, you only need these packages:

```
JavaScript Core Dependencies (6 packages):
├── js-dash-sdk (main SDK)
├── wallet-lib (wallet functionality) 
├── js-dapi-client (DAPI client)
├── wasm-dpp (WASM DPP bindings)
├── js-grpc-common (gRPC utilities)
├── dapi-grpc (gRPC protocol definitions)
└── wasm-sdk (standalone WASM SDK)
```

**Build time comparison**:
- **Full monorepo**: ~15-20 minutes (21 packages + Rust compilation)
- **JavaScript core only**: ~3-5 minutes (6 packages)
- **WASM packages only**: ~2-3 minutes (wasm-dpp + wasm-sdk)

#### 5.5.2 Optimized Build Commands

**Build JavaScript core packages only**:
```bash
# From repository root - builds only the 6 essential packages
ultra --recursive --filter "packages/@(js-dash-sdk|wallet-lib|js-dapi-client|wasm-dpp|js-grpc-common|dapi-grpc)" --build

# Add WASM-SDK if needed
yarn workspace wasm-sdk build
```

**Build individual core packages**:
```bash
# Most time-intensive (Rust → WASM compilation)
yarn workspace @dashevo/wasm-dpp build    # ~1-2 minutes

# TypeScript + webpack (complex build)
yarn workspace dash build                 # js-dash-sdk, ~30 seconds

# Simple JavaScript builds
yarn workspace @dashevo/wallet-lib build:web     # ~10 seconds
yarn workspace @dashevo/dapi-client build        # minimal
yarn workspace @dashevo/grpc-common build        # minimal
yarn workspace @dashevo/dapi-grpc build          # protocol buffers

# Standalone WASM SDK
cd packages/wasm-sdk && ./build.sh              # ~1-2 minutes
```

**Ultra-runner shortcuts**:
```bash
# Build only WASM packages
ultra --recursive --filter "packages/@(wasm-dpp)" --build
yarn workspace wasm-sdk build

# Build only TypeScript packages  
ultra --recursive --filter "packages/@(js-dash-sdk)" --build

# Build only wallet ecosystem
ultra --recursive --filter "packages/@(wallet-lib|js-dash-sdk)" --build
```

#### 5.5.3 Packages to Skip During JavaScript Development

**You can skip these 15+ packages** (saves massive build time):

**Infrastructure packages**:
- `dashmate` - CLI tool for node management
- `dapi` - DAPI server component
- `platform-test-suite` - End-to-end testing suite

**Contract packages** (unless specifically needed):
- `dashpay-contract`
- `dpns-contract` 
- `feature-flags-contract`
- `masternode-reward-shares-contract`
- `withdrawals-contract`
- `token-history-contract`
- `keyword-search-contract`
- `wallet-utils-contract`

**Utility packages**:
- `bench-suite` - Benchmarking tools
- `wasm-drive-verify` - Drive verification (not a JS-SDK dependency)
- `dash-spv` - SPV implementation

#### 5.5.4 Setup Optimization

**Fast JavaScript setup** (instead of `yarn setup`):
```bash
# 1. Install dependencies (workspace links created automatically)
yarn install

# 2. Build only JavaScript core packages
ultra --recursive --filter "packages/@(js-dash-sdk|wallet-lib|js-dapi-client|wasm-dpp|js-grpc-common|dapi-grpc)" --build

# 3. Build WASM SDK if needed
yarn workspace wasm-sdk build

# 4. Skip configuration (only needed for full platform)
# yarn configure  # <- Skip this for JS-only development
```

**Development build optimization**:
```bash
# Use development WASM builds (much faster)
cd packages/wasm-dpp
CARGO_BUILD_PROFILE=dev yarn build:wasm

cd ../wasm-sdk  
CARGO_BUILD_PROFILE=dev ./build.sh
```

#### 5.5.5 When to Use Full vs Optimized Builds

**Use JavaScript-only builds when**:
- Developing wallet applications
- Working on SDK features
- Building browser applications
- Creating examples and demos
- Testing SDK functionality

**Use full builds when**:
- Contributing to platform infrastructure
- Testing end-to-end platform functionality
- Working on DAPI server features
- Running comprehensive integration tests

#### 5.5.6 Incremental Development Workflow

**Optimal development cycle**:
```bash
# 1. Initial setup (one time)
yarn install
ultra --recursive --filter "packages/@(js-dash-sdk|wallet-lib|js-dapi-client|wasm-dpp|js-grpc-common|dapi-grpc)" --build

# 2. During development - rebuild only what changed
yarn workspace @dashevo/wallet-lib build:web     # If you changed wallet-lib
yarn workspace dash build                        # If you changed js-dash-sdk

# 3. Test your changes
yarn test:wallet-lib  # Tests wallet-lib + dependents
yarn test:sdk         # Tests js-dash-sdk
```

**Hot development tips**:
- Keep WASM builds in development mode (`CARGO_BUILD_PROFILE=dev`)
- Use TypeScript watch mode: `yarn workspace dash start:ts`
- Only rebuild WASM when Rust code changes
- Use workspace-specific test commands to avoid full test suite

---

## 6. Debugging and Troubleshooting

### 6.1 Common WASM Issues

#### 6.1.1 "Module not found" errors

**Symptom**: `Cannot find module './wasm_dpp_bg.wasm'`

**Cause**: WASM build artifacts missing

**Solution**:
```bash
cd packages/wasm-dpp  # or relevant package
yarn clean
yarn build
```

#### 6.1.2 "WebAssembly module is not a valid module" 

**Symptom**: WASM runtime errors

**Cause**: Corrupted or incompatible WASM build

**Solution**:
```bash
# Clean and rebuild with verbose output
yarn clean
CARGO_BUILD_PROFILE=release yarn build:wasm
```

#### 6.1.3 Browser vs Node.js compatibility

**Symptom**: Code works in Node.js but fails in browser

**Cause**: Different WASM loading mechanisms

**Solution**: Ensure you're testing both environments:
```bash
yarn test:node      # Node.js environment
yarn test:browsers  # Browser environment
```

### 6.2 Cross-Package Debugging

#### 6.2.1 Dependency Version Mismatches

**Symptom**: Unexpected behavior or type errors

**Cause**: Packages using different versions of shared dependencies

**Solution**:
```bash
# From repository root
yarn install   # Ensures consistent workspace versions and recreates links
yarn build      # Rebuilds all packages

# Check workspace dependency resolution
yarn why <package-name>  # Shows which workspaces depend on this package

# Force workspace link recreation
yarn workspaces list     # Verify all workspaces are detected
```

#### 6.2.2 Build Order Issues

**Symptom**: "Cannot resolve module" for workspace dependencies

**Cause**: Dependencies not built before dependents

**Solution**:
```bash
# Force rebuild in correct order
yarn clean
yarn build  # Ultra-runner handles correct order

# Build in explicit dependency order (for debugging)
yarn workspaces foreach --topological run build

# Build only specific workspace and its dependencies
yarn workspaces foreach --from @dashevo/wallet-lib --recursive run build
```

### 6.3 Test Environment Issues

#### 6.3.1 DAPI Connection Failures

**Symptom**: Tests fail with connection timeouts

**Cause**: Local development environment not running

**Solution**:
```bash
# Check if environment is running
yarn dashmate group status

# If not running
yarn start
```

#### 6.3.2 Port Conflicts

**Symptom**: "Port already in use" errors

**Cause**: Previous environment didn't shut down cleanly

**Solution**:
```bash
yarn stop
yarn reset  # Nuclear option: clean everything and restart
```

### 6.4 Performance Debugging

#### 6.4.1 Slow WASM Builds

**Symptom**: WASM builds taking very long

**Cause**: Full optimization enabled

**Quick fix**:
```bash
# Use development builds for faster iteration
CARGO_BUILD_PROFILE=dev yarn build
```

#### 6.4.2 Large Bundle Sizes

**Symptom**: JavaScript bundles too large

**Investigation**:
```bash
# Analyze bundle size (for packages with webpack)
yarn build:web
# Check dist/ folder sizes

# For WASM packages, check optimization settings in build scripts
```

### 6.5 Debugging Techniques

#### 6.5.1 Isolating Issues

1. **Test individual packages**:
```bash
yarn workspace <package-name> test
```

2. **Test package groups**:
```bash
yarn test:wallet-lib  # Tests specific dependency chain
```

3. **Use verbose output**:
```bash
DEBUG=* yarn test  # Enable debug logging
```

#### 6.5.2 Workspace-Specific Debugging

1. **Verify workspace structure**:
```bash
yarn workspaces list --json  # See all workspaces and their locations
yarn workspaces info        # Detailed workspace dependency information
```

2. **Check workspace linking**:
```bash
# See if workspace dependencies are properly linked
ls -la node_modules/@dashevo/
# Should show symlinks to packages/ directories

# Check specific workspace dependencies
yarn workspace @dashevo/js-dash-sdk why @dashevo/wallet-lib
```

3. **Debug workspace resolution**:
```bash
# Check if workspace protocol is working
yarn why @dashevo/wallet-lib  # Should show workspace:* versions

# Verify workspace dependencies are resolved correctly
yarn workspaces focus @dashevo/wallet-lib --production
```

#### 6.5.3 Tracing Dependency Issues

1. **Check workspace dependencies**:
```bash
yarn workspaces info
```

2. **Verify build artifacts exist**:
```bash
ls packages/*/dist/  # Check for built files
ls packages/*/pkg/   # Check for WASM artifacts
```

---

## 7. Contribution Guidelines

### 7.1 Pre-Contribution Checklist

Before making any changes:

1. **Set up the environment**:
```bash
yarn setup
yarn start
```

2. **Run full test suite** to establish baseline:
```bash
yarn test
```

3. **Understand the scope** of your changes using the dependency chain

### 7.2 Development Process

#### 7.2.1 Making Changes

1. **Create a feature branch**:
```bash
git checkout -b feat/your-feature-name
```

2. **Make focused changes** to specific packages

3. **Build and test incrementally**:
```bash
# After each logical change
yarn workspace <package> test
yarn test:<relevant-group>
```

#### 7.2.2 Quality Assurance Workflow

Before submitting a PR, run this complete workflow:

```bash
# 1. Build everything (or use optimized JS-only build from Section 5.5)
yarn build  # Full platform build (slow)
# OR
ultra --recursive --filter "packages/@(js-dash-sdk|wallet-lib|js-dapi-client|wasm-dpp|js-grpc-common|dapi-grpc)" --build  # JS-only (fast)

# 2. Run linting
yarn lint

# 3. Run all tests
yarn test

# 4. Test specific affected packages
yarn test:wallet-lib  # Example: if you changed wallet-lib
yarn test:sdk         # Example: always test the main SDK

# 5. Test browser compatibility (for relevant changes)
yarn test:suite:browsers
```

### 7.3 Code Review Preparation

#### 7.3.1 Documentation Requirements

- **API changes**: Update TypeScript definitions
- **WASM changes**: Regenerate documentation with `python3 generate_docs.py`
- **New methods**: Add JSDoc comments following existing patterns

#### 7.3.2 Test Coverage Requirements

Different packages have different coverage requirements:

- **js-dapi-client**: 98%+ statement coverage
- **wallet-lib**: Comprehensive unit tests
- **js-dash-sdk**: Type definitions must pass `tsd` tests

#### 7.3.3 Browser Compatibility

For packages with browser support:

```bash
# Test in multiple browsers
yarn test:browsers

# For WASM packages, test the actual browser interface
# Open packages/wasm-sdk/index.html manually
```

### 7.4 Common Contribution Patterns

#### 7.4.1 Adding a New Method to wallet-lib

1. **Add the method** to the appropriate class in `src/`
2. **Write unit tests** in the same directory (`*.spec.js`)
3. **Test the change**:
```bash
yarn workspace @dashevo/wallet-lib test:unit
yarn test:wallet-lib  # Test all dependents
```
4. **Update TypeScript definitions** in `src/index.d.ts`

#### 7.4.2 Adding New WASM Functionality

1. **Add Rust code** to `src/` in the WASM package
2. **Expose through WASM bindings** (update `lib.rs`)
3. **Build and test**:
```bash
yarn build
yarn test:node && yarn test:browsers
```
4. **Update documentation** (for wasm-sdk: run `python3 generate_docs.py`)

#### 7.4.3 Fixing Cross-Package Issues

1. **Identify root cause** using dependency chain
2. **Fix at the lowest affected level**
3. **Test the entire chain**:
```bash
yarn test:<lowest-package>
yarn test:<middle-package>
yarn test:sdk
yarn test  # Full suite
```

### 7.5 Performance Considerations

#### 7.5.1 WASM Package Changes

- **Development**: Use `CARGO_BUILD_PROFILE=dev` for faster iteration
- **Testing**: Use production builds for final testing
- **Bundle Size**: Monitor size impact of changes

#### 7.5.2 JavaScript Package Changes

- **Dependencies**: Avoid adding heavy dependencies
- **Browser Support**: Consider polyfills and compatibility
- **Memory Usage**: Be mindful of object creation in hot paths

---

## 8. Creating Example Scripts

### 8.1 Example Script Architecture

The repository follows a consistent pattern where each package maintains its own `examples/` folder with self-contained, runnable demonstration scripts:

```
packages/
├── wallet-lib/examples/           # Wallet functionality examples
│   ├── client-usage.js           # Basic wallet operations
│   ├── offline-wallet.js         # Offline signing
│   ├── web/usage.web.html        # Browser examples
│   └── workers/                  # Web worker examples
├── js-dash-sdk/docs/examples/    # SDK examples (markdown format)
│   ├── pay-to-another-address.md # Payment tutorials
│   └── receive-money-and-check-balance.md
└── wasm-drive-verify/examples/   # WASM module examples
    └── es-modules-usage.js       # ES modules and bundler configs
```

### 8.2 Types of Examples

#### 8.2.1 Node.js Scripts (`.js` files)

**Purpose**: Demonstrate functionality that can run directly with `node filename.js`

**Structure**:
```js
/* eslint-disable no-console */
const { Wallet, EVENTS } = require('../src');

const wallet = new Wallet({
  mnemonic: 'protect cave garden achieve hand vacant clarify atom finish outer waste sword',
  network: 'testnet',
});

// Demo functionality with proper error handling
wallet.getAccount().then(async (account) => {
  // Example operations
  console.log('Balance:', await account.getConfirmedBalance());
}).catch((e) => {
  console.log('Failed with error', e);
});
```

#### 8.2.2 Browser Examples (`.html` files)

**Purpose**: Show how to use built packages in web browsers

**Structure**:
```html
<!DOCTYPE html>
<html>
<head>
  <title>Wallet Browser Example</title>
</head>
<body>
  <script src="../../dist/wallet-lib.min.js"></script>
  <script>
    // Use the globally available library
    const wallet = new DashWalletLib.Wallet({
      network: 'testnet'
    });
  </script>
</body>
</html>
```

#### 8.2.3 Documentation Examples (`.md` files)

**Purpose**: Step-by-step tutorials with explanations

**Structure**:
```markdown
## Feature Name

Description of what this example demonstrates.

```js
const Dash = require('dash');

// Step-by-step code with comments
const client = new Dash.Client({
  wallet: { mnemonic: 'your mnemonic here' }
});
```

Additional explanation and links to documentation.
```

### 8.3 Creating Example Scripts - Step by Step

#### 8.3.1 Choose the Right Location and Type

**For wallet functionality**:
```bash
# Node.js script for core wallet features
touch packages/wallet-lib/examples/my-wallet-example.js

# Browser example for web integration
touch packages/wallet-lib/examples/web/my-browser-example.html
```

**For SDK functionality**:
```bash
# Markdown tutorial with code samples
touch packages/js-dash-sdk/docs/examples/my-sdk-feature.md
```

**For WASM functionality**:
```bash
# ES modules example with bundler configs
touch packages/wasm-drive-verify/examples/my-wasm-example.js
```

#### 8.3.2 Follow Package Import Patterns

**wallet-lib examples**:
```js
// Use relative imports from package source
const { Wallet, EVENTS } = require('../src');
const logger = require('../src/logger');
```

**js-dash-sdk examples**:
```js
// Use the built package
const Dash = require('dash');
```

**WASM examples**:
```js
// ES modules with selective imports
import { verifyFullIdentityByIdentityId } from 'wasm-drive-verify/identity';
```

#### 8.3.3 Example Script Template

Here's a complete template for a wallet-lib example:

```js
/* eslint-disable no-console */
const { Wallet, EVENTS } = require('../src');

/**
 * Example: Checking wallet balance and creating a transaction
 * 
 * This script demonstrates:
 * - Creating a wallet instance
 * - Checking account balance
 * - Creating and broadcasting a transaction
 * - Handling wallet events
 */

async function walletExample() {
  // Initialize wallet with testnet mnemonic
  const wallet = new Wallet({
    mnemonic: 'protect cave garden achieve hand vacant clarify atom finish outer waste sword',
    network: 'testnet',
  });

  try {
    // Get the default account
    const account = await wallet.getAccount();
    
    // Check balances
    const confirmedBalance = await account.getConfirmedBalance(false);
    const unconfirmedBalance = await account.getUnconfirmedBalance(false);
    
    console.log(`Confirmed balance: ${confirmedBalance} satoshis`);
    console.log(`Unconfirmed balance: ${unconfirmedBalance} satoshis`);
    
    // Get a new receiving address
    const address = await account.getUnusedAddress();
    console.log(`New address: ${address.address}`);
    
    // Create a transaction (only if we have balance)
    if (confirmedBalance > 1000) {
      const transaction = account.createTransaction({
        satoshis: 1000,
        recipient: 'ycyFFyWCPSWbXLZBeYppJqgvBF7bnu8BWQ'
      });
      
      // In a real app, you'd broadcast this
      console.log(`Transaction created: ${transaction.id}`);
      // const transactionId = await account.broadcastTransaction(transaction);
    }
    
    // Set up event listeners
    account.on(EVENTS.CONFIRMED_BALANCE_CHANGED, (info) => {
      console.log('Balance changed:', info);
    });
    
    account.on(EVENTS.GENERATED_ADDRESS, () => {
      console.log('New address generated');
    });
    
  } catch (error) {
    console.error('Example failed:', error.message);
    process.exit(1);
  }
}

// Run the example
walletExample();
```

#### 8.3.4 Testing Your Example

**Test the script directly**:
```bash
# Navigate to the package directory
cd packages/wallet-lib

# Run your example
node examples/my-wallet-example.js
```

**Test with built packages** (if needed):
```bash
# Build the package first
yarn workspace @dashevo/wallet-lib build:web

# Then run your example
node examples/my-wallet-example.js
```

**For browser examples**:
```bash
# Make sure the package is built
yarn workspace @dashevo/wallet-lib build:web

# Open the HTML file in a browser
open packages/wallet-lib/examples/web/my-browser-example.html
```

### 8.4 Best Practices for Examples

#### 8.4.1 Code Quality Standards

**Always include**:
- `/* eslint-disable no-console */` for Node.js scripts
- Proper error handling with try/catch
- Descriptive comments explaining each step
- Testnet credentials (never mainnet)

**Example structure**:
```js
/* eslint-disable no-console */

/**
 * Example: [Description]
 * 
 * This script demonstrates:
 * - Point 1
 * - Point 2
 * - Point 3
 */

async function exampleFunction() {
  try {
    // Main logic here
  } catch (error) {
    console.error('Example failed:', error.message);
    process.exit(1);
  }
}

exampleFunction();
```

#### 8.4.2 Documentation and Discovery

**Make examples discoverable**:
- Examples are automatically included in package distributions
- Use descriptive filenames: `wallet-balance-checker.js` not `example1.js`
- Add brief descriptions in comments at the top of files

**For markdown examples**:
- Include links to relevant API documentation
- Explain what each code block does
- Show both success and error scenarios

#### 8.4.3 Security and Safety

**Testnet only**:
```js
// ✅ Good
network: 'testnet'

// ❌ Bad - never use mainnet in examples
network: 'livenet'
```

**Safe mnemonics**:
```js
// ✅ Good - well-known test mnemonic
mnemonic: 'protect cave garden achieve hand vacant clarify atom finish outer waste sword'

// ❌ Bad - real mnemonic with funds
mnemonic: process.env.REAL_MNEMONIC
```

### 8.5 Advanced Example Patterns

#### 8.5.1 WASM Examples with Bundler Configurations

For WASM packages, include bundler configuration examples:

```js
// Example: Using ES modules with wasm-drive-verify

// Import only what you need (best for bundle size)
import { verifyFullIdentityByIdentityId } from 'wasm-drive-verify/identity';

async function verifyIdentity() {
  const proof = new Uint8Array([/* ... */]);
  const identityId = new Uint8Array([/* ... */]);
  const platformVersion = 1;
  
  const result = await verifyFullIdentityByIdentityId(proof, identityId, platformVersion);
  console.log('Identity verification result:', result);
}

// Webpack configuration
export const webpackConfig = {
  experiments: {
    asyncWebAssembly: true,
  },
  // ... additional config
};
```

#### 8.5.2 Worker Examples

For advanced functionality like web workers:

```js
// packages/wallet-lib/examples/workers/MyWorker.js
const { Wallet } = require('../../src');

class MyWorker {
  constructor() {
    this.wallet = new Wallet({
      network: 'testnet'
    });
  }

  async processInBackground() {
    // Heavy processing that doesn't block the main thread
  }
}

module.exports = MyWorker;
```

### 8.6 Integration with Development Workflow

#### 8.6.1 When to Create Examples

**During feature development**:
- Create examples as you develop new features
- Use examples to test your changes in realistic scenarios
- Examples serve as informal integration tests

**During bug fixes**:
- Create examples that reproduce the bug
- Keep the example after fixing to prevent regressions

**For documentation**:
- Convert complex examples into markdown tutorials
- Create examples for common use cases developers ask about

#### 8.6.2 Example Maintenance

**Keep examples current**:
- Update examples when APIs change
- Test examples as part of your testing workflow
- Remove or update examples that become obsolete

**Version compatibility**:
- Examples should work with the current package version
- Avoid using deprecated APIs in new examples
- Update existing examples when deprecating APIs

## Quick Reference

### Essential Commands

```bash
# Setup
yarn setup                    # Initial setup
yarn start                    # Start dev environment
yarn stop                     # Stop dev environment
yarn reset                    # Nuclear reset

# Building
yarn build                    # Build all packages (full platform - slow)
ultra --recursive --filter "packages/@(js-dash-sdk|wallet-lib|js-dapi-client|wasm-dpp|js-grpc-common|dapi-grpc)" --build  # JS-only (fast)
yarn workspace <pkg> build    # Build specific package

# Testing
yarn test                     # All tests
yarn test:<package-group>     # Test package group
yarn workspace <pkg> test     # Test specific package

# WASM-specific
cd packages/wasm-dpp && yarn build:wasm    # Build WASM
cd packages/wasm-sdk && ./build.sh         # Build WASM SDK

# Example testing
node packages/wallet-lib/examples/my-example.js    # Run Node.js example
open packages/wallet-lib/examples/web/my.html      # Run browser example
```

### Package Groups for Testing

- `yarn test:wallet-lib` - wallet-lib + dependents
- `yarn test:dapi-client` - js-dapi-client + dependents  
- `yarn test:sdk` - js-dash-sdk + dependents
- `yarn test:dpp` - All DPP packages
- `yarn test:dapi` - All DAPI packages

### Example Script Locations

```bash
# Node.js examples
packages/wallet-lib/examples/*.js
packages/wasm-drive-verify/examples/*.js

# Browser examples  
packages/wallet-lib/examples/web/*.html
packages/js-dapi-client/examples/web/*.html

# Documentation examples
packages/js-dash-sdk/docs/examples/*.md
```

### Troubleshooting Quick Fixes

```bash
# WASM build issues
cd packages/<wasm-package> && yarn clean && yarn build

# Dependency issues  
yarn install && yarn build  # Or use JS-only build for faster recovery

# Workspace linking issues
yarn workspaces list && yarn install  # Verify and recreate workspace links

# Test environment issues
yarn stop && yarn start

# Nuclear option
yarn reset
```

### Workspace-Specific Quick Reference

```bash
# Essential workspace commands
yarn workspaces list                              # List all workspaces
yarn workspace @dashevo/wallet-lib <command>      # Run command in specific workspace
yarn workspaces foreach run build                 # Run command in all workspaces
yarn why @dashevo/wallet-lib                     # Check workspace dependencies

# Debugging workspace issues  
ls -la node_modules/@dashevo/                     # Check workspace symlinks
yarn workspaces info                             # Detailed workspace information
yarn workspaces focus @dashevo/wallet-lib        # Install only specific workspace deps
```

---

*This guide provides a comprehensive foundation for JavaScript developers working on the Dash Platform repository. The key to success is understanding the dependency relationships, following established patterns, and always testing the complete chain when making changes.*