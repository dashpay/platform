# Platform E2E reliability and CI trigger spec

## Status

The initial design was approved and implemented. The first PR run disproved
one browser diagnosis and exposed the missing genesis initialization described
below. That amendment was independently reviewed, user-aligned, implemented,
and locally verified. A fresh PR run proved the Core convergence fix but showed
that another SPV rejection was still hidden by repeated browser-stream cleanup.
The diagnostic amendment preserved that primary error. Run `30029414181`
confirmed that the remaining failure is regtest difficulty validation, and the
consensus amendment below pins and fixes that final root cause.

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
4. The amended wallet design awaits and memoizes provider initialization.
   This prevents historical synchronization from racing WASM/genesis setup and
   prevents a second account from initializing the wallet-shared provider
   again. `getAccount` also memoizes account creation by index while that await
   is pending, preserving one account object per index.
5. Direct functional ownership now includes `wasm-sdk`, prerequisite workflows
   are enumerated, and Dashmate E2Es require the JS build to succeed before
   they try to download its artifact.
6. Fixture-defining changes now invalidate both consumers' shared cache.
   Triggering those paths without changing the Dashmate-only fingerprint would
   have restored old volumes and skipped the code under test.
7. The regtest no-retarget check runs before the short-history return, and its
   negative regression uses only genesis as context. This proves the new
   equality guard itself rejects a changed target instead of accidentally
   passing through the existing DGW rejection after 24 headers.

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
2. On an earlier rerun, the network survived because the next mined block
   happened to arrive after Core synchronization. Both browser batches then
   failed deterministically during wallet bootstrap. A new wallet has no
   stored block headers, and `createAccount` only initializes the SPV chain
   when stored headers are present. The subsequent height-1 historical batch
   is therefore treated as an unauthenticated remote checkpoint and rejected.
   In Chrome, rejection cleanup cancels the gRPC-web response, and later
   historical cleanup cancels the same tracked stream again. The resulting
   `Client already closed - cannot .close()` masks the underlying SPV
   initialization error.
3. After genesis initialization was added, fresh run `30025175452` still
   reached an SPV rejection in both browser shards. Repeated cleanup of the
   same gRPC-web stream again replaced the primary error with the second-close
   exception. Run `30029414181` preserved the primary error:
   `SPV: Header 2e53e5ed...d8afbd55 is invalid`. The historical request starts
   at height 1 and asks for all 1393 regtest headers. `dash-spv` begins applying
   Dark Gravity Wave after 24 headers, but Dash Core 23 regtest has difficulty
   retargeting disabled and keeps its existing target while blocks are mined
   rapidly. The SPV client therefore rejects Core's valid next header as soon
   as its artificial DGW window fills.
4. A PR that fixes either path does not normally run the affected E2Es.
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
- Rerun `30007308397`: network readiness and browser `getStatus`
  passed, but both browser batches failed from the same block-header
  double-close stack. The successful network startup coincided with a later
  mined block being accepted after every Core node had synchronized,
  corroborating that timing currently decides whether the startup race
  manifests. Later run `30025175452` proved that the convergence gate closes
  it.
- PR run `30017112796`, job `89243212550`: the Core convergence gate worked
  and the local network remained healthy, but browser shard 1 reproduced the
  close stack during a new wallet's `before all` hook. Source tracing showed
  that the close originates from rejection of the first historical headers,
  before the provider has a genesis root.
- PR run `30025175452`: all three Dashmate E2Es passed, including the cached
  local-network job, proving the convergence gate under CI. Both browser
  shards nevertheless failed from `BlockHeadersProvider.headersHandler`
  through historical retry cleanup. The only visible exception was the
  already-closed gRPC-web client, so the underlying SPV error remained hidden.
- PR run `30029414181`, browser shard 2 job `89284360816`: the diagnostic
  change exposed `SPV: Header 2e53e5ed...d8afbd55 is invalid` during a
  height-1 historical request for all 1393 regtest headers. The already-closed
  exception did not replace it.

### Local deterministic reproduction

`createAccount` can be reproduced without a live network by supplying an
online mock transport and an empty chain store, then creating an unsynchronized
account. The current implementation never calls
`blockHeadersProvider.initializeChainWith([], -1)`. The provider later sets a
pending start height of 1, and `SpvChain.addHeaders` explicitly rejects a batch
whose head equals that unauthenticated checkpoint.

