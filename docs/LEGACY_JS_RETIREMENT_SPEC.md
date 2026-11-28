# Retiring the legacy JavaScript stack: spec

**Status:** revision 2.1. It folds in a five-lens review (feasibility, scope/simplicity, CI and release, dashmate, external users) and a consistency check of the result; §14 lists what changed. Spec only: no code, PRs or issues come from this document.
**Base:** `origin/v5.1-dev` at `60cc5f4508`. File:line references are at that commit.
**Working artifact:** never commit this file (`AGENTS.md`, "Specs are working artifacts").

> **Superseded in part (2026-10-29) by `docs/E2E_TEST_ARCHITECTURE_SPEC.md`.** That spec makes this plan one track of a single e2e test system. Superseded here:
> - **All of §5** (options, recommendation, funding in §5.3, the asset-lock builder in §5.4, the grpc-js Core client in §5.5). The old suite's protocol cases move to one Rust protocol-e2e suite, not into wasm-sdk's functional suite, and wasm-sdk functional stays as the JS SDK smoke. §5.6's rule survives, but its claim that "unknown document type" is covered by `batch/tests` is wrong; that case stays in the Rust suite.
> - **PR 4:** off the critical path; an optional product feature. D2 stays attached to it.
> - **PR 5:** dropped. What survives: its CI filter change becomes E1, and its `identityCreate`/`identityTopUp` un-skip becomes wasm-sdk unit tests (S1).
> - **PRs 6-7:** replaced by the Rust domain PRs E3-E6.
> - **PR 8:** deletion only, gated on E3-E6 and the in-process test PR L1-tests. Its ports of the read-only and Core-endpoint specs go to `dapi_core` (E4) and rs-sdk `tests/fetch`.
> - **§11.1:** "one JS e2e harness" and "header-stream test with grpc-js".
> - **§10, §12 and §13:** the e2e rows (port risks, verification for PRs 4-8, and the rejection of the Rust option B and of hand-written asset-lock serialization).
> - **D4:** the new spec recommends Platform-only remote runs after deletion (E8). That still awaits Ivan.
>
> Everything else here is unchanged: U1, PR 0/0s, PR 1, PRs 2a-2c, PR S, PR 3, PRs 9-10, D1, D3, deprecation, and the wasm32 findings (§6).

## 1. Problem

Platform still builds, tests and publishes a JavaScript stack that predates the Rust SDK:

- **`dash`** (`packages/js-dash-sdk`)
- **`@dashevo/wallet-lib`**
- **`@dashevo/dapi-client`** (`packages/js-dapi-client`)
- **`@dashevo/wasm-dpp`**
- **`@dashevo/dash-spv`** (JS)
- **`@dashevo/grpc-common`**
- the JS side of `@dashevo/dapi-grpc`
- the private JS DAPI server (`packages/dapi`)
- `packages/bench-suite`

All of it rests on the external **`@dashevo/dashcore-lib` 0.22**.

Ivan wants it gone because:
- **It is insecure.** The `.yarnrc.yml` audit excludes for `elliptic`, `bn.js` and `crypto-js` exist for it. elliptic has an advisory with no patched release (§8.3).
- **It lacks features:** no tokens, platform addresses, shielded pool, groups or voting.
- **It costs maintenance on every protocol change.**

The replacement exists: `@dashevo/evo-sdk` over `@dashevo/wasm-sdk` (rs-sdk compiled to wasm), with `wasm-dpp2`.

What we need to keep is the end-to-end coverage `packages/platform-test-suite` gives today: a real local network driven from JavaScript, in Node and in Chrome.

**Deadline: Dash Core V24.** dashcore-lib 0.22 cannot parse V24's formats (§4). Legacy client discovery has been broken on testnet since V24 activated on 2026-10-23 (draft PR #5609). The whole JS e2e suite stops working once CI's local network runs Core 24.

## 2. Goals and non-goals

**Goals**
1. Delete the legacy JS packages and their CI, Docker and release plumbing.
2. Keep e2e coverage of protocol behaviour over a real network, of the JS bindings, and of the browser (wasm + gRPC-web) path, with **one** JS e2e harness, not two.
3. Give external npm users a deprecation path, with notice and a final version.
4. Answer whether a wallet with Core sync can run in JS/wasm, and what it would take.

**Non-goals**
- A browser SPV wallet (§6 sizes it as a separate initiative).
- Replacing dashmate's non-monorepo npm dependencies: `@dashevo/bls`, `@dashevo/dashd-rpc` (partly, see PR 2b), and the `@dashevo/docker-compose` fork.
- Retiring `wasm-drive-verify` (current-era code, with no legacy dependency).
- Any consensus change.
- Deprecating `@dashevo/dashcore-lib` itself. It has real non-Platform users (§8.1).

## 3. Inventory

### 3.1 Package by package

