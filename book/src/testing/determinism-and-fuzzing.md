# Determinism and Fuzzing

Every masternode must commit the same application hash for the same block.
The strategy tests prove that one process agrees with itself; this chapter
covers the two pieces of release evidence that go further: a replay recorded
on two CPU architectures and compared byte for byte, and bounded fuzzing of
the decoders that untrusted bytes reach first.

## The Determinism Artifact

The strategy test
`determinism_tests::should_replay_identically_from_saved_state_across_the_protocol_upgrade`
(in `packages/rs-drive-abci/tests/strategy_tests/test_cases/`) runs one
seeded workload twice on a chain that starts at the previous protocol
version and upgrades to `PlatformVersion::latest()`:

- **Continuous**: 70 blocks, then 60 more on the same open platform.
- **Reopened**: 70 blocks, drop the platform, reopen it from the persisted
  state with `TempPlatform::open_with_tempdir`, then 60 more.

The split sits inside the upgrade window: the new version is locked in at the
first epoch change (block 60) and activates at the second (block 120), so the
activation runs on a platform that was restarted from disk. Every block also
re-runs `process_proposal` as an independent validator, which is the proposer
versus validator parity check. The workload creates identities, writes,
replaces and deletes DashPay contact requests, tops identities up from signed
instant locks and transfers credits between identities. Withdrawals and
tokens are left to the comprehensive simulation because they need core RPC
mocks and their own key setup.

In-process, the test asserts that the two runs agree on every application
hash, every kept transition's code and fee, the protocol version of every
block, every identity balance, the credit totals and the final root; that the
upgrade locked in before the split and activated after it; that every
operation kind executed successfully on both sides of the activation; and
that `calculate_total_credits_balance` reports a conserving state.

When `PLATFORM_DETERMINISM_ARTIFACT_DIR` is set, the reopened run is written
to `<dir>/determinism-<target_arch>.json`. The schema lives in
`packages/rs-drive-abci/tests/strategy_tests/determinism_artifact.rs` and has
three sections with three different rules:

| Section | Contents | Rule |
|---|---|---|
| `consensus` | workload seed, reopen height, per-block application hash, protocol version and kept transitions (name, code, fee), final root, credit totals, identity balances | Must be byte-identical between two recordings. Any difference rejects the pair. |
| `profile` | target architecture, OS, pointer width, endianness, detected CPU features, protocol version start and end, engine pin | Protocol versions must match; target architecture must differ; the rest is printed so a mismatch can be explained. |
| `diagnostic` | elapsed time, block count, raw engine fuel per trapping case | Printed side by side, never compared. |

Serialisation is canonical: struct fields in declaration order, every map a
`BTreeMap`, every hash lowercase hex. A unit test encodes the same value twice
and asserts identical bytes, then mutates one application hash and asserts
the bytes differ.

### Recording and comparing locally

Two processes on one machine are the cheapest way to catch a
`HashMap`-ordered path in the harness before it reaches CI:

```bash
PLATFORM_DETERMINISM_ARTIFACT_DIR=/tmp/run-a RUST_MIN_STACK=4194304 \
  cargo test -p drive-abci --test strategy_tests -- determinism_tests
PLATFORM_DETERMINISM_ARTIFACT_DIR=/tmp/run-b RUST_MIN_STACK=4194304 \
  cargo test -p drive-abci --test strategy_tests -- determinism_tests
python3 .github/scripts/compare-determinism-artifacts.py \
  /tmp/run-a/determinism-aarch64.json /tmp/run-b/determinism-aarch64.json \
  --allow-same-architecture
```

Without `--allow-same-architecture` the comparator rejects two recordings
from one target, so a CI job cannot pass by comparing an architecture with
itself. Exit codes: 0 match, 1 consensus or profile mismatch (the first
differing block and field are named), 2 malformed input or unknown schema.

## The Cross-Architecture Workflow

`.github/workflows/tests-rs-determinism.yml` (`Determinism: cross-architecture
replay`) runs nightly, on manual dispatch, and on pull requests that touch the
pipeline files. It uses GitHub-hosted runners only, so fork PRs are safe and
the self-hosted PR fast path is untouched.

1. **record** (matrix `ubuntu-24.04` for x86_64 and `ubuntu-24.04-arm` for
   aarch64, 90 minutes each): builds the strategy test binary, runs the
   determinism test with the artifact directory set, checks the file exists
   and uploads it.
2. **compare** (10 minutes): downloads both recordings and runs three steps
   in order:
   - the comparator's `--self-test`, which builds artifacts in memory and
     asserts that an identical pair passes and that a differing application
     hash, fee, code, per-block protocol version, credit total, balance or
     block count fails, that a differing diagnostic passes with a note, and
     that an unknown schema is malformed;
   - the real comparison, with the report appended to the step summary;
   - a perturbed comparison: one byte of one application hash in the x86_64
     recording is flipped and the comparator must reject it.

The last two steps are why a green run means "the two architectures agreed"
and not "the check was skipped". The pipeline exercises its own failure path
on every run instead of asserting that the checks pass.

The run timeouts, the nightly hour and the workload size are engineering
proposals recorded in the workflow comments, not owner-confirmed values;
adjust them after the first weeks of timings.

## What the Engine Adds

The contract engine is not in this tree yet. When it lands, its
cross-architecture corpus (canonical NaN bits, protocol-metered cost of every
trapping case) plugs into the same artifact and comparator:

- The engine build, code generator settings, memory guard layout and NaN
  canonicalisation go into `profile.engine`, so a trapping case can be tied
  to the settings that produced it.
- Canonical NaN bits and the protocol-metered cost of each trapping case go
  into `consensus`: they are what nodes agree on, so a mismatch rejects the
  profile.
- Raw engine fuel goes into `diagnostic.engine_fuel` while it does not affect
  consensus costs or limits. A protocol version that makes it
  consensus-visible moves it into `consensus`.

Two release-evidence proposals from the plan are recorded here as proposals
only: a seven-day window of clean nightly comparisons before a release
candidate, and two full upgrade rehearsals on a devnet. Neither is a CI gate,
and neither creates an audit, funding or deployer-allowlist step or a new
recovery authority.

## Bounded Fuzzing

The decoders that untrusted bytes reach first (`StateTransition`,
`DataContract`, `Document` and `check_tx` itself) are fuzzed with `proptest`
under an explicit case budget. The property is the same everywhere: arbitrary
or fixture-mutated bytes must produce `Ok` or `Err`, never a panic, and
`check_tx` must return a validation result rather than an internal error.

Fuzz modules are named `fuzz_tests` so one nextest filter,
`test(/fuzz_tests::/)`, selects them across crates. The default 256 cases run
inside the workspace test phase on every pull request; the long-running
nightly runs the same targets with `PROPTEST_CASES=20000` under a 45-minute
bound. Regression files are not committed; the nightly budget does the work.
The fuzz targets themselves land in a follow-up to the determinism pipeline.