A full local E2E run is not a safe baseline on this checkout: another checkout
has a long-running Dashmate network on the host, and this checkout lacks the
generated test environment. The unrelated network must not be stopped or
mutated. CI logs provide the distributed-system reproduction; focused unit
tests will pin both deterministic contracts locally.

The consensus failure has a network-independent local reproduction. Starting
from the configured regtest genesis, mine 23 linked headers one second apart
at the canonical `0x207fffff` target, then mine the next header at the same
target. All headers have valid X11 proof of work and timestamps, but the
current validator rejects the next header once its 24-header DGW window is
available.

Dash Core v23.1.2 is the source of truth for the expected rule. Its regtest
parameters set `fPowNoRetargeting = true`. At the affected heights, Core's
Bitcoin-style difficulty path returns the preceding target between adjustment
boundaries and returns it unchanged at an adjustment boundary. The bundled
`@dashevo/dark-gravity-wave` implementation models regtest as a DGW network
that permits minimum-difficulty blocks; it has no no-retarget parameter.

## Goals

- A new online wallet initializes its SPV chain from the configured network's
  genesis before processing height-1 historical headers.
- Repeated cleanup of a rejected browser stream cannot replace a primary SPV
  validation error with a second-close exception.
- Regtest SPV validation follows Core's no-retarget rule instead of deriving a
  time-based DGW target.
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

### 1. Initialize an empty wallet's SPV chain from genesis

In `packages/wallet-lib/src/types/Wallet/Wallet.js` and
`packages/wallet-lib/src/types/Wallet/methods/createAccount.js`, initialize the
online wallet's block-header provider once per wallet for both populated and
empty stores.
`BlockHeadersProvider.initializeChainWith` already defines the desired
semantics: an empty array initializes the chain from the network genesis,
whereas a populated array restores the authenticated stored checkpoint.
Offline wallets must continue to skip provider initialization.

Store the initialization promise on `Wallet`, and await it before
`account.init` can start. Reuse the same promise for subsequent accounts. This
preserves the provider's existing single-initialization contract, handles
concurrent callers, and propagates initialization failure instead of detaching
an unhandled rejection.

The initialization is conditional on the transport exposing the DAPI client's
block-header provider. Wallet-lib's public transport contract does not require
that internal client, so custom transports must retain their existing
unsynchronized-account behavior. While initialization is pending, `getAccount`
stores the account-creation promise by index so concurrent requests cannot
construct duplicate accounts for the same derivation path.

Add focused regressions proving that:

- an online empty store calls `initializeChainWith([], -1)`;
- an online populated store still forwards the stored headers and height; and
- account creation remains pending until a deferred initialization resolves;
- two accounts share one provider initialization; and
- concurrent `getAccount` requests for one index return the same account;
- initialization rejection propagates before an account is stored;
- a custom online transport without a block-header provider remains usable for
  an unsynchronized account; and
- an offline wallet with no transport/provider creates an unsynchronized
  account successfully.

Also add a `BlockHeadersProvider` integration regression that initializes from
an empty array, feeds the exact regtest height-1 batch, and observes a valid
historical chain. The DAPI integration suite runs under Karma, so this pins the
browser-relevant SPV behavior rather than only asserting a spy call.

Run the empty-store test against the current conditional first and observe it
fail, then add the once-per-wallet initialization and rerun it.

Why this boundary: genesis initialization is the earliest missing invariant.
Changing SPV to trust height 1 would weaken its authenticated-checkpoint
protection.

### 1b. Preserve the primary browser-stream rejection

The upstream browser gRPC client throws
`Client already closed - cannot .close()` when DAPI calls `cancel()` on a
stream that an earlier rejection cleanup already cancelled. Treat only that
exact second-close exception as successful cleanup in
`BlockHeadersReader.cancelStream`. Other cancellation failures must still
propagate.

Add a regression that tracks the historical stream through
`readHistorical`, rejects the received headers with a concrete validation
error, lets the first cancellation succeed, and makes the second cancellation
throw the exact upstream exception. The reader must perform both cleanup
attempts, drain its tracked stream list, and emit the validation error rather
than throwing the cleanup error. Run this test against the old behavior first
and observe it fail.

