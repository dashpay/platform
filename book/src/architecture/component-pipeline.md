# Component Pipeline

This chapter traces a request from the moment it leaves a client application to
the moment its effects are committed to GroveDB. Understanding this pipeline is
essential because every bug, every audit finding, and every performance issue
lives somewhere along this path.

## The Big Picture

```text
Client App
  |
  | gRPC (protobuf)
  v
DAPI (rs-dapi) -----------> Drive-ABCI query service (reads)
  |
  | BroadcastStateTransition
  v
Tenderdash mempool
  |
  | ABCI (check_tx, prepare_proposal, process_proposal, finalize_block)
  v
Drive-ABCI (rs-drive-abci)
  |
  | DriveOperations
  v
Drive (rs-drive)
  |
  | GroveDB transaction
  v
GroveDB -> RocksDB
```

There are two fundamentally different paths through this pipeline:

1. **Reads (queries):** Client sends a gRPC query, DAPI forwards it to
   Drive-ABCI's query service, which reads from GroveDB and returns data with
   a Merkle proof. No consensus involved.

2. **Writes (state transitions):** Client broadcasts a state transition, it
   enters the Tenderdash mempool, goes through consensus, and is applied to
   GroveDB during block processing.

Let's trace the write path in detail -- it is where all the complexity lives.

## DAPI: The Entry Point

DAPI (`packages/rs-dapi`) is the internet-facing gRPC server. It implements two
roles:

- **Platform queries:** Forwarded directly to Drive-ABCI's gRPC service (which
  runs as a separate listener within the same process).
- **State transition broadcast:** Submitted to Tenderdash via its RPC interface.

```text
// Simplified from packages/rs-dapi/src/services/platform_service/broadcast_state_transition.rs
Client --gRPC--> DAPI --Tenderdash RPC--> Tenderdash mempool
```

DAPI itself does minimal validation. It is a routing layer. The heavy lifting
happens in Drive-ABCI.

## The ABCI Handler Layer

When Tenderdash needs the application to do something -- check a transaction,
build a block, validate a proposal, or finalize a block -- it calls an ABCI
method. Drive-ABCI implements these in `packages/rs-drive-abci/src/abci/handler/`:

```rust
// From packages/rs-drive-abci/src/abci/handler/mod.rs
mod check_tx;
mod echo;
mod extend_vote;
mod finalize_block;
mod info;
mod init_chain;
mod prepare_proposal;
mod process_proposal;
mod verify_vote_extension;
```

These handlers are implemented against trait bounds, not concrete types:

```rust
// From packages/rs-drive-abci/src/abci/handler/finalize_block.rs
pub fn finalize_block<'a, A, C>(
    app: &A,
    request: proto::RequestFinalizeBlock,
) -> Result<proto::ResponseFinalizeBlock, Error>
where
    A: PlatformApplication<C> + TransactionalApplication<'a> + BlockExecutionApplication,
    C: CoreRPCLike,
{ ... }
```

The `FullAbciApplication` struct wires everything together. It holds a reference
to `Platform`, a GroveDB transaction, and the current block execution context:

```rust
// From packages/rs-drive-abci/src/abci/app/full.rs
pub struct FullAbciApplication<'a, C> {
    pub platform: &'a Platform<C>,
    pub transaction: RwLock<Option<Transaction<'a>>>,
    pub block_execution_context: RwLock<Option<BlockExecutionContext>>,
}
```

It implements the `tenderdash_abci::Application` trait, delegating each method
to the corresponding handler function.

## Block Processing Lifecycle

Tenderdash uses a proposal-based consensus model. Here is the sequence of ABCI
calls for a single block:

### 1. check_tx -- Mempool Gatekeeper

Before a state transition enters the mempool, Tenderdash calls `check_tx`. This
is a lightweight validation that runs *outside* of consensus:

```rust
// From packages/rs-drive-abci/src/abci/handler/check_tx.rs
pub fn check_tx<C>(
    platform: &Platform<C>,
    core_rpc: &C,
    request: proto::RequestCheckTx,
) -> Result<proto::ResponseCheckTx, Error>
where
    C: CoreRPCLike,
{
    let platform_state = platform.state.load();
    let platform_version = platform_state.current_platform_version()?;

    let validation_result = platform.check_tx(
        tx.as_slice(),
        r#type.try_into()?,
        &platform_ref,
        platform_version,
    );
    // ...
}
```

`check_tx` operates in two modes: mode 0 (new transaction) and mode 1
(re-check existing mempool transactions after a new block). It returns a fee
estimate (`gas_wanted`), a priority for ordering, and a sender identifier for
deduplication. Importantly, `check_tx` does *not* run inside a GroveDB
transaction -- it reads committed state only.

### 2. prepare_proposal -- The Proposer Builds a Block

The block proposer calls `prepare_proposal` with a list of candidate
transactions. The handler starts a GroveDB transaction and runs the full block
proposal:

```rust
// From packages/rs-drive-abci/src/abci/handler/prepare_proposal.rs
// Start a GroveDB transaction
app.start_transaction();

// Run the full proposal (validates + executes all state transitions)
let mut run_result = app.platform().run_block_proposal(
    block_proposal,
    true,         // known_from_us = true (we are the proposer)
    &platform_state,
    transaction,
    Some(&timer),
)?;
```

The response tells Tenderdash which transactions to keep, remove, or delay:

- `TxAction::Unmodified` -- valid transition, include in block
- `TxAction::Removed` -- unpaid error or internal error, strip from block
- `TxAction::Delayed` -- exceeded max block size, try next block

### 3. process_proposal -- Validators Verify the Block

Non-proposing validators receive the block and call `process_proposal`. This
runs the same `run_block_proposal` logic but with `known_from_us = false`:

```rust
// From packages/rs-drive-abci/src/abci/handler/process_proposal.rs
let run_result = app.platform().run_block_proposal(
    (&request).try_into()?,
    false,        // known_from_us = false (we are validating someone else's proposal)
    &platform_state,
    transaction,
    None,
)?;
```

A key optimization: if the validator was also the proposer for this round (same
height and round), the cached result from `prepare_proposal` is reused:

```rust
// From process_proposal.rs -- cache hit path
if let Some(proposal_info) = block_execution_context.proposer_results() {
    return Ok(proto::ResponseProcessProposal {
        status: proto::response_process_proposal::ProposalStatus::Accept.into(),
        app_hash: proposal_info.app_hash.clone(),
        tx_results: proposal_info.tx_results.clone(),
        // ...
    });
}
```

If the proposal contains failed or unpaid transitions, the validator rejects it.

### 4. finalize_block -- Commit

After consensus is reached, Tenderdash calls `finalize_block`. This is where
the GroveDB transaction is committed to disk:

```rust
// From packages/rs-drive-abci/src/abci/handler/finalize_block.rs
let block_finalization_outcome = app.platform().finalize_block_proposal(
    request_finalize_block,
    block_execution_context,
    transaction,
    platform_version,
)?;

// Commit the GroveDB transaction
let result = app.commit_transaction(platform_version);
```

After commit, the block height counter is updated and, if needed, a GroveDB
checkpoint is created for crash recovery.

## Inside run_block_proposal

The `run_block_proposal` method in `packages/rs-drive-abci/src/execution/engine/run_block_proposal/v0/mod.rs`
is the heart of the block processing engine. It orchestrates everything that
happens within a single block. Here is the sequence, in order:

```text
1.  Verify protocol version matches expected version
2.  Validate block follows previous block (height, core height)
3.  Clear drive block cache
4.  Verify chain lock (if core chain lock update present)
5.  Update core info (masternode list, quorums)
6.  Update validator proposed app version
7.  Rebroadcast expired withdrawals
8.  Update broadcasted withdrawal statuses
9.  Dequeue and build unsigned withdrawal transactions
10. Run DAO platform events (vote tallying, contested documents)
11. Process raw state transitions  <-- the main work
12. Store address balance changes
13. Clean up expired address balance entries
14. Pool withdrawals into transaction queue
15. Clean up expired withdrawal amount locks
16. Process block fees and validate sum trees
17. Compute root hash (app_hash)
18. Determine validator set update
```

