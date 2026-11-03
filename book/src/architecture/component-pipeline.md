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
to `Platform`, a GroveDB transaction, the current block execution context, and
the withdrawal transactions of every proposal accepted at the current height,
which vote extensions are verified against:

```rust
// From packages/rs-drive-abci/src/abci/app/full.rs
pub struct FullAbciApplication<'a, C> {
    pub platform: &'a Platform<C>,
    pub transaction: RwLock<Option<Transaction<'a>>>,
    pub block_execution_context: RwLock<Option<BlockExecutionContext>>,
    pub unsigned_withdrawal_txs_by_round: RwLock<UnsignedWithdrawalTxsByRound>,
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

The finalize response also carries a proposer hint,
`propose_next_block_immediately` (Tenderdash 1.8.0, ABCI 1.4.0). Drive sets it
when the block leaves
withdrawal work for the next block: untied withdrawal transactions waiting in
the queue to be signed. Tenderdash then proposes round 0 of the next height without waiting
for transactions or the empty-block interval, so a withdrawal is signed one
block after it was pooled instead of one interval later. The hint is local to
the node and never part of consensus: it is read from the same GroveDB
transaction the block committed, but it does not change the state or the app
hash. See `has_pending_withdrawal_work` under
`packages/rs-drive-abci/src/execution/platform_events/withdrawals/`.

## Inside run_block_proposal

The `run_block_proposal` method in `packages/rs-drive-abci/src/execution/engine/run_block_proposal/v0/mod.rs`
is the heart of the block processing engine. It orchestrates everything that
happens within a single block. Here is the sequence, in order:

```text
1.  Verify protocol version matches expected version
2.  Validate block follows previous block (height, core height)
3.  Clear drive block cache
4.  Verify chain lock (if core chain lock update present)
5.  Update core info (Core fork activations, masternode list, quorums)
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

## Syncing with Dash Core

Steps 4 and 5 are where a block first meets Dash Core, and several later steps
read Core too. Every Platform validator is an evonode: a Dash Core masternode
that also runs Tenderdash and drive-abci. drive-abci asks that node's Core, over
RPC, about masternodes, quorums, withdrawals and asset locks. Each validator
asks its own Core, at its own moment, and those Cores are never exactly in step,
yet every validator must get the same answer. The rule that makes this work:
consensus code reads Core at the block's chain-locked core height, never at
Core's tip.

Two Core concepts carry the argument. A **quorum** is a group of masternodes
that Core selects deterministically to produce threshold signatures together. A
**chain lock** is a quorum's signature over the Core block at one height. A
quorum signs only one block per height, so the network has a single
chain-locked block at each height, and Core treats it as final: it never
reorganizes a chain-locked block away. Core produces a block about every 2.5
minutes, far less often than Platform.

### The chain-locked core height

Every block carries a chain-locked core height, written `c` below
(`core_chain_locked_height` in the proposal, `block_info.core_height` while the
block runs). It is part of the block, so every validator executes the block at
the same `c`, wherever its own Core's tip happens to be.

`c` moves only in a block that carries a chain lock update. Tenderdash enforces
this: a block with an update sets `c` to the update's height, which must be
above the previous block's `c`, and a block without one keeps the previous
block's `c`. drive-abci also checks, on its own, that `c` never goes down
(`next_block_to` in `packages/rs-drive-abci/src/execution/types/block_state_info/v0/mod.rs`).
Core blocks are rare next to Platform blocks, so most blocks carry no update.

The proposer takes the update from its own Core. `prepare_proposal`
(`packages/rs-drive-abci/src/abci/handler/prepare_proposal.rs`) asks Core for
its best chain lock and proposes it when its height is above the previous
block's `c` (in the chain's first block, whatever its height). When Core has
nothing newer, or cannot be reached, the block keeps the previous block's `c`.
Choosing the update this way is not a consensus read: the chain lock goes into
the block, and every other validator verifies it instead of asking its own Core
for one.

Init chain has no chain lock update. `init_chain`
(`packages/rs-drive-abci/src/execution/engine/initialization/init_chain/v0/mod.rs`)
uses the genesis core height: the one in the genesis file or, by default, the
height at which Core activated `mn_rr`, the Core fork Platform requires. It
waits until that height is at or below Core's best chain lock
(`initial_core_height_and_time`), then runs `update_core_info` there. Block 1
treats the genesis core height as the previous block's `c`.

### Verifying the chain lock and syncing Core to it

A validator that did not propose the block checks the update before anything
reads Core. This is step 4, `verify_chain_lock` in
`packages/rs-drive-abci/src/execution/platform_events/core_chain_lock/`:

