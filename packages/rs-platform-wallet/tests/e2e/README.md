# E2E Test Framework — `rs-platform-wallet`

## Status

This framework was assembled across Waves 1-4 and audited by QA in Wave 5. The single
`transfer_between_two_platform_addresses` test compiles cleanly, its module wiring is
sound, and `cargo check` / `cargo clippy` / `cargo fmt --check` are green. **The live
happy-path run has not yet been executed in this branch** because no testnet bank
wallet pre-funded with `>= PLATFORM_WALLET_E2E_MIN_BANK_CREDITS` credits is available
to the QA agent. Once an operator provisions one and exports
`PLATFORM_WALLET_E2E_BANK_MNEMONIC`, the run is one `cargo test` away (see
[Running tests](#running-tests)).

A reproducible defect was found while attempting the under-funded panic check: the
test attribute `#[tokio_shared_rt::test(shared)]` defaults to a **current-thread**
tokio runtime, under which `SpvContextProvider::get_quorum_public_key` panics with
`"can call blocking only when running on the multi-threaded runtime"` because it uses
`tokio::task::block_in_place`. DET's precedent uses
`#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]` on
every test for exactly this reason. A follow-up Bilby pass should either fix the
attribute on `cases::transfer::transfer_between_two_platform_addresses` and the
example in this README, or replace the `block_in_place` bridge with a channel-based
async->sync handoff inside `framework/context_provider.rs`.

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

All tests carry `#[ignore]`, so they are excluded from normal `cargo test` runs and
will never trip CI pipelines that do not set the required environment variable.

---

## Environment variables

The framework reads configuration from the process environment (or a `.env` file in the
`packages/rs-platform-wallet` directory, loaded via `dotenvy`).

| Var | Required | Default | Purpose |
|-----|----------|---------|---------|
| `PLATFORM_WALLET_E2E_BANK_MNEMONIC` | yes | — | BIP-39 mnemonic for the bank wallet. This wallet must hold at least `PLATFORM_WALLET_E2E_MIN_BANK_CREDITS` credits before the first test runs. |
| `PLATFORM_WALLET_E2E_NETWORK` | no | `testnet` | Network to connect to: `testnet`, `devnet`, or `local`. |
| `PLATFORM_WALLET_E2E_DAPI_ADDRESSES` | no | network default | Comma-separated list of DAPI endpoint URLs. Overrides the SDK's built-in seed list for the selected network. |
| `PLATFORM_WALLET_E2E_MIN_BANK_CREDITS` | no | `100_000_000` | Minimum credit balance required in the bank wallet before initialization completes. If the bank is below this threshold the process panics with the bank's receive address so you know where to top it up. |
| `PLATFORM_WALLET_E2E_WORKDIR` | no | `${TMPDIR}/dash-platform-wallet-e2e` | Base path for the slot-locked working directory. SPV block cache, the test-wallet registry, and SDK state are stored here. |
| `RUST_LOG` | no | `info,rs_platform_wallet=debug` | Tracing filter passed to `tracing-subscriber`. Increase to `debug` or `trace` for detailed sync output. |

A `.env` file is convenient for local development. Shell-exported variables take
precedence — `dotenvy` does not overwrite variables that are already set.

```bash
# packages/rs-platform-wallet/.env  (do not commit this file)
PLATFORM_WALLET_E2E_BANK_MNEMONIC="word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11 word12"
```

---

## Bank pre-funding (one-time)

The bank wallet is loaded from `PLATFORM_WALLET_E2E_BANK_MNEMONIC` on the first run.
If its credit balance is below `PLATFORM_WALLET_E2E_MIN_BANK_CREDITS`, initialization
panics with a message like:

```
Bank wallet under-funded.
  balance : 0 credits
  required: 100000000 credits
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
cd packages/rs-platform-wallet
PLATFORM_WALLET_E2E_BANK_MNEMONIC="..." cargo test --test e2e -- --ignored --nocapture
```

The first run takes **60–180 seconds**:

- SPV light-client initializes and syncs the masternode list (~30–60 s on a cold
  cache; significantly faster on repeat runs when the block cache is warm).
- The bank wallet runs a BLAST sync pass to discover its credit balances.
- The startup sweep recovers any wallets left over from previous panicked runs.
- Each test itself funds a fresh wallet, performs transfers, and tears down.

Run a single test by appending its name:

```bash
PLATFORM_WALLET_E2E_BANK_MNEMONIC="..." \
  cargo test --test e2e -- --ignored --nocapture transfer_between_two_platform_addresses
```

Tracing output (SPV sync events, balance polls, sweep results) is written to stderr.
`--nocapture` keeps it visible in the terminal.

---

## Multi-process safety

Multiple `cargo test` invocations running concurrently — for example, parallel CI jobs
on different branches — must not share the same bank wallet or working directory, or
they will conflict on nonces.

The framework handles this at two levels:

**Workdir slots** — each process tries to acquire an exclusive `flock` on the base
working directory. If that lock is already held it tries up to 10 numbered slot
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
3. Waits for the bank to observe the incoming credits (60 s timeout).
4. Removes the wallet entry from the registry and de-registers it from the manager.

### Panic path

If `teardown()` is not called — because the test panicked or returned early — the
`SetupGuard` `Drop` implementation logs a warning:

```
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

---

## Troubleshooting

- **Bank under-funded** — Initialization panics with the bank's receive address and
  the current balance. Top up the printed address from any testnet wallet and re-run.
  The minimum threshold is controlled by `PLATFORM_WALLET_E2E_MIN_BANK_CREDITS`
  (default 100 000 000 credits).

- **SPV sync timeout** — Startup waits up to 60 seconds for the masternode list to
  sync. If it times out, testnet peers may be temporarily unreachable. Check network
  connectivity and try again; the block cache in the workdir slot will make the next
  attempt faster. Setting `RUST_LOG=debug` shows which peers the SPV client is
  connecting to.

- **Workdir slot exhausted** — If all 10 slots are locked, initialization fails with:
  `No available workdir slots (tried 0..10)`. This typically means 10+ concurrent
  processes are running against the same `PLATFORM_WALLET_E2E_WORKDIR` base. Either
  wait for other processes to finish, remove stale lock files from the slot directories
  (`rm <workdir>*/.lock`), or set `PLATFORM_WALLET_E2E_WORKDIR` to a distinct path per
  environment.

- **Test panicked — registry not cleared** — On the next run, the startup sweep log
  will report `swept N wallets from previous panicked run`. This is expected behavior.
  If the sweep itself fails (the orphaned wallet has no balance, or the network is
  unavailable), the entry is marked `Failed` and retried on the following run. Entries
  with a `Failed` status do not block test execution.

---

## Future Core support

The directory is intentionally named `e2e/` rather than `platform_e2e/`. Once the
wallet's SPV-driven Core operations (UTXO selection, transaction broadcast, asset
locks) are stable enough to test end-to-end, Core-feature tests will live alongside
the existing platform-address tests under `tests/e2e/cases/core/`.

SPV is already started at framework initialization — a `SpvRuntime` is running for
the lifetime of the test process, and `SpvContextProvider` is wired to bridge
quorum-key lookups into the SDK. Future identity and Core tests get proof verification
for free without changing the initialization sequence.

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

#[tokio_shared_rt::test(shared)]
#[ignore = "requires PLATFORM_WALLET_E2E_BANK_MNEMONIC and testnet access"]
async fn transfer_between_two_platform_addresses() {
    let mut s = setup().await.expect("e2e setup failed");

    let addr_1 = s.test_wallet.next_unused_address().await.unwrap();
    s.ctx.bank().fund_address(&addr_1, 50_000_000).await.unwrap();
    wait_for_balance(&s.test_wallet, &addr_1, 50_000_000, Duration::from_secs(60))
        .await
        .unwrap();

    let addr_2 = s.test_wallet.next_unused_address().await.unwrap();
    let cs = s.test_wallet
        .transfer(std::iter::once((addr_2.clone(), 10_000_000)).collect())
        .await
        .unwrap();

    wait_for_balance(&s.test_wallet, &addr_2, 10_000_000, Duration::from_secs(60))
        .await
        .unwrap();

    let balances = s.test_wallet.balances().await;
    assert_eq!(balances[&addr_2], 10_000_000);
    assert_eq!(balances[&addr_1], 50_000_000 - 10_000_000 - cs.fee_paid());

    s.teardown().await.expect("teardown failed");
}
```

The `shared` runtime attribute is not optional. SPV spawns background tasks bound to
the runtime that created them. With `#[tokio::test]` each test would create its own
runtime; the first test's exit would drop that runtime and kill SPV's background tasks,
causing channel-closed errors in later tests.

For deeper implementation details — module responsibilities, registry schema, signer
design, workdir slot algorithm — refer to the plan file at
`.claude/plans/ok-now-we-ll-get-prancy-biscuit.md`.

> **Note (QA Wave 5):** the example above intentionally omits the runtime flavor for
> brevity, but in practice the attribute must include
> `flavor = "multi_thread", worker_threads = 12` (mirroring DET's e2e harness) — see
> the [Status](#status) section. Without it, `SpvContextProvider`'s
> `block_in_place` bridge panics on the current-thread runtime that
> `tokio_shared_rt::test(shared)` builds by default.

---

<sub>Co-authored by [Claudius the Magnificent](https://github.com/lklimek/claudius) AI Agent</sub>
