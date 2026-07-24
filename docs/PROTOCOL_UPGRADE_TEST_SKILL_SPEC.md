# Protocol upgrade review and test skill

Status: independently reviewed, user-approved, and implemented. The
v4.0.0-to-v4.1.0-beta.2 rehearsal completed successfully, but the combined
certification profile remains incomplete where noted below.

## Problem

Protocol upgrades combine three different mechanisms that are easy to test
individually while missing the production behavior:

1. updated Drive binaries advertise a desired app protocol version;
2. validator signals are accumulated and locked in at an epoch boundary;
3. the locked version activates at a later epoch boundary, where Drive runs
   consensus-critical first-block transition code before processing the block
   with the new `PlatformVersion`.

The immediate case is Platform protocol v12 to v13, delivered by Dashmate
v4.0.0 to v4.1.0-beta.2. The longer-lived deliverable is a project skill that
reviews and exercises future protocol upgrades consistently.

## Implementation and rehearsal result

The deterministic implementation now covers:

- committed v12 signaling, v12/v13 lock-in, v13/v13 activation, and restart
  reconstruction through the public ABCI strategy path;
- a populated v12 DPNS username, directly invoked rejected v13 transition,
  explicit v12 cache read after rejection, retry equality with a clean control,
  post-upgrade transfer, and exactly one history record;
- protocol-keyed system-contract materializations, including the v8/v9 case
  where the system-contract feature version is unchanged but the DPP in-memory
  structure differs;
- review of PoSe-banned validator vote accounting, with the required fix
  deferred to a future protocol-versioned method so v12/v13 consensus behavior
  remains byte-compatible during rolling software updates;
- bounded Dashmate config migrations and a next-release repair that replaces
  only beta.2 configs still carrying the official stable `:4` image pins.

The live rehearsal used the already-running three-validator local group because
a second Dashmate group collided with the fixed local subnet. It rolled
Platform and Dashmate Helper one validator at a time from v4.0.0 to the
published v4.1.0-beta.2 images without restarting Core. Height 82796, epoch 718,
committed `current=12,next=13`; height 82912 was the first epoch-719 block and
changed protocol 12 to 13. After activation, all validators reported
`current=13,next=13` at height 82949 with the same app hash.

DPNS was version 2 with transfer, purchase, and pricing history enabled;
Document History was present; and the existing root domain retained its ID and
revision. This existing network did not contain two funded user identities for
a live domain transfer, so the populated-data oracle is supplied by the
deterministic Rust fixture rather than claimed as live-network evidence.

## Reviewed v12 to v13 behavior

### Activation timeline

Let `E` be an epoch in which validators running the new binary propose app
version 13.

1. During `E`, Drive records one desired-version signal per validator. A
   validator can replace its own signal; repeated blocks by the same validator
   do not increase the count.
2. On the first block of `E+1`,
   `upgrade_protocol_version_on_epoch_change` evaluates the signals collected
   during `E`. If the threshold passes, it stores
   `next_epoch_protocol_version = 13` and clears the signal state.
3. That first block of `E+1` still executes under v12. Version selection was
   made from the last committed state before the new `next` value was computed.
4. On the first block of `E+2`, version selection observes the committed
   `next_epoch_protocol_version = 13`, changes the block state to v13, runs the
   first-block transition in the block transaction, and then processes the
   block under v13.

For protocol versions using the current method map, the threshold is:

```text
1 + floor(active_hpmns * 67 / 100)
```

The three-validator local network therefore requires all three validators to
signal v13. Each must propose at least one block in the signal-collection epoch.

### v13 first-block state transition

Before normal v13 block processing, Drive:

1. reloads its in-process system-contract cache for v13;
2. inserts the Document History system data contract;
3. applies DPNS contract v2 as an update to the stored DPNS contract.

DPNS v2 does not change domain properties or indexes. It enables these three
document-type settings:

- `keepsTransferHistory`
- `keepsPurchaseHistory`
- `keepsPricingHistory`

