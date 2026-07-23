# Platform E2E reliability and CI trigger spec

## Status

The initial design was approved and implemented. The first PR run disproved
one browser diagnosis and exposed the missing genesis initialization described
below. That amendment was independently reviewed, user-aligned, implemented,
and locally verified. A fresh PR run proved the Core convergence fix but showed
that another SPV rejection was still hidden by repeated browser-stream cleanup.
The diagnostic amendment preserved that primary error. Run `30029414181`
confirmed the regtest difficulty mismatch, and the consensus amendment pinned
and fixed it. Run `30031305459` then exposed a separate persisted-wallet
restore bug: stored headers were indexed from the stored tip height instead of
their first authenticated height. The restore amendment below preserves the
remote-checkpoint rejection while correcting that local anchor. The same run
then exposed three compatibility gaps introduced or revealed by the broadened
E2E trigger: the legacy proof verifier still requested strict execution
evidence for transition families that can only return affected-state proofs,
the WASM epoch tests still used the intentionally disabled implicit-current
proof query, and document transitions did not invoke the existing client-only
binary-property sanitizer. Run `30036234705` proved all three fixes and exposed
one further proof-context gap: the generic WASM state-transition wait APIs do
not preload dynamically created data contracts before verifying document-batch
proofs. The same run exposed a pre-existing self-hosted Swift runner failure:
one runner reuses an unwritable fixed-path CI keychain and aborts signing tests
before any branch FFI code executes. Final-head run `30041868263` proved the
WASM fix but exposed a second persistent-runner state: the user preference
domain has no resolvable default keychain, so querying the previous default
fails before ephemeral provisioning begins.

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
8. The affected-state adapter no longer claims that a snapshot proves exact
   execution or absence of a consensus error. A focused rejection regression
   is required because the legacy client's consensus-error response bypasses
   the proof adapter.
9. Epoch tests require `status.time.epoch` to be present and preserve the
   original `current - 5` range semantics; there is no implicit or zero
   fallback that could recreate the prohibited query.
10. Document normalization uses the production
    `DocumentTypeV0Methods::sanitize_document_properties` API. The 32-byte DPNS
    property must become `Value::Bytes32`, and the caller-owned document must
    remain unchanged.
11. `IPlatformProofVerifier` documentation distinguishes full query binding
    from the state-transition adapter's affected-state guarantee.
12. The custom-contract preparation stays in the WASM layer because that is
    the narrowest owner of both the trusted cache and all four generic
    proof-wait wrappers. Pure extraction tests cover duplicate and distinct
    contract IDs plus non-batch transitions; a mock SDK/trusted-context test
    covers proved fetch, cache replacement, absence, and error propagation.
    The existing E2E failures remain the integration proof.
13. Preparation proof-fetches and overwrites referenced contracts even on a
    cache hit. The cache is keyed only by contract ID, so reusing a hit could
    retain a pre-update schema and leave the generic wait non-self-contained.
14. Contract proofs are current at preparation time but are not bound to the
    later state-transition result's proof height. Verification follows the
    existing trusted-schema semantics; this fix does not claim same-root
    atomicity.
15. Swift cleanup is installed before any mutating keychain setup, uses a
    random password and finite unlock timeout, and preserves the primary exit
    status while making cleanup failure fail an otherwise successful job.
16. The ephemeral Swift keychain is added to the complete user-domain search
    list before it becomes the default. Cleanup restores both the selected
    baseline default and its live search list because `WalletStorage` reads and
    deletes through search-list resolution. An already coherent list is
    preserved exactly.
17. A missing user-domain default is recoverable runner state, not a reason to
    skip Swift tests. Capture and sanitize the complete search list first,
    prefer a valid per-user login keychain and otherwise its first valid
    per-user entry as a deterministic cleanup fallback, and require a
    restorable fallback before mutating Security preferences. A fallback must
    be a canonical regular file below the runner's canonical
    `$HOME/Library/Keychains`, owned and writable by that user; shared/system or
    arbitrary temporary files are not automatic default candidates. The
    repaired live-only list becomes the new baseline only after list, default,
    and readback all succeed. A mocked shell regression pins the exact exit-44
    condition and partial-mutation rollback.
18. The Swift keychain shell code remains compatible with Apple's
    `/bin/bash` 3.2: it uses indexed arrays and exact linear deduplication, not
    associative arrays, `mapfile`/`readarray`, or Bash 4 case-conversion
    expansions. The fake Security implementation validates command order and
    permits mutation only of the job's temporary keychain.
