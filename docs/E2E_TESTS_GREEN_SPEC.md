# Platform E2E reliability and CI trigger spec

## Status

Approved by the user and implemented. Post-implementation correctness,
standards, testing, maintainability, reliability, and adversarial reviews were
folded into the change. PR CI validation remains outstanding.

## Independent spec review findings applied

Three reviewers checked the draft against the source from complementary
Dashmate/consensus, CI scope/cost, and regression/failure-mode lenses. Their
must-fix findings are incorporated below:

1. The Dashmate regression now proves that every config contributes its own
   correctly configured RPC client, that peer checks precede convergence, and
   that a deferred convergence gate actually blocks mining. Ordering alone
   could have passed with only the miner client and left the race intact.
2. The convergence safeguard no longer claims Platform services have not
   started. `startNodeTask` starts the full node stack; this gate specifically
   prevents the first post-restore mined block and chain lock until Core tips
   converge.
3. Startup uses the five-minute convergence timeout already established by
   local setup, instead of relying on the helper's one-minute default on loaded
   CI runners.
4. The block-header red test explicitly cancels a mistakenly returned
   reconnecting wrapper in `finally`; otherwise its ten-minute timer could keep
   Mocha alive after the expected assertion failure.
5. Direct functional ownership now includes `wasm-sdk`, prerequisite workflows
   are enumerated, and Dashmate E2Es require the JS build to succeed before
   they try to download its artifact.
6. Fixture-defining changes now invalidate both consumers' shared cache.
   Triggering those paths without changing the Dashmate-only fingerprint would
   have restored old volumes and skipped the code under test.

The reviewers differed on trigger breadth. The failure-mode review recommended
also treating every immediate/transitive `js-dapi-client` dependency (starting
with `dapi-grpc` and `dash-spv`) as an E2E trigger. The CI-scope review
recommended the smaller direct E2E-facing package set because the repository's
full dependency graph quickly expands into common Rust/WASM packages and
launches ten expensive jobs for broad changes. This spec chooses the explicit
direct set, including the client containing this bug, and narrows the goal
accordingly. Broad source coverage continues to use the existing
platform-version gate.

## Problem

PR #4210's platform E2E run exposed two independent failures, while the current
workflow only launches those E2Es for a root platform-version change, a
schedule, or a manual dispatch:

1. The first browser run crashed Tenderdash during Platform height 2.
   Restored Core nodes started with different tips: the seed was at height 1392
   after faucet minting, while the masternodes were at height 1019. Dashmate
   started the miner after peer connectivity, but before all nodes had reached
   the same block hash. One evonode rejected the new chain lock as having an
   invalid signature, Drive rejected the proposal, and Tenderdash stopped.
   The browser's later `getStatus` failure was a secondary symptom of the
   unavailable Tenderdash service.
2. On the latest rerun, the network survived because the next mined block
   happened to arrive after Core synchronization. Both browser batches then
   failed deterministically during wallet bootstrap with
   `Client already closed - cannot .close()`. The default block-header provider
   passes its historical and continuous stream factories to
   `BlockHeadersProvider` in reverse order. A finite historical read therefore
   becomes an unbounded reconnecting stream and reaches an invalid double-close
   path during cancellation.
3. A PR that fixes either path does not normally run the affected E2Es.
   `.github/workflows/tests.yml` gates Docker builds and all platform,
   functional, and Dashmate E2Es on `version-changed`, even when the E2E
   harness or one of its clients changes. A fake version edit would trigger the
   run, but would be unrelated product churn rather than a durable CI rule.

The initial browser error in
`GetStatusResponse.createFromProto` is not a third root cause. RS-DAPI
intentionally permits partial status responses when a downstream service is
unavailable; the JavaScript parser assumes all nested fields are present.
Hardening that parser may be worthwhile separately, but it would only replace
the observed `TypeError` with the test's intended missing-Tenderdash assertion.
It cannot keep Platform consensus alive, so it is outside this fix.

## Evidence and reproduction

### CI evidence

- Original run `30000921951`, commit `d7345cdffb`: both browser jobs failed.
  Core logs show the restored height split, local evonode 3 rejecting the new
  chain lock, Drive returning `InvalidChainLock`, and Tenderdash terminating
  with a consensus failure. The main and functional suites were later
  cancelled; all three Dashmate E2E matrix jobs passed.