1. It checks the signature against the chain lock quorums in Platform's state.
   When those cannot settle it, it leaves the check to its Core.
2. It submits the chain lock to its Core (`make_sure_core_is_synced_to_chain_lock`,
   Core's `submitchainlock`). Core answers with the height of the best chain
   lock it now holds.
3. If that height is at or above `c`, Core is synced. If it is one or two blocks
   short (`recent_block_count_amount`), drive-abci asks for Core's best chain
   lock up to five more times, 200 ms apart. If Core is further behind, or
   still short after that, the proposal is refused with
   `ChainLockedBlockNotKnownByCore`.

A chain lock that fails the check is refused with `InvalidChainLock`, and so is
one the validator cannot check or submit because a Core call fails. A Core too
far behind to know the quorum that signed the chain lock lands here too:
`submitchainlock` then answers with an error. Both refusals are rejections:
`process_proposal` answers `Reject`, the validator does not vote for the
proposal, and consensus moves on to another round. The proposer skips these
checks, because the chain lock came from its own Core.

### Why every node's Core answers about the same block

Once a Core holds a chain lock and has the block it names, it puts that block
on its active chain and never reorganizes it away, and the lock covers every
block below it too. So once a validator's Core holds a chain lock at or above
`c` and has the block it names, the block at `c` on its active chain is the
network's single chain-locked block at `c`. `getblockhash(c)`, and every read
keyed by `c` or a lower height, answers about that block on every validator. A
read at Core's tip would instead answer about whichever block each Core had
reached.

One gap remains: Core accepts a chain lock before it has the block, and counts
it in the height `submitchainlock` reports. Such a Core usually has no block at
`c` yet, so a read at `c` fails and the block fails on that node (see
[When Core fails](#when-core-fails)). If its active chain holds a competing
block at `c` instead, the read answers about that block, and that node's results
for the block can differ from everyone else's. A difference in what it writes to
GroveDB shows at once, because the app hash is GroveDB's root hash; a difference
kept only in Platform state, which the app hash does not cover, can stay hidden
longer. Checking that Core's block at `c` is the one the chain lock names,
before any read at `c`, would close the gap; drive-abci does not check it.

### What Platform reads from Core

Two places keep what Platform learns from Core. GroveDB holds the state the app
hash commits to. Platform state (`PlatformState`) is drive-abci's view of the
chain around it: the masternode list, the quorums and validator sets, the
protocol versions and the last committed block. It is saved after every block in
GroveDB's auxiliary storage, which the app hash does not cover.

Several reads use the coinbase payload: the first transaction of every Core
block, the coinbase, carries a versioned payload with commitments such as the
credit pool balance.

`update_core_info` (step 5,
`packages/rs-drive-abci/src/execution/platform_events/core_based_updates/update_core_info/`)
runs at init chain and in every block that moves `c`. In a block that keeps
`c` it returns before asking Core anything. It reads, in this order (paths
relative to `core_based_updates/`):

| Read | Asked of Core | Code | Kept in |
|---|---|---|---|
| Core fork activations, from protocol version 15 | The coinbase payload version of the block at `c`, until the fork is recorded | `update_core_info/v1/` | GroveDB `Misc`; see [Core Fork Activations](core-fork-activations.md) |
| Masternode list | `protx listdiff` from the previous block's `c` to `c`; also the coinbase payload version at `c` when a reported operator key may only be re-encoded | `update_masternode_list/` | Platform state, and masternode identities in GroveDB |
| Quorums | `quorum listextended` at `c`, `quorum info` for each new quorum | `update_quorum_info/` | Platform state: the validator sets and the chain lock and instant lock quorums |

Other steps read Core at `c` or below too. The paths in the first two rows are
under `packages/rs-drive-abci/src/execution/platform_events/withdrawals/`:

| Step | Read | Asked of Core | Code | Kept in |
|---|---|---|---|---|
| 14 | Core's credit pool balance (the Dash locked on Core that backs Platform credits), from protocol version 14 | The coinbase of each Core block up to `c` not yet recorded, back to the start of the withdrawal limit's window, at most 32 per block | `scan_core_blocks_for_withdrawals/`; `calculate_core_anchored_withdrawal_limit/` asks Core itself only for a height not yet recorded | GroveDB, `WithdrawalTransactions (80) / [6]`, one balance per core height |
| 8 | The statuses of the asset unlocks Platform broadcast (the Core transactions that pay out withdrawals) | `getassetunlockstatuses` at `c`, in blocks that move `c` | `update_broadcasted_withdrawal_statuses/` | The withdrawal documents |
| 11 | Chain asset lock proofs (Dash locked on Core to fund an identity, proven by the chain-locked height that includes it) | The asset lock transaction, which must be mined at or below the proof's core height, itself no higher than Platform's last committed core height | `packages/rs-drive-abci/src/execution/validation/state_transition/common/asset_lock/` | What the state transition writes |

### When Core fails

Core RPC calls retry for about 11 seconds on connection errors and on the
errors Core returns while it is still starting, and give up on any other error
at once (`retry!` in `packages/rs-drive-abci/src/rpc/core.rs`). Once the
retries are spent, no step reads the error as a default such as "not active",
"no change" or a zero balance. What happens next depends on the step:

- In step 4, the proposal is refused with `InvalidChainLock`, as above.
- For a chain asset lock proof, the state transition fails with an internal
  error: the proposer leaves it out of its block, and a validator rejects a
  proposal that contains it.
- Everywhere else, including `update_core_info`, the credit pool reads and the
  asset unlock statuses, the step returns the error and the node cannot execute
  the block. drive-abci hands the error to Tenderdash as an exception, which
  Tenderdash treats as fatal: it stops on that node, and after a restart the
  node fails the same block again until its Core answers. If that holds for
  more than a third of the validators, the chain stops until their Cores
  recover.

A rejection is the node's verdict on the proposal, and consensus moves on to
another round. An exception means the node could not judge the block at all, so
it does not vote on it either way.

In a block that keeps `c`, `update_core_info` asks nothing, the credit pool scan
asks nothing once it has caught up, and only a state transition with a chain
asset lock proof still reads Core. So a proposer whose Core is unreachable still
proposes, at the previous block's `c`, and the chain goes on through short Core
outages.

### Waiting for Core at startup

Before drive-abci opens its ABCI port, `wait_for_core_to_sync`
(`packages/rs-drive-abci/src/core/wait_for_core_to_sync/`) polls Core every 5
seconds until Core has a chain lock and reports its blockchain and masternode
sync complete. It then makes one test read, the credit pool balance at Core's
best chain lock, so that a Core whose RPC whitelist does not allow
`getspecialtxes` stops the node at startup rather than at the first block that
needs it. Tenderdash does not start before the ABCI port is open. The wait is
not part of consensus: it only keeps a node from joining before its Core can
answer.

### Adding a Core read

A new read of Core in block execution follows the same rules:

- **Go through `CoreRPCLike`** (`packages/rs-drive-abci/src/rpc/core.rs`), so
  tests can answer the call with `MockCoreRPCLike`.
- **Read at `c`.** Ask about `block_info.core_height`, or a lower height,
  inside the block. Never ask about Core's tip, its mempool, or anything else
  whose answer depends on when it is asked. `get_fork_info` reads Core's
  deployments at its tip; it is fit for choosing the genesis core height and
  for nothing inside a block. Proposing a chain lock reads Core's best chain
  lock, but that answer goes into the block, where every other validator
  verifies it.
- **Keep nothing between blocks except in state.** No field on `Platform` that
  holds an answer, or something derived from one. A failed round throws its
  transaction away, but such a field would keep what that round read, and a
  restarted node would start without it. A cache of Core's raw answers keyed by
  a chain-locked height, like the prefetcher in
  `packages/rs-drive-abci/src/rpc/prefetch.rs`, is fine: it returns what Core
  would return anyway.
- **A Core error fails the block.** Return it; never turn it into a default. An
  answer that cannot be read is an error too (`DashCoreBadResponseError`).
- **Store what later blocks depend on.** When a later block's outcome depends
  on the answer, keep it in GroveDB, where the app hash covers it, as the credit
  pool scan and the fork record do, instead of asking Core again later.
  Platform state also survives restarts, but the app hash does not cover it, so
  a wrong answer kept there can go unnoticed.
- **Prefer blocks that move `c`.** A read in a block that keeps `c` makes every
  block depend on Core, and a proposer whose Core is unreachable can then no
  longer propose.
- **Allow the RPC for drive-abci.** dashmate gives drive-abci a Core RPC user,
  `drive_consensus`, with a method whitelist
  (`packages/dashmate/configs/defaults/getBaseConfigFactory.js`). A new method
  goes in it, with a config migration for existing nodes; otherwise every node
  fails the first block that makes the call.
- **It is consensus behaviour.** A new read, or a change in what an answer
  decides, is a new generation selected by the unreleased protocol version, like
  any other behaviour change.

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