19. Swift workflow wiring is part of the Swift trigger surface. Changes to the
    reusable workflow itself launch its path-filtered job, and the deterministic
    keychain regression runs explicitly under `/bin/bash`.

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
4. Final run `30031305459` passed the new consensus regressions, then browser
   shard 2 failed during DPNS setup with `SPV chain cannot initialize from an
   unauthenticated remote checkpoint`. The cached faucet wallet restores an
   ordered array of authenticated headers plus the height of its last header.
   `createAccount` passed that tip height to
   `BlockHeadersProvider.initializeChainWith` as the first header's height.
   The resulting height map could not supply the trusted predecessor requested
   by the next historical sync, so the provider reset into the intentionally
   prohibited remote-checkpoint state.
5. Browser shard 1 in the same run failed while topping up an identity during
   the duplicate-asset-lock regression. The test-suite proof verifier calls
   Evo SDK's strict `waitForResponse`. Proof hardening now rejects the
   `VerifiedPartialIdentity` returned for top-ups because it authenticates the
   affected identity snapshot, not execution of the exact transition. Evo SDK
   provides `waitForAffectedState` for precisely these transition families,
   while preserving consensus-error failures and strict execution results when
   they are available.
6. The WASM functional job first exhausted all addresses after three successful
   `getEpochsInfo` responses. The swallowed first error was
   `proved descending epoch queries require an explicit start epoch`.
   Security hardening intentionally disabled `getCurrentEpoch` because its
   implicit descending bound came from unsigned response metadata. The
   functional tests still call that method before every explicit epoch query
   and swallow the error, leaving only the misleading address-exhaustion
   symptom.
7. The same functional job rejected the first DPNS preorder create with
   `not an array of bytes`. `Document` converts the JavaScript
   `Uint8Array` property through a schema-agnostic JSON representation, which
   becomes an integer `Value::Array`. DPP already has a schema-aware,
   client-only sanitizer that converts integer arrays on `byteArray` fields to
   canonical binary Value variants (`Value::Bytes32` for this field), but the
   Rust SDK document transition path never invokes it.
   The remaining document failures are dependent cascades from that first
   create and the missing custom contract/document IDs.
8. Run `30036234705` passed the functional package suite, including all six
   explicit epoch queries and the document normalization path. Browser shard 1
   then failed after a successful top-up when it created another document
   under a custom test contract. The affected-state proof was returned, but
   verification failed with `unknown contract in documents batch transition`.
   The transition contains the contract ID, yet none of the four generic WASM
   proof-wait methods loads that contract into the trusted context before the
   Drive proof verifier asks for it.
9. A PR that fixes either path does not normally run the affected E2Es.
   `.github/workflows/tests.yml` gates Docker builds and all platform,
   functional, and Dashmate E2Es on `version-changed`, even when the E2E
   harness or one of its clients changes. A fake version edit would trigger the
   run, but would be unrelated product churn rather than a durable CI rule.
10. Swift job `89307788524` failed both
    `IdentityResolverSignIntegrationTests` when `SecItemAdd` returned `-61`
    (`Write permissions error`). The same two failures occur on base branch run
    `30023633141` on `mac-runner-1`, while nearby runs on `mac-runner-2` pass.
    `run_tests.sh` always reuses
    `~/Library/Keychains/dash-ci-tests.keychain-db` and suppresses
    `security create-keychain` failure, so stale persistent runner state reaches
    the test suite without any writable-keychain check.
11. Final-head Swift job `89327459555` in run `30041868263` failed before
    `mktemp` or the cleanup trap. Its first
    `security default-keychain -d user` command returned exit 44 with
    `SecKeychainSearchCopyNext: The specified item could not be found in the
    keychain`. Apple documents that a missing default is not recreated
    automatically; the user-domain search list is the remaining authoritative
    source of pre-existing keychains.

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
- PR run `30031305459`, browser shard 2 job `89290927444`: the regtest header
  sequence advanced past the former difficulty failure. DPNS bootstrap then
  exposed the persisted-header indexing bug as
  `SPV chain cannot initialize from an unauthenticated remote checkpoint`.
- PR run `30031305459`, browser shard 1 job `89290927408`: the identity
  duplicate-asset-lock test failed during its successful top-up setup because
  strict proof waiting rejected a verified affected-state identity snapshot.
- PR run `30031305459`, functional job `89290927040`: all three nodes returned
  `getEpochsInfo` successfully, but local proof verification rejected the
  implicit descending query and exhausted the address pool. Replaying the
  exact CI WASM bundle against the existing local network with SDK tracing
  exposed the hidden proof error. The first document broadcast later reached
  Tenderdash and was rejected with the schema binary conversion error; the
  following document failures were setup cascades.
- PR run `30036234705`: the functional package job passed, proving the explicit
  epoch selectors and Rust SDK document normalization in the full WASM path.
  All three Dashmate jobs and Rust/clippy also passed. Browser shard 1 job
  `89307436802` advanced through the previous top-up failure, then rejected a
  document-batch proof because its custom contract was absent from the trusted
  context. The main suite's data contract update case failed at the same
  custom-contract document boundary.
