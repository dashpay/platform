# Block Failure Classes

Dash Platform is a replicated state machine: every masternode executes every
block and must arrive at the same app hash. That constraint turns "something
went wrong while executing a block" into a question that has to be answered
before anything else: went wrong *where*, and *for whom*? A failure that only
one node sees, a failure every node sees and agrees on, and a failure every
node sees and cannot get past have nothing in common except the word, and
handling one of them like another is how a bug report becomes a network
outage.

This chapter names the three classes, shows where each one surfaces in
`rs-drive-abci` and in Tenderdash, states what the network does about each,
and reports honestly where the existing recovery path ends. The class
boundaries matter more than ever once contracts run inside blocks: a guest
that traps, a node that cannot load a compiled module, and a scheduled job
whose host integration is broken must land in three different classes.

The strategy tests in
`packages/rs-drive-abci/tests/strategy_tests/test_cases/scheduled_host_fault_tests.rs`
pin each class against the real block-execution entry points. See
[Rehearsal tests](#rehearsal-tests) for how to run them.

## Why classification comes first

The block loop in
`packages/rs-drive-abci/src/execution/engine/run_block_proposal/v0/mod.rs`
runs a fixed sequence of per-block events around the state transitions (the
[Component Pipeline](component-pipeline.md) chapter lists all eighteen steps).
Every per-block event uses a bare `?`. An `Err` from one of them aborts the
whole proposal on the node that saw it, whereas an `Err` from processing one
state transition does not. That asymmetry is deliberate, and it is the seam
between class 1 and class 3 below.

The other seam runs between "this node" and "every node". Block execution is
deterministic by design, so most `Err` values are reproduced by every
validator. Some are not: they come from this node's Core connection, this
node's disk, this node's caches. Those are class 2, and they are the only
failures a restart fixes.

## Class 1: deterministic paid failure

**What it is.** A state transition fails validation or execution in a way
every node reproduces, and the transition had an identity or funded address
to pay with.

**Where it surfaces.** `process_raw_state_transitions` turns the outcome of
each transition into a `StateTransitionExecutionResult`
(`packages/rs-drive-abci/src/platform_types/state_transitions_processing_result/mod.rs`):

```text
SuccessfulExecution      applied, charged
PaidConsensusError       rejected, charged (the "paid failure")
UnpaidConsensusError     rejected, nobody to charge
InternalError            the node could not evaluate it
NotExecuted              the proposer ran out of time
```

A returned `Err` from processing a single transition never escapes the loop.
`process_raw_state_transitions_v0` maps it to an `InternalError` result and
carries on with the next transition.

**What the network does.** The proposer maps results to transaction actions in
`packages/rs-drive-abci/src/abci/handler/prepare_proposal.rs`: successful and
paid-invalid transitions stay in the block (`TxAction::Unmodified`), unpaid and
internal-error transitions are stripped (`TxAction::Removed`) and their writes
rolled back to a savepoint, delayed transitions wait for the next block.
Validators in `process_proposal.rs` reject any block that still contains an
internal-error or unpaid result. A paid failure therefore stays in the block,
the owner pays, the chain advances, and nothing halts.

**For contracts.** A guest trap, an explicit rejection, a bounds or budget
overrun, a failed native check on the way in, and a scheduled job's failed
attempt are all class 1. They produce a result and a charge. They are never
allowed to become a block-level `Err`.

## Class 2: node-local failure

**What it is.** One node cannot execute a block that the rest of the network
executes normally. The block is valid; this node is not ready.

**Where it surfaces.** Three shapes exist in the tree:

- *Chain lock verification.* `run_block_proposal_v0` verifies a proposed chain
  lock against Core. A Core RPC error, an invalid signature, or a Core that is
  not yet synced to the locked height all produce a `ValidationResult` error
  (`InvalidChainLock`, `ChainLockedBlockNotKnownByCore`). The handler answers
  Tenderdash with `ProposalStatus::Reject`. Tenderdash's `BlockExecutor`
  turns a rejection into `ErrBlockRejected`
  (`internal/state/execution.go`, tag v1.7.0), and the prevoter's
  `handleError` in `internal/consensus/prevoter.go` prevotes nil for exactly
  that error instead of panicking. The node abstains on this round and catches
  up when Core does.
- *Storage and cache errors.* A GroveDB, RocksDB or cache `Err` from a
  per-block event aborts the proposal with an `Err`. `FullAbciApplication`
  (`packages/rs-drive-abci/src/abci/app/full.rs`) maps it through
  `error_into_exception` into a `ResponseException`. Tenderdash panics on an
  exception from `PrepareProposal` (`create` in
  `internal/consensus/block_executor.go`) and from `ProcessProposal`
  (`mustEnsureProcess` in the same file, and the unknown-error branch of
  `handleError`). The node goes down until an operator repairs it.
- *App hash mismatch.* `prepare_proposal` and `process_proposal` compare the
  app hash in memory with the root hash on disk and panic on a mismatch, so
  the node restarts and reloads its state from disk. A restart heals this one
  by itself.

**What the network does.** Nothing, and that is the point. Block validity
never depended on the failed node. Once repaired, the node re-executes the
same block from its committed state and reaches the same app hash as everyone
else. The other validators kept producing blocks in the meantime and the
repaired node catches up through Tenderdash block sync.

**For contracts.** A missing compiled artifact, a failed allocation, a host
resource the node could not acquire, are class 2. They must surface as a
block-level `Err` on that node (so the node stops and says why), never as a
paid result (which would charge a user for the node's problem) and never as an
invalid block (which would make the network vote against a valid block).

## Class 3: reproducible per-block fault

**What it is.** A per-block event fails on every node, on every round, for
every proposal at that height, including empty proposals. The failure is in
the code every node runs, not in one node's environment.

**Where it surfaces.** Any bare `?` in the per-block events of
`run_block_proposal_v0`, or a panic anywhere on that path (the panic hook in
`packages/rs-drive-abci/src/main.rs` cancels the node; there is no
`catch_unwind`). The scheduled-event integration point sits in this sequence,
after `run_dao_platform_events` and before `process_raw_state_transitions`.
The test-only hook `PlatformTestConfig::scheduled_event_host_fault` marks that
position; compiled only with the `testing-config` feature, it stands in for a
defect in the jobs phase that every node reproduces.

**What the network does.** Every proposer's `PrepareProposal` fails, every
validator's `ProcessProposal` fails, Tenderdash panics on each, and on restart
`catchupReplay` in `internal/consensus/replay.go` re-dispatches every
write-ahead-log message of the unfinished height, so the same proposal reaches
the same binary and fails the same way. The one replay option,
`WalSkipRoundsToLast`, fast-forwards rounds within a height; nothing skips a
message or a height. Dropping transactions cannot help because the fault fires
before any transaction is looked at. Block production stops. This is a halt.

**For contracts.** A defect in how the node drives scheduled jobs (the queue
walk, the payer check, the host wiring) is class 3 if every node hits it. The
jobs phase must therefore be fallible only through `Err`, never through
`unwrap` or indexing, so that the class is at least observable and does not
become a panic loop.

## Why a signalling upgrade cannot progress on a halted chain

The normal protocol upgrade is the selected recovery policy for anything that
changes execution or fees, so it is worth being exact about what it needs.
Three things happen, each inside a block:

1. **The vote is written by a block.** `run_block_proposal_v0` records the
   proposer's `proposed_app_version` through
   `update_validator_proposed_app_version` inside the block transaction. A
   proposal that fails at the scheduled-event point never reaches commit, so
   the vote it carried is rolled back with everything else.
2. **The tally runs on an epoch-change block.**
   `upgrade_protocol_version_on_epoch_change_v0`
   (`packages/rs-drive-abci/src/execution/platform_events/protocol_upgrade/upgrade_protocol_version/v0/mod.rs`)
   counts the votes and sets the next epoch's protocol version only when the
   block it runs in is the first block of a new epoch. That block runs the
   same per-block events as every other block, including the failing one.
3. **Activation is a later block.** `run_block_proposal` in
   `packages/rs-drive-abci/src/execution/engine/run_block_proposal/mod.rs`
   switches to the next protocol version on the first block of the epoch after
   the one that locked it in.

When every proposal fails, none of the three blocks can be produced. The
strategy test
`reproducible_scheduled_fault_halts_every_node_before_ordinary_transactions`
drives this: with the fault armed on two nodes, proposals from two proposers,
three rounds, with and without transactions, signalling a higher version, and
one past the epoch boundary all fail, the committed state does not move, the
versions counter holds no vote for the signalled version, and the next epoch
protocol version is unchanged.

## The existing recovery path, as far as the evidence goes

**What exists and is rehearsed: an execution-and-fee-identical hotfix.** If
the defect can be repaired without changing what any block computes, the
repair is a software release, not a protocol change. Every validator installs
it and restarts. The write-ahead log re-delivers the proposal that faulted,
the fixed code executes it to the app hash the network expects, finalize
runs, and the chain resumes at the height where it stopped. No protocol
version, no state surgery, no special binary, no coordination beyond "everyone
installs the same release".

The strategy test
`execution_identical_hotfix_replays_the_faulted_proposal_and_the_chain_continues`
rehearses exactly this on twin platforms. A control node executes a concrete
proposal (non-empty, signalling an upgrade, first block of a new epoch). A
faulted node fails it as proposer and twice as validator, leaving committed
state, height and the versions counter untouched. The faulted node is then
disarmed and reopened from disk, processes the identical request, and reaches
the control node's app hash and transaction results. Both finalize through the
normal handler, both hold the same single vote in the versions counter, the
identities the proposal created exist on both, and five further blocks keep
the two in step.

**What the historical exceptions in the tree are.** The code carries a few
network-and-height gated special cases, and it is easy to read them as a
precedent for restarting a halted chain. They are not:

- The `evo1` height gates in `abci/handler/{info,prepare_proposal,process_proposal}.rs`
  skip the app-hash check below mainnet height 33000, and
  `abci/handler/finalize_block.rs` tolerates a specific RocksDB "busy" commit
  error at heights 32326 to 32328. Both preserve the *replay* of a history the
  network had already committed while a Tenderdash bug let it proceed
  (platform#2309, tenderdash#966). They let a fresh node reproduce blocks that
  already exist; they never restarted a chain that had stopped.
- `consensus_params_update` (`execution/engine/consensus_params_update/{v0,v1}`)
  pushes an emergency consensus parameter update on the first block of mainnet
  epoch 3 and testnet epoch 1480. It is *returned from* block processing, so
  it required those blocks to be produced.
- `check_for_desired_protocol_upgrade_v0` lowered the signalling threshold to
  51 percent for the move to protocol version 3 on mainnet and testnet. It is
  *evaluated inside* the epoch-change block, so it too required blocks.

They are evidence that coordinated releases have been done before. None of
them is a procedure for a fault that fails every block.

**The limitation.** A repair that changes execution or fees needs the normal
protocol upgrade process, and that process needs committed blocks: a block to
carry the vote, an epoch-change block to tally it, a later block to activate
it. When the fault stops every block, no progressing path has been
demonstrated under the selected policy. The owner has explicitly declined an
emergency pause, an activation-height override, a recovery binary and a
guest-skip rule, so this chapter does not propose one. It records the gap so
that the jobs phase is designed with it in mind: keep class 3 faults
observable, keep them out of the `unwrap` family, and keep the scheduled phase
narrow enough that a hotfix is the likely repair.

## Rehearsal tests

```bash
cargo test -p drive-abci --test strategy_tests scheduled_host_fault
```

The module runs four simulations, each on platforms built from one seed so a
healthy node and a faulted node can be compared block for block:

| Test | Class | What it pins |
|---|---|---|
| `paid_failure_stays_in_the_block_pays_and_the_chain_advances` | 1 | a contract update with a skipped position is charged and the chain reaches block 10 |
| `node_local_fault_stops_only_that_node_and_it_commits_the_same_block_after_repair` | 2 | one node fails the network's block, is repaired, re-executes it to the same app hash and results, both finalize |
| `reproducible_scheduled_fault_halts_every_node_before_ordinary_transactions` | 3 | every proposal fails on every node, nothing commits, no vote is recorded, the next epoch version does not move |
| `execution_identical_hotfix_replays_the_faulted_proposal_and_the_chain_continues` | 3 recovery | the disarmed node reopened from disk replays the faulted proposal to the control node's result and the chain continues |

Two unit tests next to the dispatcher in
`packages/rs-drive-abci/src/execution/engine/run_block_proposal/mod.rs` pin
the hook's shape: armed, `run_block_proposal` returns `Err` with the injected
message for an empty proposal; unarmed, the same proposal is valid.

## Rules

**Do:**

- Classify before you handle. Ask "does every node see this?" and "was the
  transition able to pay?" before choosing between a result, an `Err` and a
  rejection.
- Return `Err` from the scheduled phase for anything that is not a job's own
  outcome. Never `unwrap`, `expect` or index without a bounds check on that
  path: a panic turns an observable class 3 fault into a panic loop.
- Treat a scheduled job's trap, rejection or budget overrun as a class 1
  result: recorded, charged, never a block-level failure.
- Keep node-local failures node-local. A missing artifact or a failed
  allocation must surface as an `Err` on that node, not as a charge to the
  user and not as an invalid block.

**Don't:**

- Turn a class 3 `Err` into a paid result to "keep the chain moving". The
  charge would be for a fault the user did not cause, and the block would
  still have to agree on it.
- Turn a class 2 failure into an invalid block. The block is valid; the node
  is not ready.
- Read the historical `evo1` gates or the emergency consensus parameter
  updates as a recovery procedure. They preserved replay of committed history
  or ran inside blocks that were being produced.
