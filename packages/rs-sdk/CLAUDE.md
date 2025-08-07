# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Build Commands

```bash
# Build the SDK (from packages/rs-sdk)
cargo build

# Build with specific features
cargo build --features mocks
cargo build --features offline-testing
cargo build --no-default-features --features network-testing

# Build all packages in the workspace
cargo build --workspace
```

### Testing Commands

```bash
# Run all tests with default features (offline mode using test vectors)
cargo test -p dash-sdk

# Run specific test
cargo test -p dash-sdk test_name

# Network testing (requires live platform connection via .env)
cargo test -p dash-sdk --no-default-features --features network-testing

# Generate test vectors (requires running platform)
./scripts/generate_test_vectors.sh

# Generate test vectors for specific test
./scripts/generate_test_vectors.sh test::name

# Mock testing
cargo test -p dash-sdk --features mocks

# Run tests with verbose output
cargo test -p dash-sdk -- --nocapture
```

### Development Tools

```bash
# Connect to remote node for testing (creates .env file)
./scripts/connect_to_remote.sh

# Check code without building
cargo check -p dash-sdk

# Run linter
cargo clippy -p dash-sdk -- -D warnings

# Format code
cargo fmt

# Generate documentation
cargo doc -p dash-sdk --open
```

## Architecture

### Core SDK Structure

**Entry Point** (`src/sdk.rs`):
- `Sdk` struct: Main interface for all SDK operations
- `SdkBuilder`: Fluent API for SDK configuration
- Supports both real DAPI client and mock implementations
- Thread-safe with Arc<AtomicU64> for request counters

**Platform Module** (`src/platform.rs`):
- Implements CRUD operations via traits: `Fetch`, `FetchMany`, `FetchUnproved`
- Query system with `DocumentQuery` and `DriveQuery`
- State transitions for identities, documents, and contracts
- Token management (balances, pricing, supply)

**Core Module** (`src/core.rs`):
- Transaction handling and building
- Dash Core client integration
- SPV support interfaces

**Mock System** (`src/mock/`):
- Full SDK mocking with expectation-based testing
- Mock wallet, provider, and request implementations
- Test vector support for deterministic testing

### Key Design Patterns

1. **Trait-Based Architecture**: Extensible via `Fetch`, `FetchMany`, `FetchUnproved` traits
2. **Builder Pattern**: `SdkBuilder` for configuration
3. **Strategy Pattern**: Swappable implementations (real vs mock)
4. **Context Provider System**: Pluggable state management
5. **Proof Verification**: Built-in cryptographic proof validation

### Testing Strategy

**Three Testing Modes**:
1. **Offline Testing** (default): Uses pre-generated test vectors in `tests/vectors/`
2. **Network Testing**: Live platform integration via `.env` configuration
3. **Mock Testing**: Expectation-based testing with full control

**Test Structure**:
```
tests/
├── main.rs              # Test entry point
├── fetch/              # Fetch operation tests
│   ├── mock_fetch.rs  # Mock examples
│   └── *.rs          # Feature-specific tests
└── vectors/           # Pre-generated test data
```

### Configuration

**Environment Variables** (`.env`):
```bash
DASH_SDK_PLATFORM_HOST=         # Platform gRPC endpoint
DASH_SDK_PLATFORM_PORT=         # Platform gRPC port
DASH_SDK_CORE_HOST=            # Dash Core RPC host
DASH_SDK_CORE_PORT=            # Dash Core RPC port
DASH_SDK_CORE_USER=            # Dash Core RPC username
DASH_SDK_CORE_PASSWORD=        # Dash Core RPC password
```

**Feature Flags**:
- `mocks`: Enable mock implementations
- `offline-testing`: Use test vectors (default)
- `network-testing`: Live platform testing
- `generate-test-vectors`: Create test vectors
- Contract features: `dpns-contract`, `dashpay-contract`, etc.

### Error Handling

**Error Types** (`src/error.rs`):
- `ProtocolError`: DPP consensus violations
- `DapiClientError`: Network/connectivity issues
- `ProofError`: Cryptographic verification failures
- `StaleNodeError`: Outdated platform state
- `TimeoutError`: Operation timeouts

### Adding New Fetch Operations

When implementing new `Fetch`/`FetchMany` operations:

1. Update protobuf definitions in `packages/dapi-grpc/protos/platform/v0/platform.proto`
2. Add request/response to `VERSIONED_REQUESTS`/`VERSIONED_RESPONSES` in `packages/dapi-grpc/build.rs`
3. Link transport in `packages/rs-dapi-client/src/transport/grpc.rs`
4. Implement `FromProof` trait in `packages/rs-drive-proof-verifier/src/proof.rs`
5. Implement `Query` trait in `src/platform/query.rs`
6. Implement `MockResponse` in `src/mock/requests.rs`
7. Implement `Fetch`/`FetchMany` traits in `src/platform/fetch*.rs`
8. Add tests in `tests/fetch/`
9. Update mock SDK expectations in `src/mock/sdk.rs`
10. Generate test vectors with `./scripts/generate_test_vectors.sh`

### Dependencies

**Core Dependencies**:
- `dpp`: Dash Platform Protocol implementation
- `drive`: Storage layer with proof verification
- `dapi-grpc`: gRPC protocol definitions
- `rs-dapi-client`: DAPI client implementation
- `drive-proof-verifier`: Cryptographic proof validation

**Key External Dependencies**:
- `tokio`: Async runtime
- `dashcore-rpc`: Dash Core RPC client
- `serde`/`serde_json`: Serialization
- `thiserror`: Error handling
- `tracing`: Structured logging