- PR run `30036234705`, Swift job `89307788524`: 281 tests executed, eight
  skipped, and only the two signing integration tests failed with keychain
  error `-61` on `mac-runner-1`. Base push run `30023633141` failed identically
  on the same runner and commit `ae554fdd`; three nearby jobs passed on
  `mac-runner-2`, establishing persistent runner state rather than a branch
  behavior regression.

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

The epoch failure also has a safe, read-only local reproduction. The exact
WASM bundle built by run `30031305459` succeeds at trusted-context setup and
receives a proved `getEpochsInfo` response from the existing local gateway.
SDK tracing then reports that the descending request has no explicit start
epoch. An explicit query using the epoch already returned by `getStatus`
verifies successfully.

The custom-contract failure has a deterministic, network-independent boundary:
a batch state transition exposes every referenced contract ID through
`transitions_iter()`, while `waitForResponse`, `broadcastAndWait`,
`waitForAffectedState`, and `broadcastAndWaitForAffectedState` currently pass
the transition directly to proof verification without consulting
`get_or_fetch_contract`. A focused unit test can construct a batch containing
duplicate transitions for one non-system contract and prove that context
preparation extracts that ID exactly once.

## Goals

- A new DAPI-backed online wallet initializes its SPV chain from the configured
  network's genesis before processing height-1 historical headers. Custom
  online transports without DAPI internals retain their existing behavior.
- Repeated cleanup of a rejected browser stream cannot replace a primary SPV
  validation error with a second-close exception.
- Regtest SPV validation follows Core's no-retarget rule instead of deriving a
  time-based DGW target.
- A persisted wallet restores each authenticated header at its actual height,
  so subsequent synchronization stays anchored to local state.
- Legacy test-suite state-transition verification accepts authenticated,
  height-pinned affected-state results for transition families whose proofs
  cannot establish exact execution, without accepting unproved data or
  suppressing consensus errors.
- WASM functional tests issue request-bound epoch proofs instead of the
  intentionally prohibited implicit-current proof query.
- Rust SDK document transitions normalize schema-declared binary properties at
  the client boundary before strict serialization.
- Generic WASM proof-verifying state-transition waits preload every referenced
  batch contract into the trusted context, including dynamically created
  contracts, before Drive proof verification.
- Swift CI provisions a fresh writable keychain for each job and restores the
  runner's prior default keychain even when tests fail.
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
- Re-enabling implicit-current proved epoch queries or trusting response
  metadata as a proof selector.
- Weakening Drive's unknown-contract proof rejection or trusting a contract
  supplied by an unverified state-transition payload.
- Changing production `WalletStorage` behavior or replacing Keychain-backed
  signing tests with an in-memory implementation.
- Relaxing platform-value's strict binary serializer, which is also used on
  consensus/app-hash paths.
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

### 1d. Restore persisted headers at their actual first height

`ChainStore` keeps headers in ascending order and records
`lastSyncedHeaderHeight` for the array's final header. Before calling
`BlockHeadersProvider.initializeChainWith`, derive the first authenticated
height as:

`lastSyncedHeaderHeight - blockHeaders.length + 1`

Keep the existing empty-store value unchanged because the provider ignores
that height when it initializes from genesis. Add a regression with three
stored headers ending at height 42 and require initialization at height 40.
Run it against the current caller first and observe the exact 42-versus-40
failure.

Why this boundary: weakening `SpvChain` to accept a remote root would undo the
security invariant added by the batch-validation fix. Fetching a new remote
checkpoint would not authenticate it. Correctly indexing the already
authenticated local headers restores the predecessor that `ensureChainRoot`
expects without changing any trust decision.

### 1e. Use affected-state proof waiting in legacy test clients

Change the platform-test-suite proof verifier's state-transition check from
`waitForResponse` to `waitForAffectedState`. Apply the same correction to the
bench-suite copy so the two legacy `Dash.Client` factories retain one proof
contract. Update `IPlatformProofVerifier` documentation to distinguish its two
method guarantees: data-contract history remains fully request-bound, while
state-transition success can mean either exact execution evidence or an
authenticated, height-pinned affected-state snapshot for families whose proof
cannot bind execution.

The affected-state API remains a proved path: it verifies the GroveDB proof
and Tenderdash quorum signature, accepts strict execution-proved outcomes when
available, propagates proved-wait errors, and additionally accepts the explicit
affected-state result variants used by balance top-ups, transfers, withdrawals,
address movements, shields, and no-history token operations. Its weaker
guarantee must be stated accurately in the verifier comments: a snapshot
authenticates affected state at a height but is not evidence that the exact
transition executed or that it had no consensus error. The legacy caller
rejects consensus errors from its original response before invoking the
adapter, and the top-level adapter documentation must not retain either
stronger claim.