The transition does not enumerate, rewrite, or backfill existing DPNS
documents. Existing documents remain in their existing trees. Subsequent v13
transfer, purchase, and pricing operations create history under the newly
registered history contract.

Protocol v13 also selects other consensus behavior changes from the platform
version map. A test that checks only the two contract writes is not sufficient
to prove the boundary is processed under the complete v13 method set.

## Review findings and gate status

### Deterministic lifecycle coverage

The public strategy test now exercises validator signaling, the v12/v13
lock-in boundary, a process reopen while locked in, the separate v13 activation
boundary, persisted DPNS and Document History reads directly from GroveDB, and
a second reopen after activation. The populated DPNS fixture separately proves
that a rejected transition leaves the committed root unchanged, the retry
matches a clean transition from identical v12 state, the existing domain
survives, and a v13 transfer creates exactly one history record.

One combined failure-path gate remains skipped: the rejected candidate and
populated fixture use the transition dispatcher directly rather than rejecting
an entire public proposal round from the lock-in snapshot. The cache and
transition roots are covered deterministically, but this is not represented as
a passing public-round failure-path check.

### System-contract cache consistency

`perform_events_on_first_block_of_protocol_change` reloads the process-wide
`ArcSwap` system-contract cache before the candidate block transaction commits.
The gRPC query and check-tx server shares the same `Platform` and can run
concurrently with the sequential consensus server.

This created a confirmed versioned-read defect: a rejected activation-boundary
proposal leaves DPNS v2 in memory while committed state and a requested
`PlatformVersion` are still v12. Document History is explicitly hidden below
v13, but DPNS has been active since v1 and `find_by_id(DPNS, 12)` returns the
singleton v2 object after the rejected candidate reload.

No consensus divergence has been demonstrated. Consensus execution carries an
explicit `PlatformVersion`, and the DPNS fields read by the current check-tx
consumer are unchanged between v1 and v2. Nevertheless, the cache violates its
versioned fetch contract and can expose an inconsistent committed-state/cache
snapshot to concurrent readers.

The cache now retains exact protocol-version materializations and lazily
materializes explicit historical reads without changing the current pointer.
The red-to-green regression covers v12 versus v13 DPNS and the v8 versus v9
case where the system-contract feature revision is shared but the in-memory
DPP representation differs. A versionless `load_dpns()` call still means
"current process pointer"; consensus and versioned query paths must use the
explicit protocol API.

A check-tx that traverses `get_system_or_user_contract_with_fee` remains a
regression smoke assertion, not the version-selection oracle because its
currently read DPNS fields are identical in v1 and v2. No sleep-based
concurrency probe is used.

### Threshold documentation

The stale “over 75%” comments now describe the effective version-configured
threshold instead. This is documentation-only; it does not alter consensus.

### PoSe-banned validator vote accounting

The active HPMN denominator excluded PoSe-banned validators, but the persisted
per-validator proposal and aggregate numerator were removed only when a
masternode left the list completely. A validator could vote, become banned,
and still count against the smaller active-validator denominator. A focused
regression also showed that removing the vote during the Core diff is
insufficient if the same newly banned validator proposed that block, because
the current v0 block path adds its desired version afterward.

The production fix is deliberately not part of this change. Protocols 12 and
13 both select `run_block_proposal` v0 and `update_masternode_list` v0; changing
their persisted vote writes would make old and new binaries at the same
protocol compute different roots during a rolling deployment. A future
protocol must introduce versioned methods that remove newly inactive votes and
skip inactive proposers together. The existing v0 methods remain unchanged.

### Dashmate prerelease image migration

The released beta.2 CLI updates `configFormatVersion` from 4.0.0 but does not
repin explicit `dashpay/drive:4` and `dashpay/rs-dapi:4` fields to `:4-beta`.
This cannot be repaired retroactively by a migration also keyed beta.2 because
equal-version migrations do not run. The next-release migration is keyed
`4.1.0-beta.3`, repairs only those exact official stable pins, and preserves
custom registries, digests, and tags. The migration runner now also refuses to
run keys newer than its requested target version.