- Latest rerun `30007308397`: network readiness and browser `getStatus`
  passed, but both browser batches failed from the same block-header
  double-close stack. The successful network startup coincided with a later
  mined block being accepted after every Core node had synchronized,
  corroborating that timing currently decides whether the startup race
  manifests. Only the resulting CI run can prove that the proposed gate closes
  it.

### Local deterministic reproduction

Calling the default provider's historical factory with
`fromBlockHeight = 5, count = 10` sends only `{ fromBlockHeight: 5 }` through
the continuous reconnecting path. This reproduces the factory inversion
without a live network. The existing unit tests only assert provider type and
options, so they do not detect callback semantics.

A full local E2E run is not a safe baseline on this checkout: another checkout
has a long-running Dashmate network on the host, and this checkout lacks the
generated test environment. The unrelated network must not be stopped or
mutated. CI logs provide the distributed-system reproduction; focused unit
tests will pin both deterministic contracts locally.

## Goals

- A default browser wallet bootstrap uses a finite historical block-header
  stream with both `fromBlockHeight` and `count`.
- A local Dashmate group does not start mining until every Core node reports
  the same height and block hash.
- Changes to the explicit direct E2E-facing package and orchestration set
  automatically build the prerequisites and launch the platform E2Es without a
  version bump.
- The regression tests fail against the current implementation and pass after
  the fixes.
- The resulting PR runs to green for all launched E2E jobs.

## Non-goals

- Redesigning the cached local-network fixture or faucet-minting topology.
- Changing Core chain-lock validation or Drive consensus handling.
- Making every DAPI stream/client close operation idempotent.
- Hardening partial `getStatus` parsing.
- Expanding E2Es to every source change in the monorepo.

## Chosen approach

### 1. Restore the block-header factory contract

In
`packages/js-dapi-client/lib/BlockHeadersProvider/createBlockHeadersProviderFromOptions.js`,
construct the default `BlockHeadersProvider` with:

1. `createHistoricalSyncStream`
2. `createContinuousSyncStream`

This matches the constructor contract and the already-correct
`blockHeadersProviderOptions` branch. No stream implementation or close
semantics change is needed.

Add a unit regression to
`createBlockHeadersProviderFromOptions.spec.js` that creates the default
provider, calls its historical factory with a concrete height and count, and
asserts that `subscribeToBlockHeadersWithChainLocks` receives both values and
returns the finite stream directly. The test must be run against the current
ordering first and observed failing. Because the broken branch returns a
`ReconnectableStream` with a ten-minute timer, retain the returned value and
cancel any wrapper in `finally` so the intentional red assertion cannot hang
the test process.

Why this boundary: it fixes the earliest violated contract. Making cancellation
silently tolerate double-close would hide the fact that historical sync is
using the wrong stream kind and may never terminate.

### 2. Add a Core-tip convergence gate before mining

Use Dashmate's existing
`packages/dashmate/src/core/waitForNodesToHaveTheSameHeight.js` in
`startGroupNodesTaskFactory` for local networks:

1. Start all group nodes.
2. Wait for every Core node to have peers.
3. Build an RPC client for every config and call
   `waitForNodesToHaveTheSameHeight`.
4. Only after the function confirms both equal height and equal hash, start the
   miner.
5. Continue with the existing Platform readiness checks.

Register `waitForNodesToHaveTheSameHeight` in the Dashmate dependency container
and replace the factory's currently unused `waitForMasternodesSync` injection
with it. The convergence task uses the same local-network enablement condition
as the peer and miner tasks. Pass the five-minute timeout already used by
Dashmate's local setup convergence gates: the observed catch-up normally takes
far less, but a loaded CI runner should not reintroduce a timing-dependent
failure through the helper's shorter one-minute default. Preserve the helper's
mismatched-hash error so startup fails loudly instead of allowing an invalid
Platform proposal.

Add a focused unit test for `startGroupNodesTaskFactory` using at least two
distinct local configs and RPC clients. It must assert:

- `createRpcClient` is called once per config with that config's Core RPC port,
  password, and resolved host;
- all peer-connectivity waits complete before convergence starts;
- `waitForNodesToHaveTheSameHeight` receives the complete client array and the
  chosen timeout;
- a deferred convergence promise blocks `dockerCompose.execCommand`, and only
  resolving it permits miner startup.

This test must fail before the new task exists and pass afterward. Also verify
that non-local or non-mining groups do not run the gate.