Run the existing duplicate-asset-lock browser regression against the strict
wait first and retain its observed failure. Its successful top-up exercises the
compatibility need, but the later duplicate-spend consensus error is thrown by
the legacy client before the proof adapter is called. Add a focused adapter
regression with an injected Evo SDK loader to prove that an affected-state wait
rejection propagates unchanged instead of being converted to success.

Why this boundary: weakening the Evo SDK strict wait would hide guarantee
differences for all consumers. The dedicated affected-state API makes the
limited guarantee explicit at the legacy adapter that can accept it.

### 1f. Make functional epoch queries request-bound

The WASM functional setup already calls `getStatus` and records the evonode
ProTxHash. Record `status.time.epoch` at the same point and use that value as
the explicit basis for every query in the file. Preserve the range tests'
existing coverage by using `Math.max(0, epochIndex - 5)` as the start for
epoch/finalized-epoch ranges and `epochIndex` itself for proposed-block
queries. Require the optional status field to be present and fail loudly
without an implicit or zero fallback. Remove the repeated
`getCurrentEpoch().catch(() => null)` calls.

The status value is not treated as authenticated query output; it only chooses
the exact epoch the subsequent GroveDB proof must authenticate. This matches
the migration contract documented by the proof-hardening change: callers must
fetch a specific epoch by explicit index until Platform exposes an
authenticated current-epoch marker. The tests assert API shape and proof
handling, not a cryptographic claim that the selected status epoch is the
unique chain tip.

Why this boundary: restoring the old metadata fallback would reintroduce a
stale-proof attack, while skipping the tests would discard coverage for six
public WASM query methods. Retrying address exhaustion would only repeat a
deterministic local proof rejection.

### 1g. Sanitize document properties at the Rust SDK client boundary

Before `PutDocument` constructs either a create or replacement transition,
clone the supplied document, import `DocumentTypeV0Methods`, and call the
production `sanitize_document_properties` trait method on its
`properties_mut()`. Use the sanitized clone for transition construction and
leave the caller's document unchanged.

Add a focused Rust SDK regression using a schema-declared `byteArray` property
and the exact integer-array shape produced by WASM's schemaless conversion.
Put clone-and-sanitize behavior in a pure transition-boundary helper so the
regression compiles before the behavior changes. The helper must initially
return the unchanged integer array, then return the canonical binary Value
variant after the fix; the exact 32-byte DPNS `saltedDomainHash` must become
`Value::Bytes32`, while the original caller-owned document remains an integer
array. Invoke the helper once before the create/replacement branch so both
transition paths use the same normalization.

The sanitizer is deliberately client-only. Consensus continues to require a
canonical binary Value variant; platform-value serialization and Drive
validation are unchanged. Creation and replacement share the same boundary
because both serialize document properties, while delete, transfer, price, and
purchase transitions do not need property normalization.

Why this boundary: `DocumentWasm` cannot identify binary properties before it
fetches the data contract. The Rust SDK has both the document and its resolved
document type immediately before building the transition, making it the
narrowest shared client boundary for WASM and other SDK callers.

### 1h. Prepare trusted contract context before state-transition proof waits

In `packages/wasm-sdk/src/state_transitions/broadcast.rs`, derive the unique
data contract IDs referenced by a `StateTransition::Batch` using
`DocumentsBatchTransitionAccessorsV0::transitions_iter()` and each batched
transition's `data_contract_id()`. Before invoking any SDK operation that waits
for and verifies a state-transition proof, proof-fetch each referenced
contract from Platform and overwrite its trusted-context entry. Factor the
existing fetch-and-cache half of `WasmSdk::get_or_fetch_contract` into a
private refresh method; ordinary document/query callers retain cache-first
behavior, while proof-wait preparation always refreshes because the cache is
keyed only by contract ID and carries no version or proof height.

Apply the preparation to all four proof-verifying public methods:
`waitForResponse`, `broadcastAndWait`, `waitForAffectedState`, and
`broadcastAndWaitForAffectedState`. Plain `broadcastStateTransition` remains
unchanged because it does not verify a response proof. Non-batch transitions
produce an empty ID set. Deduplicate IDs so a multi-operation batch pays at
most one lookup per contract; use deterministic ordering to make behavior and
tests stable.

Add pure Rust regressions for `referenced_contract_ids` using a batch with a
duplicate custom contract, a second distinct contract, and a non-batch
transition. Add a focused refresh test using the existing mock Dash SDK and
`WasmTrustedContext::for_testing`: start without the custom contract, return a
proved mock contract, observe it in the trusted cache, then seed an older
same-ID version and prove refresh overwrites it. Authenticated absence and
fetch errors must propagate without reaching a wait. A small `#[cfg(test)]`
constructor is acceptable; do not add production fetcher/verifier traits or
four duplicate wrapper tests. Introduce the ID helper with the current empty
behavior, run the extraction test to red, then implement traversal and refresh.
Static source-order inspection supplements these unit tests; the existing
browser/main failures provide the real network proof-verification integration.

