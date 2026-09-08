---
name: protocol-upgrade-test
description: Review and test Dash Platform Drive protocol upgrades across version maps, epoch signaling and activation, first-block transitions, persisted data, rollback/retry, and Dashmate rolling release upgrades. Use when comparing Platform release refs, reviewing perform_events_on_first_block_of_protocol_change, deciding whether testnet/mainnet state is required, adding protocol-upgrade regression tests, or rehearsing an old-to-new Dashmate upgrade on a local validator network.
---

# Protocol Upgrade Test

Review the consensus path before designing tests. Treat protocol selection,
first-block state migration, and release orchestration as separate layers with
separate evidence.

## Authority

Default to `review`, which is read-only. It permits local source inspection and
test/report design only. It does not authorize ref fetching, dependency
installation, image pulls, Docker changes, network starts, config migration, or
service restarts.

Use another mode only when the user explicitly requests it:

- `drive`: add and run deterministic Drive/ABCI tests.
- `local-release`: operate an isolated local network and release images.
- `snapshot`: inspect a supplied state artifact read-only and mutate private
  copies only.
- `all`: union of the explicitly requested modes.

Never broaden authority because a later step would be convenient.

## Review workflow

Read [references/review-checklist.md](references/review-checklist.md) before
reviewing code.

### 1. Resolve the comparison

For a released upgrade, require both baseline and target to be exact local
release-tag names. Resolve each tag to its peeled commit SHA and confirm the
Dashmate/package version matches the tag. Do not use a branch, `HEAD`, or a
mutable remote-tracking ref as evidence for code shipped in release artifacts.
If a tag is missing, stop unless the user explicitly authorizes fetching it.

Run:

```bash
.claude/skills/protocol-upgrade-test/scripts/review_transition.sh \
  <baseline-tag> <target-tag> [old-protocol] [new-protocol]
```

For pre-release preparation only, a fetched development branch may be used as
a supplemental target:

```bash
.claude/skills/protocol-upgrade-test/scripts/review_transition.sh \
  --allow-non-tag-target <baseline-tag> <target-ref> \
  [old-protocol] [new-protocol]
```

Pin and report the target SHA and label the result `pre-release`; it cannot
certify a release or substitute for rerunning the review against the eventual
release tag. Fetching branches or tags still requires explicit user authority.

Use its output as an inventory, not as the review verdict. Read the exports,
dispatchers, version maps, immediate callers, tests, and cache/storage helpers
identified by the inventory.

### 2. Prove the epoch timeline

Trace the public block proposal path and write the sequence in committed-state
terms:

1. which epoch collects validator signals;
2. the exact threshold formula and active-validator denominator;
3. which first epoch boundary writes `next_epoch_protocol_version`;
4. which protocol version executes that lock-in block;
5. which later boundary selects the new version and runs the transition;
6. when state is finalized, committed, and restored after restart.

Do not infer these from elapsed time, desired software version, or a node's
startup handshake field. Query committed `current` and `next` protocol fields
in network tests.

### 3. Review the first-block transition

List every read, write, cache mutation, version-dependent call, and external
input. Determine whether the work is inside the candidate block transaction.
Check what survives a rejected proposal or alternate consensus round, and
whether retry is idempotent.

Pay particular attention to process-global caches. A transaction rollback does
not roll back an `ArcSwap`, lock, singleton, filesystem write, or external
service call. An API accepting an explicit `PlatformVersion` must return data
for that version after speculative loading of another version.

### 4. Classify existing-state dependence

Assign one class from source behavior:

- Class N: no persisted first-block migration, although the new complete method
  map or runtime cache reload may still change behavior.
- Class A: schema-bounded work that does not enumerate existing records.
- Class B: reads or validates existing records without rewriting them.
- Class C: rewrites, deletes, backfills, moves, or rebuilds existing
  state/indexes.

