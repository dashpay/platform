# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Build and Development

```bash
# Initial setup (installs deps, builds, and configures)
yarn setup

# Start local development environment
yarn start

# Stop local environment
yarn stop

# Restart services
yarn restart

# Rebuild after changes
yarn build

# Complete reset of data and builds
yarn reset
```

### Testing

```bash
# Run full test suite (requires running node from yarn start)
yarn test

# Test specific packages
yarn test:suite              # Platform test suite
yarn test:dapi               # DAPI components
yarn test:sdk                # JavaScript SDK (original)
yarn test:dpp                # Dash Platform Protocol
yarn test:drive              # Drive storage layer
yarn test:wallet-lib         # Wallet library
yarn test:dapi-client        # DAPI client

# Test specific workspace
yarn workspace <package_name> test
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

### Other Commands

```bash
# Run linters
yarn lint

# Access dashmate CLI
yarn dashmate

# Configure test suite network
yarn configure:tests:network
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

**SDK** (`packages/js-dash-sdk-original`, `packages/rs-sdk`): Client libraries providing high-level interfaces for building applications on Dash Platform.

**Dashmate** (`packages/dashmate`): Node management tool for setting up and managing Dash Platform nodes.

### Data Contracts

Platform uses data contracts to define application data schemas:
- `dpns-contract`: Dash Platform Naming Service
- `dashpay-contract`: Social payments functionality
- `feature-flags-contract`: System feature toggles
- `masternode-reward-shares-contract`: Masternode reward distribution
- `withdrawals-contract`: Platform credit withdrawals

### Development Workflow

1. **Monorepo Structure**: Uses Yarn workspaces to manage multiple packages
2. **Cross-language Integration**: WASM bindings connect Rust and JavaScript code
3. **Local Development**: Docker Compose environment managed by dashmate
4. **Testing**: Comprehensive test suites at unit, integration, and e2e levels

### Important Patterns

- **Platform Versioning**: Uses `rs-platform-version` for protocol versioning
- **Serialization**: Custom serialization with `rs-platform-serialization`
- **Value Handling**: `rs-platform-value` for cross-language data representation
- **Proof Verification**: `rs-drive-proof-verifier` for cryptographic proofs
- **State Transitions**: Documents and data contracts use state transitions for updates