Why this boundary: changing the platform-test-suite adapter to issue a manual
`getDataContract` query would duplicate transition parsing in JavaScript and
would still not populate the trusted context because the public query currently
returns the contract without caching it. Making all public contract queries
cache results would improve later calls but would not make a standalone proof
wait self-contained. Suppressing Drive's unknown-contract error would instead
remove the schema needed to verify document proofs. Preparing the trusted
context inside the WASM proof-wait boundary preserves proof verification and
works for every caller. This mirrors the native SDK context provider's
cache-miss behavior, which fetches the current proved contract during
verification.

Combined `broadcastAndWait*` calls prepare context before broadcasting and
therefore can fail without submitting a transition. Manual
`broadcastStateTransition` followed by `waitFor*` prepares only during the
second call. The public documentation must describe the operations as the
combined form of broadcast and wait without claiming identical side-effect
ordering.

The contract fetch and the later transition-result proof are independent
quorum-authenticated requests. The fetched contract is current at preparation
time, not proven at the eventual wait result's height. If a contract update
lands between those requests, verification follows the existing
`ContextProvider` trusted-schema semantics; binding schema and result to one
proof root needs a larger proof-format and API change.

### 1i. Provision an ephemeral writable Swift CI keychain

In `packages/swift-sdk/run_tests.sh`, replace the fixed keychain under the
persistent runner home with a unique directory created by `mktemp -d` under
`RUNNER_TEMP` (falling back to the system temporary directory). Create one
keychain inside a mode-700 directory for the current job and do not suppress
provisioning errors. Generate a per-job random password without printing it,
unlock with that password, configure a finite timeout longer than the
90-minute workflow limit, add it to the user-domain search list, and select it
as the user-domain default. Capture the raw default and complete search list
before mutation, then establish the restorable baseline described below.

Before building, perform a throwaway generic-password add/read/delete cycle
after selecting the keychain, compare the read value exactly without logging
it, and assert that the selected user default is the ephemeral path. Add to the
explicit job keychain, but read and delete through the default search path just
as `WalletStorage` does. This turns stale permissions, search-list omissions,
or Security session failures into a short provisioning diagnostic instead of
two opaque test failures after the iOS framework build.

Install the EXIT trap after read-only preference discovery and immediately
after the temporary directory and keychain path are assigned, before
`security create-keychain` or any other mutating setup. Track whether the
keychain was created and whether the default changed. On every exit, capture
the original status, restore the selected baseline default,
restore the validated baseline user search list, delete only the explicit
ephemeral keychain through `security delete-keychain`, and remove its now-empty
directory. A pre-existing nonzero status remains authoritative; cleanup
failure makes an otherwise successful run fail so a dirty persistent runner is
never reported green.

The repository currently registers two independently named self-hosted macOS
runners, and a self-hosted runner agent executes one job at a time. This design
relies on one runner registration/user domain per host. The unique path removes
cross-run file reuse, but `security default-keychain -d user` is not safe for
two runner agents sharing the same macOS user; that infrastructure invariant
must remain true.

The existing `IdentityResolverSignIntegrationTests` remain the behavioral
regression because they exercise `WalletStorage`, identity resolution, and FFI
signing end to end. Verify the current fixed-path script's `-61` failure and
the new ephemeral setup on `mac-runner-1`, then run the complete Swift job so
both `swift test` and the simulator `xcodebuild` phase finish.

Why this boundary: deleting or repairing the fixed home keychain would mutate
shared runner state and can race concurrent or interrupted jobs. Retrying
`SecItemAdd` cannot make an unwritable keychain writable. Mocking Keychain in
the integration tests would hide the infrastructure contract they are intended
to verify. A per-job keychain removes cross-run state while preserving the
production storage and signing path under the one-job-per-runner-user
invariant.

### 1j. Recover when the Swift runner has no resolvable default keychain

In `packages/swift-sdk/run_tests.sh`, read and parse the complete user-domain
search list before querying the current default. Keep the raw list and repaired
candidate separate. Canonicalize and exact-deduplicate candidate entries
without reordering them. A keychain is eligible as an automatic recovery
default only when its final path component is not a symlink, it resolves to a
regular file strictly below the canonical
`$HOME/Library/Keychains` directory, and it is owned and writable by the runner
user. Candidate entries must originate in the user-domain list or be the exact
conventional login path. This prevents an arbitrary readable file in `/tmp` or
another user's/shared keychain from becoming the CI user's default merely
because it appeared in stale preferences. Do not use
`security show-keychain-info` as a validity probe: macOS 26.5.1 returns
`errSecParam` for this runner's known-good login keychain. The transactional
list/default mutation and readback below are the Security acceptance check.

