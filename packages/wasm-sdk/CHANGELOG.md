# Changelog

# WASM SDK v0.1.0 Release Notes

**Release Date:** 2025-09-03

## Installation

```bash
npm install @dashevo/dash-wasm-sdk@0.1.0
```

## 🐛 Bug Fixes

- **c31a10c20**: Issue #55: Configure alpha publishing infrastructure (@Darwin-Dash)
- **7b0eb8dda**: Issue #52: Create comprehensive TypeScript definitions with 100% API coverage (@Darwin-Dash)
- **5ff2d455e**: Issue #51: Enhanced wasm-pack metadata for comprehensive NPM package generation (@Darwin-Dash)
- **1aa22dd97**: Issue #50: Add comprehensive NPM package metadata to wasm-sdk Cargo.toml (@Darwin-Dash)

## 📖 Resources

- [API Documentation](https://dashplatform.readme.io/)
- [GitHub Repository](https://github.com/dashpay/platform)
- [Issue Tracker](https://github.com/dashpay/platform/issues)
- [Community Support](https://github.com/dashpay/platform/discussions)

## 🚨 Support

If you encounter any issues or have questions:
1. Check the [documentation](https://dashplatform.readme.io/)
2. Search [existing issues](https://github.com/dashpay/platform/issues)
3. Create a [new issue](https://github.com/dashpay/platform/issues/new/choose)
4. Join the [community discussion](https://github.com/dashpay/platform/discussions)


---

All notable changes to the `@dashevo/dash-wasm-sdk` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Community feedback collection system with issue templates
- Automated publishing pipeline via GitHub Actions
- Comprehensive release documentation and rollback procedures

## [0.1.0-alpha.1] - 2025-09-03

### Added
- Initial alpha release of the enhanced WASM SDK package
- High-performance WebAssembly SDK for Dash Platform operations
- Complete identity management functionality:
  - Identity creation and management
  - Identity top-up operations
  - Credit transfers and withdrawals
  - Public key management and updates
- Document operations support:
  - Document creation, updates, and deletion
  - Document queries with filtering and sorting
  - Document ownership transfers
  - Document pricing and purchase operations
- Data contract functionality:
  - Contract creation and updates
  - Contract history retrieval
  - Multi-contract queries
- Token operations (alpha):
  - Token minting, burning, and transfers
  - Token freezing and unfreezing capabilities
  - Direct purchase and pricing mechanisms
  - Token distribution claiming
  - Configuration management
- DPNS (Dash Platform Name Service) integration:
  - Username validation and registration
  - Name resolution and availability checks
  - Homograph-safe character conversion
- Comprehensive cryptographic utilities:
  - Mnemonic generation and validation (12/24 words, multiple languages)
  - Extended key derivation (BIP39/44, DIP9/13/14/15)
  - DashPay contact key derivation
  - Address generation and validation
  - Message signing capabilities
- Platform query operations:
  - Identity queries with proof validation
  - Balance and nonce retrieval
  - Epoch and protocol version information
  - Contested resource voting support
  - Group management operations
- Network configuration support:
  - Mainnet and testnet configurations
  - Trusted quorum pre-fetching
  - Configurable timeouts and retries
  - DAPI address management
- TypeScript support:
  - Complete TypeScript definitions
  - Full IntelliSense support
  - Type-safe API interfaces
- Browser and Node.js compatibility:
  - ES module support
  - WebAssembly optimization (6-12MB bundle)
  - Efficient initialization patterns

### Technical Details
- **Package Size**: ~3.2MB compressed, ~12.6MB uncompressed
- **Bundle Optimization**: 70% compression ratio with gzip
- **WASM Performance**: Optimized binary with tree-shaking support
- **Memory Usage**: Efficient memory management with automatic cleanup
- **Network Support**: Full testnet and mainnet compatibility

### Known Limitations
- Large bundle size (optimization ongoing)
- Node.js requires explicit WASM buffer loading
- Some advanced token features are in alpha state
- Performance optimization needed for large document queries

### Dependencies
- Compatible with modern browsers (Chrome 90+, Firefox 88+, Safari 14+)
- Node.js 16+ required for server-side usage
- No peer dependencies required

## [0.0.0] - Pre-release development

### Added
- Initial package structure and build system
- Basic WASM compilation pipeline
- Core Rust implementation foundation

---

## Release Notes Format

For each release, we include:

- **Added**: New features and capabilities
- **Changed**: Modifications to existing functionality  
- **Deprecated**: Features marked for removal
- **Removed**: Features removed in this release
- **Fixed**: Bug fixes and corrections
- **Security**: Security-related improvements

## Migration Guides

### Upgrading to 0.1.0-alpha.1

This is the initial alpha release. No migration required.

**Installation:**
```bash
npm install @dashevo/dash-wasm-sdk@alpha
```

**Basic Usage:**
```javascript
import initWasm, { WasmSdkBuilder } from '@dashevo/dash-wasm-sdk';

// Initialize WASM (required)
await initWasm();

// Create SDK instance
const sdk = WasmSdkBuilder.new_testnet().build();

// Use SDK functions
const mnemonic = generate_mnemonic();
const identity = await sdk.identityCreate(/* parameters */);
```

For detailed usage examples, see the [AI_REFERENCE.md](AI_REFERENCE.md) documentation.