## Chosen test approach

The skill exposes independent modes. Each selected mode must finish all of its
own checks; a certification profile decides which modes are required for a
given upgrade. A lightweight review does not implicitly authorize a network
run, and a deterministic Drive test does not claim to verify published release
artifacts.

### Layer 1: deterministic Drive tests

Add a lifecycle test starting from committed v12 state:

1. create two funded identities and a DPNS domain under DPNS v1;
2. make all required validators signal v13 during epoch `E`;
3. process and commit the first block of `E+1`;
4. assert `current = 12`, `next = 13`, the signal state is cleared, Document
   History is absent, and the DPNS contract remains v1;
5. close and reopen Drive to simulate a restart between lock-in and activation;
6. process the first block of `E+2` through the public proposal path, run
   finalization, and commit the transaction;
7. close and reopen Drive, then assert persisted `current = 13`, Document
   History exists, DPNS v2 is stored, the
   pre-existing domain and its indexes remain readable, and the app hash is
   stable across nodes/replay;
8. transfer the domain to the second identity and verify exactly one
   corresponding transfer-history record.

Add focused tests for:

- Document History cache activation at v12 versus v13;
- rejected v13 activation candidate followed by direct versioned-cache
  comparison against a clean v12 committed-state control, a check-tx smoke
  assertion, then a v13 retry;
- repeated consensus rounds at the activation height;
- process restart immediately before and immediately after activation;
- an isolated v4.0.0 binary/image test against a copy of locked-in state:
  healthy during `E+1`, then a documented unsupported-v13 refusal at `E+2`,
  without carrying it through the three-validator live rollout.

### Layer 2: isolated local-network rolling upgrade

Use a dedicated `DASHMATE_HOME_DIR` and detached temporary git worktrees. The
operator's current worktree may be dirty and is never stashed, reset, cleaned,
or modified. Never reuse the operator's normal `~/.dashmate` or other local
networks.

Every run creates a mode-0700 run root with a random run ID and ownership
marker. Its worktrees, Dashmate home, artifact directory, Compose project
names, ports, containers, networks, and volumes are recorded in a resource
manifest and carry that run ID. Preflight refuses occupied ports, conflicting
Compose names, or any operation that would adopt an existing Docker resource.
Cleanup is a separate explicit action and may remove only manifest-owned
resources whose canonical paths and names still match the run ID. Failed runs
preserve their evidence and resources by default. The workflow never uses
`git clean`, `git reset`, `git stash`, Dashmate group reset, Docker
system/volume/network prune, or recursive deletion outside the run root.

The live test has these phases:

1. Prepare Dashmate from `v4.0.0` and run `yarn setup`.
2. For `local_1`, `local_2`, and `local_3`, disable source builds for Drive,
   rs-dapi, and the Dashmate helper. Resolve their expected digests from an
   operator-supplied signed release manifest or CI attestation and configure
   every image as `repository@sha256:...`. A digest learned only from the pull
   being tested is evidence of what ran, not trusted provenance.
3. Optionally set a shorter epoch duration before genesis for the test run. The
   chosen value must be identical on every node and recorded in the report.
4. Run `yarn start`, then sample several heights from more than one node to
   prove the chain is progressing.
5. Populate v12 state with two funded identities and a DPNS domain owned by the
   first identity using existing platform-test-suite client helpers. Record
   both identity IDs, the domain ID, and pre-upgrade query results. The
   post-upgrade operation is a transfer to the second identity.
6. Capture the immutable baseline configuration and baseline container
   digests before the target CLI opens or migrates the isolated Dashmate home.
   Then prepare Dashmate from `v4.1.0-beta.2`, open that home,
   and explicitly replace the saved image fields with the attested beta.2
   digests.
   Merely using the new CLI is not sufficient because no post-v4.0 config
   migration repins these fields.
7. Verify the still-running containers match the baseline attestation and the
   saved desired image references match the target attestation. Refuse to
   continue if either check fails or source builds remain enabled. If trusted
   expected digests are unavailable, mark release-image verification
   `unavailable` and do not produce a passing local-release result.