If the default query succeeds and resolves to a live keychain already in the
search list, preserve it and the coherent search list exactly. If the query
returns a missing path, or fails with the observed missing-default shell status
44 or no-default status 37, first consider the conventional per-user
`$HOME/Library/Keychains/login.keychain-db` only when it passes the same
eligibility predicate, then fall back to the first eligible search-list
keychain. A recovered login keychain may be added to the candidate list even
when stale preferences omitted it because its canonical location, ownership,
writability, and conventional per-user location independently establish it as
the runner's keychain. Put the selected recovery default first and keep the
remaining eligible candidate order stable. Fail before creating the ephemeral
keychain or mutating Security preferences when the raw list is empty, neither
source yields a restorable keychain, or the default query returns any other
status. A nonempty raw list is required because the Security CLI rejects
`list-keychains -d user -s` with zero entries, so an empty list cannot be
restored transactionally.

The temporary keychain flow remains unchanged: prepend the unique keychain to
the saved search list, make it default, smoke-test unqualified reads/deletes,
and run the real Swift tests. Install the EXIT trap after a private temporary
directory and path are assigned but before any Security preference repair or
keychain creation. Treat repair as a small transaction:

1. Mark the raw search list as needing restoration before setting the candidate
   list.
2. Set the candidate list before setting its default because Security only
   accepts a default present in the active list.
3. Immediately before attempting to set the candidate default, promote the
   candidate list to the containment state. A Security command could mutate the
   preference and still return nonzero, so containment cannot wait for a
   successful exit status.
4. Read both preferences back and compare them to the candidate state.
5. Promote the candidate to the cleanup baseline only after all list, default,
   and readback operations succeed.

A list-setting failure restores the raw search list. From immediately before
the default-setting attempt onward, any command failure deliberately retains
the candidate list so a possibly newly selected default remains resolvable;
restoring a raw list that omitted the recovered login keychain could create a
new dangling preference. No temporary keychain has been created at this point.
After promotion, cleanup restores the selected baseline default while that
keychain is still in the active temporary-plus-baseline list, restores the
baseline list, deletes only the temporary keychain, and preserves the original
exit status. Flags are set before each mutating command so a partially
successful command cannot evade cleanup. This deliberately repairs only
dangling preferences by selecting a pre-existing, validated per-user keychain;
it does not create, delete, unlock, reconfigure, or item-mutate any pre-existing
keychain.

Keep this logic compatible with Apple's `/bin/bash` 3.2. Use indexed arrays,
quoted expansions, and a linear exact-string membership check for stable
deduplication. Do not use associative arrays, `mapfile`/`readarray`, or Bash 4
case-conversion syntax. Resolve directories with `cd -P` plus `pwd -P`, reject
a symlink final component before appending its basename, and do not assume GNU
`readlink -f`. Under Bash 3.2 with `set -u`, even an initialized empty indexed
array can fail on `"${array[@]}"`; length-guard every possibly empty loop or
argv expansion.

Add `packages/swift-sdk/tests/run-tests-keychain-regression.sh`, using a
PATH-injected fake `security` command and a fake `xcrun`, and invoke it from the
Swift reusable workflow under `/bin/bash` before the real build. The harness
uses a private temporary `HOME` and `RUNNER_TEMP`; its fake Security command
allowlists exact argv shapes, never records password or smoke-test values, and
models the real invariant that `default-keychain -s PATH` fails unless `PATH`
is in the active search list. It rejects any create, delete, unlock,
configuration, or item mutation against a seeded keychain; only the temporary
keychain may be changed.

Seed an existing login keychain, a second eligible keychain that precedes it,
duplicate canonical spellings, and a dead search-list entry while making the
initial default query return the observed exit 44. Against the current script,
the regression must fail with 44 before provisioning. After the fix, force a
later simulator-selection failure and assert that its status remains
authoritative, login becomes the recovered default, the canonical live-only
search list is restored, and the temporary keychain and directory are removed.
The command log must show cleanup in baseline-default, baseline-list,
temporary-delete order without containing generated secrets.

Add distinct cases for a valid coherent default that is not the first
search-list entry, exit-zero with a now-missing default path, a missing login
keychain that selects the first eligible list entry, an empty raw list under
`/bin/bash -u`, and no eligible fallback failing before preference mutation or
temporary-keychain creation. An unrelated default-query failure must preserve
its status and perform no mutation. Inject a failure at every repair and
temporary preference mutation/readback boundary; each case must preserve the
primary failure, restore the documented raw or candidate-containment or
committed-baseline state, and leak no temporary keychain. In particular,
simulate a candidate-default command that mutates then fails, and inject each
readback failure after recovering a login keychain absent from the raw list;
the candidate list must still contain the possibly newly selected default. The
EXIT trap cannot recover from SIGKILL, so the one-runner-per-user operational
invariant and future stale-entry sanitization remain necessary.