Why this boundary: peer connectivity and masternode sync status do not prove
all restored nodes share the same active-chain tip. The existing helper checks
the exact prerequisite the first CI run violated.

### 3. Add a path-scoped E2E workflow signal

Extend the `changes` job in `.github/workflows/tests.yml` with an
`e2e-tests-changed` output produced by `dorny/paths-filter@v4`. Set it for
changes to this deliberately direct-owner set:

- `packages/platform-test-suite/**`
- `packages/dashmate/**`
- `packages/js-dapi-client/**`
- `packages/js-dash-sdk/**`
- `packages/wallet-lib/**`
- `packages/wasm-sdk/**`
- `packages/dapi/.env.example`
- `packages/rs-drive-abci/.env.local`
- `.github/actions/aws_ecr_login/**`
- `.github/actions/docker/**`
- `.github/actions/local-network/**`
- `.github/actions/nodejs/**`
- `.github/actions/rust/**`
- `.github/actions/sccache/**`
- `.github/workflows/tests.yml`
- `.github/workflows/tests-build-js.yml`
- `.github/workflows/tests-build-image.yml`
- `.github/workflows/tests-test-suite.yml`
- `.github/workflows/tests-packges-functional.yml`
- `.github/workflows/tests-dashmate.yml`
- `scripts/setup_local_network.sh`
- `scripts/configure_test_suite.sh`
- `scripts/configure_dotenv.sh`
- `scripts/dashmate/volumes/**`

The filter intentionally covers direct suite owners and orchestration rather
than expanding the entire transitive package graph. The latter is already
encoded in `js-packages-no-workflows.yml`, but using it here would cause broad
Rust/WASM dependency edits to launch the expensive E2E fleet. Broad Platform
source coverage remains tied to the existing version-change signal.

Set the signal to `true` in the existing `workflow_dispatch` override.

Include `e2e-tests-changed == 'true'` in the conditions for:

- `build-js`, so workflow-only and orchestration-only changes still produce the
  JavaScript artifacts expected by the reusable tests;
- `build-images`, because all current E2E jobs depend on candidate images;
- `dashmate-e2e-tests`, `test-suite`, and `test-functional`.

Add `needs.build-js.result == 'success'` to `dashmate-e2e-tests`. Its reusable
workflow unconditionally downloads the JavaScript build artifact, and its
current `always()` condition can otherwise start after a failed build and
produce a secondary missing-artifact failure.

Preserve the existing version-change, schedule, and manual-dispatch paths. Do
not change the ECR guard or fork behavior.

The matched fixture-defining paths must also invalidate the cached local
network, or CI would launch while silently restoring a fixture created by the
old scripts. Keep the existing latest-Dashmate-commit component and append the
same `hashFiles` component to both cache consumers
(`.github/actions/local-network/action.yaml` and
`.github/workflows/tests-dashmate.yml`) covering:

- `.github/actions/local-network/action.yaml`
- `packages/dapi/.env.example`
- `packages/rs-drive-abci/.env.local`
- `scripts/setup_local_network.sh`
- `scripts/configure_test_suite.sh`
- `scripts/configure_dotenv.sh`
- `scripts/dashmate/volumes/**`

Using exactly the same key shape in both restore consumers preserves their
shared fixture while forcing the cache-miss setup path whenever fixture
construction or serialization changes. The composite action reuses the
restore step's resolved primary key when saving, preventing its two steps from
drifting. Treat any cache result other than the exact string `'true'` as a
miss; `actions/cache/restore` emits an empty `cache-hit` value when no cache is
found, so checking only for `'false'` could skip the uncached Dashmate test
entirely.

This follows the repository's existing `dorny/paths-filter` convention.
According to the action documentation, each named filter is exposed as a
`'true'`/`'false'` output and pull requests are compared against the merge base.
GitHub's workflow documentation likewise defines PR path evaluation from the
base/head diff:

- https://github.com/dorny/paths-filter
- https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax

## Alternatives rejected

### Fix the symptom in `GetStatusResponse`

Optional nested fields should eventually be handled safely, but doing so does
not fix the stopped Tenderdash process. The status assertions would still fail.
Treat separately to keep this PR tied to E2E reliability.

### Make stream cancellation or gRPC close idempotent

This would suppress the thrown error after the historical and continuous
factories were already confused. It leaves a finite history operation backed
by an infinite reconnecting stream, so it is not a root-cause fix.

### Configure a custom provider only in platform-test-suite

This bypasses the broken default for one caller and leaves every other default
consumer exposed. Fix the shared factory contract once.