Steps 1-10 are "block-level housekeeping." Step 11 is where individual state
transitions are decoded, validated, transformed into actions, and applied to
GroveDB. Steps 12-18 finalize the block's effects.

## State Transition Processing

State transition processing (step 11 above) follows its own pipeline within
`packages/rs-drive-abci/src/execution/platform_events/state_transition_processing/`:

```text
decode_raw_state_transitions
       |
       v
process_raw_state_transitions
       |
       v
   For each transition:
       |
       +-> validate (structure, state, signatures)
       +-> transform_into_action
       +-> validate_fees_of_event
       +-> execute_event (apply DriveOperations to GroveDB)
```

The validation itself is split into modules per transition type:

```rust
// From packages/rs-drive-abci/src/execution/validation/state_transition/state_transitions/mod.rs
pub mod batch;
pub mod identity_create;
pub mod identity_credit_transfer;
pub mod identity_credit_withdrawal;
pub mod identity_top_up;
pub mod identity_update;
pub mod data_contract_create;
pub mod data_contract_update;
pub mod masternode_vote;
// ... and more
```

Each module provides versioned validation methods dispatched through
`PlatformVersion`, following the same pattern shown in the Introduction.

## The Query Path

Queries bypass consensus entirely. Drive-ABCI implements the Platform gRPC
service directly:

```rust
// From packages/rs-drive-abci/src/query/service.rs
// Implements dapi_grpc::platform::v0::platform_server::Platform
// with methods like:
//   get_identity()
//   get_documents()
//   get_data_contract()
//   get_identity_balance()
//   ... (50+ query endpoints)
```

Each query reads from the committed GroveDB state (no transaction), generates
a Merkle proof, and returns both the data and the proof. The client SDK uses
`drive-proof-verifier` to independently verify that the returned data matches
the proof against a known root hash.

## Protocol Version Upgrades During Block Processing

One subtlety worth highlighting: protocol version changes happen at epoch
boundaries. The `run_block_proposal` method checks whether the current block is
the first block of a new epoch and whether the locked-in next protocol version
differs from the current one:

```rust
// From packages/rs-drive-abci/src/execution/engine/run_block_proposal/mod.rs
let block_platform_version = if epoch_info.is_epoch_change_but_not_genesis()
    && platform_state.next_epoch_protocol_version()
        != platform_state.current_protocol_version_in_consensus()
{
    let next_protocol_version = platform_state.next_epoch_protocol_version();
    let next_platform_version = PlatformVersion::get(next_protocol_version)?;

    // Perform structural changes for the new protocol version
    self.perform_events_on_first_block_of_protocol_change(
        platform_state,
        &block_info,
        transaction,
        old_protocol_version,
        next_platform_version,
    )?;

    next_platform_version
} else {
    last_committed_platform_version
};
```

This ensures that all nodes switch protocol versions at exactly the same block,
and that any structural migrations (new GroveDB trees, schema changes) are
applied atomically as part of that block's transaction.

## Rules

**Do:**
- Keep ABCI handlers thin. They should parse the request, delegate to the
  execution engine, and format the response. No business logic.
- Always pass `platform_version` through the call stack. Never hard-code a
  version number in execution logic.
- Run `check_tx` validation as a strict subset of proposal validation. If
  `check_tx` accepts a transition, `process_proposal` should not reject it
  (modulo state changes between the two calls).

**Don't:**
- Read uncommitted state in `check_tx`. It reads committed state only because
  it runs outside the block transaction.
- Assume `prepare_proposal` and `process_proposal` see the same state. Another
  block may have been committed between the two calls if a round change occurs.
- Add new block-level events without inserting them in the correct position in
  the `run_block_proposal` sequence. Order matters -- withdrawals must be
  processed before state transitions, fee accounting must happen after.
- Panic in ABCI handlers except for truly unrecoverable situations (like
  app hash mismatches, which indicate data corruption).
