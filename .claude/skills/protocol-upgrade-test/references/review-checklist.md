# Protocol upgrade review checklist

Use this checklist to produce evidence, not yes/no guesses. Record the source
path and relevant method/version for every answer.

## Version and dispatch

- For a released upgrade, verify both inputs are exact release tags, resolve
  their peeled commit SHAs, and confirm their Dashmate/package versions match.
  Treat a pinned development-branch SHA as pre-release evidence only.
- Identify the old and new `PlatformVersion` entries and the complete method-map
  delta, not only the first-block transition.
- Trace the caller that selects a block's protocol version from committed
  state.
- Trace desired-version signaling, vote replacement, active-validator
  filtering, threshold arithmetic, signal clearing, and `next` persistence.
- State the exact signal, lock-in, and activation epochs. Confirm which
  protocol executes each boundary block.
- Check behavior for an unknown, unsupported, skipped, or non-sequential target
  protocol.

## First-block transition

- Inventory every contract/tree read and write.
- Inventory cache, singleton, filesystem, metric, and external-service changes.
- Mark which effects are covered by the candidate GroveDB transaction.
- Check transaction drop, proposal rejection, alternate round, retry, and
  repeated invocation.
- Check version arguments on all nested calls. Flag ambient/latest-version
  lookups in consensus code.
- Confirm errors fail the block loudly without partially publishing
  non-transactional state.
- Verify costs and operation cardinality are bounded or explicitly measured.

## Cache consistency

- Can the transition publish a new schema before commit?
- Can query/check-tx execute concurrently with proposal processing?
- Does every cache API that accepts `PlatformVersion` return that version after
  speculative loading of another version?
- Are multiple schema revisions retained where old-version reads remain legal?
- Can concurrent cache writers lose a revision or expose a pointer before its
  backing map?
- Are restart initialization and activation reload behavior equivalent?

## Existing data classification

- Class N: no persisted first-block migration; complete method-map or runtime
  cache changes may still alter behavior.
- Class A: no enumeration of existing records; schema-bounded writes only.
- Class B: reads/validates existing records but does not rewrite them.
- Class C: rewrites, deletes, moves, backfills, or rebuilds records/indexes.

For N, exercise the real boundary lifecycle and use populated synthetic data
for method-map behavior changes. For A, require a populated synthetic fixture
when behavior changes across the boundary. For B, define representative data
and decide whether cardinality or production-only invariants require authentic
state. For C, require a checksummed committed artifact per target network and a
machine-readable invariant manifest covering counts, roots, index coverage,
expected deltas, rollback, and retry. A transition with multiple kinds of work
inherits the highest-risk applicable class.

Never treat a lone ABCI database as a live-network replay. A live replay also
needs coherent Tenderdash, Core, validator, height, and app-hash state.

## Deterministic test matrix

- Below-threshold, exact-threshold, and above-threshold signaling.
- Duplicate proposals by one validator do not inflate the vote count.
- Lock-in block: `current=old`, `next=new`, old method map still executes.
- Restart after lock-in: persisted `current` and `next` are unchanged.
- Activation block: transition runs through the public proposal/finalize/commit
  path under the new complete method map.
- Populated old-version data and its indexes remain readable.
- Rejected candidate: committed root/state stay unchanged.
- Retry matches a clean control from the same committed snapshot.
- Restart after activation preserves protocol state and query results.
- Post-upgrade operation exercises the new schema/index and checks exact
  history/count effects.
- Old software behavior at activation is explicit and isolated from the live
  quorum.

When review finds a defect, capture a focused red-to-green regression before
adding a broader lifecycle assertion.

## Dashmate release rehearsal

- Use an owned worktree and isolated `DASHMATE_HOME_DIR`; preserve dirty user
  work.
- Treat `yarn setup` as both build and environment configuration. It may enable
  source builds or attempt to create/start a network.
- Disable source builds for Drive, RS-DAPI, and Dashmate Helper when validating
  published images.
- Obtain expected digests from release CI or a signed manifest before pulling.
  For buildx output, a CI digest may be a single-platform OCI index containing
  the platform manifest later flattened into the published multi-arch tag;
  verify the containment relationship.
- Capture saved image fields before the target CLI migrates config. Test config
  migration on a copy; package defaults do not overwrite explicit saved fields
  unless a migration does so.
- Capture baseline container IDs, images, OCI versions/revisions, architecture,
  Core start times, chain height, app hash, epoch, `current`, and `next`.
- Restart one node with `dashmate restart --platform --config=<node>`. Include
  Dashmate Helper in the rollout unit and prove Core continuity.
- Gate every next node on target provenance, service health, peers, catch-up,
  and resumed block production.
- Query DAPI's committed Drive `current` and `next` fields. Tenderdash node-info
  app protocol can remain stale until that process restarts.
- Verify every validator has signaled and proposed before waiting for lock-in.
- At lock-in prohibit old-software rollback. At activation compare app hashes
  at the same committed height and observe a post-activation stability window.

## Failure and evidence handling

- Put a deadline on setup, pull, restart, catch-up, no-progress, signal,
  lock-in, activation, and total runtime.
- On failure, stop advancing nodes and capture only allowlisted status/log
  fields. Never collect keys, mnemonics, credentials, config trees, raw
  production data, or Docker environment dumps.
- Retain exact commands, exit statuses, resolved SHAs, expected/observed
  digests, timestamps, heights, epochs, protocol state, and app hashes.
- Label every check `passed`, `failed`, `skipped`, `unavailable`, `waived`, or
  `not-applicable`. Use `not-applicable` only for checks outside the selected
  mode, and `unavailable` when selected evidence could not be obtained. A
  `waived` check needs an explicit user decision and reason, and cannot satisfy
  a mandatory safety gate or support an overall passing result.