### Wait an arbitrary number of seconds or shorten the miner interval

The latest rerun passed the network phase only because timing happened to
allow a later synchronized block. A delay changes probability; it does not
establish the required equal-tip invariant.

### Ignore or retry invalid chain locks

The rejection protects consensus correctness. Weakening it would accept a
proposal whose Core chain lock is invalid for that node.

### Normalize the cached volumes during fixture creation

Faucet minting deliberately starts only the seed Core node, so the persisted
height split is expected. Normalizing the snapshot would require a broader
core-only startup lifecycle and could couple the cache to non-candidate
Platform images. A safe start command should handle a valid uneven restored
state regardless of how it was produced.

### Trigger via an unrelated version bump or a branch-specific condition

Both make this one PR run without protecting the next relevant change.

### Require a PR label

A label such as `run-e2e` controls cost, but it relies on a maintainer or bot to
remember external state. Path-scoped triggering is deterministic and
reviewable alongside the code that defines E2E relevance.

### Run platform E2Es for every monorepo change

This maximizes coverage but incurs expensive Docker builds for documentation
and unrelated packages. The selected paths cover direct E2E consumers and
orchestration while preserving the existing version gate for broad Platform
source changes.

## Interfaces and data flow

```text
PR paths
  -> changes.e2e-tests-changed
  -> build-js + build-images
  -> cache hit only when Dashmate + fixture-defining inputs match
  -> platform suite + functional suite + Dashmate E2Es

dashmate group start (local)
  -> start nodes concurrently
  -> wait for Core peers
  -> wait for equal Core height and hash
  -> start miner
  -> wait for Platform readiness

browser wallet bootstrap
  -> default BlockHeadersProvider
  -> finite historical stream(fromBlockHeight, count)
  -> continuous reconnecting stream(fromBlockHeight), only after history
```

## Failure modes and safeguards

- A Core node never catches up: group startup fails after five minutes, before
  mining can introduce a new chain lock on inconsistent restored state.
- Nodes reach the same height on different forks: group startup fails
  immediately with the helper's existing hash-mismatch error.
- There is no local miner: peer/convergence/miner tasks remain disabled as a
  group, preserving testnet and non-mining behavior.
- ECR credentials are unavailable: image builds and dependent E2Es remain
  skipped with the existing notice; no attempt is made to expose secrets to
  forks.
- A relevant workflow-only path changes: `build-js` is forced so the E2E
  reusable jobs do not become skipped through an unmet `needs` result.
- A fixture-defining script or action changes: the shared content hash changes,
  forcing both cache consumers through fixture construction with the new
  inputs.
- The path list drifts as the test architecture changes: workflow changes
  themselves are included, causing the revised orchestration to exercise its
  own E2E path.
- A caller supplies `blockHeadersProviderOptions`: behavior is unchanged and
  already uses the correct callback order.

## Verification plan

### Red-to-green unit proof

1. Add the default historical-stream regression and run it before the callback
   swap. Record the failure showing the finite `{ fromBlockHeight, count }`
   request was not forwarded.
2. Swap the callbacks and rerun the same test. Record it passing, then run the
   complete `js-dapi-client` coverage suite and its Karma browser suite.
3. Add the local-start ordering regression and run it before adding the
   convergence task. Record the missing convergence call, incomplete client
   array, or premature miner call.
4. Add the convergence task and DI registration, rerun the same test, then run
   the complete Dashmate unit suite.

If generated WASM artifacts or Yarn unplugged dependencies block those suites,
install/build the repository-prescribed prerequisites; do not bypass the test
or silently report it as passing.

### Static workflow checks

- Parse the edited YAML.
- Verify the new output is set for PR changes and manual dispatch.
- Verify every prerequisite and E2E condition consumes the signal.
- Verify Dashmate E2Es require both build results and that both local-network
  cache consumers have an identical fixture-input hash component.
- Review the filter paths against every local-network script and reusable
  workflow loaded by the platform tests.

### PR/CI proof

1. Commit and push the focused changes without a platform version bump.
2. Open a PR against `v4.1-dev`.
3. Confirm the path-scoped signal causes Docker builds and all platform E2E
   jobs to launch.
4. Monitor with `gh pr checks` and inspect failures with `gh run view`.
5. Iterate on root causes until every launched E2E job is green. Do not call the
   task complete if a job was skipped unexpectedly or the run was cancelled.