8. Restart only Platform services, one node at a time, using
   `dashmate restart --platform --config=<node>`. Dashmate's unprofiled
   `dashmate_helper` also participates in this Compose operation, so the
   rollout unit is Platform plus that node's helper; Core must remain running.
   After each restart, verify every restarted service's RepoDigest, image ID,
   architecture, and component version match the target attestation while
   untouched nodes still match baseline. Then prove the restarted node became
   healthy and caught up and block production resumed through a stability
   window. The three-member local quorum may stall while one member is down:
   the other two hold exactly two-thirds voting power, below Tenderdash's
   greater-than-two-thirds commit threshold. This bounded stall is expected; a
   whole-network shutdown is not. At one of three and two of three upgraded
   validators, with all nodes back online, assert current and next protocol
   remain v12 and consensus is progressing before upgrading the final
   validator.
9. After all three nodes advertise desired version 13, wait for each to propose
   and query protocol-upgrade vote state. Do not infer lock-in solely from
   elapsed time.
10. At the first epoch boundary, assert committed protocol remains 12 and next
    protocol is 13.
11. Restart one upgraded Platform node during the lock-in epoch and prove it
    rejoins without changing current/next protocol state.
12. At the following epoch boundary, assert committed protocol becomes 13 on
    every node, app hashes agree at the same fixed committed height, no node is
    catching up or restarting, and block production continues through a
    stability window. Verify all rollout-unit services still match the target
    attestation.
13. Re-query the v12 fixture, transfer the domain to the second identity, and
    verify exactly one corresponding transfer-history record.
14. Capture statuses, upgrade-vote responses, heights, app hashes, image
    references/digests, and allowlisted Drive/Tenderdash log ranges in the
    artifact directory.

On any restart, digest, catch-up, quorum, height, or app-hash failure, stop the
rollout rather than advancing to the next node. A node may return to its
baseline image only after proving, at a fixed committed height, both
`current = 12` and `next != 13`. Once v13 is locked in (`current = 12`,
`next = 13`), v4.0 is prohibited because it cannot process the activation
boundary. Recovery is roll-forward to working v13 software or restoration of a
complete, mutually consistent pre-lock-in capture. Individual Drive-volume
rollback after lock-in is never allowed.

All waits have configured deadlines: individual RPC, image pull, setup,
restart/catch-up, no-height-progress, maximum signal epochs, activation, and
total run. Evidence comparison records node, fixed committed height, round,
app hash, current/next protocol, catching-up state, and timestamp. A timeout
captures evidence and stops the rollout.

`dashmate group restart` is not used: it stops every node. The public operator
documentation supports `dashmate restart --platform`, and the source confirms
that this selects only Platform compose profiles.

### Layer 3: authentic state preflight

Classify each first-block transition by its actual dependency on existing
records before deciding whether authentic network state is required:

| Class | Transition shape | Required data test |
| --- | --- | --- |
| N | No persisted first-block migration; the method map or runtime cache can still change behavior | real boundary lifecycle and populated synthetic data for changed behavior |
| A | Does not enumerate or validate existing records; work is schema-bounded | deterministic first-block, rollback, and retry tests; add a synthetic existing record only when behavior changes |
| B | Reads or validates existing records without mutating them | populated representative state; authentic state is required only for unbounded cardinality or production-only invariants, otherwise recommended |
| C | Rewrites, deletes, backfills, moves, or rebuilds existing state/indexes | deterministic rollback/idempotency plus an authentic committed artifact for every target network |

The v13 transition is class A. Applying DPNS v2 writes a bounded contract
definition and index metadata; it does not enumerate or rewrite domain
documents. A populated local v12 domain is still mandatory because its
cross-version behavior matters. Testnet/mainnet artifacts are optional
release-smoke evidence for database-open and orchestration compatibility, not a
correctness prerequisite for v13.

