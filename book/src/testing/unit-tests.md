# Unit Tests

If you have spent any time reading the Dash Platform codebase, you have probably noticed
that test files are *everywhere* -- and they follow a very specific structure. This chapter
walks through the patterns that Platform's unit tests use, why those patterns exist, and
how to write your own tests that fit naturally into the codebase.

## The Test Module Convention

Nearly every test file in Platform follows the same opening stanza:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    // ... additional imports ...
}
```

This is standard Rust, but the consistency matters. The `#[cfg(test)]` attribute means
the entire module is compiled only when running `cargo test`. The `use super::*;` import
pulls in everything from the parent module, so tests can access the types and functions
they are testing without repeating import paths.

In Platform, tests that validate state transitions live in dedicated `tests.rs` files:

```
packages/rs-drive-abci/src/execution/validation/
  state_transition/state_transitions/
    address_credit_withdrawal/
      tests.rs            <-- unit tests for withdrawal transitions
    address_funds_transfer/
      tests.rs            <-- unit tests for address-to-address transfers
```

Each `tests.rs` file is a self-contained test module for one state transition type. Inside,
tests are further organized into sub-modules by *category*:

```rust
#[cfg(test)]
mod tests {
    // ... imports and helpers ...

    mod structure_validation {
        use super::*;

        #[test]
        fn test_no_inputs_returns_error() { /* ... */ }

        #[test]
        fn test_too_many_inputs_returns_error() { /* ... */ }
    }

    mod address_state_validation {
        use super::*;
        // ...
    }

    mod witness_validation {
        use super::*;
        // ...
    }
}
```

This sub-module approach groups related tests, making `cargo test` output scannable.
When a test fails, you immediately see `tests::structure_validation::test_no_inputs_returns_error`
instead of a flat list.

## TestPlatformBuilder: Setting Up the World

Most unit tests need a running Platform instance with a database, genesis state, and
configuration. The `TestPlatformBuilder` provides a fluent API for this:

```rust
// File: packages/rs-drive-abci/src/test/helpers/setup.rs

pub struct TestPlatformBuilder {
    config: Option<PlatformConfig>,
    initial_protocol_version: Option<ProtocolVersion>,
    tempdir: TempDir,
}
```

The builder creates a `TempPlatform` -- a `Platform` instance backed by a temporary
directory that is automatically cleaned up when the test finishes:

```rust
pub struct TempPlatform<C> {
    pub platform: Platform<C>,
    pub tempdir: TempDir,
}
```

Here is the typical setup pattern:

```rust
let platform = TestPlatformBuilder::new()
    .with_config(platform_config)
    .with_latest_protocol_version()
    .build_with_mock_rpc()
    .set_genesis_state();
```

Let's break this down:

- **`new()`** creates a builder with a fresh `TempDir`.
- **`with_config()`** injects a `PlatformConfig` (including test-specific overrides).
- **`with_latest_protocol_version()`** pins the platform to the current protocol version.
- **`build_with_mock_rpc()`** constructs the `Platform` with a `MockCoreRPCLike` -- no
  real Dash Core node needed.
- **`set_genesis_state()`** writes the initial state tree (system data contracts, etc.)
  into the database.

Because `TempPlatform` implements `Deref<Target = Platform<C>>`, you can call Platform
methods directly on it:

```rust
impl<C> Deref for TempPlatform<C> {
    type Target = Platform<C>;

    fn deref(&self) -> &Self::Target {
        &self.platform
    }
}
```

This means `platform.drive`, `platform.state`, and `platform.config` all work directly.

## Helper Functions: Encapsulating Test Patterns

Each test file defines local helper functions that encapsulate repeated setup logic.
For example, the withdrawal tests define helpers for creating transitions:

```rust
fn create_signed_address_credit_withdrawal_transition(
    signer: &TestAddressSigner,
    inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
    output: Option<(PlatformAddress, u64)>,
    fee_strategy: Vec<AddressFundsFeeStrategyStep>,
    output_script: CoreScript,
) -> StateTransition {
    AddressCreditWithdrawalTransitionV0::try_from_inputs_with_signer(
        inputs,
        output,
        AddressFundsFeeStrategy::from(fee_strategy),
        1, // core_fee_per_byte
        Pooling::Never,
        output_script,
        signer,
        0, // user_fee_increase
        PlatformVersion::latest(),
    )
    .expect("should create signed transition")
}
```

And helpers for submitting them to the platform:

```rust
fn check_tx_is_valid(
    platform: &TempPlatform<MockCoreRPCLike>,
    raw_tx: &[u8],
    platform_version: &PlatformVersion,
) -> bool {
    let platform_state = platform.state.load();
    let platform_ref = PlatformRef {
        drive: &platform.drive,
        state: &platform_state,
        config: &platform.config,
        core_rpc: &platform.core_rpc,
    };

    let check_result = platform
        .check_tx(raw_tx, CheckTxLevel::FirstTimeCheck,
                   &platform_ref, platform_version)
        .expect("expected to check tx");

    check_result.is_valid()
}
```

The key insight is that helpers should be *specific to the test file*. A withdrawal
test's helper knows about `AddressCreditWithdrawalTransition`; it does not try to be
a generic state transition factory.

## assert_matches! for Error Checking

Platform tests lean heavily on the `assert_matches!` macro from the `assert_matches`
crate. This is the idiomatic way to verify error variants in a deeply nested enum
hierarchy:

```rust
use assert_matches::assert_matches;

assert_matches!(
    processing_result.execution_results().as_slice(),
    [StateTransitionExecutionResult::UnpaidConsensusError(
        ConsensusError::BasicError(
            BasicError::TransitionNoInputsError(_)
        )
    )]
);
```

Without `assert_matches!`, you would need a verbose `match` block or a chain of
`if let` statements. The macro makes the *expected shape* of the error immediately
visible in the test.

For cases where you need to inspect error fields, combine `matches!` with additional
assertions:

```rust
let error = result.first_error().unwrap();
assert!(
    matches!(
        error,
        ConsensusError::BasicError(
            BasicError::TransitionOverMaxInputsError(e)
        ) if e.actual_inputs() == 17 && e.max_inputs() == 16
    ),
    "Expected TransitionOverMaxInputsError with 17/16, got {:?}",
    error
);
```

The guard clause (`if e.actual_inputs() == 17`) lets you verify both the variant
and its contents in a single expression.

## Processing State Transitions in Tests

The standard way to submit a state transition in unit tests is through
`process_raw_state_transitions`:

```rust
let raw_bytes = transition.serialize_to_bytes().unwrap();

let processing_result = platform
    .platform
    .process_raw_state_transitions(
        &vec![raw_bytes],
        &platform_state,
        &BlockInfo::default(),
        &transaction,
        platform_version,
        false,   // not dry run
        None,    // no extra data
    )
    .expect("expected to process state transition");
```

This is the same code path that runs in production -- your test transition goes
through the same validation pipeline that a real block proposer executes.

## Deterministic Randomness

Tests that need random data always use a seeded RNG:

```rust
let mut rng = StdRng::seed_from_u64(567);
let output_script = CoreScript::random_p2pkh(&mut rng);
```

The seed ensures the test produces identical results every time. If a test fails,
you can reproduce the exact same inputs. Never use `thread_rng()` or entropy-seeded
RNGs in unit tests.

## Feature-Gated Test Compilation

Some tests require features that are expensive or only available in certain contexts.
The `testing-config` feature gate controls test-specific configuration:

```rust
#[cfg(feature = "testing-config")]
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

Tests that need the full platform test infrastructure will not compile without
`--features testing-config`, keeping the main build clean.

## OnceLock for Expensive Resources

When a test suite needs an expensive-to-create resource (like a cryptographic key
that takes 30 seconds to build), the `OnceLock` pattern avoids rebuilding it for
every test:

```rust
use std::sync::OnceLock;

static STATE_TRANSITION_TYPE_COUNTER: OnceLock<Mutex<BTreeMap<String, usize>>>
    = OnceLock::new();

fn state_transition_counter() -> &'static Mutex<BTreeMap<String, usize>> {
    STATE_TRANSITION_TYPE_COUNTER.get_or_init(|| Mutex::new(BTreeMap::new()))
}
```

`OnceLock` is initialized at most once, the first time any test calls it. Because
test threads share the same process, all tests in the binary reuse the same instance.
This pattern is essential for resources like cryptographic proving keys that are
expensive to construct but immutable once built.

## Rules

**Do:**

- Follow the `#[cfg(test)] mod tests { use super::*; }` convention.
- Group tests into sub-modules by validation category.
- Use `TestPlatformBuilder` for any test that needs a platform instance.
- Use `StdRng::seed_from_u64()` for deterministic randomness.
- Use `assert_matches!` for checking error variants.
- Use `OnceLock` for expensive, immutable test resources.
- Process transitions through `process_raw_state_transitions` to test the real code path.

**Don't:**

- Use `thread_rng()` or unseeded randomness in tests.
- Create ad-hoc Platform instances without `TestPlatformBuilder`.
- Write generic helper functions that try to handle all state transition types.
- Skip `set_genesis_state()` unless you are specifically testing pre-genesis behavior.
- Use `unwrap()` on validation results -- use `assert_matches!` to verify error shapes.