Why this boundary: catching around only the first rejection-site cancellation
is insufficient because exhausted retries call `stopReadingHistorical`, which
cancels the same tracked stream again. Making the reader's exact
already-closed case idempotent covers both cleanup paths without suppressing
unrelated transport failures. This amendment deliberately reveals, rather than
guesses at, the remaining SPV failure; run `30029414181` supplied the input for
the final regression and fix.

### 1c. Honor regtest's no-retarget consensus rule

In `packages/dash-spv/lib/consensus.js`, retain the existing context-free
checks for a canonical in-range target, proof of work, and timestamp. When at
least one preceding regtest header is available, require the new header's
compact target to equal the immediately preceding trusted header's target.
Run this branch before the existing short-history return as well as before
invoking DGW. Keep the existing DGW path unchanged for devnet, testnet, and
mainnet.

Add a regression that mines the exact fast regtest shape which Core accepts:
genesis plus enough one-second, `0x207fffff` headers to fill the 24-header
window. The next valid header must be accepted. Also start from genesis, mine a
proof-of-work-valid first header with a different canonical target, and assert
that it is rejected before a DGW window exists. Run the acceptance test against
the current validator first and observe it fail.

Why this boundary: a network-wide exemption from target validation would
weaken SPV checks, and modifying `@dashevo/dark-gravity-wave` would expand an
external package API for a Core parameter that `dash-spv` already knows from
its network selection. Comparing with the preceding authenticated header
models no-retarget behavior while retaining all existing contextual checks.

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

This would suppress the browser-visible masking error but leave the new
wallet's chain uninitialized. The historical batch would still be rejected.

### Skip difficulty validation for regtest

Regtest uses cheap proof of work, but Core still enforces a specific target.
Skipping the contextual target check would allow a proof-of-work-valid header
at an arbitrary easier or harder target. The no-retarget equality is both
minimal and stricter.

### Teach `@dashevo/dark-gravity-wave` about no-retarget networks

The dependency calculates DGW targets from timestamps and exposes no height or
no-retarget parameter. Expanding and releasing that package is unnecessary:
`dash-spv` already selects the Core network and can apply regtest's fixed rule
before using DGW for networks that actually retarget.

### Swap the default provider's stream factories

The factory's no-options fallback does pass its callbacks in reverse order, but
`DAPIClient` populates `blockHeadersProviderOptions` by default and the browser
E2E uses that already-correct branch. It is a separate latent bug, not the
failing path, so its initial change and test will be removed from this focused
PR.

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
  -> createAccount initializes empty store from network genesis
  -> default BlockHeadersProvider has an authenticated chain root
  -> regtest headers retain the preceding authenticated target
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
- An online wallet restores an empty store: the existing empty-array provider
  contract initializes network genesis instead of a remote checkpoint.
- A wallet creates multiple or concurrent accounts: all callers await one
  shared initialization promise, so the provider is not initialized twice.
- Concurrent requests for the same account index: `getAccount` returns its
  in-flight creation promise instead of constructing duplicate accounts.
- A custom online transport has no DAPI client internals: account creation
  skips provider initialization, preserving the public transport contract.
- Provider initialization fails: callers receive the failure and no account is
  added to the wallet.
- An offline wallet creates an account: it still avoids all transport and
  provider access.
- A fast-mined regtest chain fills the DGW history window: the validator keeps
  using Core's no-retarget rule and does not derive a false time-based target.
- A regtest peer supplies a header with a changed target: canonical-target,
  proof-of-work, median-time, and no-retarget checks still reject it.

## Verification plan

### Red-to-green unit proof

1. Add the empty-store account regression and run it before changing the
   conditional. Record the failure showing that genesis initialization was
   skipped. Add deferred-initialization and multiple-account cases to pin
   ordering and once-only behavior.
2. Initialize the provider once per wallet for empty and populated online
   stores, await the shared promise, and rerun the focused tests.
3. Add a DAPI provider integration case that starts at genesis and accepts a
   height-1 regtest batch. Run it in Node and Karma, then run the complete
   wallet-lib unit suite and relevant DAPI browser suite.
4. Add the fast-mined regtest chain regression and run it against the existing
   DGW-only behavior. Record the valid header being rejected after the
   24-header window fills. Add the no-retarget branch, rerun the same test, and
   verify a changed-target first header is rejected before the short-history
   return.
5. Add the local-start ordering regression and run it before adding the
   convergence task. Record the missing convergence call, incomplete client
   array, or premature miner call.
6. Add the convergence task and DI registration, rerun the same test, then run
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