Add `.github/workflows/swift-sdk-build.yml` to the `swift-sdk-changed` path
filter in `.github/workflows/tests.yml`, so a change to this regression wiring
launches the Swift job. The real workflow may retain its existing `bash`
entrypoint while the compatibility regression explicitly uses `/bin/bash`; if
the real invocation is changed to `/bin/bash`, update
`.github/scripts/check-swift-self-hosted-runner.py` to recognize the new form.

Alternatives rejected:

- Treat exit 44 as an infrastructure-only failure: this leaves every job on
  that persistent runner red even though a private temporary keychain is the
  intended non-interactive solution.
- Create a permanent replacement login keychain: its password and ownership
  become new shared runner state outside the job lifecycle.
- Leave the temporary keychain behind when no default can be queried: cleanup
  would leak both a keychain and its now-forgotten random password.
- Restore the raw search list after the job: dead entries would recreate the
  incoherent default/search context that caused exit 44. The raw list is used
  only for rollback until a validated replacement baseline commits.
- Change `WalletStorage` to pass an explicit keychain: that changes production
  API behavior to solve CI host state and bypasses the integration contract.

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

### Restore unsigned metadata as the implicit current-epoch bound

The proof-hardening change intentionally removed this behavior: a node could
select a stale epoch and produce a valid proof for it. Functional coverage
must migrate to an explicit request selector instead of weakening proof
verification.

### Relax binary serialization to accept integer arrays

The strict serializer is used by consensus and app-hash paths. Accepting a new
wire shape there would create a rolling-upgrade divergence risk. Normalize
only at the SDK client boundary where the resolved schema identifies
`byteArray` fields.

### Prefetch custom contracts in the platform-test-suite adapter

The adapter can parse the transition and call `sdk.contracts.getDataContract`,
but the public query does not currently cache its result in the trusted
context. Reimplementing cache preparation in every JavaScript caller would
also leave the generic WASM proof-wait API unsafe to use directly. The WASM
boundary already owns both transition parsing and the proof context.

### Ignore unknown contracts during document-proof verification

The verifier needs the resolved document schema to interpret and authenticate
the proved values. Treating an unknown contract as success would weaken proof
verification rather than repair its missing trusted input.

### Repair or silently reuse the fixed Swift CI keychain

The self-hosted runner persists home-directory state between jobs. Reusing,
deleting, or chmod-ing one global keychain couples runs and can interfere with
an interrupted or concurrent job. Suppressing creation failure is what allowed
the unwritable state to reach the tests. A unique temporary keychain makes
ownership and cleanup job-local.

### Mock Keychain in the Swift signing integration tests

Those tests intentionally exercise production `WalletStorage` before FFI
signing. Replacing the storage backend would make CI green by removing the
failing system integration, not by providing the writable Keychain contract
the application requires.

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

legacy state-transition verification
  -> reject a consensus error in the original response
  -> parse the submitted transition
  -> wait on Evo SDK's proved affected-state endpoint
  -> accept execution proof or authenticated height-pinned affected state
  -> propagate proof or API errors

WASM epoch functional query
  -> getStatus supplies an explicit epoch selector
  -> proved query authenticates that exact requested epoch

WASM Document Uint8Array
  -> schema-agnostic integer Value::Array
  -> Rust SDK resolves document type
  -> client-only schema sanitizer produces canonical Value::Bytes32
  -> strict transition serialization remains unchanged

WASM document-batch proof wait
  -> extract and deduplicate referenced contract IDs
  -> proof-fetch every referenced contract at preparation time
  -> overwrite the trusted cache with the verified contract version
  -> verify the state-transition result proof with the complete context

Swift CI test process
  -> sanitize the user search list and establish a live prior-or-recovery default
  -> create one keychain under a job-local temporary directory
  -> unlock/select it, add it to search, and prove default add/read/delete access
  -> run signing integration and simulator tests
  -> restore default/search list and delete the temporary keychain on exit
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
- A transition family has only an affected-state proof: the legacy adapter
  accepts the quorum-authenticated snapshot but never describes it as exact
  execution evidence.
- A transition's original response contains a consensus error: the legacy
  caller throws before invoking the adapter. An invalid proved query still
  rejects, and the adapter does not convert that failure into success.
- Status selects a nonexistent or stale epoch: the explicit proved query
  returns authenticated absence or state for that exact selector rather than
  letting unsigned metadata redefine the proof query.
- A document property is not schema-declared as binary: the sanitizer leaves
  it untouched and normal validation remains responsible for rejecting it.
- A binary property reaches consensus serialization: it is already canonical
  binary Value variant; the strict shared serializer is not relaxed.
- A batch references a custom contract absent from bootstrap context: the wait
  path proof-fetches and caches it before result verification.