Class N still requires the real boundary lifecycle and populated synthetic data
for method-map behavior changes. Class A requires deterministic boundary,
rollback/retry, and a populated synthetic fixture when cross-version behavior
changes. Class B requires representative populated state; authentic state is
required only for unbounded cardinality or production-only invariants. Class C
requires deterministic idempotency plus an authentic committed artifact for
every target network and a transition-specific invariant manifest. A mixed
transition inherits its highest-risk class.

Never import a lone Drive/ABCI snapshot into a live devnet and claim replay
equivalence. Tenderdash, Core, validators, height, and app hash must be coherent.
Use an offline private-copy migration preflight unless a complete replay capture
exists.

### 5. Design tests

The minimum deterministic matrix is:

- signal threshold below, at, and above the required count;
- lock-in boundary remains on the old protocol;
- activation boundary runs the new complete method map;
- populated old-protocol data survives activation;
- rejected candidate leaves committed root/state unchanged;
- retry produces the same result as a clean activation;
- restart after lock-in and after activation preserves `current`/`next`;
- unsupported old software fails in a documented way at activation;
- post-upgrade behavior proves the migrated schema or index is usable.

For bugs discovered during review, write the failing regression first, observe
red, implement the smallest fix, then observe green.

## Local release rehearsal

Only enter this section in `local-release` or `all` mode.

Use detached owned worktrees and an isolated `DASHMATE_HOME_DIR`. Preserve dirty
operator worktrees. Disable Drive, RS-DAPI, and Dashmate Helper source builds
when testing published images. Resolve expected image digests from a trusted
release/CI attestation before pulling; a digest learned from the pull is only
operational evidence.

Capture baseline config, container IDs, image IDs, OCI version/revision labels,
Core start times, chain height, epoch, `current`, and `next` before mutation.
Test target-CLI config migration on a copy first. A newer Dashmate CLI does not
prove saved image references changed.

Roll one validator at a time with:

```bash
dashmate restart --platform --config=<node>
```

The rollout unit includes Platform services and Dashmate Helper; Core must keep
the same container identity and start time. After every node, require the
attested target revision, healthy peers, caught-up state, and resumed height
before advancing. A bounded quorum stall while one of three validators is down
is possible; a whole-network restart is not allowed.

After every validator signals, verify the lock-in boundary as
`current=old,next=new`, then the following activation boundary as
`current=new,next=new`. Compare app hashes at the same committed height.
Once `next=new`, prohibit rollback to old software; recover by rolling forward
or restoring a complete consistent pre-lock-in capture.

The phase-1 skill intentionally does not provide an automatic live-network
script. Encode commands only after a manual rehearsal has proved ownership,
digest, restart, timeout, and cleanup behavior.

## Stop conditions

Stop and report rather than claiming success when:

- refs or expected package versions do not match;
- the target does not support the requested protocol;
- a selected test was skipped or did not exercise the public path;
- source builds are enabled during a release-image test;
- trusted image provenance is unavailable;
- validators have not all signaled or the boundary is wrong;
- height stalls beyond the configured restart window;
- nodes disagree on fixed-height app hash or protocol state;
- an authentic artifact is required but lacks checksums, state metadata, or
  transition-specific invariants;
- any unexpected file or Docker resource falls outside the owned run scope.

## Report contract

Report each check as `passed`, `failed`, `skipped`, `unavailable`, `waived`, or
`not-applicable`. Use `not-applicable` when a check belongs to a mode that was
not selected; use `unavailable` only when selected evidence could not be
obtained. `waived` requires an explicit user decision and a recorded reason; it
does not satisfy a mandatory safety gate or permit the overall run to be called
passing. Include resolved SHAs, software and protocol versions, transition
class, signal/lock/activation evidence, test commands and results, data
fixtures, snapshot decision, image provenance, observed digests/revisions,
Core continuity, failures, and retained artifact paths. Never collapse
unavailable release provenance, a waiver, or skipped tests into a pass.
