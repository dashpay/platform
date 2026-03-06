# Strategy Tests

Unit tests verify that a single state transition behaves correctly. But what about
testing an entire chain of blocks with hundreds of identities creating documents,
transferring credits, and voting on contested resources -- all at the same time?

That is what strategy tests are for. They are Platform's integration-level simulation
framework: you declare *what should happen* and let the framework simulate it across
hundreds of blocks.

## The Problem

Consider everything that happens in a real Dash Platform network over 100 blocks:

- Masternodes join, leave, get banned, change IPs
- Quorums rotate and sign blocks
- Identities are created, topped up, and updated
- Documents are inserted, replaced, deleted, and transferred
- Contracts are deployed and updated
- Withdrawals are processed and batched
- Protocol upgrades happen mid-chain

Testing any of these in isolation is straightforward. Testing them *together* -- where
the output of block 47 affects the input of block 48 -- requires something more powerful
than a unit test.

## Two-Layer Strategy Architecture

Strategy tests use a two-layer design:

**Layer 1: `Strategy`** (defined in `packages/strategy-tests/src/lib.rs`) describes
*what operations* to perform:

```rust
pub struct Strategy {
    /// Identities to create on the first block.
    pub start_identities: StartIdentities,

    /// Platform addresses to fund on the first block.
    pub start_addresses: StartAddresses,

    /// Contracts to deploy on the second block,
    /// with optional scheduled updates.
    pub start_contracts: Vec<(
        CreatedDataContract,
        Option<BTreeMap<u64, CreatedDataContract>>,
    )>,

    /// Operations to execute each block.
    pub operations: Vec<Operation>,

    /// Configuration for ongoing identity creation.
    pub identity_inserts: IdentityInsertInfo,

    /// Optional nonce gaps for edge-case testing.
    pub identity_contract_nonce_gaps: Option<Frequency>,

    /// Key manager for signing state transitions.
    pub signer: Option<SimpleSigner>,
}
```

**Layer 2: `NetworkStrategy`** (defined in
`packages/rs-drive-abci/tests/strategy_tests/strategy.rs`) wraps a `Strategy` with
*network-level configuration*:

```rust
pub struct NetworkStrategy {
    pub strategy: Strategy,
    pub total_hpmns: u16,
    pub extra_normal_mns: u16,
    pub validator_quorum_count: u16,
    pub chain_lock_quorum_count: u16,
    pub instant_lock_quorum_count: u16,
    pub initial_core_height: u32,
    pub upgrading_info: Option<UpgradingInfo>,
    pub core_height_increase: CoreHeightIncrease,
    pub proposer_strategy: MasternodeListChangesStrategy,
    pub rotate_quorums: bool,
    pub failure_testing: Option<FailureStrategy>,
    pub query_testing: Option<QueryStrategy>,
    pub verify_state_transition_results: bool,
    pub max_tx_bytes_per_block: u64,
    pub independent_process_proposal_verification: bool,
    pub sign_chain_locks: bool,
    pub sign_instant_locks: bool,
    // ...
}
```

The separation is intentional. `Strategy` is about *application-level behavior*
(documents, identities, contracts). `NetworkStrategy` is about *network-level behavior*
(masternodes, quorums, block production). By composing them, you can test the same
application strategy under different network conditions.

## Operations and Frequency

Each operation in a strategy has a type and a frequency:

```rust
pub struct Operation {
    /// The type of operation to perform.
    pub op_type: OperationType,
    /// Configuration controlling how often this operation occurs.
    pub frequency: Frequency,
}
```

`OperationType` is an enum covering every kind of platform action:

```rust
pub enum OperationType {
    Document(DocumentOp),
    IdentityTopUp(AmountRange),
    IdentityUpdate(IdentityUpdateOp),
    IdentityWithdrawal(AmountRange),
    ContractCreate(RandomDocumentTypeParameters, DocumentTypeCount),
    ContractUpdate(DataContractUpdateOp),
    IdentityTransfer(Option<IdentityTransferInfo>),
    ResourceVote(ResourceVoteOp),
    // ... token operations, address operations, etc.
}
```

`Frequency` controls *when* and *how many* operations occur per block:

```rust
pub struct Frequency {
    /// Range for the number of events when a block is selected.
    pub times_per_block_range: Range<u16>,
    /// Probability (0.0 to 1.0) that events occur in a given block.
    pub chance_per_block: Option<f64>,
}
```

For example, `Frequency { times_per_block_range: 1..4, chance_per_block: Some(0.5) }`
means: on each block, there is a 50% chance that 1-3 operations of this type will occur.
This probabilistic scheduling creates realistic, varied block content.

## Running a Strategy: run_chain_for_strategy

The engine that drives everything is `run_chain_for_strategy`, defined in
`packages/rs-drive-abci/tests/strategy_tests/execution.rs`:

```rust
pub(crate) fn run_chain_for_strategy<'a>(
    platform: &'a mut Platform<MockCoreRPCLike>,
    block_count: u64,
    strategy: NetworkStrategy,
    config: PlatformConfig,
    seed: u64,
    add_voting_keys_to_signer: &mut Option<SimpleSigner>,
    add_payout_keys_to_signer: &mut Option<SimpleSigner>,
) -> ChainExecutionOutcome<'a> {
    // ...
}
```

This function:

1. Generates a deterministic RNG from `seed`.
2. Creates the specified number of masternodes and quorums.
3. For each block (up to `block_count`):
   - Determines core height increases.
   - Generates state transitions based on the strategy's operations and frequencies.
   - Simulates ABCI `PrepareProposal` / `ProcessProposal` / `FinalizeBlock`.
   - Applies masternode list changes (joins, leaves, bans).
   - Rotates quorums if configured.
4. Returns a `ChainExecutionOutcome` containing the final state.

The outcome struct captures everything you need to verify:

```rust
pub struct ChainExecutionOutcome<'a> {
    pub abci_app: FullAbciApplication<'a, MockCoreRPCLike>,
    pub masternode_identity_balances: BTreeMap<[u8; 32], Credits>,
    pub identities: Vec<Identity>,
    pub proposers: Vec<MasternodeListItemWithUpdates>,
    pub validator_quorums: BTreeMap<QuorumHash, TestQuorumInfo>,
    pub identity_nonce_counter: BTreeMap<Identifier, IdentityNonce>,
    pub end_epoch_index: u16,
    pub end_time_ms: u64,
    pub state_transition_results_per_block:
        BTreeMap<u64, Vec<(StateTransition, ExecTxResult)>>,
    // ...
}
```

## Masternode List Changes

The `MasternodeListChangesStrategy` allows simulating a dynamic validator set:

```rust
pub struct MasternodeListChangesStrategy {
    pub new_hpmns: Frequency,
    pub removed_hpmns: Frequency,
    pub updated_hpmns: Frequency,
    pub banned_hpmns: Frequency,
    pub unbanned_hpmns: Frequency,
    pub changed_ip_hpmns: Frequency,
    pub changed_p2p_port_hpmns: Frequency,
    pub changed_http_port_hpmns: Frequency,
    pub new_masternodes: Frequency,
    pub removed_masternodes: Frequency,
    pub updated_masternodes: Frequency,
    pub banned_masternodes: Frequency,
    pub unbanned_masternodes: Frequency,
    pub changed_ip_masternodes: Frequency,
}
```

Each field uses `Frequency`, so you can say "ban 1-2 HPMNs per block with 10%
probability" naturally.

## Writing a Strategy Test

Here is a minimal strategy test from the codebase
(`packages/rs-drive-abci/tests/strategy_tests/test_cases/basic_tests.rs`):

```rust
#[test]
fn run_chain_nothing_happening() {
    let strategy = NetworkStrategy {
        strategy: Strategy {
            start_contracts: vec![],
            operations: vec![],
            start_identities: StartIdentities::default(),
            start_addresses: StartAddresses::default(),
            identity_inserts: IdentityInsertInfo::default(),
            identity_contract_nonce_gaps: None,
            signer: None,
        },
        total_hpmns: 100,
        extra_normal_mns: 0,
        validator_quorum_count: 24,
        chain_lock_quorum_count: 24,
        upgrading_info: None,
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: None,
        verify_state_transition_results: false,
        ..Default::default()
    };

    let config = PlatformConfig {
        validator_set: ValidatorSetConfig::default_100_67(),
        chain_lock: ChainLockConfig::default_100_67(),
        instant_lock: InstantLockConfig::default_100_67(),
        execution: ExecutionConfig {
            verify_sum_trees: true,
            ..ExecutionConfig::default()
        },
        block_spacing_ms: 3000,
        testing_configs: PlatformTestConfig::default_minimal_verifications(),
        ..Default::default()
    };

    let mut platform = TestPlatformBuilder::new()
        .with_config(config.clone())
        .build_with_mock_rpc();

    run_chain_for_strategy(
        &mut platform, 100, strategy, config,
        15, &mut None, &mut None,
    );
}
```

This test runs 100 empty blocks with 100 masternodes and verifies that the chain
progresses without errors. It is the "smoke test" for the strategy framework itself.

## Continuing a Chain

Strategy tests support pausing and resuming with `continue_chain_for_strategy`:

```rust
let outcome = run_chain_for_strategy(
    &mut platform, 50, strategy.clone(), config.clone(),
    13, &mut None, &mut None,
);

// Later...
let continued = continue_chain_for_strategy(
    outcome, strategy, config,
    50, // 50 more blocks
    &mut None, &mut None,
);
```

This is invaluable for testing restart scenarios and verifying that state persists
correctly across platform restarts.

## How Strategy Tests Differ from Unit Tests

| Aspect | Unit Tests | Strategy Tests |
|--------|-----------|---------------|
| Scope | One state transition | Hundreds across many blocks |
| Setup | `TestPlatformBuilder` | `run_chain_for_strategy` |
| Randomness | Seeded per-test | Seeded once, flows through blocks |
| Masternodes | Not involved | Fully simulated |
| Quorums | Not involved | Rotated and signed |
| Determinism | Yes | Yes (same seed = same outcome) |
| Speed | Fast (seconds) | Slow (minutes for large chains) |

## Rules

**Do:**

- Use strategy tests for multi-block scenarios involving multiple participants.
- Start with `default_minimal_verifications()` to speed up test execution.
- Use small block counts (10-50) during development, increase for CI.
- Check `state_transition_results_per_block` to verify specific block outcomes.
- Use `continue_chain_for_strategy` for restart/persistence testing.

**Don't:**

- Use strategy tests when a unit test would suffice -- they are much slower.
- Forget to pass a deterministic seed -- non-deterministic strategy tests are useless.
- Set `verify_state_transition_results: true` unless you need it; it adds overhead.
- Create strategy tests with more than a few hundred blocks for regular CI runs.
