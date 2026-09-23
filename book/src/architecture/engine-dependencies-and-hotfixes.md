# Engine Dependencies and Hotfixes

Smart contracts on Dash Platform run inside a WebAssembly engine. Every
masternode compiles the same canonical module with the same compiler and must
produce the same result, the same trap, the same metered cost and the same
resource outcome, for current blocks and for blocks it replays from years ago.
That makes the engine a consensus dependency: a change to it is either
provably invisible to guests or it is a protocol upgrade.

This chapter states the engine policy, names the crates that form the engine,
explains how security advisories against them are tracked, gives the
classification a maintainer applies to an advisory, and states the rule that
decides whether a fix ships as a node release or as a protocol upgrade. The
last section maps the evidence the rule needs onto the work that produces it,
so a reader can see what this chapter promises against what exists today.

## Why the engine is a consensus dependency

The native parts of block execution are deterministic by construction: Rust
code that reads state, applies rules from the version tables and writes
GroveDB. The engine adds a compiler to that path. A guest module is compiled
by Cranelift into machine code on each node, and the machine code decides
what the guest computes, whether it traps, how much metered work it consumes
and how much memory it touches. A compiler bug is therefore not a crash
report; it is a divergence between nodes running different builds, or between
a node and the specification of the admitted Wasm semantics.

The engine is versioned like every other consensus behaviour. The DashVM
table in `packages/rs-platform-version` (introduced with the validation crate
and selected per protocol version) carries an `engine_profile` number: the
pinned Wasmtime release, the Cranelift flags, the target CPU feature policy,
the memory layout and the engine-level feature set. A new engine release that
changes any guest-visible outcome is a new profile number added alongside the
old one, never an edit, so historical blocks replay under the engine that ran
them. The rule below is about deciding which changes need that new number.

## The engine policy

The owner decision for the engine is confirmed and this chapter does not
reopen it:

- **Pinned upstream Wasmtime with Cranelift.** The runtime depends on
  released crates from the Bytecode Alliance, pinned to an exact version on a
  long-term-support line. Cranelift is the only compiler in the consensus
  path. Winch, the interpreter and any other backend may be used for
  diagnostics and never for a consensus result.
- **A small Dash integration layer.** The validation crate prepares canonical
  code and the runtime crate hosts it. "Small" is a boundary, not a size:
  no forked engine crate in the workspace, no vendored engine sources, no
  local flag that changes compiled code outside the engine profile.
- **Fixes go upstream.** A bug found in the engine is reported and fixed in
  the upstream repository, then picked up through a patch release on the
  pinned line.
- **A temporary patch is an exception, not a standing fork.** When a fix
  exists upstream but has not shipped in a release the node needs, a
  `[patch]` entry may carry it for a bounded time. The patch is declared with
  the upstream fix it carries and a date by which a release replaces it; the
  engine audit fails when the date passes or the declaration is missing.

## The selected dependencies

The engine set is recorded in the root `Cargo.toml` under
`workspace.metadata.dashvm.engine` as crate names. Versions are pinned where
the crates are used, and drift of the pinned configuration is checked by the
runtime work, not by this list.

| Crate | Role | Pinned by | Advisory source |
|---|---|---|---|
| `wasmtime` | Engine: instantiation, memory guards, fuel, traps, host calls | Runtime crate | RustSec (`wasmtime`), GitHub Security Advisories on `bytecodealliance/wasmtime` |
| `wasmtime-environ` | Module environment shared by compiler and runtime | Runtime crate (transitively) | Same advisories as `wasmtime` |
| `wasmtime-internal-cranelift` | Cranelift integration inside Wasmtime | Runtime crate (transitively) | Same advisories as `wasmtime` |
| `cranelift-codegen` | Code generation and lowering | Runtime crate (transitively) | RustSec (`cranelift-codegen`, one advisory from 2021) and upstream Wasmtime advisories |
| `cranelift-frontend` | IR construction | Runtime crate (transitively) | Upstream Wasmtime advisories |
| `cranelift-native` | Host feature detection feeding the target CPU policy | Runtime crate (transitively) | Upstream Wasmtime advisories |
| `wasmparser` | Binary format parsing and validation of submitted modules | Validation crate (`=` pin on the line the engine ships with) | No RustSec entries to date; upstream `wasm-tools` release notes |
| `wasm-encoder` | Emission of prepared (instrumented) modules | Validation crate (`=` pin) | No RustSec entries to date; upstream `wasm-tools` release notes |

