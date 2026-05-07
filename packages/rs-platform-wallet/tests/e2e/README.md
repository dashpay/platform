# E2E Test Framework — `rs-platform-wallet`

## Status

This framework was assembled across Waves 1-18, audited by QA in Wave 5, and exercised
end-to-end against Dash testnet. The single `transfer_between_two_platform_addresses`
test runs green: `cargo check` / `cargo clippy` / `cargo fmt --check` pass, and the
live happy-path run has been executed successfully in this branch. Future reruns
still require a testnet bank wallet pre-funded with
`>= PLATFORM_WALLET_E2E_MIN_BANK_CREDITS` credits; once an operator provisions one
and exports `PLATFORM_WALLET_E2E_BANK_MNEMONIC` (or sets it in `tests/.env`), the
harness is ready to run again via `cargo test` (see [Running tests](#running-tests)).

The runtime-flavor defect surfaced during the QA-001 reproduction (default
`tokio_shared_rt::test(shared)` lands on a current-thread runtime, which previously
panicked inside the SPV-backed context provider's `block_in_place` bridge) is
resolved. The harness now defaults to
[`TrustedHttpContextProvider`](#context-provider) and the retained
`SpvContextProvider` was rewritten in Wave 7 to use `dash_async::block_on`, which is
runtime-flavor agnostic. Multi-thread is therefore no longer strictly required, but
we still recommend
`#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]` —
it mirrors the `dash-evo-tool/tests/backend-e2e/` precedent and gives SPV background
tasks (when re-enabled per Task #15) head-room. The canonical pattern below uses it.

End-to-end tests that exercise the full wallet -> SDK -> broadcast pipeline against a
live Dash testnet. The framework validates platform-address credit operations through
the same `PlatformWalletManager` and `dash-sdk` layers used by production applications.

The design is modelled on `dash-evo-tool/tests/backend-e2e/`, with one important
difference in funding strategy: where DET uses Core asset locks to move value from
Layer 1 to Platform, this framework uses a **platform-address bank wallet** that
already holds credits. This avoids the need for a funded Core UTXO wallet and an
asset-lock broadcast during test initialization.

The directory is named `e2e/` rather than `platform_e2e/` because Core-feature tests
(SPV-driven UTXO operations) will land here too once the wallet's Core SPV pipeline is
stable enough to drive from tests. See [Future Core support](#future-core-support).

---

## Prerequisites

- A **testnet bank wallet** — a BIP-39 seed phrase for a Platform address that already
  holds enough credits to fund tests. You need this exactly once; subsequent runs
  recover unused test-wallet funds automatically.
- Network access to Dash testnet DAPI nodes (default) or a local/devnet cluster.
- Rust toolchain (stable, matches workspace `rust-toolchain.toml`).

Tests are gated behind `#[ignore]` so a stock `cargo test` (or workspace-wide
invocation) stays green for contributors and CI jobs that lack a funded testnet
bank wallet, live DAPI access, and the operator `.env`. To execute the live suite
once setup is in place, opt in explicitly with `--ignored`:

```bash
cargo test --test e2e -- --ignored --nocapture
```

If `PLATFORM_WALLET_E2E_BANK_MNEMONIC` is unset when an opt-in run starts, the
harness panics with an actionable message naming the bank's primary receive
address — the failure is operator-actionable, not silent. An under-funded bank
wallet panics with the same "top up at &lt;address&gt;" pointer.

---

## Environment variables

The framework reads configuration from the process environment and from
`packages/rs-platform-wallet/tests/.env` (anchored at `${CARGO_MANIFEST_DIR}/tests/.env`,
loaded via `dotenvy::from_path`). The path is deterministic regardless of the
shell's CWD — the framework matches the convention used by `rs-sdk` and
`rs-sdk-ffi`'s integration-test harnesses.

A canonical operator template lives at `tests/.env.example` — copy it to
`tests/.env` and fill in the bank mnemonic before the first run:

```bash
cp packages/rs-platform-wallet/tests/.env.example \
   packages/rs-platform-wallet/tests/.env
# then edit `packages/rs-platform-wallet/tests/.env` to set
# PLATFORM_WALLET_E2E_BANK_MNEMONIC
```

| Var | Required | Default | Purpose |
|-----|----------|---------|---------|
| `PLATFORM_WALLET_E2E_BANK_MNEMONIC` | yes | — | BIP-39 mnemonic for the bank wallet. This wallet must hold at least `PLATFORM_WALLET_E2E_MIN_BANK_CREDITS` credits before the first test runs. |
| `PLATFORM_WALLET_E2E_NETWORK` | no | `testnet` | Network to connect to: `testnet`, `devnet`, or `local`. |
| `PLATFORM_WALLET_E2E_DAPI_ADDRESSES` | no | network default | Comma-separated list of DAPI endpoint URLs. Overrides the SDK's built-in seed list for the selected network. |
| `PLATFORM_WALLET_E2E_MIN_BANK_CREDITS` | no | `500_000_000` | Minimum credit balance required in the bank wallet before initialization completes. If the bank is below this threshold the process panics with the bank's receive address so you know where to top it up. |
| `PLATFORM_WALLET_E2E_WORKDIR` | no | `${TMPDIR}/dash-platform-wallet-e2e` | Base path for the slot-locked working directory. SPV block cache, the test-wallet registry, and SDK state are stored here. |
| `PLATFORM_WALLET_E2E_TRUSTED_CONTEXT_URL` | no | network-builtin | Override URL for the trusted HTTP context provider. Leave unset to use the testnet/mainnet endpoint baked into `rs-sdk-trusted-context-provider`; required for devnet runs and any custom trust anchor. |
| `PLATFORM_WALLET_E2E_BANK_IDENTITY_ID` | no | auto-bootstrap | 32-byte hex id of a pre-registered bank identity used as the destination of identity-credit sweeps. Leave unset to let the harness register a fresh bank identity from the bank's primary platform address on first run and persist its id under the workdir slot at `<workdir>/bank_identity.json`. Set explicitly when sharing one bank identity across CI environments or workdir slots. |
| `PLATFORM_WALLET_E2E_BANK_CORE_GATE` | no | `900` (gate ON) | Bank Core (Layer-1) funding gate timeout, in seconds. The harness blocks at init until SPV's compact-filter scan walks far enough to observe the bank's pre-funded UTXOs (any non-zero confirmed Core balance). Default-on so fresh-workdir CR-* / ID-007 runs don't race a cold-cache scan and see `bank_core_balance=0` for an address that's been funded since last week. Set to `0` (or `disabled` / `false` / `off`) to opt out for Platform-only suites that don't need Core duffs; set to a positive integer to override the timeout. Invalid values fall back to the default with a warning. |
| `RUST_LOG` | no | `info,rs_platform_wallet=debug` | Tracing filter passed to `tracing-subscriber`. Increase to `debug` or `trace` for detailed sync output. |

Shell-exported variables take precedence — `dotenvy::from_path` does NOT overwrite
variables already set in the process environment. The workspace `.gitignore` covers
`.env` files anywhere under the tree, so the operator file never gets committed.

---

## Bank pre-funding (one-time)

The bank wallet is loaded from `PLATFORM_WALLET_E2E_BANK_MNEMONIC` on the first run.
If its credit balance is below `PLATFORM_WALLET_E2E_MIN_BANK_CREDITS`, initialization
panics with a message like:

```text
Bank wallet under-funded.
  balance : 0 credits
  required: 500000000 credits
  top up at: yXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

Send testnet platform credits to the address above, then re-run the tests.
```

Copy the printed address and use any testnet-funded wallet to send credits to it:

- **dash-evo-tool** — send from an existing DET identity's platform address.
- **wasm-sdk demo** — the browser demo supports platform-address transfers.
- Any other tool that can broadcast a platform-address credit transfer on testnet.

After the transfer confirms (typically a few seconds on testnet), re-run the tests.
The bank does not need topping up again until its balance drops below the minimum,
which the startup sweep helps prevent by recovering funds from completed test wallets.

---

## Running tests

```bash
# After copying tests/.env.example -> tests/.env and filling in the bank mnemonic:
cd packages/rs-platform-wallet
cargo test --test e2e -- --nocapture
```

Or override the mnemonic inline if you keep multiple banks:

```bash
PLATFORM_WALLET_E2E_BANK_MNEMONIC="..." cargo test --test e2e -- --nocapture
```

The first run takes **60–180 seconds**:

- The harness installs `TrustedHttpContextProvider` against the configured DAPI
  endpoints — first-run latency is dominated by the bank wallet's BLAST sync pass,
  not SPV startup. Cold runs typically finish setup in 5–15 s; subsequent runs in
  the same workdir slot reuse the SDK / token cache and are faster.
- The bank wallet runs a BLAST sync pass to discover its credit balances.
- The startup sweep recovers any wallets left over from previous panicked runs.
- Each test funds a fresh wallet, performs transfers, and tears down.

> If the optional `SpvContextProvider` is wired in (Task #15), expect an
> additional 30–60 s on cold cache for the masternode-list sync.

Run a single test by appending its name:

```bash
cargo test --test e2e -- --nocapture transfer_between_two_platform_addresses
```

Tracing output (SPV sync events, balance polls, sweep results) is written to stderr.
`--nocapture` keeps it visible in the terminal.

---

## Parallelism

The harness supports running cases in parallel within a single `cargo test`
invocation (`--test-threads=N`, N > 1) AND across multiple concurrent invocations
on the same machine.

### In-process (`--test-threads=N`)

All tests share one `E2eContext` (singleton via `tokio::sync::OnceCell`), one bank
wallet, one SPV runtime, and one workdir slot. Per-test isolation comes from:

- **Fresh per-test wallets** — every `setup()` mints a fresh OS-random 64-byte seed,
  so two parallel tests have disjoint wallet ids, addresses, identities, and nonces.
- **Serialised bank funding** — `bank.fund_address` and `bank.send_core_to` lock a
  process-global `FUNDING_MUTEX` so concurrent callers don't race UTXO selection or
  nonce assignment. Tests waiting on `wait_for_balance` do NOT hold the mutex —
  bank serialisation only covers the actual broadcast critical section.
- **Compile-time `Send + Sync`** — `E2eContext` and `SetupGuard` are statically
  asserted thread-safe (`framework/mod.rs`). A future field addition that breaks
  thread-safety fails to compile.

Two cases need a note under parallel execution:

- **PA-008c** observes the process-global `FUNDING_MUTEX_HISTORY` ring buffer to
  prove the mutex serialises. Asserts a lower bound on entry count (`>= 3`) and
  the pairwise non-overlap property — both hold regardless of sibling traffic.
- **PA-010** is `#[ignore]`'d pending a per-test bank instance API; bank is
  process-shared by design.

### Cross-process (concurrent `cargo test` invocations)

Multiple `cargo test` invocations on the same machine — for example, parallel CI
jobs or developer worktrees — must NOT share the same bank wallet or workdir slot.

**Workdir slots** — each process tries to acquire an exclusive `flock` on the base
working directory. If that lock is already held it walks up to 10 numbered slot
directories (`<workdir>-1`, `<workdir>-2`, ...). A slot holds the SPV block cache,
the SDK config, and the test-wallet registry independently from every other slot.

**Per-environment bank mnemonics** — two processes that share a mnemonic but land on
different slots will still conflict at the network level (duplicate nonces). The
correct isolation strategy is to give each CI environment its own distinct
`PLATFORM_WALLET_E2E_BANK_MNEMONIC`. The framework documents this requirement but
cannot enforce it across machines.

Typical CI setup:

```bash
# Branch A job
PLATFORM_WALLET_E2E_BANK_MNEMONIC="$BANK_MNEMONIC_BRANCH_A" cargo test ...

# Branch B job (different secret)
PLATFORM_WALLET_E2E_BANK_MNEMONIC="$BANK_MNEMONIC_BRANCH_B" cargo test ...
```

---

## Panic-safe cleanup

Every test wallet is registered in a JSON file at `<workdir>/test_wallets.json`
**before** the test starts — not after. If a test panics, the wallet's seed remains in
the registry so the next run can recover it.

### Happy path

`setup_guard.teardown()` is the explicit, recommended path:

1. Syncs the test wallet's balances.
2. Transfers any remaining credits back to the bank's primary address.
3. Removes the wallet entry from the registry and de-registers it from the manager.

> Teardown does NOT block waiting for the bank to observe the inbound credits — the
> sweep transition is broadcast and confirmed by the chain, and the bank wallet
> re-syncs lazily on its next operation. Tests that immediately follow up with bank
> ops should call `bank.sync_balances().await` to refresh the cached view.

### Panic path

If `teardown()` is not called — because the test panicked or returned early — the
`SetupGuard` `Drop` implementation logs a warning:

```text
SetupGuard dropped without explicit teardown — wallet <id>
will be swept on next test process startup
```

The wallet entry stays in `test_wallets.json`. On the next run, the startup sweep
(`sweep_orphans`) iterates all registry entries, reconstructs each wallet from its
stored seed, syncs, and transfers remaining credits back to the bank. Successfully
swept wallets are removed from the registry; wallets that fail to sweep (transient
network error) are marked `Failed` and retried on the following run.

The registry uses atomic writes (write to a temp file, then rename) to avoid
corruption from mid-write crashes.

### Bank identity

Identity-credit sweeps need an identity to receive the swept funds (the
`CreditTransfer` state transition is identity → identity, not identity →
address). The harness keeps one **bank identity** per workdir slot, recorded at
`<workdir>/bank_identity.json`. Resolution order on every `setup`:

1. If `PLATFORM_WALLET_E2E_BANK_IDENTITY_ID` is set, the harness loads that
   identity verbatim.
2. Otherwise, if `<workdir>/bank_identity.json` exists, the harness reuses the
   recorded identity id (after cross-checking that the persisted `wallet_id`
   matches the active bank mnemonic — a mismatch surfaces as a clear bank
   error rather than a silent wrong-bank sweep).
3. Otherwise, the harness registers a fresh identity at DIP-9 index `0xBA77`
   from the bank's primary receive address, persists the resulting id to the
   workdir slot, and reuses it on subsequent runs.

Bootstrap consumes a one-time funding round from the bank's primary platform
address (~80M credits). After that, swept identity credits accumulate on the
bank identity instead of leaking on every run.

---

## Troubleshooting

- **Bank under-funded** — Initialization panics with the bank's receive address and
  the current balance. Top up the printed address from any testnet wallet and re-run.
  The minimum threshold is controlled by `PLATFORM_WALLET_E2E_MIN_BANK_CREDITS`
  (default 500 000 000 credits).

- **DAPI / context-provider unreachable** — `TrustedHttpContextProvider` calls fail
  if the configured DAPI endpoints are unreachable. Check `PLATFORM_WALLET_E2E_DAPI_ADDRESSES`
  and network connectivity. Setting `RUST_LOG=debug` shows which DAPI nodes are
  being contacted. (The optional SPV path adds its own ~30–60 s masternode-list
  sync timeout — only relevant if `SpvContextProvider` is wired in.)

- **Workdir slot exhausted** — If all 10 slots are locked, initialization fails with:
  `no available workdir slots (tried 10 under <path>)`. This typically means 10+
  concurrent processes are running against the same `PLATFORM_WALLET_E2E_WORKDIR`
  base. Either wait for other processes to finish, remove stale lock files from
  the slot directories (`rm <workdir>*/.lock`), or set `PLATFORM_WALLET_E2E_WORKDIR`
  to a distinct path per environment.

- **Test panicked — registry not cleared** — On the next run, the startup sweep log
  will report `swept N wallets from previous panicked run`. This is expected behavior.
  If the sweep itself fails (the orphaned wallet has no balance, or the network is
  unavailable), the entry is marked `Failed` and retried on the following run. Entries
  with a `Failed` status do not block test execution.

---

## Context provider

The harness installs
[`rs-sdk-trusted-context-provider::TrustedHttpContextProvider`](../../../rs-sdk-trusted-context-provider)
as the SDK's context provider at construction time. That provider answers quorum
public-key lookups over a trusted HTTP endpoint (testnet / mainnet defaults are
baked into the crate), which keeps e2e runs fast and reliable without spinning up
an SPV client.

Override the endpoint via `PLATFORM_WALLET_E2E_TRUSTED_CONTEXT_URL` when running
against devnet, a custom test cluster, or any non-default trust anchor.

```bash
PLATFORM_WALLET_E2E_TRUSTED_CONTEXT_URL="https://my-trusted-quorum.example/" \
  cargo test --test e2e -- --nocapture
```

---

## Deferred

- **SPV-based context provider** (Task #15). The framework keeps the SPV plumbing
  (`framework/spv.rs`, `framework/context_provider.rs`) compilable but disabled:
  see the commented-out block in `framework/harness.rs::E2eContext::build`. Re-enable
  by uncommenting that block once SPV cold-start is stable enough to drive from
  tests; the `TrustedHttpContextProvider` swap is a single-line change.

---

## Future Core support

The directory is intentionally named `e2e/` rather than `platform_e2e/`. Once the
wallet's SPV-driven Core operations (UTXO selection, transaction broadcast, asset
locks) are stable enough to test end-to-end, Core-feature tests will live alongside
the existing platform-address tests under `tests/e2e/cases/core/`.

When Task #15 lands, an `SpvRuntime` will run for the lifetime of the test process
and `SpvContextProvider` will be live-swapped into the SDK after mn-list sync.
Future identity and Core tests will get SPV-backed proof verification at that
point without changing the public test API.

---

## Architecture quick reference

The framework initializes once per test-binary process. All tests in `tests/e2e/`
share a single `E2eContext` via a `tokio::sync::OnceCell`.

| Symbol | Where | What it does |
|--------|-------|-------------|
| `setup()` | `framework/mod.rs` | Initializes `E2eContext` (once), creates a fresh test wallet, registers it in the JSON registry, and returns a `SetupGuard`. |
| `SetupGuard.ctx` | `framework/wallet_factory.rs` | Reference to the shared `E2eContext` — holds the SDK, bank wallet, SPV runtime, and registry. |
| `SetupGuard.test_wallet` | `framework/wallet_factory.rs` | Fresh `TestWallet` for this test, pre-registered for panic-safe cleanup. |
| `ctx.bank().fund_address(addr, credits)` | `framework/bank.rs` | Transfers `credits` from the bank wallet to `addr`. Serialized within the process by `FUNDING_MUTEX`. |
| `test_wallet.transfer(outputs)` | `framework/wallet_factory.rs` | Broadcasts a platform-address credit transfer and returns a `PlatformAddressChangeSet`. |
| `wait_for_balance(wallet, addr, credits, timeout)` | `framework/wait.rs` | Polls the wallet's balance cache until `addr` holds at least `credits`, or times out. |
| `setup_guard.teardown()` | `framework/wallet_factory.rs` | Returns remaining credits to the bank, removes wallet from registry, de-registers from manager. |

Canonical test pattern:

```rust
use crate::framework::prelude::*;

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn transfer_between_two_platform_addresses() {
    let s = setup().await.expect("e2e setup failed");

    let addr_1 = s.test_wallet.next_unused_address().await.unwrap();
    s.ctx.bank().fund_address(&addr_1, 100_000_000).await.unwrap();
    wait_for_balance(&s.test_wallet, &addr_1, 70_000_000, Duration::from_secs(60))
        .await
        .unwrap();

    let addr_2 = s.test_wallet.next_unused_address().await.unwrap();
    s.test_wallet
        .transfer(std::iter::once((addr_2, 50_000_000)).collect())
        .await
        .unwrap();

    wait_for_balance(&s.test_wallet, &addr_2, 1_000_000, Duration::from_secs(60))
        .await
        .unwrap();

    // The production wallet does not surface a `fee_paid` accessor;
    // derive it from the balance delta. `received + remaining + fee
    // == funded`, so `fee = funded - received - remaining`.
    let balances = s.test_wallet.balances().await;
    let received = balances.get(&addr_2).copied().unwrap_or(0);
    let remaining = balances.get(&addr_1).copied().unwrap_or(0);
    let fee = 100_000_000_u64.saturating_sub(received).saturating_sub(remaining);
    assert!(received >= 1_000_000 && received < 50_000_000);
    assert!(fee > 0 && fee < 50_000_000);

    s.teardown().await.expect("teardown failed");
}
```

The `shared` runtime attribute is not optional. SPV (when re-enabled per Task #15)
spawns background tasks bound to the runtime that created them. With `#[tokio::test]`
each test would create its own runtime; the first test's exit would drop that runtime
and kill SPV's background tasks, causing channel-closed errors in later tests.

For deeper implementation details — module responsibilities, registry schema, signer
design, workdir slot algorithm — refer to the plan file at
`.claude/plans/ok-now-we-ll-get-prancy-biscuit.md`.

> **Runtime flavor is recommended, not strictly required.** With the current
> `TrustedHttpContextProvider` default and the retained `SpvContextProvider`'s
> `dash_async::block_on` bridge (Wave 7), tests no longer panic on a
> current-thread runtime. We still recommend
> `flavor = "multi_thread", worker_threads = 12` to mirror the DET precedent and
> to leave head-room for SPV-backed providers and other concurrent background
> work; the canonical example uses it.

---

<sub>Co-authored by [Claudius the Magnificent](https://github.com/lklimek/claudius) AI Agent</sub>