- A referenced contract is cached at an older version: preparation refreshes
  and overwrites it with the latest proved version available before waiting.
- Multiple operations reference one contract: context preparation performs one
  lookup for that ID.
- Contract prefetch fails or returns authenticated absence: the proof wait
  fails loudly before attempting verification with incomplete context.
- A non-batch transition is awaited: no contract prefetch is performed and its
  existing wait behavior is unchanged.
- A contract changes after preparation but before the result proof: the two
  independent proofs remain authentic and verification retains the existing
  trusted-schema semantics; exact same-height binding is not claimed.
- The Swift runner has a stale or unwritable keychain from another job: the
  unique path avoids it, and the smoke check fails before the expensive build
  if the new keychain is not writable.
- The Swift user domain has no resolvable default: the eligible conventional
  login or first eligible search-list keychain becomes the repaired cleanup
  fallback. Dead, out-of-home, symlinked, unowned, or unwritable entries are
  not restored. An empty raw list, no eligible fallback, or an unrelated
  default-query error fails before mutating keychain preferences.
- Swift baseline repair fails while setting only the candidate list: the trap
  restores the nonempty raw list and does not create a temporary keychain.
  From before the default-setting attempt onward, cleanup retains candidate
  containment so a possibly mutated default stays resolvable. The candidate
  becomes the cleanup baseline only after list/default readback.
- Swift tests or setup fail: the exit trap restores the selected baseline
  default keychain before deleting this job's keychain. Cleanup errors do not
  hide a primary failure and fail an otherwise successful run.
- The Swift job is killed with SIGKILL: no shell trap can restore preferences
  or delete the temporary keychain. The next job's eligibility checks and
  unique path avoid blindly reusing it, but runner maintenance remains
  responsible for orphan cleanup.
- Two runner agents share one macOS user domain: unique files do not serialize
  default-keychain mutation. CI requires one runner registration per host/user;
  violating that invariant is an infrastructure configuration error.

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
7. Run the persisted-header regression against the old tip-as-start call and
   record the exact 42-versus-40 failure. Derive the first stored height and
   rerun the complete `createAccount` unit file.
8. Retain the browser shard 1 failure from the strict execution wait, switch
   the two legacy verifier copies to the explicit affected-state API, and run
   the focused browser shard plus verifier lint.
9. Replay the CI WASM bundle's implicit-current failure with SDK tracing.
   Change the functional epoch tests to use the status epoch as an explicit
   selector and run all six epoch/proposed-block queries read-only against the
   existing local network.
10. Add the Rust SDK binary-property boundary regression and observe the
    integer array before the fix. Invoke the existing client-only sanitizer,
    verify `Value::Bytes32` after the fix, then run the Rust SDK unit target
    and the WASM document functional suite in CI.
11. Add the pure WASM contract-ID regression with the extraction boundary
    returning no IDs and record the missing custom contracts. Traverse and
    deduplicate batch transition contract IDs, then prove distinct IDs,
    duplicate suppression, and non-batch no-op behavior. With the mock SDK and
    test trusted context, prove fetch-and-cache, stale-version replacement,
    authenticated absence, and error propagation. Rerun the focused tests,
    then the WASM SDK Rust tests, formatting, and clippy. Confirm all four
    proof-verifying entry points prepare context before their wait call.
12. On `mac-runner-1`, retain the fixed-path `SecItemAdd(-61)` failure as the
    red observation. Provision and smoke-test a unique temporary keychain,
    force an early nonzero exit to verify default restoration and removal, then
    run `swift test --filter IdentityResolverSignIntegrationTests` and the
    complete Swift workflow through its simulator `xcodebuild` phase.
13. Add the mocked exit-44 keychain regression and run it against the current
    script. Record that it exits before provisioning and does not install a
    cleanup trap. Reorder capture, apply the strict per-user fallback
    eligibility checks, transactionally repair the baseline, and rerun the
    same test to prove the later forced failure is preserved while live
    default/search-list state and temporary files are restored. Run the fake
    state machine's failure injection at each repair and temporary preference
    boundary. Pin coherent non-first-default, exit-zero missing-default,
    no-login first-eligible fallback, unrelated query-error, empty-raw-list,
    and no-eligible-fallback cases. Include a default mutation that returns
    failure and prove candidate containment remains active. Run the regression
    explicitly with `/bin/bash -u`, then invoke it with `/bin/bash` from the
    reusable Swift workflow before the real keychain setup.

If generated WASM artifacts or Yarn unplugged dependencies block those suites,
install/build the repository-prescribed prerequisites; do not bypass the test
or silently report it as passing.

### Static workflow checks

- Parse the edited YAML.
- Verify `.github/workflows/swift-sdk-build.yml` is itself in the
  `swift-sdk-changed` path filter and that its keychain regression uses
  `/bin/bash`.
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