Facts as of September 2026, for a reader judging the lifetime of the line:
Wasmtime 36 is the long-term-support line released in August 2025 and
supported until August 2027, with patch releases roughly monthly; Wasmtime 48
is the next long-term-support line, released in August 2026. Security fixes
are backported to every supported line. The exact release pin is an open
allocation owned by the runtime crate work and is not decided here; the
validation crate already pins the `wasm-tools` line that Wasmtime 36 ships
with, so parser and engine agree on the binary format down to the patch
level.

Crates that are not in the set on purpose: `wat` and `wasmprinter` are test
and tooling dependencies, never on a consensus path, and stay under the
base-wide audit. Crate names the lockfile does not resolve yet are reported
by the audit as planned and skipped, so the set can name the engine before
the runtime crate lands and needs no change when it does; the runtime work
corrects internal crate names to whatever the pinned release resolves.

## How advisories are tracked

Three sources cover the set:

1. **RustSec** through `cargo-audit`. This is the automated signal. Entries
   exist for `wasmtime` and `cranelift-codegen`; the `wasm-tools` crates have
   had none.
2. **GitHub Security Advisories on `bytecodealliance/wasmtime`.** The
   upstream source of truth. Every RustSec entry for Wasmtime points at one,
   and the upstream advisory carries the affected configurations that the
   classification below needs.
3. **Bytecode Alliance release notes and announcements.** The signal for the
   `wasm-tools` crates and for fixes that were not filed as advisories.

What is automated is the workflow `Security: DashVM Engine`
(`.github/workflows/security-audit-engine.yml`) running
`.github/scripts/check-engine-advisories.py`:

- It runs nightly after the base-wide Rust audit, on manual dispatch, and on
  every pull request that changes the lockfile, a manifest, the audit
  configuration, the checker or the workflow. A dependency bump that pulls in
  an affected engine crate is therefore red on the pull request, not
  discovered the next night.
- It runs `cargo-audit` from outside the workspace, so the ignore list in
  `.cargo/audit.toml` cannot hide an engine advisory. The base-wide audit has
  its own backlog and its own exceptions; the engine has neither.
- It fails on any vulnerability against an engine crate, on an engine crate
  reported as unmaintained, unsound or yanked (the policy is pinned
  upstream-maintained releases, so those are policy failures rather than
  advisories to defer), on an exception that expired or has no reason, and on
  a `[patch]` entry for an engine crate without a declaration.
- Exceptions are scoped and dated. An acknowledgement names one advisory,
  states why it cannot reach a node under the pinned profile, and expires. A
  temporary patch names the crate, the upstream fix and its expiry. Neither
  may run more than 90 days from the day the audit runs (a provisional bound
  recorded in the checker; the issue register has no value for it).
- It proves its own failure path first: the workflow runs the checker's
  `--self-test` before the real audit, so a green run means the set was
  audited, not that nothing was compared.

Reproducing a red run locally:

```bash
cargo install cargo-audit --version 0.22.2 --locked
python3 .github/scripts/check-engine-advisories.py --self-test
python3 .github/scripts/check-engine-advisories.py
# or, against a saved report:
cargo audit --json --file Cargo.lock > /tmp/audit.json   # from a directory without .cargo/audit.toml
python3 .github/scripts/check-engine-advisories.py --report /tmp/audit.json
```

What stays outside the repository: subscriptions to upstream advisory
feeds, disclosure contacts and communication with node operators are
operational actions and are not encoded here.

No external audit report, named security owner or closed stage audit is a
prerequisite to building or to merging engine work. The check is mechanical
and it does not wait for a sign-off, a subscription or a person. A reviewer
who wants an external audit of the engine profile before activation asks for
it as release evidence, not as a gate on the code.

## Classifying an advisory

An advisory against an engine crate is classified on two axes before anything
is changed. The upstream advisory usually states both.

**Reachability under the pinned profile.**