Authentic-state testing is an offline, migration-only data compatibility
preflight and is destructive only to a temporary copy. Full signaling,
activation-state persistence, and production app-hash behavior are certified
by the deterministic lifecycle and live-network modes, not inferred from an
isolated database artifact:

1. accept an operator-supplied, quiesced post-commit Drive/GroveDB checkpoint
   paired with `platform_state.bin` and a manifest containing network, height,
   app hash, protocol version, file inventory, cryptographic checksums, and the
   explicit migration `BlockInfo` values;
2. canonicalize the source without following content symlinks; reject special
   files, path overlap with the run root, traversal, symlink escape, and
   configured file-count or expanded-size limits;
3. verify the manifest read-only, check free space for source, migrated copy,
   extraction, and safety headroom, then copy or safely extract into a
   mode-0700 private directory before any database tool opens it;
4. open and verify only the private v12 copy, then clone it again for migration;
5. invoke the first-block transition dispatcher with the supplied committed
   `PlatformState` and `BlockInfo` in a transaction against the migration copy;
   commit it, reopen the raw Drive database, run database verification, and
   execute the transition-specific queries with an explicit target
   `PlatformVersion`; do not claim a production app hash or persisted target
   protocol from this migration-only preflight;
6. for class C, compare complete invariants: record counts, invalid/null
   counts, index root/cardinality/coverage, aggregate hashes, app/state root,
   rollback, and retry; deterministic samples are diagnostic only;
7. retain the source unchanged, verify its manifest again after the run, and
   record the migrated copy's state root and timing.

For class C, review mode must emit a machine-readable invariant manifest:
paths, preconditions, expected record/count preservation or deltas, index
coverage checks, postconditions, and rollback/retry oracles. Snapshot mode
fails as unsupported when that manifest is absent; a generic script must not
guess which trees or aggregates define correctness.

An ABCI snapshot alone must not be imported into a live local devnet and
expected to continue: Tenderdash, Core, validator, height, and app-hash state
would not be coherent. Full live replay requires a mutually consistent capture
of those components. The offline dispatcher preflight intentionally avoids
that false equivalence.

## Skill interface

Create the canonical project skill at:

```text
.claude/skills/protocol-upgrade-test/
```

The existing `.codex/skills -> ../.claude/skills` symlink makes it available to
both Claude Code and Codex without duplicating its source.

Suggested invocation:

```text
/protocol-upgrade-test from v4.0.0 to v4.1.0-beta.2
```

Inputs:

- baseline and target git refs;
- expected old and new protocol versions, or `auto` to derive them;
- test mode: `review`, `drive`, `local-release`, `snapshot`, or `all`;
- optional certification profile that declares which modes are required;
- trusted release-manifest/attestation path for `local-release`;
- optional testnet/mainnet snapshot paths;
- optional epoch duration and artifact directory.

### Authority and trust boundaries

`review` is the read-only default: no git fetch, package installation, image
pull, Docker mutation, worktree creation, or network start. `drive` may create
an owned detached worktree and write build/test artifacts only inside its run
root. `local-release` additionally authorizes digest-addressed pulls and
creation, restart, and removal of only that run's Docker resources. `snapshot`
authorizes read-only ingestion of the supplied source and mutation of private
copies only. `all` grants the union; the skill never escalates modes.

Resolve refs to SHAs before execution. Run setup scripts only from verified
release tags, or after an explicit untrusted-ref override that displays the
resolved SHA and Docker-socket risk. Use a sanitized environment without
unrelated host credentials. Review and snapshot modes have no network egress
by default. Bind RPC/admin/metrics surfaces to loopback unless P2P requires
otherwise.

Artifacts are local-only: mode-0700 directories and mode-0600 files. Never
collect Dashmate config trees, private keys, mnemonics, RPC credentials,
registry credentials, authorization headers, Docker environment dumps, or raw
production snapshot contents. Store only allowlisted fields/log ranges, redact
known secret patterns before persistence, and fail closed if redaction cannot
be verified.

