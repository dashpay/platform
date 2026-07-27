# Test Configuration

Platform tests need to run fast. A production node verifies block signatures, checks
instant lock proofs, persists platform state to disk, and creates database checkpoints.
All of that is essential for security -- and all of it makes tests slow.

This chapter covers the configuration system that lets tests disable expensive
checks selectively, the builder that wires everything together, and the mock RPC
layer that eliminates the need for a real Dash Core node.

## PlatformTestConfig

The heart of test configuration is `PlatformTestConfig`, defined in
`packages/rs-drive-abci/src/config.rs`:

```rust
#[cfg(feature = "testing-config")]
pub struct PlatformTestConfig {
    /// Whether to perform block signing.
    pub block_signing: bool,

    /// Whether to store platform state to disk.
    pub store_platform_state: bool,

    /// Whether to verify block commit signatures.
    pub block_commit_signature_verification: bool,

    /// Whether to disable instant lock signature verification.
    pub disable_instant_lock_signature_verification: bool,

    /// Whether to disable contested documents validation.
    pub disable_contested_documents_is_allowed_validation: bool,

    /// Whether to disable checkpoint creation during tests.
    pub disable_checkpoints: bool,
}
```

Notice the `#[cfg(feature = "testing-config")]` gate. This struct does not exist in
production builds. You cannot accidentally ship code that disables signature verification.

### Two Default Profiles

`PlatformTestConfig` provides two defaults, and choosing the right one matters:

**Full defaults** (`Default::default()`): Everything enabled. Tests run like a production
node, just with a mock RPC backend:

```rust
#[cfg(feature = "testing-config")]
impl Default for PlatformTestConfig {
    fn default() -> Self {
        Self {
            block_signing: true,
            store_platform_state: true,
            block_commit_signature_verification: true,
            disable_instant_lock_signature_verification: false,
            disable_contested_documents_is_allowed_validation: true,
            disable_checkpoints: true,
        }
    }
}
```

**Minimal verifications** (`default_minimal_verifications()`): Disables everything that
is not needed to test application logic:

```rust
impl PlatformTestConfig {
    pub fn default_minimal_verifications() -> Self {
        Self {
            block_signing: false,
            store_platform_state: false,
            block_commit_signature_verification: false,
            disable_instant_lock_signature_verification: true,
            disable_contested_documents_is_allowed_validation: true,
            disable_checkpoints: true,
        }
    }
}
```

Use `default()` when testing consensus-critical behavior (block signing, quorum
verification). Use `default_minimal_verifications()` for everything else -- it is
significantly faster because it skips cryptographic operations.

### When to Override Individual Fields

Sometimes you need a custom combination. For example, testing withdrawal transitions
requires disabling instant lock verification but keeping everything else:

```rust
let platform_config = PlatformConfig {
    testing_configs: PlatformTestConfig {
        disable_instant_lock_signature_verification: true,
        ..Default::default()
    },
    ..Default::default()
};
```

The `..Default::default()` spread syntax fills in the remaining fields with their
defaults. This pattern lets you express "default with one override" clearly.

## TestPlatformBuilder

`TestPlatformBuilder` is the fluent API for constructing a test platform. It lives in
`packages/rs-drive-abci/src/test/helpers/setup.rs`:

```rust
pub struct TestPlatformBuilder {
    config: Option<PlatformConfig>,
    initial_protocol_version: Option<ProtocolVersion>,
    tempdir: TempDir,
}
```

### The Builder Chain

The builder supports three configuration methods:

```rust
impl TestPlatformBuilder {
    /// Create a new builder with a fresh temporary directory.
    pub fn new() -> Self { Self::default() }

    /// Override the platform configuration.
    pub fn with_config(mut self, config: PlatformConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Pin a specific protocol version.
    pub fn with_initial_protocol_version(
        mut self,
        initial_protocol_version: ProtocolVersion,
    ) -> Self {
        self.initial_protocol_version = Some(initial_protocol_version);
        self
    }

    /// Use the latest protocol version.
    pub fn with_latest_protocol_version(mut self) -> Self {
        self.initial_protocol_version =
            Some(PlatformVersion::latest().protocol_version);
        self
    }
}
```

### Building

The builder has two build methods:

**`build_with_mock_rpc()`** creates a `TempPlatform<MockCoreRPCLike>` -- no real
Dash Core node needed:

```rust
pub fn build_with_mock_rpc(self) -> TempPlatform<MockCoreRPCLike> {
    let config = self.config.map(|mut c| {
        c.db_path = self.tempdir.path().to_path_buf();
        c
    });

    let platform = Platform::<MockCoreRPCLike>::open(
        self.tempdir.path(),
        config,
        self.initial_protocol_version
            .or(Some(PlatformVersion::latest().protocol_version)),
    )
    .expect("should open Platform successfully");

    TempPlatform {
        platform,
        tempdir: self.tempdir,
    }
}
```