| Reachability | Meaning | Examples |
|---|---|---|
| Guest-reachable | The affected code is compiled in and a guest can drive it: Cranelift lowering of an admitted operator, memory guard logic, fuel accounting, table operations, the host-call boundary | An aarch64 miscompile of a heap access; a wrong bounds check |
| Compiled in, not guest-reachable | The affected code is present but only the host can trigger it: API misuse, async futures, cloning a `Linker`, allocating tables beyond the address space | A use-after-free after cloning a `Linker`; a panic on an oversized host-side table allocation |
| Not compiled in | The affected feature is outside the profile: Winch, the component model, WASI, threads, SIMD, memory64, the pooling allocator when the runtime does not enable it | Winch `table.fill` panics; component-model string transcoding; filesystem sandbox escapes |

**Consequence if reached.**

| Consequence | Meaning | What a divergence would look like |
|---|---|---|
| Host safety | Sandbox escape, out-of-bounds read or write, panic, leak between instances | Nodes crash or leak; results do not change |
| Guest-visible semantics | The guest computes a different result or traps differently than the admitted semantics specify | Nodes on the affected build disagree with nodes on the fixed build |
| Metering | Fuel or host-call counts differ from the schedule | Fees or limits differ between builds |
| Resources | Memory growth, stack depth or limit enforcement differ | A module runs on one build and is rejected on another |

Worked examples from the April 2026 batch of Wasmtime advisories, all fixed
in the 36.0.7 patch release on the long-term-support line:

- **Miscompiled guest heap access on aarch64 Cranelift** (a Critical sandbox
  escape). Guest-reachable, host safety and guest-visible semantics, on one
  architecture only. A node on the affected build already disagrees with the
  specification of the admitted operators. The patched engine restores the
  specified behaviour. Whether the fix is a hotfix or an incident is decided
  by the differential run over recorded executions: if no committed
  execution depended on the bug, the outputs are identical and the release is
  a hotfix; if one did, a committed wrong result exists and the incident path
  applies. Before contract execution is active on any network, the corpus
  comparison alone decides.
- **Segfault or out-of-sandbox load with `f64x2.splat` on x86-64 Cranelift.**
  A SIMD operator. SIMD is not in the initial profile, so the code path is
  unreachable from admitted modules. Acknowledged with that reason until the
  routine patch bump on the line, which happens anyway because the same
  release fixes the previous example.
- **Winch `table.fill` panic, Winch `table.grow` masking, Winch sandbox
  escape.** Not compiled in; Winch never produces a consensus result. Same
  handling as the SIMD case.
- **Component-model string transcoding (three advisories), component `flags`
  lifting.** Not compiled in.
- **Data leakage between pooling allocator instances.** Not compiled in
  unless the runtime enables the pooling allocator; if it does, this is
  compiled in but not guest-reachable (host safety, no result change) and a
  routine patch bump.
- **Use-after-free after cloning `Linker`** (a later fix on a newer line).
  Compiled in, not guest-reachable. Host safety only. Fixed by using the
  release that contains it; no result changes.
- **Filesystem sandbox escape with trailing slashes** (August 2026). WASI, not
  compiled in.

The point of the exercise is the last column: a maintainer must be able to
say, before touching the pin, whether the fix can change a result, a trap, a
cost or a resource outcome for any admitted module. If the answer is "no" by
construction (not compiled in, or not guest-reachable), the change is a
routine patch bump. If the answer is "possibly", the rule in the next section
applies.

## The execution/fee-equivalence hotfix rule

This section is normative. It is the single statement of the rule; other
chapters link here rather than restate it.

**Definition.** An engine hotfix is a change to the engine dependency set or
to the Dash integration layer that ships in a node release without a protocol
version change. A patch bump on the pinned line, a temporary patch, a fix in
the validation or runtime crate, and a change of the Cranelift configuration
are all engine changes and all fall under this rule.

**The rule.** An engine change may ship as a hotfix only with equivalence
evidence over accepted code and recorded executions, on both supported
architectures. Equivalence means all of the following, with no exception:

1. For every accepted canonical module (every stored contract version,
   whether or not it is currently executable), the prepared bytes are
   unchanged. The preparation generation is a separate pin and an engine
   hotfix never touches it.
2. Every such module compiles successfully under the changed engine.
3. For every recorded execution and every vector of the deterministic
   corpus, the changed engine produces the same result bytes, the same trap
   class, the same metered computation units and host-call counts, the same
   resource outcomes (memory, stack, limits) and the same proposed
   operations as the recorded ones.
