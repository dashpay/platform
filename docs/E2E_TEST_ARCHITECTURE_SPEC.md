# One end-to-end test system for Dash Platform: spec

**Status:** revision 3. Revision 2 folded in a five-lens review (feasibility, coverage against the old suite, CI cost, maintainability across SDKs, simplicity). Revision 3 re-assesses draft #3549 as the base of the protocol-e2e framework, module by module, and makes its wallet-free core the base (§2.3, §3.4); §15 lists both rounds. Re-checked against `origin/v5.1-dev` at `03945d3aa7` (2026-10-29): the six newer commits change none of the cited facts. Spec only: no code, PRs or issues come from this document.
**Base:** `origin/v5.1-dev` at `b9abe42e03`. File:line references are at that commit. CI timings come from read-only `gh run view` on the nightly runs of 2026-10-26 (v5.0-dev) and 2026-10-27 (v5.1-dev).
**Supersedes:** the e2e parts of `docs/LEGACY_JS_RETIREMENT_SPEC.md` rev 2.1: all of §5, PRs 4-8, D4, the e2e rows of §10-§13 there, and §11.1's "one JS e2e harness" and grpc-js decisions. D2 stays, attached to the optional PR 4. The rest of that spec stays valid as one track of this plan (§10).
**Working artifact:** never commit this file (`AGENTS.md`, "Specs are working artifacts").

## 1. Problem

Platform has many suites that talk to a network, each with its own harness, network selection, funding and data:

- the legacy JS `platform-test-suite`;
- the wasm-sdk functional suite (Node and Chrome);
- rs-sdk's recorded-vector tests;
- the Swift integration tests;
- the Kotlin nightly;
- dashmate's e2e specs;
- the unmerged platform-wallet e2e framework (draft #3549).

The legacy suite is the only one that covers Platform behaviour broadly over a network, and it does that through the legacy JS SDK, which is being retired.

The retirement spec proposed moving those cases into the JS SDK's e2e suite. Ivan's objection: the old suite tested both Platform and the SDK. Moving its cases into an SDK suite makes that SDK's e2e carry Platform's functional coverage.

**The answer here is no, that is not the right home.** Platform behaviour is the same for every client. Every SDK we ship is a binding over rs-sdk:
- wasm-sdk compiles rs-sdk to wasm;
- Swift goes through rs-sdk-ffi and rs-platform-wallet-ffi;
- Kotlin goes through rs-unified-sdk-jni.

So protocol behaviour is tested once, in Rust, against a real network. Each SDK then needs only proof that its own binding and transport work, offline from recorded responses and live from a thin smoke.

**Goal:** one structure, with
- a few levels with a clear purpose and owner, and a rule for where a new test goes;
- tiers (smoke, full) and domains (identities, documents, tokens, …);
- one way to select the network;
- one funding model;
- one fixture pipeline;
- a mapping of every old-suite case.

The legacy suite is deleted at the end.

## 2. Inventory

### 2.1 What exists today

| Suite | What it proves | Runs against | How it runs in CI | Size |
|---|---|---|---|---|
| **drive-abci unit and in-process tests** | Consensus rules per transition, queries and provers | In-process `Platform<MockCoreRPCLike>` on a temp GroveDB (`rs-drive-abci/src/test/helpers/setup.rs:24-117`) | Every Rust PR, nextest (`tests-rs-workspace.yml:445-486`) | About 4,800 test attributes; 2,531 under `state_transitions/`. Also 76 signed-answer and proof-compatibility tests in `tests/` |
| **Strategy tests** (`rs-drive-abci/tests/strategy_tests`) | Multi-block behaviour: epochs, quorum rotation, upgrades, withdrawals over time, voting, tokens, addresses | `FullAbciApplication` through `mimic_execute_block` (`src/mimic/mod.rs:147-419`), mocked Core, BLS-signed test locks | PRs run only `comprehensive_mixed_operations` and 5 collision tests (`tests-rs-workspace.yml:482`). The full binary runs on push and nightly (`:503-522`). 4 upgrade tests are in `tests-rs-nightly-long-running.yml` | 125 tests. `shielded_tests.rs` (6) is not compiled (`test_cases/mod.rs:14-15`) |
| **System-contract JS tests** | Contract JSON schemas, through wasm-dpp | In-process | `js-packages`, only when the contract package changes | 228 cases, mostly v1 schemas (retirement spec §3.4) |
| **rs-sdk `tests/fetch`** | Request building plus proof verification of real responses, read-only | **Offline:** 108 recorded directories (`tests/vectors`, 16 MB). **Live:** `network-testing` against the node in `tests/.env`, which needs Core RPC (`fetch/config.rs:101-110`) and SDK_TEST_DATA genesis data | Offline only, in the workspace nextest run (`--all-features`, where offline wins, `fetch/config.rs:201-252`). No job records or runs live | 130 test fns (+35 parametrised rows). **No writes.** 11 are ignored offline for missing vectors; 15 are ignored live because they need manual setup |
| **rs-sdk-ffi integration tests** | The C FFI over the same vectors | `dash_sdk_create_handle_with_mock` (`rs-sdk-ffi/src/sdk.rs:798`) | Workspace nextest | 34 integration tests. Test name → directory is a hand table (`tests/integration_tests/ffi_utils.rs:42-53`) |
| **Swift unit tests** (`SwiftDashSDKTests`) | Swift wrappers. 2 vector directories are replayed through the FFI mock (`SDKMethodTests.swift:107-138`); one of them, `test_identity_read_v1`, is an orphan recorded on testnet that no rs-sdk test regenerates | Offline | `swift-sdk-build.yml` → `run_tests.sh`, self-hosted macOS, about 5 min, on swift/rs-sdk/ffi paths (`tests.yml:333-350,517-538`) | About 861 |
| **Swift integration tests** (`SwiftDashSDKIntegrationTests`) | platform-wallet through FFI on a live network: SPV sync and restart, Core send, Core → Platform funding, identity registration | A local dashmate network (`Support/LocalDevnet.swift`, `IntegrationTestEnv.swift:33-41`); 1 testnet read | **Not in CI.** `run_integration_tests.sh` is manual (`RUN_INTEGRATION_TESTS=1`) | 15 |
| **SwiftExampleApp tests, UAT** | App logic. Manual or agent-driven UAT on testnet (`simulator-control` skill, `TEST_PLAN.md`) | Unit; UAT on testnet | Unit tests in `run_tests.sh`. The UI smoke workflow is manual and was last run 2026-05-26 (failed) | 228 + 5 UI |
| **Kotlin SDK tests** | JVM unit, instrumented; 3 testnet reads (`TestnetQueriesTest.kt:24-70`) | Emulator; testnet for reads | `kotlin-sdk-build.yml` on PRs (testnet tests skip). `kotlin-sdk-nightly.yml` daily with `-Ptestnet=true`, 12-19 min, no secrets | 538 + 58 + app 242 + 6 |
| **wasm-sdk functional** | wasm-sdk API end to end: 15 query specs (91 tests) and 6 transition specs (39) | **Hardcoded local:** `WasmSdkBuilder.local()` (`wasm-sdk/src/sdk.rs:41-43,452-466`), quorum sidecar `:22444`. Signs with the SDK_TEST_DATA genesis identities | `test-functional` (`tests.yml:673-687`) → `tests-packges-functional.yml`, 6-7 min, Node then karma. It runs **after** the wallet-lib and `dash` steps, so a legacy failure skips it (`:37-44`) | 130. 3 skipped: `identityCreate`/`identityTopUp` (no asset lock, `transitions/identity.spec.ts:201,211`) and a proof error |
| **js-evo-sdk** | Unit only (sinon stubs) | — | `js-packages` | 278. Its functional specs were deleted in #2868; `karma.functional.conf.cjs:9` points at the empty folder |
| **Legacy `platform-test-suite`** | Platform **and** the legacy SDK | Local; devnet/testnet via `configure_test_suite_network.sh`, which no workflow calls | `test-suite` (`tests.yml:636-671`): Node 17 min, Chrome batch 1 19-22 min, batch 2 7-8 min | 95 network tests + 5 unit (§8) |
| **wallet-lib, `dash` functional** | The legacy packages | Local | Inside `tests-packges-functional.yml` | 12 + 4 |
| **dashmate e2e** | Node lifecycle: setup, start, restart, stop, reset | Local (3 nodes); testnet fullnode; testnet "evonode", which is really a fullnode with `isHP` (`testnetEvonode.spec.js:61-64`) | `dashmate-e2e-tests` (`tests.yml:603-634`): 5, 3 and 3.5 min | 3 specs |
| **bench-suite** | Nothing; its setup is broken | — | None | Deleted by retirement PR 1 |
| **Draft #3549** (platform-wallet e2e) | Wallet and protocol flows, live | Testnet or devnets | **Never ran in CI** | §2.3 |

### 2.2 Cross-cutting findings

1. **No network test runs on Rust-only PRs.** `build-images` and every e2e job require one of `version-changed`, `e2e-tests-changed`, the schedule or a manual dispatch (`tests.yml:441,634,649,686`); `build-js` also runs on any `js-packages` change (`:429`). The `e2e-tests-changed` path list (`tests.yml:123-154`) has no Rust crates. An rs-dpp or rs-drive-abci change merges without any network test; for example, the 2026-10-28 run for "fix(dpp): state the low-S rule…" skipped the image builds.
2. **Every suite picks its network differently:**
   - rs-sdk: `DASH_SDK_*`;
   - wasm-sdk: hardcoded;
   - legacy: `DAPI_SEED`/`NETWORK`/`EVO_SDK_DEVNET_NAME`;
   - #3549: `PLATFORM_WALLET_E2E_*`;
   - Swift: `RUN_INTEGRATION_TESTS` + `dashmate config get`;
   - Kotlin: `-Ptestnet`.
3. **Every suite funds differently:**
   - legacy: a wallet-lib faucet over DAPI streams;
   - wasm-sdk: genesis identities, no Core;
   - Swift: Core RPC `sendtoaddress` plus mining (`IntegrationTestEnv.swift:394-432`);
   - #3549: a hand-funded mnemonic;
   - Kotlin and UAT: a testnet faucet by hand.
4. **The vectors are stale, partly hand-edited, and hard to refresh.**
   - **Old protocols:** they were recorded at protocol versions 4 to 14, none at 15 (`rs-platform-version/src/version/mod.rs:37`). 87 of 163 successful responses are from versions 4 to 12.
   - **Hand edits:** 75 of 188 `msg_*.json` filenames don't match the hash of their request, and #4701 edited 70 files in place.
   - **Hand-pinned values break on a re-record:**
     - the recorded protocol version (`fetch/document.rs:15-19`, PV12);
     - the proTxHash, pinned twice (`fetch/config.rs:279-284`, `rs-sdk-ffi/tests/integration_tests/config.rs:133-134`);
     - the FFI directory table.
   - **Never checked live:** CI never records and never runs the read suite against a network.
5. **Proof checking over a real network** is covered only by the legacy suite and wasm-sdk functional.
   - Strategy tests check GroveDB proofs against the app hash. Only two places check the quorum signature (`strategy_tests/query.rs:80-165`, `address_tests.rs:2077-2240`).
   - There is no proof check at all for shielded, moderation, fee-claim and key-limits transitions (`verify_state_transitions.rs:1359-1378`).
6. **The nightly is not trusted.** `NIGHTLY_STATUS.md` (2026-07-05, still about v4.1-dev) lists the legacy suite and the functional tests as failing or flaky.

### 2.3 Draft #3549

References in this section are at #3549's head `0eff5ebc62`, under `packages/rs-platform-wallet/tests/e2e/` unless a path says otherwise. Revision 3 read every framework module, the 342 commits that touch `tests/e2e`, `TEST_SPEC.md` and the PR thread. 338 of those commits are authored by lklimek; the PR was opened by an agent account acting for lklimek.

- **Size and stack:** +66,210 / −1,083 lines, 295 files, draft (`postponed`). 67 commits behind `origin/v5.1-dev` on 2026-10-30.
  - It carries #5150 by merge ("follows its fate", PR body), and with it #5207, #5210 and #5528: about 21k lines.
  - It also carries older copies of three more open PRs (#5564, #5565, #5570) and a change to the default runtime of dash-platform-macros' `#[stack_size]`.
  - The e2e part is `framework/` (27 files, 17.6k lines), `cases/` (102 files, 21.2k) and `TEST_SPEC.md`.
- **What the framework depends on.** It uses **no API from the four stacked PRs**. It needs #3549's own platform-wallet hooks:
  - `PlatformWalletManager::with_handlers`;
  - the `test-utils` accessors `inject_address_balance` and `platform_payment_account_max_derived_index`;
  - the `e2e-seams` shielded builders;
  - `try_sync` from #5565, which is open and stacked on #5564.

  This was checked by names and signatures, not by compiling. The stack matters only at run time. The asset-lock accounting cases (CR-003/005/006, ID-002b, AL-001) assert #5150's fix, and the restore and drain cases need #5210's `WALLET_RESTORE`.
- **Coverage:** 105 tests.
  - About 55 files are protocol-level: platform-address transfers, identities from addresses, top-ups and transfers, 17 token cases, DPNS register, a funded asset lock, and shielded transitions and abuse cases.
  - About 47 are wallet-level: SPV, sync, gap limit, sweep, persistence, the shielded wallet.
  - Every case, protocol-level included, funds and signs through `PlatformWallet`.
  - **Not implemented:** documents, contracts, DashPay, withdrawals, DAPI Core.
- **Running it:**
  - Testnet by default; it has also run on the porter and paloma devnets.
  - `local` maps to regtest (`framework/config.rs:854`), but it has never run against a dashmate network. SPV has no regtest port default (`framework/spv.rs:507-519`), and nothing in it uses Core RPC.
  - 30-50 min per full run. The last full run (testnet, 2026-09-09) was 158 passed and 35 failed of 193; it has not been re-run since the 2026-10-23 merge.
- **It would turn CI red as it stands.**
  - `tests-rs-wallet.yml:241-250` and the workspace job run nextest on platform-wallet with `--all-features` and a 4 MiB stack.
  - That turns on `e2e`, which satisfies the target's `required-features`, so every case whose name lacks "shield" runs and panics at the stack guard.
  - Draft PRs skip CI, so this has never shown up.
- **Funding:**
  - a mnemonic funded by hand: about 6.27 DASH net per full run on paloma, with a 550B-credit floor (`framework/config.rs:153`);
  - per-test seeds registered before use and swept back afterwards;
  - a cost-ordered planner and a Platform→Core refill.

  Core money is visible to it only through SPV.
- **What it found:** 33 Found-* entries, which are bugs it found in the wallet, rs-sdk and rust-dashcore. None of them is in Drive. Two were fixed in the SDK: #3650, and Found-033 by #5562. The "fail-closed persist" in its title was dropped at the 2026-09-30 merge, in favour of upstream #4586's retry of a failed cache write.
- **Fixtures:** none.
- **Comments:** they carry about 300 tracker references, such as QA-*, Found-*, Wave and Task numbers, and case IDs. About 55 of them are in the modules lifted in E2.

**Its framework, module by module.** "Core" means it needs no platform-wallet and no SPV.

| Module | Lines | What it is | Verdict |
|---|---|---|---|
| `sdk` | 265 | SDK build: DAPI list, short ban, protocol-version probe, trusted context provider with a contract cache | Core, once its SPV branch is dropped (`config.rs:320-324` picks SPV for devnet and local when no quorum URL is set) |
| `config` | 1,187 | `PLATFORM_WALLET_E2E_*` keys; testnet, devnet, local→regtest | About 350 lines core; the rest are bank and SPV knobs |
| `wait` | 1,358 | Waits | About 1,000 lines core: proof-verified waits for addresses, nonces, identities, contracts, DPNS names and tokens. About 340 wallet/SPV |
| `harness`, `mod` | 1,165 + 634 | Shared context, stack guard, `setup()` | About 280 lines core: the `OnceCell` context, the stack guard, SDK init. The rest builds the wallet manager, the bank and SPV |
| `shielded` | 1,024 | Shielded binding, prover, raw broadcast | `broadcast_raw`, `wait_commit_raw` and `assert_consensus_rejection` (`:618-760`) are core. The rest is wallet, partly behind `e2e-seams` |
| `tokens` | 1,386 | Token contract publish, mint, reads | About 70% core (dash-sdk only). Identity setup goes through the wallet |
| `signer`, `signer_derivation` | 332 + 201 | Identity and address signers derived from a seed | No wallet types beyond one call (`signer.rs:200`), but needed only by a seed-based bank. Protocol e2e uses `SimpleSigner` with random keys |
| `context_provider` | 174 | Quorum source plus contract cache; an SPV provider | Useful only with SPV |
| `funding_ledger`, `workdir`, `registry` | 762 + 125 + 286 | Per-test cost ledger, `flock` workdir slots, crash-safe wallet registry | No wallet types, but useful only with a persistent bank |
| The other 13 modules | About 11.8k | The bank wallet and its identity, the planner, the Core↔Platform refill, sweep-back, test wallets, gap limit, transaction accounting, SPV, the event hub, the SQLite persister | Wallet/SPV |

**In short,** about 2k lines need nothing from the wallet and are worth lifting now, plus about 1k lines of token helpers later. The other ~14.5k lines are a wallet test harness.

**Problems it already solved.** Tags: **L** = also hits protocol e2e on the local network; **R** = on a devnet or testnet.

1. **A process-shared SDK needs one shared runtime** (L).
   - rs-dapi-client pools lazily connected tonic channels (`rs-dapi-client/src/transport/tonic_channel.rs:79`, `connection_pool.rs:119-140`). Their workers live on the runtime that created them.
   - Under plain `#[tokio::test]`, those workers die with the first test. The later tests get dead channels, the node gets banned, and nothing recovers.
   - #3549 uses `#[tokio_shared_rt::test(shared)]` and a `OnceCell` context (`framework/harness.rs:86`).
2. **The test stack** (L). Debug-build futures plus proof verification overflow the default 2 MiB stack. A startup guard names the command to run (`framework/harness.rs:88-117`).
3. **Concurrent funding races the nonce** (L).
   - #3549's bank spends from a platform address. It holds a mutex up to broadcast and retries nonce rejections (`framework/bank.rs:62,491-600,961`).
   - An identity bank needs less (§5.2).
4. **Reading your own write across nodes** (R; L only with more than one DAPI address).
   - One proof-verified read proves only that one node has the data. The next transition then lands on a lagging node ("Address does not exist").
   - The fix waits for 2 (or 4) proof-verified reads in a row (`framework/wait.rs:196-237`). The upstream issue, #3611, is open.
5. **Contracts deployed during a run** (L) must be registered with the context provider, or proofs over them don't verify (`framework/sdk.rs:58-73`).
6. **A small node pool and the 60 s ban** (R). A node one block behind gets banned; #3549 uses a 5 s base ban (`framework/sdk.rs:56,236-238`). With the local network's single address, any ban leaves no node at all, so banning is off there (§3.3).
7. **Accepted by `check_tx` is not committed** (L). `broadcast_raw`, `wait_commit_raw` and `assert_consensus_rejection` assert the typed consensus code (`framework/shielded.rs:618-760`). That is smoke item 7 (§6).
8. **Client and network versions** (R).
   - A pinned SDK version decoded proofs in the wrong format: 83 of 88 failures on 2026-07-23.
   - #3549 probes and refreshes the protocol version (`framework/sdk.rs:47,120`, `framework/harness.rs:380`), and prices fees at `sdk.version()`, not `latest()` (`dc0e47844c`).
9. **Fee estimates below the fee charged on chain** (L): #3040, still open. The harness adds headroom, and PA-3040 pins the bug.
10. **Key levels** (L): contract create signs with HIGH, token batches with CRITICAL (`framework/tokens.rs:20-33`).
11. **Timeouts that say why** (L): they report the last observed value and the streak (`framework/wait.rs:103-110`). Fetch errors are logged only at debug level, which is a gap to close.
12. **Wallet- or testnet-only** (sweep-back, the planner, cold SPV scans, devnet InstantSend under concurrency, memory above about 4 threads): these matter only for the wallet domains (E9).

## 3. The target structure

### 3.1 Levels

| Level | Proves | Runs against | Lives in | Owner |
|---|---|---|---|---|
| **In-process** | Consensus rules, fees, state, multi-block behaviour, upgrades, provers and verifiers | No network | drive-abci unit and strategy tests, rs-dpp (gaining the system-contract schema tests, retirement PR 3), rs-drive, and platform-wallet's own tests for wallet logic that needs no network (`test-utils`, `lib.rs:24-25`) | Whoever changes the rule or the wallet logic |
| **Protocol e2e** | A real network accepts or rejects each transition and returns proofs that verify with quorum signatures. Also that the components work together: Core ↔ Platform (asset locks, withdrawals paid on Core), rs-dapi's Core endpoints and streams, Tenderdash timing | A live network: local in CI; a devnet or testnet on demand | **One Rust test crate**, `packages/rs-platform-e2e` (§3.3). Companion: rs-sdk `tests/fetch` in live mode (local only) | Platform team. A protocol change that is visible over the network adds its case here |
| **SDK checks** | Each shipped SDK correctly marshals real responses (**offline replay**) and works end to end over its own transport, signing and error mapping (**live smoke**). **No protocol rules** | Replay: recorded vectors. Smoke: a live network, selected as above | Replay: rs-sdk, rs-sdk-ffi, Swift (wasm later, §7.3). Smoke: wasm-sdk functional (Node + Chrome), Swift integration, Kotlin instrumented | Each SDK's maintainers. The SDK team owns the recording workflow; the author of a protocol bump re-records (§7.2) |
| **Node operations** | dashmate lifecycle | Local; public testnet (Core sync only) | dashmate `test/e2e` | dashmate maintainers |

**Where a new test goes**

| You are adding | Tests to add |
|---|---|
| A rule (validation, fee, state change) | In-process |
| Anything over several blocks | In-process strategy test |
| A new transition | In-process tests for its rules; one protocol-e2e happy path; a binding unit test in each SDK that exposes it |
| A new query or proof shape | A prover/verifier round trip in-process. One rs-sdk `tests/fetch` test, which runs live and records the vector. A replay row in each binding that replays it. This is the book's checklist, `coding-conventions.md:559-569` |
| Anything that needs real Core, Tenderdash, rs-dapi, quorum signatures, network timing, or a flow across components | Protocol e2e |
| A binding change | The binding's unit tests plus a replay row. Add a smoke case only for a new transport, signing or wait path |
| Node lifecycle | Node operations |

**SDK smoke does not re-check protocol rules.** It keeps one rejected transition per SDK, asserting the error *code*, to prove codes reach the caller (`wasm-sdk/src/error.rs:413-416`).
- wasm-sdk functional already has a few protocol-flavoured cases, such as the token-payment rejections at `transitions/documents.spec.ts:458,477,497`. They stay until protocol e2e has equivalents.
- New ones are refused in review, by the book's rule.

### 3.2 Tiers and domains

**Domains** (protocol e2e):

| Domain | Covers |
|---|---|
| `identities` | create from addresses, IS and CL asset locks; top-up; transfer; keys; masternode identities |
| `contracts` | create, update, history |
| `documents` | CRUD, transfer, purchase, index-only, queries |
| `dpns` | |
| `dashpay` | |
| `withdrawals` | identity withdrawals; the payout on Core |
| `dapi_core` | rs-dapi's Core endpoints and its three streams |
| `tokens`, `groups`, `voting` | after parity (§9) |
| `addresses`, `wallet`, `shielded` | after parity (§9) |

**Tiers:**
- **smoke:** about a dozen cases across domains, under 10 minutes of test time. It runs on PRs whose paths can change network behaviour (trigger: E1; protocol-e2e smoke from E2).
- **full:** everything. It runs nightly, on manual dispatch, and before each release.

**Selection:**
- Each domain is a module. The tier is a submodule: `tests/e2e/<domain>/smoke.rs` and `full.rs`, or `smoke/` and `full/` directories with one file per case once a domain grows (#3549's cases are one file each, about 200 lines). The filters below work either way.
- Test names keep the repo's `should_…` convention (`coding-conventions.md:403`). Moving a test between tiers moves it between files; its name doesn't change.
- **Filters:**
  - `cargo test -p platform-e2e --features e2e --test e2e -- ::smoke::` (smoke tier);
  - `-- tokens::` (one domain);
  - `-- tokens::smoke::` (both);
  - no filter (full).
- Plain `cargo test`, not nextest: one process shares one context on one shared runtime (§3.3). nextest's one process per test would redo setup for every test.
- Every run sets `RUST_MIN_STACK=8388608`; the harness refuses to start without it (§2.3, item 2).
- **CI:** the workflow takes `workflow_dispatch` inputs `tier` and `domains`. Schedule means full; a PR means smoke.
  - No label trigger. Adding `labeled` to `pull_request.types` (`tests.yml:17`) would restart and cancel the whole Tests run on every label change, because of `cancel-in-progress` (`:25-27`).

**SDK checks have no tiers.** Each SDK's smoke suite is small by design, and replay always runs in full.

### 3.3 The protocol-e2e crate

```
packages/rs-platform-e2e/          crate `platform-e2e`, publish = false
  Cargo.toml                       [[test]] name = "e2e", required-features = ["e2e"]
  .env                             written by the local-network setup (§4), gitignored
  tests/e2e.rs
  tests/e2e/framework/             sdk, config, wait, harness, broadcast, bank, faucet, asset_lock
  tests/e2e/<domain>/{smoke,full}  .rs files, or directories with one file per case
```

**Dependencies for parity** (§9, E2-E6): `dash-sdk`, `dpp` (with `core_key_wallet` for `TransactionBuilder`), `dashcore-rpc`, `dapi-grpc`, `simple-signer`, `rs-sdk-trusted-context-provider` (with its `dpns-contract`, `dashpay-contract` and `withdrawals-contract` features), `tokio`, `tokio-shared-rt` (new to the workspace), `dotenvy` and `tracing`. Each one is used directly, or `cargo machete` flags it. **No platform-wallet and no dash-spv** until the wallet domains (E9).

**Why a separate crate, not rs-sdk's `tests/`:**
- **CI safety.** `dash-sdk` is in the nextest package list, which runs with `--all-features` (`tests-rs-workspace.yml:445-482`). A live target there would run in every Rust PR's job.
- **Runner.** It runs under plain `cargo test`, not nextest.
- **Ownership.** Protocol coverage is owned by the Platform team, not the SDK.
- **Later dependencies.** It can depend on platform-wallet later without rs-sdk taking a dev-dependency on its own dependent.

**Guards:**
- **No library target.** The framework lives under `tests/`, so there is nothing to add to the nextest list. If the core ever becomes a library crate (§3.4), that crate stays out of the nextest list too.
- **Not in the nextest list:** the crate is not added to `tests-rs-workspace.yml:445-482`, so nextest never builds or runs it.
- **`required-features = ["e2e"]`:** `cargo test --workspace` skips it.
- **Clippy still compiles it.** Workspace clippy runs `--workspace --all-targets --all-features` (`:227-232`), which keeps the code compiling and linted.
- **Workspace wiring:** add the crate to the root `Cargo.toml` members and to `.github/package-filters/rs-packages-no-workflows.yml`.

**The framework: #3549's wallet-free core, plus local funding** (§3.4).

**Lifted from #3549** (about 2k lines, §2.3):
- **`sdk`:** the DAPI list, the protocol version, and the trusted context provider with its contract cache. Contracts a test deploys are registered with it.
- **`config`:** the network part.
- **`wait`:** the proof-verified waits with their streaks, and a generic `wait_for`.
- **`harness`:** the `OnceCell` context on a shared runtime, and the stack guard.
- **`broadcast`:** `broadcast_raw`, `wait_commit_raw` and `assert_consensus_rejection`, for rejected transitions.

**Changed while lifting.** The API stays the one #3549's wallet layer calls, so that layer can rebase onto it cheaply (§3.4). The only changes:
- **Keys:** the keys become `DASH_E2E_*` (§4).
  - Secrets are redacted in every print, as #3549 already does for its mnemonic.
  - Today `fetch/config.rs:106` and `rs-sdk-ffi/tests/integration_tests/config.rs:86` print whole configs, Core password included.
- **Context provider:** the SPV branch is dropped. `QUORUMS` is required for devnet and local networks.
- **System contracts:** they come from the trusted provider's contract features, not hand registration.
- **Protocol version:** on the local network it starts at `latest()`. The probe from version 10 stays for remote networks.
- **Banning:** it is off on the local network, which has one DAPI address.
- **Broadcast helpers:** they lose the `adversarial_enabled()` gate (`shielded.rs:71`). With the gate, smoke item 7 would fail with a configuration error.
- **Timeouts:** `wait_for` reports the last error in its timeout. This is additive; signatures don't change.
- **Comments are rewritten to be self-contained.** QA-*, Found-*, Task and Wave numbers, persona names and case IDs are replaced by the failure they describe. The mapping from old IDs goes in the PR description. The same rule applies in E7 and E9.
- **`.env` lookup:** the one that walks up to a `.claude` directory is dropped.

**Not lifted for parity:**
- `signer`: `SimpleSigner` with random keys, plus the bank key from the environment, is enough.
- `context_provider`: it is SPV-only.
- Everything wallet-bound. That moves with the wallet layer (§3.4).

**New:** `bank` (§5.1, §5.2), `faucet` (§5.3) and `asset_lock` (§5.3). Neither #3549 nor anything else in the repo has them for a local network.

**Later from #3549:** the token helpers (E7) and the wallet layer (E9).

**Rules for the lifted code:**
- Retries and streak waits wrap only setup steps: funding, and waiting until something is visible. The transition under test is never retried, so they can't hide a regression.
- The transition under test is built and signed by the protocol framework, never by platform-wallet, so a wallet bug can't fail a protocol test. On remote networks the E9 bank wallet may hand a case its Core coins; the case still builds and signs the asset lock itself.

At startup the framework prints the network, its protocol version (from `getStatus`) and the bank identity.

### 3.4 Why #3549's core is the base

Three ways to get the framework, compared on what #3549 actually contains (§2.3):

| | **A. Fresh framework** (revision 2) | **B. #3549's core as the base** (chosen) | **C. Land #3549 as is, grow it** |
|---|---|---|---|
| What happens | Write about 1.5k lines and read #3549 for ideas | E2 lifts its ~2k wallet-free lines into `rs-platform-e2e`. #3549 rebases onto them soon after | Merge it inside platform-wallet, then add the parity domains there |
| Reaches legacy deletion | Yes | Yes | No. It has no documents, contracts, DashPay, withdrawals or DAPI Core, and its protocol cases fund and sign through the wallet |
| Local-network funding | New code: genesis bank, Core RPC faucet, asset-lock builder | The same new code | None: a hand-funded testnet mnemonic, with Core money seen only through SPV. It has never run on a local network |
| Problems already solved (§2.3) | Re-implemented from #3549's code and notes, without its run history | Most come with the lifted code. The funding ones are re-applied in the new bank (§5.2); key levels come in E7 | Come with the code |
| Wallet domains later (E9) | Port about 14k lines of wallet harness and 47 cases onto a core with a different API, or keep two frameworks | Already rebased onto the API they were written against | Already there, in a second framework |
| Needs first | Nothing | lklimek's agreement to the copy | Rebasing it off the stack it carries by merge (its framework uses no stack API), or waiting for the stack. Then a nextest exclusion, because `--all-features` would run it |
| CI | e2e crate outside nextest | The same | Red until platform-wallet's two nextest jobs exclude the `e2e` binary |
| Weeks to deletion (§9) | About 5.5-7 | About 5.25-6.75 | Not comparable: protocol e2e still has to be built |
| Ownership | Platform team; #3549 stays separate | Proposed: the core and the protocol cases with the Platform team, lklimek reviewing; the wallet layer with lklimek | lklimek; protocol e2e is separate again |

**B, because:**
- most of #3549's value for protocol e2e is in problems it already hit;
- its wallet layer is the most expensive part to write again;
- neither needs the stack.

**B saves little time before deletion.** About as many lines get adapted as would be written, and the local funding is new either way. What it saves:
- debugging cycles in E2 and E8 (the lifted core already handles testnet);
- most of E7;
- most of E9.

**How the lift works** (a proposal to lklimek; nothing here is settled without them):
- **Before E2:** lklimek agrees to the copy, and to the copy then being the maintained one. lklimek also reviews E2.
- **The copy:**
  - It is taken at a pinned #3549 commit, recorded in the E2 PR.
  - The PR body links #3549 at that commit and lists the lifted files, because `git blame` over its 342 commits is lost.
  - The squash commits of E2 and E7 carry a `Co-authored-by:` trailer for lklimek, added by hand.
- **Soon after E2 merges, #3549 rebases onto it:**
  - its wallet layer moves under `rs-platform-e2e/tests/e2e/framework/wallet/`, and its own copy of the core is deleted;
  - from then on, fixes to the core are PRs to `rs-platform-e2e`;
  - E9 then lands that already-rebased branch as lklimek's PRs, and they supersede #3549.

  This keeps the two copies from drifting for the months between E2 and E9. #3549 had 26 e2e commits in October alone.
- **If lklimek declines the copy,** the choice goes back to Ivan: lift with credit anyway, or option A.

**Where the wallet layer lives** is lklimek's choice, needed by the time E2 merges, because the rebase depends on it:
- **Recommended: the same crate.**
  - A `wallet` feature, never on by default, enables platform-wallet with its seams, platform-wallet-storage and dash-spv. They are optional normal `[dependencies]`, because Cargo has no optional dev-dependencies.
  - One runner, one job, one set of keys.
  - The `--all-features` problem goes away, because the crate is not in the nextest list.
  - The early rebase needs no extra crate.
- **Alternative: it stays in platform-wallet's `tests/e2e`.**
  - The core then becomes a small `publish = false` library crate that both use.
  - That costs a crate, two key sets, and a `not binary_id(platform-wallet::e2e)` filter in both nextest jobs.

## 4. Network selection: one set of keys

**Every new live harness (protocol e2e, wasm-sdk functional, Swift, Kotlin) reads the same `DASH_E2E_*` keys from the process environment.** rs-sdk's own test configs keep `DASH_SDK_*`, written by the same setup.
- **Local:** the local-network setup writes them into a package `.env`. That happens in the cache-miss step of `.github/actions/local-network/action.yaml:44-58`, so `**/.env` caches them (`:64-71`).
- **Remote:** for a devnet or testnet, CI inputs and secrets set the environment directly.
- **No new file format and no new script.** `configure_test_suite.sh`/`configure_dotenv.sh` gain the lines. Writing on a cache miss matters:
  - on a cache hit `logs/setup.log` doesn't exist;
  - `dashmate wallet mint` can't run while the network is up (`DockerCompose.js:91-93`).

| Key (`DASH_E2E_`) | local (written by setup) | devnet / testnet (env, CI secrets) |
|---|---|---|
| `NETWORK`, `DEVNET_NAME` | `local` | `devnet` + name, or `testnet` |
| `DAPI_ADDRESSES` | `http://127.0.0.1:<gateway port>` for native clients, as rs-sdk tests and Swift do (`fetch/config.rs:131-142`, `IntegrationTestEnv.swift:131`). JS uses `https://127.0.0.1:2443` with verification off | From inputs; the SDK default list for testnet |
| `QUORUMS` | sidecar `http://127.0.0.1:22444` | `https://quorums.<name>.networks.dash.org`, or testnet's trusted context |
| `CORE_RPC_URL`, `_USER`, `_PASSWORD` | local_seed, user `dashmate` (no whitelist, `getBaseConfigFactory.js:69-73`) | Absent: Core cases skip loudly |
| `BANK_IDENTITY_ID`, `BANK_TRANSFER_KEY`, `BANK_TRANSFER_KEY_ID` | The genesis identity of the runner's slot (§5.1) | A funded identity (secret) |
| `CORE_FAUCET_WIF`, `CORE_FAUCET_ADDRESS` | A key minted during setup, before start | Absent |

**Adoption:**
- **Protocol e2e:** from day one.
- **rs-sdk and rs-sdk-ffi test configs:** keep `DASH_SDK_*` for now, written by the same setup.
- **wasm-sdk functional (S1):**
  - Node reads `process.env`;
  - karma gets an explicit allowlist through webpack `EnvironmentPlugin`, as the legacy suite does (`platform-test-suite/karma.conf.js:81-83`). The allowlist never includes Core RPC credentials or faucet keys.
- **Swift (S2):** reads the keys if they are set, and otherwise uses `dashmate config get` as today.
- **Kotlin:** when its write smoke comes (after E8), it forwards the keys as instrumentation arguments next to `sdk/build.gradle.kts:45-49`, rewriting `127.0.0.1` to `10.0.2.2`.

**Which targets CI uses**
- **Local:** on PRs and nightly.
- **Testnet, read-only:** the Kotlin nightly, as today.
  - rs-sdk `tests/fetch` can't do testnet reads: it needs Core RPC and genesis data.
  - A testnet read smoke inside protocol e2e is possible only once remote runs exist (E8).
- **Remote write runs:** a manual dispatch with a bank-identity secret. These are Platform-only cases (E8). Core cases need a persistent bank with sweep-back, which arrives with #3549's wallet layer in E9 (§13). That bank only hands a case its Core coins; the case builds the asset lock itself (§3.3).

## 5. Funding and test data: one model

### 5.1 Roles

- **Platform bank:** an identity with a CRITICAL TRANSFER key.
  - **Local:** the SDK_TEST_DATA genesis identities `[1;32]`, `[2;32]`, `[3;32]`. Each has 10^13 credits and deterministic keys; key 3 is TRANSFER/CRITICAL (`create_genesis_state/test/tokens.rs:57,198-203`; `generateTestIdentityKeys`, `wasm-sdk/src/wallet/key_generation.rs:230-258`).
  - The setup writes one identity per concurrent runner ("slot"). In CI each job has its own local network, so jobs never share a bank. A developer running two suites against one local network gives them different slots.
  - **Remote:** a funded identity, given as a secret.
- **Core faucet:** protocol e2e only, local only.
  - A key minted during setup (`configure_test_suite.sh` already mints `FAUCET_1`/`FAUCET_2` before start, `:24-32`). The script writes the e2e faucet straight into the crate's `.env`, avoiding the `mint.log` overwrite that `configure_dotenv.sh:35-36` depends on.
  - Plus Core RPC.
  - `mint` refuses non-local networks (`mint.js:51-53`), so devnets get no faucet.
- **Test wallets:** fresh random keys per test. Local networks are disposable, so nothing is swept back.

### 5.2 One recipe in every SDK

**A funded platform address:** transfer credits from the bank identity to a fresh address.
- rs-sdk: `TransferToAddresses::transfer_credits_to_addresses` (`rs-sdk/src/platform/transition/transfer_to_addresses.rs:36`).
- evo-sdk: `addresses.transferFromIdentity` (`js-evo-sdk/src/addresses/facade.ts:184`).

**A funded identity:** create it from that address.
- rs-sdk: `put_with_address_funding_fetching_nonces` (`put_identity.rs:139`).
- evo-sdk: `addresses.createIdentity` (`:264`).
- Swift: `Address/Addresses.swift`.
- Kotlin: `registerFromAddresses`/`transferCreditsToAddresses`.

**Minimums:** 100k credits per input, 500k per output, 200k to fund an identity (`dpp_state_transition_versions/v4.rs:46-48`).

**SDK smoke uses only this recipe; none of it touches Core.** The genesis *platform addresses* (`test/addresses.rs:11-16`) have no known keys. They are query fixtures, not a funding source.

**Bank concurrency:**
- **No funding mutex.** #3549 needs one because its bank spends from a platform address (§2.3, item 3). An identity bank doesn't:
  - the SDK hands out distinct identity nonces under its own lock (`rs-sdk/src/internal_cache/mod.rs:280-301`);
  - Drive accepts them out of order within 24 (`rs-dpp/src/identity/identity_nonce.rs:17`).

  `transfer_credits_to_addresses` fetches the nonce, broadcasts and waits in one call (`transfer_to_addresses.rs:98,123`), so a mutex around it would serialise every funding call through its wait.
- **Retries.** Only `InvalidIdentityNonceError` is retried, up to 3 times with backoff, after `sdk.refresh_identity_nonce`. The SDK refreshes it by itself only after a failed broadcast, not after a failed wait.
- **No retry on a timeout,** or the test address could be funded twice.

### 5.3 Core-side funding (protocol e2e, local)

1. **Coins.** The framework spends the faucet's UTXOs (`getaddressutxos`; local_seed has `addressindex`).
   - It keeps a spent-outpoint set within its single process.
   - It sends each funding transaction to the masternodes' mempools as well, as Swift does (`IntegrationTestEnv.swift:405-414`), so the InstantSend quorum signs at once.
2. **The asset-lock transaction** is built in test code with key-wallet's `TransactionBuilder`:
   - inputs at `transaction_builder.rs:239`;
   - payload at `:303`;
   - the burn output at `:537,605-610`;
   - signing through `TransactionSigner::sig_and_pubkey` (`:902-962`) or the blanket `impl<S: Signer>` (`:987`).

   This duplicates part of platform-wallet's production builder (`rs-platform-wallet/src/wallet/asset_lock/build.rs:94-175`). That is accepted for parity. E9 adds one wallet-domain case that goes through the production builder.
3. **The proof.** rs-sdk's own `start_instant_send_lock_stream` + `wait_for_asset_lock_proof_for_transaction` (`rs-sdk/src/core/transaction.rs:223-308`) returns an instant proof, or a chain proof as the fallback. That also exercises rs-dapi's transaction stream.
   - **ChainLock-proof cases** wait for Platform's `core_chain_locked_height` (`:402-420`).
   - **InstantSend works on the local network:** SPORK_2 on (`configureCoreTaskFactory.js:237`), `llmqtestinstantsenddip0024` (`dash.conf.dot:127`), and Swift waits for `instantlock=true` (`IntegrationTestEnv.swift:417-432`).
4. **Withdrawals** check the payout on Core: `getaddressutxos` on the destination. If asset-unlock outputs turn out not to be address-indexed, the check falls back to `getrawtransaction` on the unlock txid (E4).

**Binding coverage of asset-lock transitions:**
- wasm-sdk's skipped `identityCreate`/`identityTopUp` become wasm-sdk **unit** tests on a fixed asset-lock proof vector, comparing the transition bytes with rs-sdk's.
- The retirement spec's PR 4 (an asset-lock builder in rs-sdk/wasm-sdk/evo-sdk for JS tests) is **off the critical path**. It stays as an optional product feature for apps like dash-bridge.

## 6. The protocol-e2e smoke set

The same list is the minimum for each SDK's live smoke (§7.4).

1. Status and protocol version.
2. Fetch the bank identity with proof.
3. Bank → fresh platform address, waiting for the proven result.
4. Identity from that address.
5. Contract publish, document create, query it back.
6. DPNS register and resolve.
7. One rejected transition, asserting its consensus code.
8. *Protocol e2e only:* an identity from an InstantSend asset lock, a top-up from one, and an identity → identity credit transfer.

## 7. Fixtures

### 7.1 What is recorded, and from what

- **Source:** rs-sdk's `tests/fetch` read suite, run live against a fresh local network with SDK_TEST_DATA genesis data (`generated_data.rs:17-62`).
- **Not recorded:** protocol-e2e writes. Their state depends on fresh keys, entropy and timestamps, so every recording would churn every file. Replaying writes would need a fixed clock and seeded randomness through the SDK (§14).
- **Missing data** (contested names, groups) comes first from **extending the SDK_TEST_DATA genesis** (`create_genesis_state/test/`, Regtest/Devnet only, `test/mod.rs:33-40`). A write step is used only for what genesis can't hold, such as masternode votes. Recording never depends on protocol-e2e code.

### 7.2 Recording, versioning, refresh

**Recording**
- A `workflow_dispatch` job, "Record SDK vectors", with a `tests` filter input:
  1. builds the images at the commit;
  2. starts the local network;
  3. runs `packages/rs-sdk/scripts/generate_test_vectors.sh [filter]`;
  4. uploads `packages/rs-sdk/tests/vectors` as an artifact, with a diff summary.

  A developer commits the result.
- Local recording stays allowed (rs-sdk `README.md:183-191`, and contributors from forks), with the guards below as the provenance check.
- **A full regeneration happens only at a protocol bump or a release.** A new query records only its own directory, so a PR doesn't churn all 16 MB.

**`vectors/manifest.json`** is written by every recording:
- protocol version;
- chain id;
- commit;
- the voter proTxHash.

rs-sdk, rs-sdk-ffi and the binding replay tests read pinned values from it, instead of `document.rs:15-19` and the two proTxHash constants.

**Guards (rs-sdk offline tests)**
- **The filename matches the request:** `rs-dapi-client/src/dump.rs:116-128`. The hash includes `type_name::<Vec<u8>>()`, whose output may change between compiler versions; replace it with a fixed tag in the same re-record.
- **No orphans:** every vector directory maps to an rs-sdk test namespace. Swift's testnet-recorded `test_identity_read_v1` gets replaced by a locally recorded directory.
- **A protocol floor:** every response was recorded at a protocol ≥ `PlatformVersion::latest() - 1`. The floor is computed, so nobody raises it by hand. The PR that bumps the latest protocol still passes; the next bump forces a re-record.

**Ownership**
- The author of the protocol-bump PR owns the re-record.
- The stable-release checklist (`.claude/skills/release/SKILL.md`) gets a line.
- **If the network can't produce a vector,** the test is ignored with a reason and its directory deleted. A stale vector is never kept.

**The live run**
- Nightly, rs-sdk `tests/fetch` runs live (`--no-default-features --features network-testing`) on the nightly local network.
- That proves the read suite passes against a real network at the commit.
- Freshness of the committed files comes from the manifest and the floor.

**Format:** unchanged (Rust serde of prost types, `dump.rs:47-64`). Every binding replays through the Rust mock.

### 7.3 Replay in each SDK, through Rust

Every SDK sends its requests through rs-sdk and verifies proofs in Rust. "Replay the same vectors" therefore means exposing rs-sdk's mock through the binding. A native JS or Kotlin replay would have to rebuild rs-sdk's requests byte for byte and re-implement proof verification.

| SDK | Today | Plan |
|---|---|---|
| rs-sdk | `SdkBuilder::new_mock().with_dump_dir()` (`src/sdk.rs:1645-1687`) | — |
| rs-sdk-ffi | `dash_sdk_create_handle_with_mock`, 34 tests | Read pinned values from the manifest |
| Swift | `SDK(mockVectorsDirectory:)` (`SDK.swift:371-385`), 2 directories | **R2:** a replay table, one row per query family |
| wasm-sdk | `mocks` is a default feature (`Cargo.toml:11-22`); the mock is used only in Rust unit tests | **Deferred** (§13). It needs more than byte loaders: an async in-memory expectation loader, because `load_expectations_sync` uses `block_on`, which always fails on wasm32 (`mock/sdk.rs:157`, `rs-dash-async/src/block_on.rs:121-130`). It also needs `DumpData::from_bytes`, an in-memory `MockContextProvider` (`rs-context-provider/src/provider.rs:314-408`), a mock build without `with_dump_dir`, and a test-only export under `./raw`. It would make `mocks` load-bearing, against the TODO at `rs-sdk/Cargo.toml:82`. wasm-sdk functional already exercises the same marshalling live |
| Kotlin | No mock over JNI | Not planned |

**A replay row:**
- names the rs-sdk namespace it replays and the exact request parameters, because the mock matches the serialized request exactly (`rs-dapi-client/src/mock.rs:187-207`);
- calls the binding;
- asserts only fields that a re-record does not change (ids, key data, counts, genesis token balances), or values read from the manifest. Proofs, heights, quorum keys and fee-dependent balances change on every fresh network.

Rows can be tracked per binding in the CI-checked `docs/sdk/sdk-parity-manifest.json` (`scripts/check_sdk_parity_manifest.py`). Then a new request type without a row, or without an explicit opt-out, fails CI.

### 7.4 SDK live smoke

- **JS: wasm-sdk functional** (Node + Chrome). It covers smoke items 1, 2, 5 and 6 today (`status.spec.ts:23`, `identities.spec.ts:37`, `transitions/contracts.spec.ts:39`, `documents.spec.ts:126,238`, `dpns.spec.ts:44,87`). **S1** adds:
  - items 3 and 4 (no JS test covers transfer-then-create yet);
  - an `err.code` assertion on one rejection (item 7). Today's rejections match the message only.
  - It also removes the silent early return in `transitions/dpns.spec.ts:89-93`.
- **Swift: `SwiftDashSDKIntegrationTests`** against a local network. S2 adds smoke items 3-7 (bank → address, identity from it, contract and document, DPNS, an error code). Its existing wallet, SPV and Core-funding cases stay.
  - New wallet-logic cases go to platform-wallet's own tests when they need no network, and to the protocol-e2e `wallet` domain when they do.
  - Swift keeps FFI, persistence and keychain cases.
- **Kotlin:** the nightly's 3 testnet reads stay. A write smoke waits for the remote bank-identity secret (E8).

## 8. Mapping the legacy `platform-test-suite`

Counts are active cases; "smoke" and "full" are protocol-e2e tiers. The "Covered in-process by" column names the test each drop relies on.

| Old file (active) | Goes to | Covered in-process by (drops) |
|---|---|---|
| `Identity.spec.js` (19) | `identities`. **smoke:** create from an IS asset lock, top-up, credit transfer. **full:** asset-lock amount too low; invalid IS lock (10513, no in-process test, since Core is mocked); reused output; no key proof (one negative); ChainLock create and fetch; documents failing without credits and working after a top-up; top-up with an invalid IS lock or reused output; transfer above balance; add and disable a key. **Masternode identities:** walk the masternode list (the rs-dapi stream, or `protx list valid`), then fetch owner, operator and voting identities. That needs no secret, so it runs on every network (`Identity.spec.js:664-731`). Get-by-id and by-key run inside the flows | — |
| `DataContract.spec.js` (7) → 4 | `contracts`. **smoke:** create and fetch. **full:** update; unknown owner (one negative) | Wrong version: `validate_update/v0/mod.rs:137` and drive-abci `data_contract_update/state/v0/mod.rs:830`. Incompatible schema: `document_type/methods/validate_update/common/mod.rs:1792`, `data_contract_update/mod.rs:1369`. Non-contiguous positions (10411): only generations 0/1 are tested (`try_from_schema/v0/mod.rs:800`, `v1/mod.rs:809`), so **add a `PlatformVersion::latest()` test** (L1-tests) before dropping |
| `Document.spec.js` (8) → 7 | `documents`. **smoke:** create and query. **full:** fetch by `$createdAt`; replace; delete; unique-index violation; **unknown type** (no in-process test asserts `InvalidDocumentTypeError` on the batch path; built at `batch/transformer/v0/mod.rs:559`). The retirement spec §5.6 was wrong here | Unknown owner: a legacy-client error, not a Platform rule |
| `IndexOnlyDocument.spec.js` (17) → 13 | `documents`. Keep: the contract; the agreeing like; **nonexistent post** (in-process coverage is on a stored type only, `creation.rs:7246`); covering-index synthesis; chained query with verified proofs; composite feed query; delete by values; second identity's like/delete; skipIfAbsent create, out-of-index and delete. **Subset projection without proofs and fetch-by-id refused** need a raw unproved `getDocuments` request asserting the server's error (`rs-drive/src/query/mod.rs:3927,2708`); rs-sdk always asks for proofs | Hashtag mismatch: `batch/tests/document/index_only.rs:542`. Both skipIfAbsent refusals: `:620`. Duplicate like: `:266` |
| `e2e/contacts.spec.js` (10) | `dashpay` (full). The test publishes **its own** contract with `requiresIdentityEncryptionBoundedKey`/`DecryptionBoundedKey` (`contacts.spec.js:46-48`). It adds contract-bound MEDIUM keys and checks `getIdentitiesContractKeys` (`:167-222`); rs-sdk `tests/fetch` has no test for that. Then profile create and update, contact request, approve, remove approval | — |
| `e2e/dpns.spec.js` (8) → 7 | `dpns`. **smoke:** register and resolve. **full:** TLD refused, homograph refused, subdomain without parent refused, search, resolve by record | Contract existence: rs-sdk `fetch/data_contract.rs:26` |
| `e2e/withdrawals.spec.js` (6) → 5 | `withdrawals` (full): withdraw and see the payout on Core; recent withdrawal updates; a completed withdrawal document can be deleted (`withdrawals.spec.js:115-118`); delete of an incomplete one and update refused. "More than balance" was a js-dash-sdk pre-check (`creditWithdrawal.ts:78`). It is **replaced** by a real consensus negative that broadcasts and asserts the code | Authentication-key purpose: a legacy client-side error. The Platform rule (`validate_state_transition_identity_signed/v0/mod.rs:171-174`) has no test anywhere, so **add an in-process test** (L1-tests) |
| `functional/core/*` (6) | `dapi_core` (full): broadcastTransaction, getTransaction ×2, getBlockchainStatus through rs-dapi-client's transports (`transport/grpc.rs:887-912`); getBlockHash ×2 over JSON-RPC | — |
| `functional/dapi/subscribeToBlockHeadersWithChainLocks…` (2) | `dapi_core` (full), with dapi-grpc's tonic client. Compare the header bytes with `getblockheader <hash> false`, and assert a ChainLock arrives (the old test checks `bestChainLock.height`) | — |
| Epoch, ProtocolVersion ×2, getStatus (4) | rs-sdk `tests/fetch` (live and replay); getStatus is also smoke item 1 | rs-sdk `fetch/epoch.rs`, `protocol_version_*` |
| `proofs.spec.js` (3) | **Dropped:** they check proof shape only; every fetch verifies proofs with quorum signatures | — |
| `e2e/wallet.spec.js` (5) | **wallet-lib behaviour dropped.** The **rs-dapi transaction-stream** coverage it gave (historical bloom-filter sync plus a new transaction) moves to `dapi_core`: subscribe, send to a faucet address, compare with Core RPC. The **masternode-list stream** (used by the masternode-identities case) also gets its own `dapi_core` case, checked against `protx list valid`. platform-wallet syncs over P2P and never calls either stream; `masternode_list_stream.rs` has no unit tests | — |
| `unit/createPlatformProofVerifier` (5) | **Dropped:** legacy adapter | — |
| Chrome runs (81 cases) | **Reduced on purpose** (open question). Kept: 130 wasm-sdk functional tests in Chrome, including writes. **Lost in the browser:** protocol flows, gRPC-web *server streaming* through Envoy (wallet-lib synced over the transaction and header streams in Chrome), and asset-lock identity create/top-up broadcast from a browser | — |

**Rules**
- A case is dropped only when the test in the last column exists and covers the same rule. The L1-tests PR adds the two missing ones.
- Each family keeps at least one negative case in protocol e2e.
- **Totals.** Of the 95 network cases:
  - about 73 are ported (one replaced by a real consensus negative);
  - 5 are covered by rs-sdk's read tests;
  - about 17 are dropped with a reason;
  - 2 new `dapi_core` stream cases are added.
- **Parity** is every row merged. It gates retirement PR 8.

## 9. Phased plan

**Sizes** are rough lines changed, excluding lockfiles and vectors. "→" means "must merge after".

### Phase 0: close the trigger gap

**E1. A consensus PR runs a network smoke** (about 50 lines of YAML).
- **A new filter output, `protocol-e2e-changed`:** rs-dpp, rs-drive, rs-drive-abci, rs-dapi, rs-dapi-client, rs-sdk, rs-platform-version, rs-drive-proof-verifier, rs-context-provider, wasm-sdk, wasm-dpp2, js-evo-sdk, the setup scripts, and the e2e crate once E2 creates it.
- **Its OR goes only into `build-js`, `build-images` and `test-functional`** (later the protocol-e2e step). The legacy `test-suite` and `dashmate-e2e-tests` keep `e2e-tests-changed`. Without this split, every Rust PR would also run about 47 legacy runner-minutes and 3 dashmate jobs.
- **All three images are rebuilt on such a PR:** Drive, rs-dapi and dashmate-helper. `local-network` pulls all three, and the helper is built from the Dockerfile `build-js` stage, which copies the Rust crates (`Dockerfile:673-749`).
- **rs-sdk-only PRs reuse images.** Neither drive-abci, rs-dapi nor dashmate depends on rs-sdk. For a PR that touches only rs-sdk (7 of the 20 newly triggered PRs in the last 3 weeks), pull the base commit's images, falling back to the nightly SHA. That saves the 32-43 runner-minutes of image builds, and takes the helper image (13.9-19.1 min) off the critical path.
- **wasm-sdk functional stops depending on the legacy steps.** It is already a separate step (`tests-packges-functional.yml:37-44`); give it `if: ${{ !cancelled() }}` so a wallet-lib or `dash` failure can't skip it.
- **Interim:** wasm-sdk functional is the protocol smoke until E2.
- **Cost:** §11; this is an open question.

### Phase 1: protocol e2e to parity, then delete the legacy suite

**E2. The crate, framework, smoke set and CI** (about 3k lines, about 2k of them lifted from #3549).
- **Before it starts:** lklimek agrees to the copy (§3.4, §13 Q5) and reviews the PR.
- §3.3, §3.4 and §6, plus the setup lines that write the crate's `.env` and mint its faucet.
- **CI wiring:**
  - **A separate compile job** builds the test binary (`cargo test --no-run`) in parallel with the image builds and uploads it as an artifact (stripped, `retention-days: 1`). Nothing is shared with the Rust job: that runs on self-hosted `rust-ci` runners with a local `target/` and no S3 sccache, while the e2e jobs are GitHub-hosted. Estimate 6-12 min warm, 15-25 min cold on `ubuntu-24.04`.
  - **Smoke runs inside the functional job, on the same network, after wasm-sdk functional.** That is one network start per run, not two. It has to run after, not alongside: wasm-sdk functional signs with all three genesis identities. Running them in parallel would need more genesis bank identities.
  - **Full** runs in its own nightly job, and on dispatch.
  - The crate must be added to the Dockerfile's three `COPY --parents` lists, because the Cargo workspace must be complete in the image. As a side effect, an edit to e2e tests alone rebuilds the images.
- **Book chapter `book/src/testing/e2e-tests.md`:** levels, the placement table, tiers, the keys, funding and the smoke list. Short now, completed in PR 8.

**E3 (→ E2). `identities`** (about 1.5k): Identity.spec. This is the Core-touching work.

**E4 (→ E2). `withdrawals` + `dapi_core`** (about 1.2k), including the transaction, masternode-list and header streams. Also Core-touching.

**E5 (→ E2). `contracts` + `documents`** (about 2k): DataContract, Document, IndexOnlyDocument, including the raw unproved request.

**E6 (→ E2). `dpns` + `dashpay`** (about 1k).

**L1-tests (independent; small). Two in-process tests:**
- a withdrawal signed with an AUTHENTICATION key;
- non-contiguous positions at `PlatformVersion::latest()`.

**→ Retirement PR 8 (→ E3-E6, L1-tests). Delete `platform-test-suite`:**
- its job and image, `configure_test_suite_network.sh`, and the platform-test-suite part of `configure_test_suite.sh`, keeping the faucet mint and the `DASH_E2E_*` lines;
- `NIGHTLY_STATUS.md`;
- the `AGENTS.md:8,63,67,90` references.

The book chapter is completed in the same PR.

### Phase 2: fixtures (in parallel with Phase 1)

**R1a. Recording and guards** (about 400 lines + vector churn).
- The workflow, `manifest.json`, the filename (fixed tag), orphan and floor guards.
- A re-record of everything that records on a fresh network.
- The nightly live run of `tests/fetch`.
- Secret redaction in the two test configs.
- Fixes to `rs-sdk/README.md:100,134` and the header of `generate_test_vectors.sh`.
- The floor applies only to directories that record on a fresh network until R1b; the rest stay on an explicit list.

**R1b (→ R1a). Genesis data for groups and contested documents, and a vote step.**
- Re-record the rest and un-ignore the 11 offline-ignored tests.
- Remove the list.

**R2 (→ R1a). The Swift replay table** (about 500).

### Phase 3: SDK smoke alignment (after E2)

**S1. wasm-sdk functional.**
- Reads `DASH_E2E_*`, with local as the default.
- Adds smoke items 3, 4 and 7.
- Moves `identityCreate`/`identityTopUp` to unit tests on a fixed proof.
- Deletes evo-sdk's dangling functional karma config.
- Updates `wasm-sdk/README.md:147-150`.

**S2. Swift integration tests.**
- Run nightly on the self-hosted macOS runner against a local network, if that runner can run Docker. Otherwise they stay manual.
- Add the missing smoke items.
- Replace `swift-sdk/TESTING.md` (stale) with a pointer to the book.

### Phase 4: beyond parity

**E7. `tokens`, `groups`, `voting`.**
- **Minimum:** token transfer, mint, burn, freeze, pause, price/purchase, a group action, a contested DPNS name with masternode votes.
- **Network-only behaviour:** a replayed broadcast; nonce skew and `check_tx` refusals (#5562); masternode owner-key withdrawals.
- **From #3549:** its `tokens` helpers and its 17 TK cases move over.
  - Their setup switches from the wallet to the genesis bank. Their bodies already call dash-sdk.
  - They are renamed to `should_…`, and their comments are cleaned (§3.3).
  - `tk_001` moves to `tokio_shared_rt::test(shared)`. Its `#[stack_size(…, worker_threads)]` runs the shared context on a runtime of its own, the hazard of §2.3 item 1, and needs #3549's macros change.

**E8. Remote runs.**
- A manual dispatch with network inputs and a bank-identity secret, for Platform-only cases and a testnet read smoke.
- The lifted core already handles testnet and devnets that publish a quorums host: seed lists, the trusted quorum provider, the version probe, the ban period and the streak waits. E8 adds the dispatch, the secret and `requires_protocol(n)`. Devnets without a quorums host, like porter, need SPV and wait for E9.
- `requires_protocol(n)`: a loud skip on a remote network that runs an older protocol, a failure on local.

**E9. Wallet domains: #3549's wallet layer lands.**

By now #3549 has been rebased onto the core (§3.4), so E9 lands that branch as lklimek's PRs rather than moving code.
- **Domains:**
  - `addresses`;
  - `wallet`: live-network cases only. It folds in `rs-platform-wallet/tests/spv_sync.rs`;
  - `shielded`: nightly, in its own shard.
- **What lands:** #3549's bank wallet, planner, refill, sweep-back, registry, workdir slots, funding ledger, SPV runtime, persister, the wallet waits, and its ~47 wallet cases.
- **Its protocol-level PA, ID and SH cases** move into the protocol domains (`addresses`, `identities`, `shielded`), funded from the genesis bank wherever they don't test the wallet.
- **Ownership** (proposed, §3.4):
  - lklimek owns `framework/wallet/` and the `wallet` domain;
  - the protocol-level cases sit with the Platform team, with lklimek reviewing.
- **Review routing:** the path rules for both go into the PR Hygiene policy that generates `.github/CODEOWNERS` (`dashpay/stale_prs_are_bad`, `policies/platform.json`).
- **Clean-up on the way in:**
  - names become `should_…`;
  - comments follow §3.3;
  - wallet settings move from `PLATFORM_WALLET_E2E_*` to `DASH_E2E_WALLET_*`, with no aliases, since the old names were only ever in a draft.
- **Prerequisites:**
  - **#3549's own platform-wallet hooks** land first, as a small PR: the `test-utils` accessors and the `e2e-seams` shielded builders. `with_handlers` can be replaced by one fan-out handler passed to `new`.
  - **`try_sync`:** either land #5564 → #5565, or switch the harness to `coordinator.sync(force)`, which is on v5.1-dev.
  - **The four stacked PRs are not prerequisites.** The cases that assert their fixes (asset-lock accounting, restore, drain) are `#[ignore = "needs #5150"]` (or `#5210`) until those PRs merge, so the nightly stays green.
- **Funding:**
  - **Local network:** Core RPC sends to the bank wallet's Core address, and a genesis identity tops up its Platform address. No hand funding in CI.
  - **Testnet and devnet:** they keep the hand-funded mnemonic and sweep-back.
- **CI:** a nightly shard.
  - The runner has 16 GB and also hosts the dashmate network. #3549 ran out of memory above 4 threads on 19 GB with no network on the machine, so the shard starts at `--test-threads=2` and is measured.
  - SPV runs against the local network, as Swift does, with an explicit regtest P2P port.
- **Housekeeping:**
  - `check-wallet-closure.py` `EXPECTED_DEPENDENTS` gets `platform-e2e`;
  - `tests-rs-wallet.yml`'s scoped clippy (`:216-227`) gets `--package platform-e2e`, so that a wallet-only PR can't break the wallet layer unseen. It does not go into the nextest list.
- **#3549** is then superseded by these PRs.

### CI at each step

| After | On a PR (paths permitting) | Nightly |
|---|---|---|
| Today | Legacy suite, 3 jobs (17 + 22 + 8 min); functional 7; dashmate 5 + 3 + 3. Only JS/e2e paths or a version bump; **nothing on Rust-only PRs** | The same; Rust long-running; Kotlin testnet reads |
| E1 | + build-js, all three images and functional on `protocol-e2e-changed` (about 135 more runs a week) | Unchanged |
| E2 | + compile job; smoke appended to the functional job | + protocol e2e full |
| Retirement PR 8 | − legacy suite | − legacy suite |
| R1a | Replay unchanged, inside the Rust job | + live `tests/fetch` on the nightly protocol-e2e network (a second binary to compile) |
| E7-E9 | Unchanged (smoke stays small) | Protocol e2e full split into domain shards, each with its own network, once it passes 30 min; the wallet shard (SPV, starting at `--test-threads=2`); `shielded` alone |
| S2 | Unchanged | + Swift integration (self-hosted) |

### Effort (rough, engineer-weeks)

| Work | Weeks |
|---|---|
| E1 | 0.25 |
| E2 | 2-2.5 (lift and adapt about 2k lines; about 1k new for the bank, faucet, asset lock and the smoke set; CI and the book chapter). Option A: 2.25-3 |
| E3-E6 + L1-tests | 3-4 |
| R1a | 1 |
| R1b | 1 |
| R2 | 0.5-1 |
| S1-S2 | 1 |
| E7-E9 | 4.5-6 (local SPV, which #3549 never ran; the hooks PR; memory tuning). #3549's token helpers, token cases and wallet layer move rather than being rewritten |

**To legacy deletion (E1-E6, L1-tests): about 5.25-6.75 weeks** (option A: about 5.5-7), against 4-6 for the retirement spec's JS port. The difference buys two things:
- one protocol suite that no binding owns;
- a Core path with no dashcore-lib V24 problem.

The fixtures, remote runs and wallet domains are separate work, listed separately.

## 10. The legacy JS retirement track inside this plan

**Unchanged from `LEGACY_JS_RETIREMENT_SPEC.md` rev 2.1:**
- U1, PR 0 and PR 0s (dashcore-lib vs Core 24);
- PR 1 (delete JS DAPI and bench-suite);
- PRs 2a/2b/2c (dashmate);
- PR S (stop publishing);
- PR 3 (contract tests to rs-dpp, now part of the in-process level);
- PRs 9 and 10;
- the npm deprecation path;
- the wasm32 wallet findings (its §6).

**Changed:**

| Retirement spec | Now |
|---|---|
| §5.2 option A: port into wasm-sdk functional | Port into protocol e2e (E3-E6). wasm-sdk functional stays the JS smoke |
| PR 4: asset-lock builder in rs-sdk/wasm-sdk/evo-sdk, on the critical path | An optional product feature, with D2 (`waitForProof`). The test builder lives in protocol e2e (§5.3) |
| PR 5: a JS Core faucet, karma middleware, a grpc-js Core client, a Node/browser split | Dropped. Replaced by E2 (a Rust faucet; dapi-grpc's tonic client) |
| PRs 6-7: port into wasm-sdk | E3-E6 |
| PR 8: delete after the JS ports | Delete after E3-E6 and L1-tests |
| §5.6: "unknown type covered by `batch/tests`" | Wrong; the case stays in protocol e2e (§8) |
| D4: remote runs dropped | Platform-only remote runs come back in E8. Core-side remote runs wait for #3549's bank wallet (E9, §13) |

**Deadline:**
- The legacy suite breaks at bootstrap once CI's local network runs dashd:24 (retirement spec §4). Protocol e2e is unaffected: rust-dashcore parses V24.
- Parity takes slightly longer than option A, so U1/PR 0 (or PR 0s) is what keeps the legacy suite alive until E6. That is the retirement spec's first decision (D1).

## 11. CI cost and time

### Measured

Nightly runs 40417401516 and 40599546110; PR runs 40826562377, 40823175547, 40827343941 and 40722904818.

| Job | On a PR | Nightly |
|---|---|---|
| build-js | 9.3-16.2 min | 7-9 |
| Drive image | 10.0-12.8 | 11 cold / 1 cached. PR builds never write the layer cache (`cache_to_name` is set only on push), so "cached" never happens on a PR |
| rs-dapi image | 8.5-10.9 | 7 / 1 |
| dashmate-helper image | 13.9-19.1 (the slowest, and on the critical path) | 1 |
| Functional job | 7.5-9.3 (network start 2.7-5.1, wasm-sdk tests 2.4-2.7) | 6-7 |
| Legacy suite (3 jobs) | 45-51 runner-min | 17 + 19-22 + 7-8 min |
| dashmate e2e | — | 5 / 3 / 3 |
| Rust workspace | 10.7-26.8 (self-hosted `rust-ci`, warm `target/`) | 38-74 (with coverage) |
| Swift unit | about 5 | — |
| Kotlin nightly | — | 12-19 |

### Volume (v5.1-dev, last 3 weeks)

- **Merged PRs:** 54, with forward merges excluded.
  - 32 (59%) match `protocol-e2e-changed`.
  - 16 (30%) match today's `e2e-tests-changed`.
  - **20 (37%) would trigger network jobs for the first time**, about 7 merged PRs a week.
- **Runs** matter more than merges: each push to a PR reruns. In the last 7 days, 364 non-draft Tests runs: about **135 newly triggered runs a week**, and about **215 protocol-smoke runs a week**.

### Cost per run and per week (runner-minutes)

| Item | Per run | Per week |
|---|---|---|
| E1, newly triggered runs: three images 32-43, functional 7.5-9.3, build-js where not already run (it runs today on 17 of the 20 PRs, through the JS filters) | 42-54 | About 5.7-7.3k (upper bound; about 320 if each PR ran once). The rs-sdk-only image reuse removes about a third |
| E2: compile 6-12 + smoke under 10, sharing the functional network | 16-22 | About 3.5-4.7k (×215) |
| PR 8: legacy suite removed | −45-51 | About −5k (about 109 runs) |

- **Net:** roughly **+4-7k runner-minutes a week after PR 8**, and **+9-12k a week during the 5-6 weeks before it**, when the legacy suite and the new jobs overlap.
- **What it actually costs.** The repo is public, so GitHub-hosted `ubuntu-24.04` minutes are free. The real costs are:
  - **Concurrency:** queueing behind other jobs.
  - **PR latency:** the slowest path becomes the helper image plus functional, about 22-28 min under E1 and 28-35 min under E2. That runs in parallel with the 11-27 min Rust job.
  - **AWS:** ECR pushes rise about 2.2× (from about 109 to about 245 runs a week, three images each), plus pull egress. The ECR lifecycle policy is unverified.
- **Not required to merge.** On v5.1-dev only `PR Hygiene` is a required check, so a red smoke is advisory unless it is made required (§13).

### Rejected or deferred ways to cut it

- **Skipping test- or doc-only changes:** rejected.
  - Only 2-3 of the 20 newly triggered PRs are test-only, and one of those (`79cb05685a`) edits an inline `#[cfg(test)]` that no path rule can see.
  - Negation also needs the `predicate-quantifier: every` workaround (`js-packages-no-workflows.yml`).
- **Choosing smoke or functional by crate:** rejected. The images are about 75% of the cost.
- **A merge queue:** one run per merge, about 5× fewer runs. But it needs required checks and a change in how the team works (§13).

### Other costs

- **The recording workflow** can reuse the nightly SHA images instead of building its own.
- **Swift nightly (S2):** the test images are built `linux/amd64` only (`tests-build-image.yml`), and the Swift runner is `[self-hosted, macOS, ARM64]`, shared with Swift PR builds. S2 needs either emulation or nightly arm64 builds of all three images (§12).
- **Savings that come later:**
  - PR 8 also drops `+dash`/evo-sdk from the Dockerfile `build-js` stage (`:747-749`, "the platform test suite's proof verifier"), which shortens the helper build.
  - The wasm-dpp saving in build-js waits for dashmate PRs 2a-2c, because dashmate needs the legacy stack to start the network.

## 12. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| E1/E2 make most Rust PRs pay for images and a network | About +4-7k runner-minutes a week after PR 8; PR feedback at 28-35 min on those PRs; more ECR pushes | A dedicated filter; image reuse for rs-sdk-only PRs; smoke inside the functional job; Ivan decides (§13) |
| The local network is slow to produce ChainLocks (60 s miner, `setup_local_network.sh:5`) | ChainLock-proof cases are slow | Keep them in full; smoke uses InstantSend |
| A mistaken drop | Lost coverage | The "covered by" column must name an existing test; the L1-tests PR adds the two missing ones |
| Asset-unlock outputs aren't address-indexed | The withdrawal payout check fails | Verify in E4; fall back to `getrawtransaction` |
| The legacy suite dies on dashd:24 before E6 | A coverage gap | U1/PR 0 or PR 0s (§10) |
| The floor guard is red on day one | rs-sdk tests red | It lands with the R1a re-record, and an explicit list holds the directories R1b re-records |
| Genesis has only 3 bank slots | Contention when running locally | One suite per local network, or more genesis bank identities (test data only) |
| The self-hosted macOS runner can't run Docker, or the amd64-only test images | S2 blocked or slow | Build arm64 images nightly, keep the Swift integration tests manual, or reach a Linux-hosted network over the tailnet |
| lklimek declines the copy, or #3549 keeps changing after it | Two copies drift for months; E9 becomes a merge of two forks | Agree before E2 (§13). Pin the commit in the E2 PR. Keep the lifted API unchanged (§3.3). #3549 rebases onto the core soon after E2, deletes its own copy, and sends core fixes to `rs-platform-e2e` (§3.4). If lklimek declines, Ivan chooses between lifting with credit and option A |
| The lifted retries and streak waits hide a regression | A protocol bug passes | They wrap only setup steps; the transition under test is never retried (§3.3) |
| Lifted code carries testnet-tuned complexity the local network doesn't need | More code to review in E2 | Lift only the modules in §3.3; signers, the SPV provider, the planner, refill, sweep-back and registry come with the wallet layer. Remote-only code is not refactored before E8 |
| Lifted comments carry tracker IDs | Comments a reader can't follow | Rewrite them as the failure they describe (§3.3); the ID mapping goes in the PR description |

## 13. Open questions (for Ivan)

1. **A network smoke on every consensus PR.** Should PRs that change Rust consensus or SDK crates build the three images and run a network smoke? Today none of them run a network test before merge.
   - **Cost (§11):** about 37% of merged PRs would newly trigger it, about 135 runs a week. That adds roughly 4-7k GitHub-hosted runner-minutes a week after the legacy suite goes (free on a public repo), about 28-35 min of PR feedback on those PRs, and about 2.2× the ECR pushes.
   - **Two follow-ups:** make it a required check (otherwise it is advisory), and consider a merge queue to cut runs about 5×.
   - **Recommended:** yes, through the dedicated filter, with image reuse for rs-sdk-only PRs; advisory at first.
2. **The browser reduction.** The old suite ran about 81 protocol cases in Chrome. The new structure keeps wasm-sdk functional's 130 Chrome tests (including writes), but loses three things: protocol flows in a browser, gRPC-web server streaming through Envoy, and asset-lock identity creation from a browser. Accept? Recommended: yes. The protocol code is the same Rust code, and the browser-specific transport stays covered.
3. **Remote runs.** At deletion time, protocol e2e runs only on the local network. Platform-only remote runs (a devnet or testnet, with a funded identity as a secret) come in E8. Core-side remote runs need a persistent bank with sweep-back, which #3549's wallet layer brings in E9; they are not planned before that. The legacy suite's remote mode is unused by CI today. Accept? Recommended: yes.
4. **Replay in JS and Kotlin.** You asked whether every SDK can replay the same recordings. They can, but only through the Rust layer. Swift and the FFI already do; this plan widens Swift. JS needs more work than it looks (§7.3), and it would duplicate what the live JS suite already checks. Kotlin would need a JNI addition. Recommended: JS later, if a marshalling bug slips past the live suite; Kotlin not planned.
5. **#3549 as the base.** You asked whether its framework, not a fresh one, should be the base. Mostly yes (§2.3, §3.4).
   - **E2:** lift its ~2k wallet-free lines into `rs-platform-e2e`, credited to lklimek.
   - **Soon after E2:** #3549 rebases onto that core.
   - **E7:** its token helpers and 17 token cases move over.
   - **E9:** its wallet layer lands as lklimek's PRs, superseding #3549.
   - **Not landed as is:** it doesn't reach parity, and its protocol cases run through the wallet (§3.4).
   - **What lklimek is asked to agree to,** before E2:
     - the copy, and the copy then being the maintained one;
     - reviewing E2;
     - rebasing #3549 onto the core soon after;
     - owning the wallet layer, while the protocol-level cases sit with the Platform team;
     - the clean-up of names, comments and keys.

     By the time E2 merges, lklimek also chooses where the wallet layer lives. The recommendation is the same crate, behind a `wallet` feature.
   - **If lklimek declines,** you choose between lifting with credit anyway and option A.
   - Recommended: yes.

**Already decided elsewhere:** what keeps the legacy suite alive if Core 24 reaches CI before parity is the retirement spec's D1 (fix dashcore-lib one last time). This plan assumes that answer.

## 14. Alternatives rejected

- **Protocol e2e in the JS SDK suite** (retirement option A). Protocol coverage would belong to one binding. It would also need a JS Core faucet and an asset-lock builder for tests only.
- **A hybrid:** Core cases in Rust, Platform-only cases (about 50) in wasm-sdk functional. It saves about a week. But "where does a protocol test go" gets two answers, split by an accidental criterion, and that is exactly the objection.
- **Protocol e2e in each SDK.** Writes the same cases three or four times.
- **A fresh framework** (revision 2). It would re-implement what #3549 already solved (§2.3, items 1-11) without its run history. It would also leave E9 a choice between porting about 14k lines of wallet harness onto a different API and keeping two frameworks.
- **Landing #3549 whole** (option C, §3.4). It doesn't reach parity: no documents, contracts, DashPay, withdrawals or DAPI Core. Its protocol cases fund and sign through the wallet. It has never run on a local network and can't fund itself there. Smaller points: it carries the stacked PRs, and `--all-features` would run it in the wallet CI.
- **Protocol cases funded through #3549's bank wallet.** Every protocol case would need SPV and the wallet, and a wallet bug would fail protocol tests. On every fresh local network the bank would also have to be funded through Core and SPV.
- **A funding mutex for the identity bank.** The SDK's nonce cache and Drive's out-of-order window already handle concurrent identity nonces. A mutex around the one-call transfer would serialise all funding through its wait (§5.2).
- **Protocol e2e inside rs-sdk's `tests/`.** The `--all-features` nextest run would run the live target in every Rust job, and it needs a different runner.
- **A repo-root profile file and a new configure script.** The same keys in the environment, written by the existing setup, do the job without a second source of truth.
- **A language-neutral vector format.** Every SDK replays through Rust.
- **Recording write flows.** That needs a seeded RNG and a fixed clock through the SDK, and would churn every vector.
- **nextest for protocol e2e.** It runs one process per test.
- **A PR label for the full tier.** It would cancel and restart the whole Tests workflow on every label change.
- **Path exclusions for test- or doc-only edits in consensus crates.** They catch few PRs and miss inline `#[cfg(test)]` edits.
- **A Rust funding CLI for the other SDKs.** No SDK smoke needs Core funding.

## 15. Review log

### Revision 2 → 3: #3549 as the base

**Why.** Ivan pushed back on "write a fresh framework and harvest #3549's cases": its author has probably already solved many real problems, and writing from scratch would repeat that work.

**What was done.** Four read-only passes over #3549 at `0eff5ebc62`:
- the core modules;
- the wallet and funding modules;
- its commit history, `TEST_SPEC.md` and the Found-* entries;
- its stacked PRs, its non-test diff, CI and the PR thread.

**What changed:**
- §2.3 now has the module table and the list of solved problems. Corrected facts:
  - 67 commits behind;
  - the last full run was 158/35 on 2026-09-09, not 148/41;
  - "fail-closed persist" is gone;
  - the framework uses no API from the stack;
  - the `--all-features` hazard is concrete.
- §3.3 lifts #3549's wallet-free core instead of writing a fresh framework; §3.4 compares the three options.
- §3.2 and §3.3 add the shared runtime and the 8 MiB stack, and allow per-case files in a tier.
- E2 needs lklimek's agreement first; #3549 rebases onto the core soon after E2. E7 and E9 move #3549's code instead of rewriting it. E8 notes what the core already handles.
- The effort, CI table, risks, Q3, Q5 and §14 follow from those changes.

**Three-lens review of the revised parts:**
- **Feasibility:**
  - §5.2 no longer copies #3549's funding mutex. An identity bank needs none, retries only nonce errors after a refresh, and never retries a timeout.
  - The lifted `sdk` drops its SPV branch, so `QUORUMS` is required on devnet and local, and E8 covers only devnets with a quorums host.
  - E9 lists `try_sync`.
  - Pinned cases are `#[ignore]`d, not red, in the nightly.
  - Banning is off on the single-address local network.
  - The broadcast helpers lose their adversarial gate.
  - System contracts come from the provider's features.
  - `tk_001` moves to the shared runtime.
  - The wallet shard starts at 2 threads.
  - Remote Core cases build their own asset lock.
  - Effort raised to 2-2.5 weeks (E2) and 4.5-6 (E7-E9), with option A's figures alongside.
  - The shared-runtime claim was checked and holds.
- **Simplicity:**
  - The signers and the SPV provider are not lifted, so the lift stays about 2k lines.
  - No refactor while lifting, so the wallet layer rebases cheaply.
  - §2.3 is trimmed and its duplicates removed.
  - Option C is rejected for its deciding reasons first.
  - Option A is described fairly.
  - The walkthrough states where the read-lag problem applies.
- **Maintainability:**
  - Lifted comments lose their tracker IDs.
  - The drift between two copies is closed by rebasing #3549 soon after E2, not at E9. That is a second reason for keeping the wallet layer in the same crate.
  - Ownership is a proposal to lklimek, named, with a fallback if declined.
  - Credit is given through `Co-authored-by` and a PR-body link.
  - Review routing goes through the PR Hygiene policy.
  - Tiers can be directories.
  - Wallet cases are renamed `should_…`, and the wallet keys move to `DASH_E2E_WALLET_*`.
  - The `wallet` feature is optional and off by default.
  - The wallet fast path's scoped clippy gets the crate.

### Revision 1 → 2

- **Feasibility:**
  - E1 needs its own filter output; the shared one would start the legacy suite and dashmate.
  - The guards are made consistent (no library target; clippy compiles the e2e target).
  - The local values are written in the cache-miss setup step, because `mint` can't run on a live network and `setup.log` doesn't exist on a cache hit.
  - rs-sdk `tests/fetch` can't do testnet reads.
  - wasm replay's real scope (`block_on` on wasm32).
  - The recording/seeding order (now R1a, then R1b).
  - Native clients use http on the local network.
  - InstantSend already works locally.
  - Use rs-sdk's proof wait.
  - The `labeled` side effect.
  - Tier filters that compose (`smoke.rs` modules).
  - Corrected line refs.
  - Devnets can't mint.
  - Workspace and filter wiring; platform-wallet seams are a prerequisite for E9.
- **Coverage:**
  - "Unknown type" has no in-process test, so it is kept.
  - The rs-dapi transaction and masternode-list streams get `dapi_core` cases.
  - "More than balance" never reached Platform; it is replaced with a real negative.
  - The auth-key-purpose and positions rules get in-process tests.
  - Correct "covered by" references for IndexOnly and DataContract.
  - Masternode identities need no secret.
  - The contacts row is described accurately.
  - Raw unproved requests.
  - The smoke list vs wasm-sdk today.
  - The browser losses are named.
  - Missing withdrawal and header-stream assertions.
  - Minimum token/group/voting sets.
- **Maintainability:**
  - The manifest replaces hand-pinned values; the orphan guard; Swift's testnet orphan vector.
  - Replay rows assert only stable fields and name their request.
  - Targeted recording with local recording allowed.
  - A computed protocol floor and named ownership.
  - Rs-sdk live mode is not a protocol-e2e domain.
  - Wallet logic without a network stays in platform-wallet's tests.
  - The same keys in the environment instead of a profile file; the karma allowlist; the Kotlin host rewrite; secret redaction.
  - Seeding through genesis data.
  - `should_` names kept.
  - The protocol-flavoured cases in wasm-sdk functional are named.
  - The doc updates are listed.
- **Simplicity:**
  - The parity crate needs no platform-wallet or SPV.
  - The framework is written fresh.
  - The profile file and script are dropped.
  - Four levels instead of five (replay and smoke are one "SDK checks" level).
  - wasm replay deferred; Kotlin replay dropped.
  - Deferred: the persistent bank, remote Core runs, `requires_protocol` (until E8), the label, and sharding until needed.
  - An honest effort comparison.
  - Porting PRs grouped by Core versus Platform-only.
  - Open questions reduced; the browser and remote-run questions added.
- **CI cost** (the first attempt hit a rate limit and was re-run on revision 2):
  - PR-side timings replace nightly ones; the dashmate-helper image (the slowest) is added.
  - Weekly volume and net runner-minutes; the real costs are concurrency, latency and ECR.
  - No compile-cache sharing with the Rust job; a separate compile job with an artifact.
  - Smoke shares the functional job's network; image reuse for rs-sdk-only PRs.
  - The "not a required check" caveat; the merge-queue option.
  - The Swift arm64 image gap; the Dockerfile `COPY --parents` side effect; the transition-window cost.