Notice how the builder automatically sets `db_path` to the temp directory -- you
cannot accidentally write to a real database.

**`build_with_default_rpc()`** creates a `TempPlatform<DefaultCoreRPC>` for
integration tests that need a real Dash Core connection.

### Initializing State

After building, you choose what initial state to install:

```rust
// Minimal: just the GroveDB tree structure
let platform = TestPlatformBuilder::new()
    .build_with_mock_rpc()
    .set_initial_state_structure();

// Full: genesis state with system data contracts
let platform = TestPlatformBuilder::new()
    .with_latest_protocol_version()
    .build_with_mock_rpc()
    .set_genesis_state();

// Genesis with specific activation info
let platform = TestPlatformBuilder::new()
    .build_with_mock_rpc()
    .set_genesis_state_with_activation_info(
        genesis_time,
        start_core_block_height,
    );
```

Most tests want `set_genesis_state()`. Use `set_initial_state_structure()` only when
testing the state structure itself.

### Loading Test Data Contracts

`TempPlatform` provides convenience methods for loading test contracts:

```rust
let (platform, card_game_contract) = TestPlatformBuilder::new()
    .build_with_mock_rpc()
    .set_initial_state_structure()
    .with_crypto_card_game_transfer_only(Transferable::Always);
```

This loads a predefined "crypto card game" data contract from
`tests/supporting_files/contract/` and applies it to the platform. The returned
`DataContract` can be used to create documents in subsequent test steps.

## Mock RPC: Simulating Dash Core

The `MockCoreRPCLike` type (from the `mockall` crate) replaces the real Dash Core
RPC client. It lets tests control exactly what Core "reports" -- which transactions
are confirmed, what the current block height is, which asset locks exist, etc.

In strategy tests, the mock is configured automatically by `run_chain_for_strategy`.
In unit tests, you typically let the default mock behavior handle things:

```rust
let platform = TestPlatformBuilder::new()
    .with_config(platform_config)
    .build_with_mock_rpc()  // <-- MockCoreRPCLike
    .set_genesis_state();
```

The mock RPC means unit tests require zero external services. They run in CI, on
developer laptops, and in sandboxed environments with no network access.

## PlatformConfig for Tests vs Production

`PlatformConfig` is a large struct with many subsections. Here is how tests typically
configure it:

```rust
let config = PlatformConfig {
    // Validator set: 100 nodes, 67% threshold
    validator_set: ValidatorSetConfig::default_100_67(),

    // Chain lock quorum config
    chain_lock: ChainLockConfig::default_100_67(),

    // Instant lock quorum config
    instant_lock: InstantLockConfig::default_100_67(),

    // Execution settings
    execution: ExecutionConfig {
        verify_sum_trees: true,
        ..ExecutionConfig::default()
    },

    // Block timing
    block_spacing_ms: 3000,

    // Test-specific overrides
    testing_configs: PlatformTestConfig::default_minimal_verifications(),

    // Fill the rest with defaults
    ..Default::default()
};
```

The `default_100_67()` methods create configs for a 100-node network with a 67%
signing threshold -- the standard test network size.

## Platform Restart Testing

`TempPlatform` supports simulating a platform restart by reopening from the same
temporary directory:

```rust
pub fn open_with_tempdir(
    tempdir: TempDir,
    mut config: PlatformConfig,
) -> Self {
    config.db_path = tempdir.path().to_path_buf();
    let platform = Platform::<MockCoreRPCLike>::open(
        tempdir.path(), Some(config), None,
    )
    .expect("should open Platform successfully");

    Self { platform, tempdir }
}
```

The pattern for restart testing:

```rust
// Run first phase
let outcome = run_chain_for_strategy(
    &mut platform, 50, strategy, config.clone(),
    seed, &mut None, &mut None,
);

// Extract tempdir (ownership transfer)
let tempdir = platform.tempdir;

// Reopen -- simulates restart
let platform = TempPlatform::open_with_tempdir(tempdir, config);

// Verify state survived the restart
```

## Rules

**Do:**

- Use `default_minimal_verifications()` for tests that do not need signature verification.
- Use `Default::default()` for `PlatformTestConfig` when testing block signing or quorum logic.
- Always build with `build_with_mock_rpc()` unless you specifically need Dash Core.
- Let the builder manage `db_path` -- never set it manually in test configs.
- Use `set_genesis_state()` for most tests; use `set_initial_state_structure()` only for
  low-level storage tests.

**Don't:**

- Disable verifications in production code -- `PlatformTestConfig` is `#[cfg(feature)]` guarded.
- Create `Platform` instances directly -- always use `TestPlatformBuilder`.
- Share temporary directories between tests -- each test gets its own `TempDir`.
- Forget `with_latest_protocol_version()` -- without it, the builder still defaults to latest,
  but being explicit prevents surprises during protocol upgrades.
- Use `build_with_default_rpc()` in CI -- it requires a running Dash Core node.