4. Points 2 and 3 hold on x86_64 and on aarch64, under the recorded CPU
   feature and guard settings of the profile.
5. Raw Wasmtime fuel matches wherever it feeds a consensus cost or limit.
   Where it does not, it is recorded diagnostically and a difference is
   reported, not failed.

Anything that does not meet all five is a protocol upgrade: a new
`engine_profile` number in the DashVM table, the old profile retained so
historical blocks replay under it, activation through the normal upgrade vote.
There is no emergency pause and no new authority for engine changes; the
owner decision is that engine behaviour changes go through normal protocol
upgrades and preserve historical replay.

**A committed wrong result is not a hotfix case.** If the differential run
shows that a committed execution depended on the bug, the chain has already
diverged from the specification and the engine fix cannot restore it. That is
an incident under the existing recovery path, described in the Block Failure
Classes chapter once it lands: failing transitions are removed by the
proposer, no denylist is distributed and no bespoke recovery mechanism is
added.

**Node-local changes are not engine changes.** The compiled-artifact cache
format, compile speed, memory use of the compiler, diagnostics and logging
are never consensus-visible and need only the ordinary tests. A change that
starts as node-local and turns out to alter a result is, by that fact, an
engine change.

**Before activation the rule is vacuous over history.** Until a network
activates contract execution, the set of accepted modules and recorded
executions is empty and the rule reduces to the corpus comparison. Patch
bumps on the long-term-support line are routine in that period and should
be taken promptly, so the line the first profile pins is as fresh as the
evidence allows.

**Until the evidence exists, there is no hotfix.** The rule needs tooling:
the deterministic corpus, cross-architecture replay, and a differential
runner over stored contract versions and recorded executions. Until that
tooling exists and has run, an engine change after activation cannot be
declared equivalent and takes the protocol-upgrade path. This is the
containment window the rehearsal work discloses, and this chapter states it
plainly rather than assuming the evidence.

## Runbook

1. **Signal.** The engine audit turns red, or a maintainer reads an upstream
   advisory or release note that affects a crate in the set.
2. **Classify.** Fill in both axes from the section above using the upstream
   advisory's affected configurations and the pinned profile. Record the
   classification in the pull request that acts on it.
3. **Act on the dependency.** Bump to the patched release on the pinned
   long-term-support line. If no release contains the fix yet, declare a
   temporary patch under `workspace.metadata.dashvm.engine` with the upstream
   fix and an expiry, and add the matching `[patch]` entry. If the advisory
   is unreachable under the profile and no release is available, add a scoped
   acknowledgement with the reason and an expiry.
4. **Produce the evidence.** Not compiled in or not guest-reachable: the
   ordinary test suite plus the corpus run. Guest-reachable: the full
   equivalence run of the rule above, on both architectures, over the corpus
   and (after activation) the recorded executions.
5. **Decide.** Equivalent: node release, with the advisory named in the
   release notes and the classification recorded. Not equivalent, or no
   evidence possible: new engine profile and protocol upgrade. Committed wrong
   result found: incident path.
6. **Retire the exception.** When the patched release ships, remove the
   temporary patch and its declaration; when an acknowledged advisory stops
   being reported, remove the acknowledgement. The audit reports both as
   notices so they are not forgotten.

## Task map

The evidence sources the rule depends on and where they come from. Task
identifiers refer to the smart-contract plan in issue 4626.

| Evidence or mechanism | Provided by |
|---|---|
| Engine crate set, advisory audit, exception expiry, temporary-patch declaration | This chapter and `check-engine-advisories.py` (R08-10) |
| Exact engine release, Cranelift configuration and target profile | Runtime crate, allocation A04 (R08-01) |
| Drift detection of the pinned configuration and dependency identification | R11-01 |
| Deterministic comparison corpus on x86_64 and aarch64 | R03-01 |
| Replay of recorded blocks under the original engine behaviour | R03-02 |
| Cross-architecture replay artifacts compared in CI | R14-01 |
| Differential tooling over stored contract versions and recorded executions | R11-09 |
| Hotfix versus upgrade rule as seen from the recovery side | R11-07 (links here) |
| Rehearsal of engine upgrade containment | FIX-04 |
| Advisory tracking for host-capability dependencies | R03-05 (links here) |