The append-only run manifest records resolved SHAs and tag verification,
package/CLI versions, expected and observed image digests, Compose project IDs,
epoch-configuration hash, sanitized commands and exit statuses, phase
timestamps, and hashes of raw and normalized responses. Finalization writes
SHA-256 hashes for all retained artifacts so the report can be recalculated
independently.

The skill must stop loudly when:

- refs are missing or do not resolve to expected package versions;
- an unexpected file changes outside the run root or owned worktree;
- the target does not support the requested protocol version;
- trusted image digests cannot be verified;
- source builds are unexpectedly enabled in release-image mode;
- the isolated Dashmate home is not being used;
- chain height stalls outside an active one-node restart window, or the
  expected restart stall exceeds its configured deadline;
- validators have not all signaled;
- lock-in or activation occurs at the wrong boundary;
- app hashes differ;
- any test is skipped without being reported.

## Skill implementation shape

Phase 1 is a deliberately small, review-first skill:

- `scripts/review_transition.sh`: resolve refs, find the version maps and
   first-block dispatcher, and emit the transition diff.
- `references/review-checklist.md`: transition classification, cache/rollback,
  mixed-version, failure, and data-selection checklist.
- `SKILL.md`: mode authority, review routing, classification, stop conditions,
  and the human-readable report contract.

First rehearse the v12-to-v13 local procedure manually and turn the proven
commands into `local_release_upgrade.sh`, `watch_upgrade.mjs`, and
`populate_fixture.js`. Add `snapshot_preflight.sh` only when a reviewed
transition requires authentic state. Initialize and validate the skill with
the standard skill-creator scripts, then forward-test its review mode on:

- `v4.0.0` to `release_4.1.0-beta.2`, expected v12 to v13 class A;
- `v3.0.2` to `v3.1.0-dev.8`, expected v11 to v12 class C because the
  transition strips unknown properties from stored contract schemas;
- `v2.0.0` to `v2.1.3`, expected v9 to v10 with no first-block transition.

## Alternatives rejected

### Change checkout and run `yarn restart`

Rejected because `yarn restart` invokes group restart and stops the network.
`yarn setup` also enables source builds, so this would not prove published
release images.

### Trust Dashmate's new defaults to change saved images

Rejected because saved v4.0 configs retain their explicit image fields. The
v4.1.0-beta.2 CLI derives the mutable `4-beta` tag for new defaults, not an
attested digest for the `4.1.0-beta.2` artifact, and there is no later config
migration that repins an existing v4.0 config.

### Test only the private v13 transition helper

Rejected because it bypasses signaling, lock-in, activation timing, the public
block path, rollback, restart, and mixed-version behavior.

### Import only a mainnet/testnet ABCI volume into the local network

Rejected because the local Tenderdash/Core state would not match its height,
validator set, or app hash. Use an offline migration preflight, or a complete
coherent replay capture.

## Verification and completion criteria

The v12-to-v13 beta.2 release is certified only when:

- the deterministic lifecycle and failure-path tests pass;
- the cache-consistency gate is resolved;
- the exact v4.0.0 to v4.1.0-beta.2 rolling release-image test reaches v13
  without whole-network shutdown and preserves the v12 DPNS fixture;
- trusted expected and actual image digests match;
- snapshot status is reported, with no snapshot required for this class-A
  transition;
- the final report distinguishes passed, failed, skipped, unavailable, and
  waived checks.

The rehearsal in this implementation is not labeled certified: it did not
create the two-identity live DPNS fixture, did not restart a live validator
during the lock-in epoch, and did not reject a populated activation candidate
through the complete public proposal path. Those checks are `skipped`, while
the corresponding deterministic contract, restart, cache, and retry oracles
passed.

The skill implementation is complete independently when:

- the canonical shared skill validates;
- review mode succeeds on v12-to-v13, an older structural migration, and a
  no-migration protocol bump;
- the manual release rehearsal has been encoded and its owned-resource cleanup
  path has been tested;
- required class-C snapshot preflights have full-invariant coverage;
- the final report distinguishes passed, failed, skipped, and unavailable
  checks.