| Package | npm | Who uses it in the repo | Verdict |
|---|---|---|---|
| `packages/dapi` (JS DAPI) | private | **Nobody at runtime.** Replaced by rs-dapi (#2716). Its image, Dockerfile stage and release job were removed in `d7c3536939` (#2827). dashmate's compose file has only `rs_dapi` (`packages/dashmate/docker-compose.yml:138-180`). Still tested on most rs-dpp PRs through the `*wasm-dpp` filter alias (`.github/package-filters/js-packages-no-workflows.yml:77-81`). | Delete now (PR 1) |
| `packages/bench-suite` | private | Not in any CI filter. Its setup is likely broken: nothing writes `DRIVE_LOG_PATH` (`scripts/configure_test_suite.sh:76`). | Delete now (PR 1) |
| `dash` 8.0.1 | public | platform-test-suite, bench. Its README already says "no longer recommended" (`packages/js-dash-sdk/README.md:10-12`). | Delete after the port |
| `@dashevo/wallet-lib` 12.0.1 | public | platform-test-suite (funding, asset locks). dashmate's copy is **dead code** (§3.3). Functional tests: `tests-packges-functional.yml:37-38`. | Delete after the port |
| `@dashevo/dapi-client` 5.0.1 | public | platform-test-suite; dashmate's readiness probe (§3.3); wallet-lib; `dash` | Delete after the port and PR 2a |
| `@dashevo/dash-spv` 6.0.1 | public | js-dapi-client (`lib/BlockHeadersProvider/BlockHeadersProvider.js:2`). Its npm dependencies `@dashevo/dark-gravity-wave` and `@dashevo/dash-util` have no other users. | Delete with dapi-client |
| `@dashevo/grpc-common` 5.0.1 | public | js-dapi-client, dapi, `dash` (`GrpcError`, `broadcastStateTransition.ts:13`), wallet-lib's workers, one platform-test-suite spec, **and dapi-grpc's npm package** (`packages/dapi-grpc/package.json`, `workspace:*`) | Delete together with dapi-grpc's npm package (PR 9) |
| `@dashevo/dapi-grpc` (JS side) | public | JS clients used only by js-dapi-client and dapi; `dash` declares it but never imports it. The Rust crate reads only `protos/` (`build.rs:43-76`) plus two examples, `clients/core/v0/rust` and `clients/platform/v0/rust` (`Cargo.toml:90-97`). | Keep the crate, protos and both Rust example directories. Remove the npm package and the multi-language codegen (PR 9) |
| `@dashevo/wasm-dpp` 5.0.1 | public | `dash`, dapi-client, wallet-lib, dapi, bench, test-suite; the ten contract packages (devDependency, tests only). No Rust crate depends on it. | Delete after PR 3 and the port |
| `@dashevo/dashcore-lib` 0.22 | public, **external repo** | Everything above, plus dashmate (§3.3) | Remove every in-repo use. Do not deprecate it (§8) |
| Ten `*-contract` packages | public | npm: `dash` (contract ids at runtime), test-suite, dashmate (withdrawals id), bench. Rust: `data-contracts`, dpp, drive and the SDKs. | Keep the crates. Move the JS tests to Rust (PR 3). Stop publishing to npm in the same PR as `dash` (PR S), because the published `dash` depends on them at runtime |

These stay: js-evo-sdk, wasm-sdk, wasm-dpp2, wasm-drive-verify, dashmate, all Rust crates, `@dashevo/bls`, `@dashevo/dashd-rpc`, `@dashevo/docker-compose`.

### 3.2 platform-test-suite

Abbreviation: `PTS` = `packages/platform-test-suite`.

**Size.** About 95 active network cases in Node, plus 5 unit cases. About 81 also run in Chrome through karma, in two batches (`PTS/karma.conf.js:21,30-52`; `.github/workflows/tests.yml:636-671`).

**Every write goes through the legacy stack.** evo-sdk is used in two places only:
- as the proof verifier injected into `Dash.Client` (`PTS/lib/test/createPlatformProofVerifier.js:148-275`);
- for two queries (`PTS/test/functional/platform/IndexOnlyDocument.spec.js:321-411`).

**Funding today**
1. `dashmate wallet mint` uses Core RPC to mine to two fresh keys, `FAUCET_1_*` and `FAUCET_2_*` (`scripts/configure_test_suite.sh:24-32`).
2. A wallet-lib wallet holds one of those keys and syncs over DAPI streams (`createFaucetClient.js:43-57`). dashcore-lib parses every transaction in that sync (`packages/wallet-lib/src/plugins/Workers/TransactionsSyncWorker/utils.js:54-56`).
3. `fundWallet` pays each test client's HD wallet (`packages/wallet-lib/src/utils/fundWallet.js:32-46`).
4. js-dash-sdk builds the asset locks with dashcore-lib (`packages/js-dash-sdk/src/SDK/Client/Platform/createAssetLockTransaction.ts`).
5. wallet-lib never accepts InstantSend locks (`TransactionsSyncWorker.js:70`), so every registration waits for a ChainLock.

**Cases by feature area**

| Area | File (active cases) | Port? |
|---|---|---|
| Identity | `Identity.spec.js` (19): asset-lock consensus errors 10530/10513/10504/20009, top-up, double spend, credit transfer, key add/disable, masternode identities | Yes. Needs the asset-lock builder (PR 4) and Core RPC (`protx info` for masternode keys) |
| Data contracts | `DataContract.spec.js` (7) | Yes, minus cases already covered in drive-abci (§5.6) |
| Documents | `Document.spec.js` (8), `IndexOnlyDocument.spec.js` (17) | Yes, minus cases already covered in drive-abci (§5.6) |
| DashPay | `contacts.spec.js` (10) | Yes |
| DPNS | `dpns.spec.js` (8) | Yes |
| Withdrawals | `e2e/withdrawals.spec.js` (6; already skipped in the browser, `:21-25`) | Yes; Core side via Core RPC |
| Read-only queries | Epoch, ProtocolVersion, getStatus (4) | Yes |
| DAPI Core endpoints | broadcastTransaction, getTransaction, getBlockHash, getBlockchainStatus, `subscribeToBlockHeadersWithChainLocks` (8) | Yes, with a Node gRPC client on `dapi-grpc/protos` (§5.5) |
| Proof shape | `proofs.spec.js` (3): checks the proof *shape* only | Drop |
| Wallet sync | `e2e/wallet.spec.js` (5): tests wallet-lib itself | Drop |

**Also dropped**, because they test the legacy stack rather than Platform:
- the proof-verifier adapter unit test;
- client-side-only checks: `creditWithdrawal.ts:78`, the wasm-dpp signing-error string at `withdrawals.spec.js:188-202`, and wallet-lib's error at `Document.spec.js:126-155`;
- the skipped `waitForStateTransitionResult`, `getBlock` and `getMasternodeStatus` specs;
- dead helpers and unused dependencies.

**The new SDK already has an e2e suite in CI.** `@dashevo/wasm-sdk test:functional` runs against the CI local network in Node and in karma (`.github/workflows/tests-packges-functional.yml:43-44`; `packages/wasm-sdk/package.json:35`): 15 query specs and 6 transition specs. They cover:
- credit transfer, key add/disable, Platform-side withdrawal;
- contract publish and update;
- document create, replace, delete and transfer;
- DPNS register and resolve;
- epoch, protocol and status queries.

They sign with the identities `SDK_TEST_DATA=true` seeds at genesis. Those identities have deterministic keys (`generateTestIdentityKeys`, `packages/wasm-sdk/src/wallet/key_generation.rs:251-280`, which match the genesis keys in `.../create_genesis_state/test/tokens.rs:179-209`) and 10^13 credits each. The suite skips identity create and top-up for lack of a Core funding path (`tests/functional/transitions/identity.spec.ts:201-211`).

### 3.3 dashmate

Paths are under `packages/dashmate`. Dashmate does not use `dash`, wasm-dpp, evo-sdk or wasm-sdk. The inventory below was re-derived independently and is complete: there are no hits in `test/`, `scripts/`, `bin/` or templates beyond these.

| Dependency | Live use | Replacement |
|---|---|---|
| wallet-lib | **None.** `src/util/generateHDPrivateKeys.js` is registered in DI (`src/createDIContainer.js:93,212`). With CLASSIC injection it is a parameter of `setupLocalPresetTaskFactory.js:28`, but it is never called. | Delete the file, the registration and the parameter |
| dapi-client + withdrawals-contract | `--wait-for-readiness`: `getDataContract(<withdrawals id>, {retries:0, prove:false})` against the gateway with a hard-coded `no-ssl` (`waitForNodeToBeReadyTaskFactory.js:29-52`). It swallows every error, retries every 500 ms with no timeout, and runs in CI through `yarn start` → `group start -w`. **Bug, found by reading the code (two reviewers agree; not executed):** the gateway offers a plaintext listener only for the `self-signed` provider (`templates/platform/gateway/envoy.yaml.dot:268-303`), while the default is `zerossl` (`configs/defaults/getBaseConfigFactory.js:270`) and `file` is also TLS-only. So `start -w` loops forever on every default evonode. `start.js:49-53` also runs the probe when `platform.enable=false`, where no gateway exists. | Keep the same query and meaning (gateway → rs-dapi → Drive): a unary `Platform/getDataContract` for the constant withdrawals id with `prove:false`, as native gRPC over `node:http2`. **Always use TLS**; the self-signed listener accepts TLS too. Use no SNI, as `src/ssl/probeServedCertificate.js:97-107` does. On verification, see "TLS in the probe" below. Add a timeout (per network, or a flag) that reports the last error. Skip the probe when Platform is disabled. Do not use rs-dapi JSON-RPC `getStatus`: it returns success with defaults when Drive errors, and caches the result for 10 s (`packages/rs-dapi/.../get_status.rs:62,84,118`) |
| dashcore-lib `PrivateKey` → address/WIF | Spork key (`configureCoreTaskFactory.js:49-76`), miner address (`startGroupNodesTaskFactory.js:116`), throwaway mining addresses (`generateBlocks.js:19`, `waitForBalanceToConfirm.js:22`) | A small helper: `randomBytes(32)` plus `ECDH.setPrivateKey`, compressed public key, WIF with the 0x01 suffix, hash160, base58check. Version bytes: local/devnet/testnet `0x8c`/`0x13`/`0xef`; mainnet `0x4c`/`0x10`/`0xcc`. Generate test vectors with dashcore-lib before removing it |
| dashcore-lib `Address` | Prompt validation in regular setup (`src/listr/prompts/validators/validateAddress.js:11`); no tests today | base58check plus version check; must accept P2SH as dashcore-lib does |
| dashcore-lib `Transaction` | Collateral and reward funding in local setup (`src/core/wallet/sendToAddress.js:28-58`) | See below |

**TLS in the probe.** Skipping verification here is defensible:
- the probe dials the node's own gateway on a local address;
- it sends no credentials or secrets;
- it acts on nothing it receives except "the call succeeded".

The certificate names the node's external address, so a verifying client fails against 127.0.0.1. The stricter option, which PR 2a should take unless it proves awkward: verify the chain (public CAs for zerossl and Let's Encrypt; dashmate's generated CA for self-signed and `file`), with `checkServerIdentity` against the configured external IP instead of the dialled address. Either way, a verification failure must be logged, not silently retried.

**Collateral funding without dashcore-lib.** dashmate's RPC client cannot make these calls. `@dashevo/dashd-rpc` 19.1.0 types `createRawTransaction` as `'str str int'`, so arrays and objects are sent as `"[object Object]"`, and it has no `signrawtransactionwith*` at all.

Use a small JSON-RPC helper over `jayson`, which is already a dependency (`src/tenderdash/createTenderdashRpcClient.js`), with the `dashmate` user:
1. Take inputs from `getaddressutxos`.
2. `createrawtransaction` with outputs `{collateral: amount, fundingAddress: change}`, a size-based fee of at least 1000 duff/kB, and no change output if the change would be dust.
3. `signrawtransactionwithkey(hex, [fundingWif])`.
4. `sendrawtransaction`.

**Not `sendtoaddress`:** its change goes to a fresh wallet address. That breaks the next node's balance check (`registerMasternodeTaskFactory.js:95`), and its coin selection can spend the 0.1 DASH reward balance or a not-yet-registered collateral, all of which sit in wallet `main`.

**Packaging and images**
- The dashmate-helper image copies the legacy packages at `Dockerfile:851-867` (line 850 is dashmate itself), and the stage is built `FROM build-js` (`:821`), which compiles wasm-dpp and wasm-sdk. Once dashmate has no legacy dependencies, the helper needs neither. The Node 26/cbor comment at `:833-835` goes stale with it.
- `scripts/pack_dashmate.sh:54-74` is generic and needs no change.
- dashmate's filter entry aliases `*wallet-lib`, `*dapi-client` and contract anchors (`js-packages-no-workflows.yml:99-107`).
- `packages/dashmate/Cargo.toml.template` is unreferenced and lists wasm-dpp.

**Keep** the `3.0.0` config migration, the "Remove legacy DAPI stack" start task (`startNodeTaskFactory.js:115-133`) and the `platform-dapi-deprecated` profile. Upgrades from installs older than 3.0 still need them.

### 3.4 System-contract JS tests

There are 228 mocha cases across the ten packages. Each creates the contract from `schema/v1/*.json` with wasm-dpp and asserts the single `JsonSchemaError`'s `keyword`, `instancePath` and `params`. Underneath, they call the Rust `DataContract::validate_document` (`packages/rs-dpp/src/data_contract/methods/validate_document/mod.rs:17-38`).

**Problems**
- **They test outdated schemas.** The latest protocol uses dpns v3, dashpay v2, withdrawals v2 and token-history v2 (`packages/rs-platform-version/src/version/system_data_contract_versions/v4.rs:12-21`). About 127 cases test v1 schemas of those four contracts.
- **CI does not run them on rs-dpp changes.** Each filter is `packages/<contract>/**` only. That is why 14 were red on v5.1-dev until `c9c98b56de` (2026-10-28).

**Rust already has the pieces**
- App Connect tests (`packages/rs-dpp/src/system_data_contracts.rs:292-379`) cover everything in its JS file.
- Moderation charter tests (`:381-1230`) cover most of its JS file.
- `JsonSchemaError` has `keyword()`, `instance_path()` and `params()` (`packages/rs-dpp/src/errors/consensus/basic/json_schema_error/error.rs:106-122`).

wasm-dpp2 cannot host these tests: there is no document schema validation binding (`packages/wasm-dpp2/src/data_contract/model.rs:920-927`), and its `ConsensusError` exposes only `message` and `code`.

**Decision.** Port to Rust in rs-dpp: `system_data_contracts.rs` modules or `packages/rs-dpp/tests/`. Target "every rule in the latest schema version has a Rust test", not a 1:1 copy of the v1 cases. Rust tests do not go in the contract crates themselves: dpp → data-contracts → contract crate means a second crate instance in tests. Also add the `data-contracts` crate, whose id test is at `packages/data-contracts/src/lib.rs:331`, to the nextest list (`.github/workflows/tests-rs-workspace.yml:471-479`).

### 3.5 CI, Docker and release

**CI jobs**
- `tests.yml`: the `js-packages` matrix (`:563-582`), `build-js` (`:425-431`, a full `yarn build` that compiles wasm-dpp), `test-suite` (`:636-671`), `test-functional` (`:673-687`), `dashmate-e2e-tests`.
- `tests-build-js.yml:22-31` runs dapi-grpc's `setup-codegen.py --install`.
- Only "PR Hygiene" is a required check on v5.1-dev and v5.0-dev, so removing jobs cannot block merges.

**Filters.** `js-packages*.yml` alias almost everything to `*wasm-dpp`. Today:
- an rs-dpp change runs the dapi, dapi-client, wallet-lib, `dash`, dashmate and test-suite jobs;
- but **not** the wasm-sdk or evo-sdk jobs.

Deleting a key whose anchor is aliased elsewhere breaks paths-filter: the `changes` job fails and Tests is skipped. Every removal PR must write the aliases out as literal paths. The manual-dispatch override (`tests.yml:415-416`) runs `test` for every key.

**Docker**
- `build-js` cooks `wasm-dpp` and `wasm-sdk` (`Dockerfile:665`).
- `packages/wasm-dpp` appears in four `COPY --parents` lists (`:418,547,705,979`).
- `build-js` (`:643-752`) copies the legacy packages (`:726-730`) and builds `+dash` (`:747`) and `+@dashevo/evo-sdk` (`:749`).
- `build-dashmate-helper` (`:821`) and `build-test-suite` (`:877`) are both `FROM build-js`, and they are its only consumers. After PR 2c and PR 8, nothing uses `build-js`, so PR 9 deletes the stage outright. Three `COPY --parents` lists outside it (`:418,547,979`) still name `packages/wasm-dpp`.
- The `test-suite` image (`release.yml:262-272`) is never built in PR CI. Its only external user found is dashpay/dash-network-ci, which pins `1.0.0-dev.5` and has had no commit since 2024-02-21.

**Release**
- `.github/actions/npm-release-build/action.yaml:126-132` packs every non-private workspace, and `release.yml:173-212` publishes each tarball. A removed or private workspace drops out automatically.
- Only stable releases get the `latest` dist-tag (`release.yml:153-161`).
- `yarn.config.cjs:7-11` offsets (dash-spv +1, `dash` +3, wallet-lib +7) are all legacy.
- Measured: build-npm took 28m56s for v5.0.1, about 10 minutes of it wasm-dpp. The dashmate-helper image takes about 19-22 minutes per architecture because of `build-js`. e2e wall-clock is set by the Drive image build, so dropping wasm-dpp saves runner minutes, not e2e time.

### 3.6 Code that exists only for the legacy stack

- **rs-dpp `extended-document`.** Only wasm-dpp enables it (`packages/wasm-dpp/Cargo.toml:38`); `ExtendedDocument` is in 18 rs-dpp files and no other crate. A follow-up.
- **dapi-grpc multi-language codegen:**
  - `scripts/build.sh`, `codegen.json`, `setup-codegen.py`;
  - `test-client-codegen.yml`;
  - `npm-runner-validation.yml:66-69` (not `:62-65`, which is the release build itself);
  - `npm-release-build/action.yaml:16`;
  - `tests-build-js.yml:22-31`;
  - `client_codegen` in `runner-requirements.json:205` / `.arm64.json:118`. These are checked for equality by `test_runner_image.py:356`, so changing them needs a runner-image promotion (`NPM_RUNNER.md:33-37`), done in a later PR.

### 3.7 Docs and other references

These are updated in the PR that removes the thing they reference (§7):
- `README.md:176`;
- `packages/README.md:5-7,19-23,29-30`;
- `AGENTS.md:7-8,22-24,28-33,67,90`;
- `CONTRIBUTING.md:72`;
- root `package.json` `test:*` scripts (`:11-28`). Keep `test:suite` while the test-suite job calls it (`tests.yml:655` resolves it from the root);
- the book: `api-reference.md:39-40`, `architecture/overview.md:282`, `introduction.md:163`, `state-transitions/lifecycle.md:221`, `getting-started.md:98`, and the `wasm/binding-patterns.md` and `wasm/error-macros.md` chapters, whose examples are wasm-dpp code;
- `docs/typedoc.json` (its only entry point is js-dash-sdk), `docs/tsconfig.typedoc.json`, `docs/SDK_ARCHITECTURE.md:27,216-229`;
- `book.yml:13-16,89,139-164`;
- `scripts/configure_dotenv.sh:14,16,38-40,53-55` (runs under `set -e` and `touch`es files inside the legacy packages; line 15 is rs-sdk and stays);
- `scripts/configure_test_suite.sh:68-77`;
- `scripts/prepare_docs.sh`;
- `.codecov.yml:27`, `pr.yml:49,51`, `.pre-commit-config.yaml:82,84`, `mkdocs.yml`, `docs/README.md`;
- `.claude/skills/release/SKILL.md`;
- `packages/swift-sdk/run_integration_tests.sh:27`, whose "is it built" sentinel is wasm-dpp's `dist`;
- `.github/scripts/tests/test_dashmate_pack.py:42-54` (uses wallet-lib only as a fixture name).

Outside this repo: `dashpay/stale_prs_are_bad` lists `packages/js-dash-sdk/`.

## 4. The near-term blocker: dashcore-lib against Core V24

### What breaks, in the order you hit it

1. **CbTx v4 coinbase.** `coinbasepayload.js:63-80` handles versions up to 3.
   - Masternode-list discovery and every wallet holding coinbase-funded coins fail to parse.
   - This is already live on testnet.
   - Fixed in dashcore-lib master by #318 (merged, unreleased).
2. **ExtAddr (ProTx v3) masternode entries.** The SML merkle root no longer matches.
   - dashcore-lib #319 fixes it, plus a 0.25.0 regression that caps LLMQ_60_75 at 24 quorums.
   - #319 is open: +8,451 lines, mostly a fixture.
3. **Stale testnet whitelist** in dapi-client's `networkConfigs.js`. #5609 regenerates it.
4. **v2 asset unlocks** (Platform PV15 with V24; sibling `ASSET_UNLOCK_V2_SPEC.md` §0 hands the JS side to this plan).
   - `assetunlockpayload.js:96-99` accepts only payload version 1.
   - Even with that relaxed, the txid is wrong. A v2 txid is `sha256d` with the trailing 132 payload bytes zeroed (rust-dashcore `qualified_asset_unlock.rs:65-74`, `transaction/mod.rs:247-279`), but dashcore-lib hashes the full serialization (`transaction.js:171-173`).
   - No upstream change exists yet.
5. **Unknown special transaction types (10-12) and ProRegTx v3** throw (dashcore-lib #317). The suite's masternode-identity case parses a ProRegTx (`PTS/test/functional/platform/Identity.spec.js:690`).

### Effects

- **wallet-lib:** a parse error is uncaught (`TransactionsSyncWorker/utils.js:54-56`, before address filtering).
- **The whole JS e2e suite:** fails at bootstrap on any V24 regtest because of item 1, before item 4 matters.
- **Not affected:** dashmate and rs-dapi.
- **Why CI is green today:** dashmate defaults to `dashpay/dashd:23` (`packages/dashmate/configs/defaults/getBaseConfigFactory.js:57`), and v2 emission is not implemented yet.
- **The deadline:** dashmate's switch to dashd:24, or v2 emission landing while CI runs V24, whichever comes first.
- **Beyond Platform:** Insight (`insight-api`, `dashcore-node`, both `^0.22.0`) parses every block with dashcore-lib. It will break on v2 unlocks once PV15 emits them on a V24 network. Insight is the UTXO source for dash-bridge and xchainjs. This is a PV15 release risk owned by whoever ships Insight; flag it in the asset-unlock-v2 PR.

### Plan

This is aligned with draft #5609.

- **Upstream (dashcore-lib maintainers; Ivan is one).** On master:
  - land #319;
  - add the v2 asset-unlock rule (version bound **and** stable txid, with the DIP-0027 vectors at `qualified_asset_unlock.rs:303-308`);
  - add a raw-payload fallback, so unknown special types or versions stop throwing.

  Release from master under `latest` (0.25.1 or 0.26.0). The release workflow at `v0.22.0` cannot run today (retired runner image, `actions/cache@v2`, a classic `NPM_TOKEN`), so the release needs either the CI update on master or a manual 2FA publish.

  Also backport the v2 rule and the fallback to a **0.22.x** release for `^0.22.0` consumers. Platform does not need it (it moves to the master release), but Insight (`insight-api`, `dashcore-node`) does: its `^0.22.0` range never picks up 0.25+. It must use a non-`latest` dist-tag (`0.22-lts`), because the library's workflow publishes non-"dev" tags as `latest`. Afterwards, check that `dist-tags.latest` is still the master release. This backport is recommended as part of U1. If the maintainers decline it, Insight must move to the master release itself before PV15 activates with V24.
- **Platform (#5609).** Bump every workspace package from `~0.22.0` to that release. 0.22 → 0.25 only adds an optional `llmqParamsOverride` argument. Keep the whitelist refresh.
  - Backport to v5.0-dev; that is what gives the final legacy version (§8.2) working discovery on V24.
  - A 0.22.1 alone would not have fixed external users anyway: `dash@8.0.1` pins `@dashevo/dapi-client` exactly, so the whitelist fix needs a republish regardless.
- **CI stopgap (PR 0s), only if dashd:24 reaches dashmate's defaults before U1 ships.** It is its own PR with no dependency; PR 0 reverts it.
  - Add a temporary `yarn patch` of 0.22.0 with #318 and the v2 rule.
  - Set `DAPI_ADDRESSES` in `configure_test_suite.sh`. The platform-test-suite client factories already accept it (`PTS/lib/test/createClientWithoutWallet.js:9-17`), and it skips masternode-list discovery, so #319 is not needed.
  - The wallet-lib and `dash` functional tests get their env from `configure_dotenv.sh` (`DAPI_SEED`), so they still run discovery. Give them explicit addresses there too, or skip them for the window.
  - Skip the masternode-identity case until it is ported (it moves to `protx info`).
- **Effort.**
  - v2 rule and fallback: about 40 lines plus vectors, applied twice (master and the 0.22 backport).
  - #319: review and merge.
  - The releases: a CI fix or manual publishes.
  - #5609: lockfile and cache bump.

**Porting withdrawals first does not save the suite**, because of item 1. The ported withdrawal test never parses Core transactions in JS (§5.3), so item 4 stops mattering for CI once the port lands.

## 5. Options for the e2e suite

### 5.1 The three options

| | (A) Port to the new JS SDK | (B) Rust e2e (rs-sdk + platform-wallet) | (C) Hybrid |
|---|---|---|---|
| **What** | Port the about 75 portable cases into the **existing** wasm-sdk functional suite (one JS harness, §5.2); Node and Chrome | Port the cases to Rust against the local network. Base: the platform-wallet e2e framework on `feat/rs-platform-wallet-e2e` (draft PR #3549: +66,130 lines, 298 files, unmerged, testnet "bank wallet" oriented, not in CI) | A for SDK, browser and protocol flows; B for wallet, SPV and Core flows |
| **Effort (rough, engineer-weeks)** | 4-6. Asset-lock builder (PR 4) 1-1.5; harness additions 1; porting 2-3 | 5-8, plus landing #3549. Local-network funding and CI 1-2; porting 3-4; SPV sync time in CI | A plus 2-4 |
| **Protocol over a real network** | Yes | Yes | Yes |
| **JS bindings** | Yes (wasm-sdk API; evo-sdk's thin facade stays covered by its unit tests) | **No** | Yes |
| **Browser (wasm, gRPC-web)** | Yes. A wasm-only transport bug was fixed on 2026-10-27 (#5611), so this matters | **No** | Yes |
| **Core wallet / SPV** | No: Core work goes through Core RPC | Yes | Yes |
| **Main risks** | API gaps found mid-port; suite runtime grows (needs batching); SDK_TEST_DATA dependency | A very large unmerged PR; loses browser coverage; SPV in CI is slow and flaky | Two harnesses |

### 5.2 Recommendation

**Option A, ported into the existing wasm-sdk functional suite (one JS e2e harness).** At the end, `platform-test-suite` is deleted together with its CI job and image.

Why one home:
- The wasm-sdk suite already has what a rewritten platform-test-suite would build: the local-network readiness hook, the seeded-identity signers, the Node and karma runners, and a CI job.
- Rewriting platform-test-suite on evo-sdk would leave two JS e2e suites on the same SDK.

**What we lose:** platform-test-suite can run against devnets and testnet (`scripts/configure_test_suite_network.sh`); the wasm-sdk suite is local-only. Decision D4 in §11.

**B keeps going on its own track** as the wallet-focused suite. It is not a prerequisite for anything here; once it runs in CI, the repo is at C.

### 5.3 How the ported tests get funds

**1. Platform bank, with no Core (most tests; Node and browser).**
- Each runner gets one SDK_TEST_DATA identity (`0x01`, `0x02`, `0x03`). Its key 3 is a CRITICAL TRANSFER key, which is what transfer-to-addresses requires (`identity_signed.rs:15-21`).
- At start, the runner funds one platform address it controls once (`addresses.transferFromIdentity`, `packages/js-evo-sdk/src/addresses/facade.ts:184`).
- Tests then create identities from that address as needed (`addresses.createIdentity`, `:264`).
- One shared SDK client per runner plus a mutex avoids nonce races. wasm-sdk's own tests already need `refreshIdentityNonce`.
- Minimum amounts: 100k credits per input, 500k per output.
- The genesis-seeded *platform addresses* (`.../create_genesis_state/test/addresses.rs:11-16`) have no known keys. They are not a funding source.
- No JS test exercises this transfer-then-create path yet. PR 5 adds one first.

**2. Core faucet (Node; local network only).** For asset-lock, withdrawal and Core-endpoint tests.
- **Runners that share a network need separate faucet keys.** With a shared key, concurrent asset locks double-spend: `getaddressutxos` lists confirmed UTXOs only, and the local network mines every 60 s (`scripts/setup_local_network.sh:5`).
  - Once PR 5 splits Node and browser into separate jobs, each job has its own network and its own `FAUCET_1`/`FAUCET_2` from `dashmate wallet mint`.
  - Within one runner, keep a spent-outpoint set, or use `getaddressmempool`.
  - Caveat until PR 9: `configure_dotenv.sh:35-36` takes the wallet-lib and `dash` functional tests' faucet from `logs/mint.log`, which every mint overwrites. Any new mint must write its own log, and PR 5 names which `FAUCET_n` each consumer uses.
- Core RPC calls:
  - `getaddressutxos`; local_seed has `addressindex` (`setupLocalPresetTaskFactory.js:170`);
  - `sendrawtransaction`;
  - `getislocks` for InstantSend lock bytes;
  - `getrawtransaction` verbose for ChainLock status.
- Credentials: `configure_test_suite.sh` exports local_seed's `core.rpc.port` and the `dashmate` user's password. That is the only user whose whitelist includes `getislocks`; the values are already readable as in `configure_dotenv.sh:65-68`. These go into the suite's env, which the harness PR adds.
- Chain asset-lock proofs also need Platform's `core_chain_locked_height` to reach the transaction's block, as rs-sdk waits for (`packages/rs-sdk/src/core/transaction.rs:402-420`).
- This also covers **instant** asset-lock proofs, which today's suite never does.

**3. Browser specs that need Core** call the same faucet through a karma middleware: a small Node handler in the karma process, reached over HTTP. That keeps today's browser coverage of identity create and top-up: the asset lock is built in wasm and the identity is created over gRPC-web.

**4. Withdrawals.** `identities.creditWithdrawal` to a faucet-generated Core address. Assert the document reaches COMPLETED, then check Core with `getaddressutxos` on that address. Verify in PR 6 that asset-unlock outputs are address-indexed. If not, fall back to `getrawtransaction` on the unlock txid.

**5. Masternode identities.** Read ProRegTx keys with Core RPC `protx info`, instead of parsing the ProRegTx in JS.

### 5.4 PR 4: an asset-lock builder (the only new SDK feature the port needs)

**What and where**
- `buildAssetLockTransaction(inputs: {outpoint, txOut, privateKey}[], creditOutputs, changeAddress, feeRate)` returns the signed transaction bytes.
- The logic lives in a Rust crate that compiles to wasm (rs-sdk), on key-wallet's `TransactionBuilder`:
  - `add_inputs(Utxo)`, `transaction_builder.rs:239`;
  - `set_special_payload`, `:303`;
  - the burn output is handled at `:342-394`;
  - signing through a small adapter that maps raw keys, since `TransactionSigner` is keyed by derivation path (`:904-960`). Precedent: `packages/rs-platform-wallet/src/masternode/update_service.rs:610-622`.
- wasm-sdk and evo-sdk only marshal.

**Why not reuse what exists**
- key-wallet's `asset_lock_builder` needs a `&Wallet` with synced accounts (`asset_lock_builder.rs:367-560`), so it does not fit caller UTXOs.
- An upstream key-wallet helper would add a rust-dashcore pin bump in front of PR 4. Prefer rs-sdk.

**Value beyond tests**
- It un-skips wasm-sdk's `identityCreate`/`identityTopUp` tests.
- Apps that hand-roll this today get a supported path. dash-bridge serializes asset locks itself in `src/transaction/builder.ts`.

**Not in PR 4: `waitForProof`.** rs-sdk's IS-lock wait uses `tokio::time::{sleep, timeout}` (`packages/rs-sdk/src/core/transaction.rs:23`), which is not wasm-safe. It hard-codes output index 0, and it has no callers. Making it a JS feature means:
- a wasm-safe rework: the transport `sleep` (`rs-dapi-client/src/transport.rs:34-44`) and a select-based timeout;
- an output-index parameter;
- a subscribe-before-broadcast API (DAPI streams don't replay locks);
- recovery by txid into a chain proof;
- an AbortSignal.

That is **PR 4b, optional**, decided as a product question (D2). The tests don't need it: the faucet gets locks from Core RPC.

**Not added to evo-sdk:** `core.broadcastTransaction`, `getTransaction`, `getBlockchainStatus`. The tests use Core RPC and a Node gRPC client (§5.5).

### 5.5 DAPI Core endpoint tests

- These test rs-dapi, not the SDK. Port them with `@grpc/grpc-js` and `@grpc/proto-loader` on `packages/dapi-grpc/protos/core/v0/core.proto`, Node-only:
  - broadcastTransaction, getTransaction, getBlockchainStatus, getBestBlockHeight;
  - **`subscribeToBlockHeadersWithChainLocks`**: grpc-js handles server streaming. Compare the streamed header bytes with Core RPC `getblockheader <hash> false`, so no X11 hashing is needed in JS.
- `getBlockHash` is JSON-RPC, so a plain `fetch`.
- This keeps the only live tests of rs-dapi's Core endpoints without adding evo-sdk API. rs-dapi-client has no `BlockHeadersWithChainLocks` transport (`packages/rs-dapi-client/src/transport/grpc.rs:918-941` covers only TransactionsWithProofs and MasternodeList).

### 5.6 Deliberate drops

Pure Drive rules already have in-process drive-abci tests:
- `InvalidDataContractVersionError` in `data_contract_update`;
- unique-index and unknown-type errors in `batch/tests`;
- reference rules in `owner_reference.rs` and `list_element_reference.rs`.

**Rule:** keep at least one negative case per transition family, to prove error codes reach JS (`error.code`, `packages/wasm-sdk/src/error.rs:413-416`). Drop other duplicates only when a drive-abci test covers the same rule, and record that test in the PR's case table.

Keep the asset-lock negative cases. For example, `InvalidInstantAssetLockProofSignatureError` (10513) has no drive-abci test, because in-process tests mock Core.

## 6. Can we have an SPV wallet in JS?

### 6.1 What compiles for `wasm32-unknown-unknown`

Prototyped in an isolated crate in `/tmp`, with the Platform lockfile, the `getrandom` setup from `.cargo/config.toml`, and LLVM clang. The feasibility reviewer re-ran it.

| Crate (rust-dashcore `aca0202`) | wasm32 | Why |
|---|---|---|
| `key-wallet` (bls, eddsa, serde) | **Compiles** | It is already linked into wasm-sdk through dpp (`cargo tree -p wasm-sdk --target wasm32-unknown-unknown -i key-wallet`) |
| `key-wallet-manager` (bls, eddsa) | **Compiles** | rust-dashcore#590 split it out of key-wallet to keep tokio out of key-wallet for WASM. It uses only tokio `sync`/`rt`/`macros` |
| `dash-spv` | **Fails** | tokio `full` → `net` → mio ("unsupported by mio"); tracing-appender's `symlink`; its P2P layer uses TCP |
| `platform-wallet` | **Fails** | Only because it depends on dash-spv unconditionally. 12 of 180 source files reference `dash_spv` (`spv/`, `dash_spv::EventHandler` as the event trait, `BroadcastResult`, `CheckpointManager`). It also uses `tokio::spawn` and `Instant::now` |

### 6.2 What this means

- **Keys, derivation, transaction building and wallet state already work in wasm.**
- **dash-spv cannot run in a browser.** There is no raw TCP, and its sync relies on BIP157/158 filters and QRInfo, which DAPI does not serve.

### 6.3 A DAPI-stream sync backend: feasible, but a separate project

1. **Make dash-spv optional in platform-wallet.** Own event/broadcast/checkpoint types in the non-SPV paths; spawn and clock shims for wasm.
2. **Add a `DapiSyncBackend`:**
   - `subscribeToTransactionsWithProofs` with a bloom filter from key-wallet-manager's monitored scripts and outpoints, restarted when `monitor_revision` changes;
   - `subscribeToBlockHeadersWithChainLocks` for headers and ChainLocks. rs-dapi-client has **no transport for this stream yet** (§5.5), so it is new work.
3. **Upstream, key-wallet-manager:**
   - a filtered-block entry point, since `process_block_for_wallets` takes a full `Block` (`wallet_interface.rs:69`);
   - reorg detection from the header stream, feeding the existing `truncate_above`.

   rust-dashcore#585 (open) helps.
4. **Lock verification.** dash-spv's `InstantLockValidator` and `ChainLockValidator` need a masternode-list engine fed by QRInfo, which DAPI does not serve. In wasm the realistic source is the trusted quorum-key provider wasm-sdk already uses.
5. **Persistence and bindings:** in-memory or IndexedDB callbacks, plus wasm-sdk and evo-sdk bindings.

**Trust model:** the same as wallet-lib today, not real SPV. The bloom filter leaks addresses to the DAPI node, the node can withhold transactions, and locks are as good as the quorum-key source.

**Effort:** about 8-12 engineer-weeks.

**The test suite needs none of this.**

## 7. Sequence of PRs

"→" means "must merge after". Every PR updates the docs and references it makes stale (§3.7), writes CI filter aliases out as literal paths when it deletes a key (§3.5), and commits the lockfiles: `yarn.lock`, `.pnp.cjs`, `.pnp.loader.mjs`, `.yarn/cache`, `Cargo.lock`.

**Phase 0: unblock (now, in parallel)**
- **U1 (dashcore-lib, upstream).** #319; the v2 asset-unlock rule with vectors; the unknown-type fallback; a release from master; the 0.22.x backport for Insight (§4).
- **PR 0s (no dependency; only if dashd:24 lands first).** The CI stopgap (§4).
- **PR 0 (→ U1).** #5609 on v5.1-dev; backport to v5.0-dev. Reverts PR 0s if it landed.

**Phase 1: what nothing needs, and dashmate (independent of each other)**
- **PR 1. Delete `packages/dapi` and `packages/bench-suite`.**
  - Workspace entries; root `test:*` scripts.
  - Filter keys and aliases.
  - `tests-js-package.yml:71`, which copies `packages/dapi/.env.example`.
  - The cache-key hashes of that file (`local-network/action.yaml:37`, `tests-dashmate.yml:94`) and the e2e path filter that lists it (`tests.yml:137`).
  - The `configure_dotenv.sh` dapi lines and the `configure_test_suite.sh:68-77` bench lines, in this PR: the scripts run under `set -e`, and the cache key usually hides the break until some unrelated PR.
  - Badges; `test-suite-triggers.yml` (unused).
- **PR 2a. dashmate: drop wallet-lib, dapi-client and withdrawals-contract** (§3.3). Targets **v5.0-dev** and merges forward: it fixes a real operator bug, and a stable dashmate without these dependencies gates `npm deprecate` (§8.2).
  - Delete `generateHDPrivateKeys` (file, registration, parameter).
  - New probe: TLS always (preferably verifying against the external IP, §3.3), timeout, last error reported, skipped when Platform is disabled.
  - Remove `Dockerfile:851-867`.
  - dashmate's filter entry becomes `packages/dashmate/**`.
  - Delete `Cargo.toml.template`.
  - Unit tests for the probe: TLS options, timeout, the disabled-Platform skip.
- **PR 2b. dashmate: drop dashcore-lib** (§3.3). Off the critical path; it serves the security goal.
  - The key/address helper, with vectors generated by dashcore-lib.
  - The jayson JSON-RPC funding helper.
  - Keep the log strings `configure_test_suite.sh` parses: "ProRegTx transaction ID:", "Owner Private Key:", "Address:", "Private key:".
  - Coverage comes from the local-network action (`setup_local_network.sh`, `wallet mint`). `test/e2e/localNetwork.spec.js:69-72` skips setup on a cache hit, so bust the cache once.
- **PR 2c (→ 2a).** The dashmate-helper image builds from a JS-only stage instead of `build-js`, and the stale Node 26/cbor comment (`Dockerfile:833-835`) goes. About 20 minutes saved per architecture per release.
- **PR S (→ 2a released in a stable dashmate, and the final legacy release, D3). Stop publishing.** On **both** v5.0-dev and v5.1-dev:
  - mark `dash`, wallet-lib, dapi-client, wasm-dpp, dash-spv, grpc-common, dapi-grpc's npm package and the ten contract packages `private`;
  - nothing else changes, so the packages stay in the workspace and the old test suite keeps running until the port is done.

  v5.1 prereleases publish under `X.Y-suffix` tags, so v5.1-dev must stop too. This decouples deprecation (§8.2) from the port.
- **PR 3. System-contract schema tests move to Rust** (§3.4).
  - Remove the JS tests, wasm-dpp devDependencies and the mocha/lint scaffolding from the ten packages.
  - Remove their filter keys, after writing out the aliases that use them (`@dashevo/wasm-dpp`, dashmate).
  - Add `data-contracts` to nextest.

**Phase 2: the port (option A)**
- **PR 4. Asset-lock builder** in rs-sdk, with wasm-sdk and evo-sdk bindings (§5.4).
  - Rust unit tests with fixed vectors.
  - A wasm-sdk unit test.
- **PR 4b (optional, D2).** wasm-safe `waitForProof`.
- **PR 5 (→ PR 4). Harness additions in wasm-sdk's functional suite:**
  - env plumbing: `CORE_RPC_*` and per-runner faucet keys from `configure_test_suite.sh`;
  - the Platform bank, plus a functional test of transfer-then-create;
  - the Core RPC faucet and its karma middleware;
  - the grpc-js Core client;
  - Node-only gating for Core specs;
  - un-skip `identityCreate`/`identityTopUp`;
  - CI: split `test-functional` into Node and browser jobs (batches if runtime needs them);
  - filters: wasm-sdk and evo-sdk also on rs-dpp, rs-drive, rs-sdk, wasm-dpp2, the context providers and dash-platform-queries.
- **PR 6 (→ PR 5).** Port withdrawals and Identity (they carry the Core work), and delete those platform-test-suite specs.
- **PR 7 (→ PR 5).** Port DataContract, Document, IndexOnlyDocument, contacts and dpns, applying §5.6, and delete those platform-test-suite specs.
- **PR 8 (→ PR 6 and PR 7).** Port the read-only and Core-endpoint specs (§5.5). Then **delete `platform-test-suite`**:
  - the package;
  - the `test-suite` job (`tests.yml:636-671`) and `tests-test-suite.yml`;
  - the Dockerfile `test-suite` stage and the `release-test-suite-image` job (`release.yml:262-272`);
  - the root `test:suite*` scripts;
  - the platform-test-suite part of `configure_test_suite.sh`, keeping faucet minting and env for the new suite;
  - `configure_test_suite_network.sh`, unless D4 keeps remote runs.

**Phase 3: delete the stack (→ PR 2a, 2b, 2c, PR 3, PR 8, PR S)**
- **PR 10 (any time before or with PR 9).** The two book chapters on wasm bindings move their examples to wasm-dpp2 (`packages/wasm-dpp2/CONVENTIONS.md`).
- **PR 9. Remove `js-dash-sdk`, `wallet-lib`, `js-dapi-client`, `dash-spv`, `js-grpc-common`, `wasm-dpp` and dapi-grpc's npm `package.json`.**
  - Keep the dapi-grpc crate, `protos/`, and both `clients/{core,platform}/v0/rust` example directories.
  - Delete the codegen: `scripts/build.sh`, `codegen.json`, `setup-codegen.py`, `test-client-codegen.yml`, and the non-Rust `clients/` trees.
  - **Dockerfile:**
    - delete the `build-js` stage (`:643-752`), which has no consumers after PR 2c and PR 8;
    - drop `packages/wasm-dpp` from the remaining `COPY --parents` lists (`:418,547,979`).
  - **CI:**
    - the `tests-packges-functional.yml` wallet-lib and `dash` steps;
    - the legacy filter keys, with wasm-dpp2's paths written out;
    - `tests-build-js.yml:22-31` and the codegen lines in `npm-release-build/action.yaml:16` and `npm-runner-validation.yml:66-69`;
    - the `book.yml` TypeDoc job and `docs/typedoc.json`, in the same PR, or the Pages deploy fails.
  - **Config and scripts:**
    - `yarn.config.cjs` offsets;
    - `.yarnrc.yml` audit excludes still needed by others, checked with `yarn why`;
    - `configure_dotenv.sh`, `prepare_docs.sh`, `.codecov.yml`, `pr.yml` scopes, `.pre-commit-config.yaml`;
    - the Swift sentinel;
    - `AGENTS.md` and the READMEs.

**Follow-ups, not on the critical path**
- Remove rs-dpp `extended-document`.
- Change `client_codegen` in the runner manifests (runner-image promotion).
- Delete the unused `js-packages.yml` and `rs-packages.yml`, and the dead prune step at `tests.yml:380-407`.
- A dashd-rpc wrapper over jayson, extending PR 2b's helper.
- bls via `@noble/curves`.
- The bn.js root resolution (§8.3).

**Phase 4: npm deprecation (§8.2).** Ivan's explicit go is required; it is outward-facing.

## 8. External users and deprecation

### 8.1 Who uses these packages outside this repo

Sources:
- `npm view`;
- `api.npmjs.org` (point, range and per-version);
- ecosyste.ms and deps.dev;
- read-only `gh api search/code`, on 2026-10-28.

npm's dependents tab sits behind a Cloudflare challenge.

**Download numbers.** Packages created a month ago with zero dependents get 360-410 downloads a week; that is this family's noise floor. evo-sdk already has about twice the monthly downloads of `dash` (9,831 against 4,654).

| Package | External users found |
|---|---|
| `dash` | A few active third parties, e.g. hilawe/dash-tegara-mn-pooling (`^7`; HD top-ups, SML parsing, deep import of `dash/build/.../broadcastStateTransition`), hilawe/dash-mno-verify (`^4`; documents and derived keys; evo-sdk covers it today), EmoBanana/Scatterly, densmirnov/dash-singles. Many ecosyste.ms "dependents" are installs of the unrelated 2011 `dash` package |
| wallet-lib, dapi-client | Mostly DCG repos (below), plus PastaPastaPasta/dapi-explorer |
| wasm-dpp, dash-spv, grpc-common, dapi-grpc, contract packages | None outside lockfiles that come through `dash` |
| dashcore-lib | **Broad and non-Platform:** xchainjs (133★, `^0.25.0`), fireblocks/recovery (`^0.21.0`), anypay, crowdnode.js, Insight (`^0.22.0`), altcoin forks; about 7.9k downloads a week |

**DCG and dashpay repos on legacy packages** (to migrate before `npm deprecate`; owned outside this repo):

| Repo | Uses | Notes |
|---|---|---|
| dash-network-deploy | Smoke tests (`test/smoke/dapi.js`) | Per-node probes work with evo-sdk's `addresses: [one]`. `getBlockByHeight`/`getBlockByHash` have no evo-sdk equivalent; Core RPC or the grpc-js client works instead |
| dash-bridge | Builds asset locks **itself** (`src/transaction/builder.ts`) and gets UTXOs from Insight. Uses dapi-client for IS-lock subscription, SML and status; dashcore-lib for parsing and message signing | PR 4 and 4b replace its builder and IS-lock wait. The SML needs stay |
| dash-platform-website (private) | `dash 8.0.0-rc.5`, with deep imports into `dash/build/...` | Its comments document more dash 8.x wallet breakage: dark-gravity-wave rejecting valid blocks, header sync from block 1 |
| dash-3d-nft-marketplace | Presented as the js-sdk example app | — |
| yappr | `register-contract.js` | — |

**Docs.** docs-platform moved its tutorials to evo-sdk in March 2026 (#133, #134, #136, #152). Still legacy:
- the JavaScript tabs of its two DAPI endpoint reference pages;
- `tutorials/building-platform.md` "Use JS SDK";
- the versioned 2.0.0 js-dash-sdk pages.

**dashmate blocks a clean deprecation.** dashmate 5.0.1 depends at runtime on wallet-lib, dapi-client, withdrawals-contract and dashcore-lib. Deprecating them now would warn every dashmate installer.

### 8.2 Path and minimal timeline

| When | Action |
|---|---|
| **Weeks 0-2** | U1 and PR 0 (§4). README banners on every legacy package, with **absolute** URLs: the js-dash-sdk banner's relative link resolves against the stale `dashevo/DashJS` repository field on npmjs.com. Add the missing `license` and `repository` fields to the **kept** packages (evo-sdk, wasm-sdk and wasm-dpp2 show no license on npm today). A "Deprecations" section in release notes, a pinned GitHub Discussion, and a banner on the docs-platform legacy pages and on the Read the Docs 2.0.0 version |
| **Final legacy release (D3)** | The last stable release that contains the packages, with PR 0. Its packages are the "final version". Known issues go in the READMEs, not fixes. Acceptance smoke test on testnet: default discovery, `getBestBlockHeight`, identity fetch, document query |
| **Weeks 2-6** | A migration guide in docs-platform, linked from the evo-sdk README, the legacy READMEs and the deprecation message. It maps `Dash.Client` calls to evo-sdk, gives a recipe for funding a one-time address, and has explicit "no equivalent" rows: HD Core wallet, balance and history, sending DASH, the masternode list, raw DAPI gRPC. Funding from Core gets a row of its own: "evo-sdk 5.1 (PR 4); until then, build the asset lock yourself as dash-bridge does". Then migrate the DCG repos, starting with dash-bridge |
| **Once a stable (`latest`) dashmate without the legacy dependencies is out (PR 2a), and PR S has merged on every release branch** | Announce "npm deprecation in 30 days" |
| **30 days later** | `npm deprecate <pkg>@"*" "<message naming the final version and evo-sdk>"` for `dash`, wallet-lib, dapi-client, wasm-dpp, dash-spv, grpc-common, dapi-grpc, the ten contract packages, `@dashevo/dark-gravity-wave` and `@dashevo/dash-util`. Run it with `--dry-run` first. It needs a human with 2FA; OIDC covers only `publish` |

**npm mechanics**
- `"*"` includes prereleases. Only versions that already exist are marked; that is why the packages must stop publishing first.
- Undo with an empty message.
- Leave the old dist-tags alone.
- **Never unpublish.**

**No major version bump for the final version:** a major would not reach `^8` or `~` ranges. **No runtime warnings in wallet-lib or dapi-client:** dashmate 5.0.x loads them on every command. One `process.emitWarning(..., 'DeprecationWarning')` in `dash`'s Client constructor is optional.

**Total: about 2-3 months, bounded by the dashmate release, the final legacy release and the 30-day notice. Not bounded by the port:** PR S stops publishing while the old suite still runs.

### 8.3 Security side findings

- **elliptic:** GHSA-848j-6mx2-7j84 affects every version up to 6.6.1 and has no patched release. dashcore-lib depends on `^6.5.4`; dropping dashcore-lib is the fix.
- **bn.js:** the `.yarnrc.yml` exclude blames dashcore-lib, but dashcore-lib's `^4.12.0` already allows the fixed 4.12.3. Our root resolution `"bn.js": "4.12.0"` pins the vulnerable version. A small independent fix.

## 9. What we keep

- **Packages:** js-evo-sdk, wasm-sdk (its functional suite becomes the JS e2e suite), wasm-dpp2, wasm-drive-verify, dashmate, the contract crates, dapi-grpc as a Rust crate with protos.
- **CI:** the local-network CI with SDK_TEST_DATA, running the e2e suite in Node and Chrome.
- **dashmate's** `@dashevo/bls`, `@dashevo/dashd-rpc` (PR 2b adds a jayson helper next to it) and `@dashevo/docker-compose`.

## 10. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| dashd:24 reaches dashmate before U1 ships | The legacy suite fails at bootstrap | The §4 stopgap (patch, `DAPI_ADDRESSES`, skip one case); or finish Phase 2 first |
| Insight breaks on v2 unlocks once PV15 emits them on V24 | dash-bridge, xchainjs and explorers lose their UTXO source | Flag in the asset-unlock-v2 PR; an upstream dashcore-lib release before PV15 activates with V24 |
| API gaps found mid-port | The port stalls | Each gap becomes a small wasm-sdk PR; legacy specs keep running until they are replaced |
| The suite's runtime grows (asset-lock and withdrawal cases wait for blocks) | Slow CI | Node and browser split, plus batches (PR 5) |
| SDK_TEST_DATA is a build-time flag (`Dockerfile:21-24`, `scripts/setup_local_network.sh:20-34`) | No bank on stock images | CI and `setup_local_network.sh` already build with it |
| Nonce or UTXO races between runners | Flaky tests | One seeded identity and one faucet key per runner; a client and mutex per runner; a spent-outpoint set |
| Asset-unlock outputs are not address-indexed | The ported withdrawal Core check fails | Verify in PR 6; fall back to `getrawtransaction` |
| A filter-alias break when a key is deleted | The `changes` job fails and Tests is skipped | Write the aliases out in every removal PR |
| A hidden `configure_*.sh` break behind the local-network cache | A later unrelated PR breaks | Change the scripts in the same PR as the deletion; bust the cache once |
| Legacy users stranded at PV15 activation if D3 = v5.0.x | `dash` 8.0.x likely fails on PV15 networks (not verified) | State it in the README and the deprecation message. D3 alternative: final version = v5.1.x stable |

## 11. Decisions

### 11.1 Decided by this spec (reasoning in the sections cited)

- Delete JS DAPI and bench-suite now (§3.1).
- Contract tests go to Rust, targeting the latest schema versions (§3.4).
- One JS e2e harness: wasm-sdk's functional suite (§5.2).
- The header-stream test is ported with grpc-js; it is not exposed in evo-sdk (§5.5).
- Drop dapi-grpc's npm package and multi-language codegen, together with grpc-common (§3.1, PR 9).
- The contract packages stop publishing to npm in the same PR as `dash` (PR S).
- Stop releasing the test-suite image (§3.5; its only external user has been dormant since 2024).
- Do not deprecate dashcore-lib (§8.1).
- No 0.22.1 from Platform's side; align with #5609 (§4).

### 11.2 For Ivan

- **D1. dashcore-lib's last fix.** Do U1 and PR 0 (one upstream release from master, and Platform bumps to it), or declare the legacy packages broken on V24 and keep CI on dashd:23 until the port lands? Recommended: do it. It is small next to the port, the final legacy version needs working discovery, and Insight needs the 0.22 backport regardless, once PV15 emits v2 unlocks on V24 networks.
- **D2. `waitForProof` in evo-sdk (PR 4b)?** It is not needed for the tests. It is the piece dash-bridge-like apps need to drop dapi-client. Recommended: later, as a product feature, after PR 4.
- **D3. Which release is the final legacy version?**
  - (a) The last v5.0.x stable, then delete on v5.1-dev. Fastest. Legacy users likely break when networks activate PV15.
  - (b) The first v5.1.x stable. PR S on v5.1-dev and PR 9 wait until v5.1.0 is cut, and legacy must keep building through the v5.1 cycle.

  Recommended: (a), stated clearly in the deprecation message.
- **D4. Remote-network e2e runs** (testnet or devnets with an env-provided bank). platform-test-suite supports them; the wasm-sdk suite is local-only. Recommended: drop them now and add an env-provided bank later only if someone needs it.

## 12. Verification plan

- **PR 1:** CI green; a cache-busted local-network run.
- **PR 2a:** unit tests for the probe (TLS options, timeout, disabled-Platform skip). `tests-dashmate.yml` green. A manual `dashmate start -w` on a self-signed local node and on a TLS-provider node.
- **PR 2b:** helper vectors per network, generated with dashcore-lib before removal. A cache-busted local-network run (setup plus `wallet mint`).
- **PR 2c:** the dashmate-helper image builds and its container starts; `tests-dashmate.yml` green.
- **PR S:** a release dry run (`release.yml` manual dispatch) packs no legacy package on either branch; `yarn constraints` passes.
- **PR 3:** the PR description maps each latest-schema rule to its Rust test.
- **PR 4:** fixed-vector Rust tests (a known transaction from known inputs). A functional identity create from a built asset lock.
- **PRs 5-8:**
  - Each ported spec is green in Node and Chrome.
  - Show red → green once for consensus-code assertions, by breaking the input deliberately.
  - Each PR description has a case table: old case → new case, or "dropped: covered by `<drive-abci test>`", or "dropped: legacy-only".
- **PR 9:**
  - `yarn install` and `yarn build` pass with no legacy workspace.
  - `yarn why @dashevo/dashcore-lib` is empty.
  - A release dry run (`release.yml` manual dispatch) packs only the kept packages.
  - The Drive, rs-dapi and dashmate-helper images build.
  - `book.yml` passes, including the rewritten book chapters (PR 10).
- **Deprecation:** `npm view '<pkg>@*' deprecated` shows the message for stable versions and prereleases.

## 13. Alternatives rejected

- **Port the suite to Rust (B) as the replacement:** it loses browser and JS-binding coverage, and it depends on a 66k-line unmerged PR.
- **Rewrite platform-test-suite on evo-sdk next to wasm-sdk's functional suite:** two harnesses on one SDK.
- **Contract tests on wasm-dpp2:** it would need new validation bindings just for tests.
- **A broad evo-sdk Core facade (broadcast, getTransaction, status):** the tests have Core RPC and grpc-js; no product need was shown beyond asset locks.
- **Platform staying on a dashcore-lib 0.22.1:** it wouldn't fix discovery without a republish anyway (`dash` pins dapi-client exactly), and #5609 already targets the master line. The 0.22 backport in U1 exists for Insight, not for Platform.
- **Hand-written asset-lock serialization in the test harness:** that duplicates Rust logic; PR 4 also serves users.
- **Readiness via rs-dapi JSON-RPC `getStatus`:** it reports success when Drive is down.

## 14. Review log (revision 1 → 2)

- **Feasibility.**
  - key-wallet's `asset_lock_builder` can't take caller UTXOs: PR 4 now uses `TransactionBuilder`.
  - rs-sdk's IS-lock wait isn't wasm-safe: split out into optional PR 4b.
  - The proof-verifier adapter was deleted too early: moot, since platform-test-suite is now deleted whole in PR 8.
  - Dockerfile stages are edited, not deleted.
  - No header-stream transport exists: §5.5, §6.3.
  - Added: faucet UTXO reuse, the chain-proof height wait, Core RPC env plumbing, a test of the bank path, karma-middleware browser coverage, and the lock-verification and reorg gaps in §6.3. The §6.3 estimate is raised.
- **Scope.**
  - dash-bridge already builds asset locks itself, so the "no JS path" claim is removed.
  - One JS e2e harness instead of two.
  - PR 4 shrunk to one builder; no evo-sdk Core facade.
  - Deliberate-drop rule (§5.6).
  - PR 2 split.
  - Contract tests target the latest schemas.
  - Open questions cut from 8 to 4.
- **CI and release.**
  - Exact Dockerfile lines.
  - Codegen touchpoints, including `tests-build-js.yml` and the runner manifests.
  - grpc-common is tied to dapi-grpc.
  - Filter-alias rewrites in every PR.
  - Script edits move into the deleting PR.
  - TypeDoc in PR 9.
  - Lockfiles committed.
  - `test:suite` resolution.
  - Final version gated on a stable release (D3).
  - Only "PR Hygiene" is a required check.
- **dashmate.**
  - dashd-rpc can't make the funding calls: jayson helper.
  - Probe target and TLS pinned; the infinite loop confirmed by reading the code on default providers.
  - Probe skipped for Core-only nodes; timeout policy.
  - Exact COPY lines; `pack_dashmate.sh` needs no change.
  - Helper image off `build-js` (PR 2c).
  - Helper pitfalls (version bytes, compressed keys, P2SH).
  - Log strings kept; cache-hit coverage gap.
- **External users.**
  - 0.22.1 mechanics: tag and workflow, and why it is dropped.
  - Final version from a stable line, and the v5.0-dev stop-publishing PR.
  - Missed: Insight, dark-gravity-wave, dash-util.
  - npm mechanics; migration guide; absolute banner links; metadata for kept packages as a gate; timeline.

## 15. Consistency check (revision 2 → 2.1)

- Keep both dapi-grpc Rust example directories.
- PR 9 deletes the codegen files and `test-client-codegen.yml`, and the whole `build-js` stage.
- Corrected Dockerfile lines: `+dash` is at `:747`, the comment at `:833-835`.
- Phase 3 now also waits for 2b and 2c.
- The stopgap is its own PR (0s), and it covers the wallet-lib and `dash` functional tests' env.
- Faucet keys: per network, and the `mint.log` overwrite caveat.
- U1 includes the 0.22 backport for Insight; the D1 reasoning was fixed to match.
- New PR S stops publishing on both release branches, so deprecation no longer waits for the port; the timeline was corrected.
- PR 10 moved before or into PR 9.
- PR 2c needs only 2a.
- Verification added for 2c, S and 10.
- Minor fixes: grpc-common users, `configure_dotenv.sh` lines, `tests.yml:137`.
