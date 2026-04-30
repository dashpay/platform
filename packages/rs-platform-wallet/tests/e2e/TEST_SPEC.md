# `rs-platform-wallet` e2e — Test Case Specification

Brain the size of a planet, and here I am cataloguing test cases. Right then.
This document enumerates the work to do; another document, somewhere, will
presumably enumerate the joy of doing it.

---

## 1. Overview

The `rs-platform-wallet` end-to-end suite lives at
`packages/rs-platform-wallet/tests/e2e/` and executes against Dash testnet via
the SDK and a pre-funded "bank" platform-address wallet. The harness was
introduced in PR #3549 (branch `feat/rs-platform-wallet-e2e`) and ships with a
single live case — `transfer_between_two_platform_addresses` — exercising
platform-address credit transfer between two addresses owned by the same test
wallet.

This specification proposes a layered set of cases, grouped by feature area,
prioritised P0/P1/P2, and annotated with the harness extensions each requires.
Every case targets the production `PlatformWallet` API surface (no test-only
shims into the wallet), uses the bank-funded credit model already wired in
`framework/bank.rs`, and assumes the same network model PR #3549 ships with:
testnet by default, devnet/local by env override, no Layer-1 / Core-UTXO
assumptions for any P0/P1 case (Task #15 — SPV — is the gating dependency for
Core-feature tests).

The spec is implementation-agnostic. Authors should consume it, not migrate it
verbatim from `dash-evo-tool` (DET) — DET parallels are cited only to anchor
intent and to surface battle-tested edge cases. The harness lives on top of
`PlatformWalletManager<NoPlatformPersistence>` and a `TrustedHttpContextProvider`,
so anything requiring SPV proofs, asset locks, shielded notes, or fresh contract
deployment is explicitly deferred (see §5).

### 1.1 Priority scheme

Every test case carries one of three priority levels. The priority drives both
listing order within a section and CI gating tier.

- **P0 — Primary path.** The happy path that demonstrates the feature works.
  CI-gating tier; failure blocks merge. Execute first.
- **P1 — Core variants.** Negative paths and alternate-input variants of P0
  cases that protect the primary contract. Execute alongside P0 in CI.
- **P2 — Edge cases.** Boundary, empty-input, concurrency, malformed-input,
  and discovered-gap cases. Run nightly / on-demand; not gating unless an
  active regression makes one of them so. Execute after P0/P1.

Within each feature-area subsection (Platform Addresses, Identity, Tokens,
DPNS, Dashpay, etc.), test cases are listed P0 first, then P1, then P2. The
suffix-letter convention (e.g. `PA-001b`, `PA-002c`) groups variant cases next
to their parent; new top-level edge cases get fresh dense IDs (e.g. `PA-009`,
`PA-010`). No existing case ID is renumbered; new cases slot in adjacent to
their parent.

### 1.2 Mnemonic / seed source

Mnemonics used by the harness (bank wallet, every `TestWallet`) MUST be drawn
from the BIP-39 English wordlist. Out-of-band entropy paths — raw entropy,
non-BIP-39 wordlists, or arbitrary UTF-8 strings fed as "mnemonic" — are out
of scope for this suite. Any test that generates a seed does so via the
BIP-39 mnemonic generator already used by `framework/wallet_factory.rs`. Cases
that exercise non-ASCII content (e.g. Unicode display names) do so on
downstream fields, not on the seed.

---

## 2. Harness capability matrix

Honest snapshot of what PR #3549 can drive today vs. what each test area still
needs. "Wallet API exists" reflects what `packages/rs-platform-wallet/src/`
already exposes; "Harness ready" reflects whether
`packages/rs-platform-wallet/tests/e2e/framework/` can drive it without code
changes.

| Area | Wallet API exists | Harness ready | Gaps to fill | Out of scope (and why) |
|------|-------------------|---------------|--------------|------------------------|
| Platform Addresses | yes (`platform_addresses/{transfer,sync,withdrawal,fund_from_asset_lock}`) | yes for transfer/sync; partial for withdrawal | needs `wait_for_balance_eq` (exact-equality variant), needs explicit-input transfer helper, needs withdrawal Core-balance verification stub | `withdraw` end-to-end (Layer-1 observation, blocked on Task #15); `fund_from_asset_lock` (Core UTXO needed, bank holds credits not coins) |
| Identity | yes (`identity/network/{register_from_addresses,top_up_from_addresses,registration,update,transfer,transfer_to_addresses,withdrawal}`) | no | `Signer<IdentityPublicKey>` impl, identity-key derivation helper, `TestWallet::register_identity_from_addresses`, `wait_for_identity_balance` | asset-lock-funded register/top-up (DET territory; bank holds credits); identity withdrawal (Layer-1 observation) |
| Tokens | yes (`tokens/wallet.rs` and `identity/network/tokens/*`) | no | `Signer<IdentityPublicKey>`, identity setup, contract-token discovery helper, `TestTokenContract` fixture pointer | fresh contract deployment (no testnet contract registry); group-action workflows that need multi-identity coordination outside one harness |
| Core / SPV | yes (`core/{wallet,balance,broadcast,balance_handler}`) | no — `spv_runtime: None` by design | enable SPV runtime (gated on Task #15), `wait_for_core_balance`, faucet helper | broadcast tests until SPV stable; tx-is-ours flag tests (DET parity, P2) |
| Asset Lock | yes (`asset_lock/{build,manager,sync,tracked,lock_notify_handler}`) | no | needs Core-UTXO funded test wallet, SPV runtime, `wait_for_asset_lock` | full path until Task #15 — bank wallet has no Core UTXOs |
| Shielded | yes (`shielded/{keys,note_selection,operations,prover,store,sync}`) | no | not a small extension — prover, viewing keys, note selection | entire surface — separate prover/keys complexity, defer to a dedicated suite |
| Contracts | yes (`identity/network/contract.rs::create_data_contract_with_signer`) | no | identity signer, schema fixtures (`tests/fixtures/contracts/`), `wait_for_contract_visible` | `replace`/`transfer` of an arbitrary deployed contract owned elsewhere — gated on a contract-registry strategy |
| DPNS | yes (`identity/network/dpns.rs::{register_name_with_external_signer,resolve_name,sync_dpns_names,contest_vote_state}`) | no | identity signer, name uniqueness (random suffix), `wait_for_dpns_name` | contested-name auctions (P2; multi-identity orchestration heavy) |
| Dashpay | yes (`identity/network/{profile,contact_requests,contacts,payments,dashpay_sync}`) | no | identity signer, two test identities + DPNS for one of them, `wait_for_contact_request` | full multi-step lifecycle relying on contact-request acceptance round trips beyond a single happy-path |
| Contested Names | yes (via DPNS contest API) | no | identity signer, multi-identity setup, vote orchestration | P2 only; testnet contest auctions are slow and DET already covers this end-to-end |

Source citations for the "Wallet API exists" column are listed inline per case
(§3) using `file:line` form.

---

## 3. Test cases — ranked

### Quick index

| ID | Title | Priority | Complexity |
|----|-------|----------|------------|
| PA-001 | Multi-output platform-address transfer | P0 | S |
| PA-002 | Partial-fund + change handling | P0 | S |
| PA-004 | Sweep-back: drain test wallet, observe bank credit | P0 | S |
| PA-003 | Fee scaling: one-output vs. five-output | P1 | M |
| PA-005 | Address rotation: gap-limit + observed-used cursor | P1 | M |
| PA-006 | Replay safety: same outputs, second submission rejected | P1 | M |
| PA-007 | Sync watermark idempotency | P1 | M |
| PA-008 | Concurrent funding from bank: serialised | P1 | S |
| PA-002b | Zero-change exact-equality (`Σ outputs + fee == input balance`) | P1 | S |
| PA-010 | Bank starvation: typed `BankUnderfunded` error | P1 | S |
| PA-001b | Transfer with `output_change_address: None` vs `Some(addr)` | P2 | S |
| PA-001c | Zero-credit single-output transfer | P2 | S |
| PA-004b | Sweep dust threshold boundary triplet | P2 | M |
| PA-004c | Sweep with exactly zero balance | P2 | S |
| PA-005b | `DEFAULT_GAP_LIMIT` triplet (19 / 20 / 21 unused) | P2 | M |
| PA-006b | Two concurrent broadcasts of identical ST bytes | P2 | M |
| PA-007b | Two concurrent `sync_balances` on one wallet | P2 | M |
| PA-008b | Two `TestWallet`s × three concurrent funders each | P2 | M |
| PA-008c | Observable serialisation of `FUNDING_MUTEX` | P2 | M |
| PA-009 | `min_input_amount` boundary triplet for cleanup | P2 | M |
| PA-011 | Workdir slot exhaustion at `MAX_SLOTS + 1` | P2 | M |
| PA-012 | `sync_balances` racing with `transfer` | P2 | M |
| PA-013 | Broadcast retry under transient DAPI 5xx | P2 | M |
| PA-014 | Multi-output at protocol-max output count | P2 | M |
| ID-001 | Register identity funded from platform addresses | P0 | L |
| ID-002 | Top-up identity from platform addresses | P0 | M |
| ID-003 | Identity-to-identity credit transfer | P0 | M |
| ID-004 | Identity update: add and disable a key | P1 | L |
| ID-005 | Transfer credits from identity to platform addresses | P1 | M |
| ID-006 | Refresh and load identity by index | P1 | M |
| ID-001c | Non-default `StateTransitionSettings` (`wait_for_proof = false`) | P2 | M |
| ID-005b | `transfer_credits_to_addresses` with empty outputs | P2 | S |
| ID-006b | Identity-key derivation index boundary (`0` and `DEFAULT_GAP_LIMIT - 1`) | P2 | M |
| TK-001 | Token transfer between two identities | P1 | L |
| TK-001b | Token transfer of amount 0 | P2 | S |
| TK-002 | Token claim (perpetual / pre-programmed distribution) | P2 | L |
| TK-003 | Token mint (authorised identity) | P2 | M |
| TK-004 | Token burn | P2 | M |
| CR-001 | SPV mn-list sync readiness | P1 | M |
| CR-002 | Core wallet receive address derivation | P1 | M |
| CR-003 | Asset-lock-funded identity registration (full path) | P2 | L |
| CT-001 | Document put: deploy a fixture data contract | P1 | M |
| CT-002 | Document put / replace lifecycle | P2 | M |
| CT-003 | Contract update (add document type) | P2 | M |
| DPNS-001 | Register and resolve a `.dash` name | P0 | M |
| DPNS-001b | Name-length boundary quartet (2 / 3 / 63 / 64 chars) | P2 | M |
| DPNS-001c | DPNS name with a multibyte character | P2 | S |
| DPNS-002 | Resolve a known external name (negative-only) | P2 | S |
| DP-001 | Set DashPay profile | P1 | M |
| DP-001b | Profile with optional fields `None` vs `Some` | P2 | M |
| DP-001c | Profile `display_name` containing emoji / RTL text | P2 | S |
| DP-002 | Send and accept a contact request | P1 | L |
| DP-003 | Send a DashPay payment | P2 | L |
| CN-001 | Initiate a contested DPNS name (premium / 3-char) | P2 | L |
| CN-002 | Cast a masternode vote on a contested name | DEFERRED | — |
| Harness-G1a | Corrupted registry JSON: refuse to overwrite | P2 | M |
| Harness-G1b | Registry forward-compatible unknown field | P2 | S |
| Harness-G4 | Drop `wallet.transfer` future mid-flight, recover on next sync | P2 | L |

Counts by priority: **P0: 7**, **P1: 16** (incl. 2 post-Task #15), **P2: 34** (incl. 1 post-Task #15, 1 gated), **DEFERRED: 1** (58 total entries; 57 implementable cases + 1 deferred placeholder).

### Platform Addresses (PA)

#### PA-001 — Multi-output platform-address transfer (one tx, N outputs)
- **Priority**: P0
- **Wallet feature exercised**: `wallet/platform_addresses/transfer.rs:31` (`PlatformAddressWallet::transfer`)
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/wallet_tasks.rs:561` (`tc_014_wallet_platform_lifecycle`) covers a transfer; multi-output is a derivative variant.
- **Preconditions**: bank funded; `setup()` returns a fresh `TestWallet`.
- **Scenario**:
  1. Derive `addr_1` on test wallet; bank-fund with `90_000_000` credits; wait for balance.
  2. Derive `addr_2`, `addr_3` after the funding sync (two consecutive `next_unused_address` calls return distinct addresses only because the pool cursor advanced — see PA-005 for the assertion).
  3. Self-transfer `{addr_2: 20_000_000, addr_3: 30_000_000}` from `addr_1` in one call.
  4. Wait for `addr_2` and `addr_3` to each reach their target balance.
- **Assertions**:
  - `balances[addr_2] == 20_000_000`
  - `balances[addr_3] == 30_000_000`
  - `total_credits == 90_000_000 - fee` (fee derived from balance delta)
  - `0 < fee < 5_000_000` (fee scales sub-linearly with output count — guards regression of fee strategy)
  - One observable on-chain change-set update, not two (wallet returned a single `PlatformAddressChangeSet`).
- **Negative variants**:
  - Outputs total exceeds funded balance → expect `PlatformWalletError` of insufficient-funds shape.
  - Empty output map → expect a typed validation error (not a panic).
  - Duplicate output address (two entries with same `PlatformAddress`) → BTreeMap dedup is implicit; assert collapsed semantics.
- **Harness extensions required**: none.
- **Estimated complexity**: S
- **Rationale**: Closes the obvious gap left by `PR #3549` — the only existing case is one-input/one-output. Multi-output catches fee-scaling regressions, change-output handling, and any off-by-one on the `BTreeMap` plumbing into `transfer()`.

#### PA-002 — Partial-fund + change handling (output < input balance)
- **Priority**: P0
- **Wallet feature exercised**: `wallet/platform_addresses/transfer.rs:31`, `InputSelection::Auto` path (`platform_addresses/mod.rs:30`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/wallet_tasks.rs:234` (`step_transfer_credits`).
- **Preconditions**: bank-funded test wallet.
- **Scenario**:
  1. Bank-fund `addr_1` with `60_000_000`.
  2. Transfer `5_000_000` to a fresh `addr_2`.
  3. Sync `addr_1` post-transfer.
- **Assertions**:
  - `balances[addr_2] == 5_000_000`
  - `balances[addr_1] == 60_000_000 - 5_000_000 - fee` (≈ `54_999_…`)
  - `fee > 0`
  - Inputs were drawn only from `addr_1` (assert `balances` over a third address `addr_3` not derived — sanity).
- **Negative variants**:
  - Same scenario but with `InputSelection::Explicit({addr_2: …})` where `addr_2` has zero balance → typed insufficient-funds error.
- **Harness extensions required**: none for the happy path; the negative variant needs a thin `TestWallet::transfer_with_inputs` helper (~10 LoC).
- **Estimated complexity**: S
- **Rationale**: Confirms `Σ inputs == Σ outputs + fee` invariant — the property recently fixed in commits `aaf8be74ee` and `9ea9e7033c`. Without this case those regressions would be invisible.

#### PA-004 — Sweep-back: drain test wallet, observe bank credit
- **Priority**: P0
- **Wallet feature exercised**: `wallet/platform_addresses/transfer.rs:31` invoked from `framework/cleanup.rs::teardown_one`.
- **DET parallel**: implicit in DET — every test ends with bank refund. We surface it as a first-class case.
- **Preconditions**: bank-funded; test wallet seeded; baseline bank balance recorded before fund.
- **Scenario**:
  1. Record `bank_pre = bank.total_credits()`.
  2. Bank-fund `addr_1` with `40_000_000`.
  3. Wait for test wallet to observe.
  4. Call `setup_guard.teardown()` (sweep path).
  5. Wait for bank balance to reflect the inbound sweep.
- **Assertions**:
  - `bank_post >= bank_pre - 40_000_000 - fund_fee - sweep_fee`
  - `bank_post <= bank_pre - 40_000_000 - fund_fee + 40_000_000` (no double-credit)
  - The test wallet's registry entry is removed (`registry.get(wallet_id).is_none()`).
  - Total round-trip fee ≤ `1_000_000` credits (regression bound on combined cost).
- **Negative variants**:
  - Test wallet balance below `SWEEP_DUST_THRESHOLD` (5M) → sweep is skipped, wallet still de-registered with `Skipped` status (assert `cleanup` log + final registry state).
- **Harness extensions required**: needs a `Bank::total_credits` accessor exposed to tests (already implemented at `framework/bank.rs:225`); needs `TestRegistry::get_status(wallet_id)` (~10 LoC if not already present).
- **Estimated complexity**: S
- **Rationale**: Validates the cleanup invariant the README promises in §"Panic-safe cleanup". Without this, a regression in `cleanup.rs` would silently leak credits across runs — bank slowly drains, eventually trips under-funded panic, no test ever names the cause.

#### PA-003 — Fee scaling: one-output vs. five-output transfers
- **Priority**: P1
- **Wallet feature exercised**: `wallet/platform_addresses/transfer.rs:31`, fee-strategy `AddressFundsFeeStrategyStep::DeductFromInput(0)` from `wallet_factory.rs:210`.
- **DET parallel**: none directly — DET tests `tc_014` lifecycle but not fee scaling explicitly.
- **Preconditions**: bank-funded test wallet with ≥ `200_000_000`.
- **Scenario**:
  1. Bank-fund `addr_1` with `100_000_000`.
  2. Transfer `5_000_000` to `addr_2` (single output). Record `fee_1`.
  3. Bank-fund `addr_3` with `100_000_000`.
  4. Transfer `1_000_000` each to `addr_4..addr_8` (five outputs). Record `fee_5`.
- **Assertions**:
  - `fee_1 > 0`, `fee_5 > 0`
  - `fee_5 > fee_1` (more outputs ⇒ larger byte size ⇒ larger fee)
  - `fee_5 < 5 * fee_1` (sub-linear — outputs share inputs/headers)
  - Documented bound: `fee_5 - fee_1 < 1_000_000` (regression guard; tighten once empirical numbers are known).
- **Negative variants**: none — this is a property test.
- **Harness extensions required**: none.
- **Estimated complexity**: M (two transfers + bookkeeping ≈ 100-150 LoC)
- **Rationale**: Encodes fee scaling as an asserted property. CodeRabbit fee-headroom regressions (commit `687b1f86cd`) and future fee-formula tweaks become test failures rather than silent behaviour shifts.

#### PA-005 — Address rotation: gap-limit + observed-used cursor
- **Priority**: P1
- **Wallet feature exercised**: `wallet/platform_addresses/wallet.rs:180` (`next_unused_receive_address`); `provider::PerAccountPlatformAddressState`.
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/wallet_tasks.rs:19` (`tc_012_generate_receive_address`).
- **Preconditions**: bank-funded test wallet; `DEFAULT_GAP_LIMIT = 20`.
- **Scenario**:
  1. Call `next_unused_address()` three times back-to-back BEFORE any sync. All three must return the same address (cursor is parked until first observed-used).
  2. Bank-fund the address; wait for balance.
  3. Call `next_unused_address()` once more. Must return a different address.
  4. Repeat steps 2-3 fifteen times (total 16 distinct addresses), funding each.
  5. After 16 used addresses, derive the 17th via `next_unused_address()` — still inside gap window.
- **Assertions**:
  - First three calls return the same `PlatformAddress` (cursor not advanced).
  - Each post-funding call advances the cursor: 16 distinct addresses observed.
  - The 17th address is derivable (within `DEFAULT_GAP_LIMIT`).
  - `signer.cached_key_count() >= 17`.
- **Negative variants**:
  - Derive 21+ unused addresses without funding — expect either gap-limit growth or a typed "gap exceeded" error (whichever the wallet contract defines; this case will surface that contract).
- **Harness extensions required**: `signer.cached_key_count()` is already public (`signer.rs:144`); no other harness change.
- **Estimated complexity**: M (bookkeeping ≈ 200 LoC; 16 funding round-trips means a long-running test — gate it under a slow-tests feature or accept ~3 min runtime).
- **Rationale**: The fix in commit `60f7850ab0` ("sort auto-select candidates by balance descending") is one of several invariants in the address provider that needs a regression test. PA-005 also documents the "cursor advances on observed-used" property that bit Wave 8 in PR #3549 (see `cases/transfer.rs:91-97`).

#### PA-006 — Replay safety: same outputs, second submission rejected
- **Priority**: P1
- **Wallet feature exercised**: nonce handling inside `PutPlatformAddresses::put_with_address_funding_fetching_nonces` (re-broadcast).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/wallet_tasks.rs:234` indirectly tests nonces.
- **Preconditions**: bank-funded test wallet.
- **Scenario**:
  1. Fund `addr_1` with `50_000_000`.
  2. Capture the underlying state-transition bytes (requires exposing the changeset's `serialized_transition` — see harness extension below).
  3. Transfer `10_000_000` to `addr_2` (succeeds).
  4. Submit the captured bytes a second time via `sdk.broadcast_state_transition` directly.
- **Assertions**:
  - Second submission returns a "stale nonce" / "already exists" SDK error (assert error class).
  - Wallet's view of `addr_1` and `addr_2` is unchanged after the failed re-submit.
- **Negative variants**: none — this case IS the negative variant of PA-001.
- **Harness extensions required**: a `TestWallet::transfer_capturing_st_bytes` helper that returns the encoded ST alongside the change-set. ~30 LoC, plumbs through the SDK's `put_*` builder rather than `transfer()`.
- **Estimated complexity**: M (single-file, harness touch)
- **Rationale**: Closes a quiet but high-blast-radius regression class — nonce handling. If the SDK ever stops bumping nonces correctly, every wallet's "spam-click" UX breaks. PA-006 surfaces it deterministically.

#### PA-007 — Sync watermark idempotency
- **Priority**: P1
- **Wallet feature exercised**: `wallet/platform_addresses/sync.rs:24` (`sync_balances`); `wallet/platform_addresses/wallet.rs:153` (`restore_sync_state`).
- **DET parallel**: implicit in DET's wallet-task lifecycle.
- **Preconditions**: bank-funded test wallet.
- **Scenario**:
  1. Bank-fund `addr_1` with `30_000_000`; wait.
  2. Call `sync_balances` three times in a row.
  3. Capture the post-sync watermark via `wallet.platform().<provider>.last_known_recent_block` (read through public state guard).
- **Assertions**:
  - All three syncs succeed.
  - Watermark is monotonic non-decreasing across calls.
  - Cached balances are byte-equal across calls (no spurious mutation on re-sync).
- **Negative variants**:
  - Disconnect from DAPI (config override to a bogus URL) and call `sync_balances` → typed network error; cached balances unchanged.
- **Harness extensions required**: an accessor on `TestWallet` to read the platform-address provider's sync state (or expose it through the existing `platform_wallet()` borrow + a public watermark getter on the provider — already on the API, just needs threading).
- **Estimated complexity**: M
- **Rationale**: Re-sync idempotency is silently load-bearing — UI clients call `sync_balances` on every refresh tick. A regression that double-counts on re-sync would be visually obvious in apps and silent in unit tests; PA-007 makes it explicit.

#### PA-008 — Concurrent funding from bank: serialised by FUNDING_MUTEX
- **Priority**: P1
- **Wallet feature exercised**: `framework/bank.rs::fund_address` and its `FUNDING_MUTEX` invariant.
- **DET parallel**: none — DET's bank model differs.
- **Preconditions**: bank-funded test wallet.
- **Scenario**:
  1. Derive `addr_1`, `addr_2`, `addr_3`.
  2. Spawn three concurrent `bank.fund_address` tasks (each `10_000_000`).
  3. Await all three.
  4. Sync.
- **Assertions**:
  - All three addresses end with the funded amount (no nonce collisions, no lost funding).
  - Total bank decrease == `30_000_000 + 3 * fund_fee`.
  - No panic in `FUNDING_MUTEX` path.
- **Negative variants**: none — this case validates concurrency safety as a property.
- **Harness extensions required**: none.
- **Estimated complexity**: S
- **Rationale**: Encodes the FUNDING_MUTEX guarantee documented in `framework/bank.rs:39`. Without it, a future refactor that drops the mutex (or misuses it) would corrupt nonces and only surface intermittently.

#### PA-002b — Zero-change exact-equality (`Σ outputs + fee == input balance`)
- **Priority**: P1
- **Wallet feature exercised**: `wallet/platform_addresses/transfer.rs:31`; change-output suppression at the `Σ inputs == Σ outputs` boundary recently fixed in `aaf8be74ee` and `9ea9e7033c`.
- **DET parallel**: none — this is a regression-pinning case for our own commits.
- **Preconditions**: bank-funded test wallet.
- **Scenario**:
  1. Bank-fund `addr_1` with `60_000_000` and let it settle. Record `bal_1 = addr_1` balance.
  2. Build a one-output transfer `{addr_2: bal_1 - estimated_fee}` where `estimated_fee` is derived from the wallet's fee preview (or a calibrated PA-003 measurement).
  3. Tighten the output by 1 credit at a time until `Σ outputs + actual_fee == bal_1` exactly. Submit.
- **Assertions**:
  - Transfer succeeds (no spurious "below dust" or change-output validation error).
  - The on-wire state-transition contains exactly **one** output (the destination); no change output is materialised.
  - `addr_1` post-balance == `0` exactly. Not `1`, not `dust_threshold`, not `None`.
  - `balances[addr_2] == bal_1 - actual_fee` exactly.
- **Negative variants**: none (this case IS the boundary).
- **Harness extensions required**: a `TestWallet::estimate_transfer_fee(&outputs)` helper, or fall back to PA-003's empirical fee constants.
- **Estimated complexity**: S
- **Rationale**: Pins the `Σ inputs == Σ outputs + fee` invariant the wallet just shipped regressions on. Without an exact-equality boundary case, that bug-class re-emerges silently the next time the change-output predicate is touched.

#### PA-010 — Bank starvation: typed `BankUnderfunded` error
- **Priority**: P1
- **Wallet feature exercised**: `framework/bank.rs::fund_address` precondition checks.
- **DET parallel**: none — operator-actionable harness contract.
- **Preconditions**: bank deliberately underfunded for the test (e.g. configure a fresh test bank with `5_000_000` total credits).
- **Scenario**:
  1. Configure the harness so `bank.total_credits()` is below the test's requested fund amount.
  2. Call `bank.fund_address(addr_1, 30_000_000)`.
- **Assertions**:
  - `bank.fund_address` returns a typed `BankError::Underfunded { available, requested }` (or the equivalent named variant — pin whatever the code calls it). No panic, no generic `anyhow!` shape.
  - Error message names the bank wallet id, the available balance, and the requested amount, so an operator can act without code-diving.
  - The bank's funding mutex is released cleanly (a follow-up successful call after re-funding the bank works).
  - Test wallet registry contains no half-created entry from the failed fund.
- **Negative variants**: none.
- **Harness extensions required**: a typed error variant on `framework/bank.rs` (most likely already present; confirm name); a way to construct an underfunded bank for the test (a `Bank::with_balance_for_test(...)` constructor or a fresh bank wallet pre-drained).
- **Estimated complexity**: S
- **Rationale**: Bank starvation is the single most common "weird CI failure" mode for this suite, and the failure mode shouldn't be a panic from inside `fund_address`. PA-010 makes the operator-actionable error part of the contract.

#### PA-001b — Transfer with `output_change_address: None` vs `Some(addr)`
- **Priority**: P2
- **Wallet feature exercised**: `wallet/platform_addresses/transfer.rs:31`; the `output_change_address: Option<PlatformAddress>` argument routes change either to an auto-derived address or to an explicit one.
- **DET parallel**: none — exercises an Option-branch the existing PA cases never split.
- **Preconditions**: bank-funded test wallet.
- **Scenario**:
  1. Bank-fund `addr_1` with `60_000_000`.
  2. Run transfer `{addr_2: 5_000_000}` with `output_change_address: None`. Record the address that ended up holding the change.
  3. Bank-fund a fresh `addr_3` with `60_000_000`.
  4. Derive an explicit `change_addr` separately from `addr_3` (and from any output address).
  5. Run transfer `{addr_4: 5_000_000}` from `addr_3` with `output_change_address: Some(change_addr)`.
- **Assertions**:
  - `None` branch: change lands on the wallet-internal documented "auto-derive change" address (likely the next unused receive address); record exactly which one and pin the rule in the assertion.
  - `Some(change_addr)` branch: change balance shows up on `change_addr` exactly, and not on the source or any other address.
  - In both branches `Σ inputs == Σ outputs + fee` holds.
- **Negative variants**:
  - `output_change_address: Some(addr_with_existing_balance)` → assert merge-or-reject contract (whichever the wallet defines).
- **Harness extensions required**: none.
- **Estimated complexity**: S
- **Rationale**: The `Option<PlatformAddress>` argument has no asserted contract today — `None` could drift into "change is silently lost" without a single test failing.

#### PA-001c — Zero-credit single-output transfer
- **Priority**: P2
- **Wallet feature exercised**: `wallet/platform_addresses/transfer.rs:31` boundary at output-amount zero.
- **DET parallel**: none.
- **Preconditions**: bank-funded test wallet.
- **Scenario**:
  1. Bank-fund `addr_1` with `30_000_000`.
  2. Call `transfer({addr_2: 0})` from `addr_1`.
- **Assertions**: pin one of the two contracts (whichever the wallet implements):
  - **(a) Reject**: a typed validation error of "amount must be positive" shape; no state-transition broadcast; balances unchanged.
  - **(b) Accept as fee-only**: transfer broadcasts; `balances[addr_2] == 0`; `addr_1` decreased by `fee` only.
- **Negative variants**: none — this case IS the zero-amount boundary.
- **Harness extensions required**: none.
- **Estimated complexity**: S
- **Rationale**: Zero-amount transfers are a classic boundary. The wallet's contract here is currently undocumented; whichever it is, an explicit case pins it.

#### PA-004b — Sweep dust threshold boundary triplet
- **Priority**: P2
- **Wallet feature exercised**: `framework/cleanup.rs` sweep gate at `SWEEP_DUST_THRESHOLD` (5_000_000 credits).
- **DET parallel**: none.
- **Preconditions**: bank-funded test wallet × 3 (one per boundary).
- **Scenario**: run three sub-cases independently, with wallet balance configured exactly:
  1. Balance == `SWEEP_DUST_THRESHOLD - 1` (i.e. `4_999_999`). Call cleanup. Assert sweep is **skipped** (registry status `Skipped`, no broadcast).
  2. Balance == `SWEEP_DUST_THRESHOLD` (i.e. `5_000_000`). Call cleanup. Assert sweep is **attempted** (broadcast emitted, bank credit observed minus fees).
  3. Balance == `SWEEP_DUST_THRESHOLD + 1` (i.e. `5_000_001`). Call cleanup. Assert sweep is **attempted**.
- **Assertions**: each sub-case asserts the registry status string and whether a state-transition was broadcast. The boundary at `==` must distinguish from `< threshold`.
- **Negative variants**: none.
- **Harness extensions required**: a way to configure a test wallet to hold an exact balance after fund + fee accounting (likely fund a slightly larger amount, then transfer the excess to a sink). May require the `TestWallet::transfer_with_inputs` helper (Wave F).
- **Estimated complexity**: M
- **Rationale**: The dust threshold is one of the few hard numeric gates in the cleanup path. Off-by-one at this boundary is the canonical bug class.

#### PA-004c — Sweep with exactly zero balance
- **Priority**: P2
- **Wallet feature exercised**: `framework/cleanup.rs` sweep path with empty inputs.
- **DET parallel**: none.
- **Preconditions**: bank-funded harness; test wallet seeded but never funded (or fully drained before cleanup).
- **Scenario**:
  1. Create a fresh `TestWallet`. Do not fund it.
  2. Call `setup_guard.teardown()`.
- **Assertions**:
  - Cleanup returns `Ok(())`.
  - Registry status for the wallet is `Skipped` (no broadcast attempted).
  - No DAPI broadcast call is made (assert via a counter on the test SDK harness, or by absence of nonce consumption on the bank).
- **Negative variants**: none.
- **Harness extensions required**: a "did we broadcast?" hook on the harness SDK, or a registry status accessor.
- **Estimated complexity**: S
- **Rationale**: A no-op cleanup must not throw. Without this case a refactor that moves the empty-input check could regress to `Err(InsufficientFunds)` and the test suite would never notice.

#### PA-005b — `DEFAULT_GAP_LIMIT` triplet (19 / 20 / 21 unused)
- **Priority**: P2
- **Wallet feature exercised**: `wallet/platform_addresses/wallet.rs:180` gap-limit enforcement at `DEFAULT_GAP_LIMIT = 20`.
- **DET parallel**: none direct; PA-005 covers cursor rotation but not the gap-limit boundary.
- **Preconditions**: bank-funded test wallet.
- **Scenario**: three sub-cases run on separate `TestWallet` instances:
  1. Derive **19** unused addresses (no funding). Then derive a 20th. Assert all 20 are returned without error or gap-limit growth event.
  2. Derive **20** unused addresses (no funding). Then derive a 21st. Pin the contract: either the wallet returns a typed `GapLimitExceeded` error, or it grows the limit (assert a `GapLimitGrown` event, or whatever the wallet exposes).
  3. Derive **21** unused addresses by request, asserting the same contract as (2).
- **Assertions**: each sub-case nails the wallet's contract at the `DEFAULT_GAP_LIMIT` boundary.
- **Negative variants**: none — this case is the boundary.
- **Harness extensions required**: a way to derive without funding (already supported via `next_unused_address` repeatedly; confirm cursor doesn't auto-park).
- **Estimated complexity**: M
- **Rationale**: PA-005's "21+ unused addresses" line is exploratory; PA-005b promotes it to an asserted boundary on each side of `DEFAULT_GAP_LIMIT`.

#### PA-006b — Two concurrent broadcasts of identical ST bytes
- **Priority**: P2
- **Wallet feature exercised**: nonce / replay-protection at the SDK / DAPI boundary.
- **DET parallel**: none.
- **Preconditions**: bank-funded test wallet; PA-006's `transfer_capturing_st_bytes` helper.
- **Scenario**:
  1. Fund `addr_1` and capture the encoded ST bytes for a transfer (do not broadcast yet).
  2. Spawn two concurrent `tokio::spawn` tasks each calling `sdk.broadcast_state_transition(captured_bytes)`.
  3. Await both.
- **Assertions**:
  - Exactly one of the two futures returns success; the other returns the documented stale-nonce / already-exists / duplicate-broadcast error class.
  - Final wallet state matches a single applied transfer (no double-debit).
- **Negative variants**: none.
- **Harness extensions required**: PA-006's `transfer_capturing_st_bytes`.
- **Estimated complexity**: M
- **Rationale**: PA-006 covers sequential replay; the race-condition variant is materially different code path inside the SDK / DAPI mempool.

#### PA-007b — Two concurrent `sync_balances` on one wallet
- **Priority**: P2
- **Wallet feature exercised**: `wallet/platform_addresses/sync.rs:24` reentrancy / internal locking.
- **DET parallel**: none.
- **Preconditions**: bank-funded test wallet.
- **Scenario**:
  1. Fund `addr_1` with `30_000_000`; wait for visibility.
  2. Spawn two concurrent `sync_balances()` futures on the same `TestWallet` handle.
  3. Await both.
- **Assertions**:
  - Both futures return `Ok(())`.
  - Post-state cached balance equals on-chain truth (not 2× — no double-counting).
  - Sync watermark advanced exactly once net (no spurious double-bump).
- **Negative variants**: none.
- **Harness extensions required**: same accessor PA-007 already requires.
- **Estimated complexity**: M
- **Rationale**: PA-007 is sequential; double-counting under concurrent re-sync is a UI-tier hazard worth pinning.

#### PA-008b — Two `TestWallet`s × three concurrent funders each
- **Priority**: P2
- **Wallet feature exercised**: `framework/bank.rs::fund_address` cross-wallet contention.
- **DET parallel**: none.
- **Preconditions**: bank with `≥ 70_000_000 + 6 * fund_fee` credits.
- **Scenario**:
  1. Spin up two independent `TestWallet` instances, A and B.
  2. Derive `a1, a2, a3` on A and `b1, b2, b3` on B.
  3. Spawn six concurrent `bank.fund_address` calls (three on A's addresses, three on B's, each `10_000_000`).
  4. Await all six.
- **Assertions**:
  - All six addresses end with the funded amount (no nonce collision across wallet boundaries).
  - Total bank decrease == `60_000_000 + 6 * fund_fee`.
  - No panic, no missing balances on any sub-set after sync.
- **Negative variants**: none.
- **Harness extensions required**: helper to instantiate two independent `TestWallet`s in one harness setup.
- **Estimated complexity**: M
- **Rationale**: PA-008 keeps contention inside one `TestWallet`; PA-008b proves the bank's serialisation works under cross-wallet contention too — the realistic CI shape.

#### PA-008c — Observable serialisation of `FUNDING_MUTEX`
- **Priority**: P2
- **Wallet feature exercised**: `framework/bank.rs::FUNDING_MUTEX` invariant.
- **DET parallel**: none.
- **Preconditions**: bank-funded test wallet; instrumentation hook on `FUNDING_MUTEX` (entry/exit timestamps or per-call sequence number).
- **Scenario**:
  1. Spawn three concurrent `bank.fund_address` tasks.
  2. Each task records its mutex-entry timestamp and mutex-exit timestamp via a test-only instrumentation hook.
  3. Await all three.
- **Assertions**:
  - The three intervals `[entry_i, exit_i]` are pairwise non-overlapping (proves serialisation, not just correctness).
  - Equivalently / additionally: the bank's funding-tx nonces are strictly monotonic in the same order as the mutex entries.
- **Negative variants**: none.
- **Harness extensions required**: an instrumentation hook on `framework/bank.rs` (test-only `cfg(test)` accessor for the mutex's last-entry sequence, or a `parking_lot::Mutex` instrumentation wrapper).
- **Estimated complexity**: M
- **Rationale**: PA-008 tests "all three calls succeed" — a future refactor that drops the mutex but happens to win the race in CI would still pass. PA-008c asserts the *mechanism* observably, so a silent removal of the mutex fails the test deterministically.

#### PA-009 — `min_input_amount` boundary triplet for cleanup
- **Priority**: P2
- **Wallet feature exercised**: `framework/cleanup.rs::min_input_amount`, sourced from `platform_version.dpp.state_transitions.address_funds.min_input_amount`.
- **DET parallel**: none.
- **Preconditions**: bank-funded harness; test wallet × 3, each with a precisely tuned balance.
- **Scenario**: read `min` = `platform_version.dpp.state_transitions.address_funds.min_input_amount`. Run three sub-cases:
  1. Balance == `min - 1`. Call cleanup. Assert `Skipped` (cleanup must not attempt sweep).
  2. Balance == `min`. Call cleanup. Assert sweep is attempted (broadcast emitted; or fails with the documented "fee pushes below threshold" typed error).
  3. Balance == `min + 1`. Call cleanup. Assert sweep is attempted and succeeds.
- **Assertions**: each sub-case pins the cleanup status (`Skipped` vs attempted) and the typed error if the attempt fails.
- **Negative variants**: none.
- **Harness extensions required**: PA-004b's exact-balance setup helper; a way to read `min_input_amount` from the active `PlatformVersion` inside the test.
- **Estimated complexity**: M
- **Rationale**: `min_input_amount` is currently entirely uncovered. A protocol-version bump that changes the value would silently shift cleanup behaviour, with no failing test to flag the shift.

#### PA-011 — Workdir slot exhaustion at `MAX_SLOTS + 1`
- **Priority**: P2
- **Wallet feature exercised**: `framework/workdir.rs` `flock`-based slot allocation; `MAX_SLOTS = 10`.
- **DET parallel**: none — operator-actionable harness contract.
- **Preconditions**: a clean workdir base path with no held slots.
- **Scenario**:
  1. Spawn `MAX_SLOTS` sub-processes (or `MAX_SLOTS` concurrent harness contexts within one process) that each acquire and hold a workdir slot.
  2. Spawn one additional (i.e. the 11th) harness context attempting to acquire a slot.
- **Assertions**:
  - The first `MAX_SLOTS` acquisitions succeed and land on distinct slot indices.
  - The 11th returns a typed `WorkdirError::NoAvailableSlots { tried, base_path }` (pin the variant name) within a bounded time — no silent infinite wait.
  - Cleanup releases all slots; a subsequent acquisition succeeds.
- **Negative variants**: none.
- **Harness extensions required**: a typed error variant on `framework/workdir.rs` (likely already there; confirm name); a way to spawn sub-processes for the test, or simulate slot holders within one process via held `flock` guards.
- **Estimated complexity**: M
- **Rationale**: Slot exhaustion is the second most common "weird CI failure" mode after bank starvation. PA-011 makes its failure mode explicit.

#### PA-012 — `sync_balances` racing with `transfer`
- **Priority**: P2
- **Wallet feature exercised**: internal locking between `wallet/platform_addresses/sync.rs:24` and `wallet/platform_addresses/transfer.rs:31`.
- **DET parallel**: none.
- **Preconditions**: bank-funded test wallet.
- **Scenario**:
  1. Bank-fund `addr_1` with `40_000_000`; wait.
  2. Spawn two concurrent tasks: `wallet.sync_balances()` and `wallet.transfer({addr_2: 5_000_000})`.
  3. Await both.
- **Assertions**:
  - Both return `Ok(...)`.
  - Final state is consistent with sequential execution: `balances[addr_2] == 5_000_000`, `balances[addr_1] == 40_000_000 - 5_000_000 - fee`. No "fee charged twice", no "in-flight transfer double-counted".
  - The transfer's fee was computed against a non-stale balance view (i.e. no `InsufficientFunds` because `sync_balances` clobbered the cache mid-build).
- **Negative variants**: none.
- **Harness extensions required**: none beyond what PA-002 / PA-007 already need.
- **Estimated complexity**: M
- **Rationale**: Mobile clients call `sync_balances` aggressively while the user is typing into a transfer form. A regression where these two paths race silently produces wrong fees or stale balances; PA-012 pins the contract.

#### PA-013 — Broadcast retry under transient DAPI 5xx
- **Priority**: P2
- **Wallet feature exercised**: SDK retry policy on `broadcast_state_transition` under transient HTTP 5xx; downstream wallet state-finalisation on partial success.
- **DET parallel**: none direct; PA-007's negative variant covers a permanently-bogus URL only.
- **Preconditions**: a test-only DAPI proxy (or a `httpmock`-based DAPI stub) that returns `503 Service Unavailable` on the first call to `/broadcastStateTransition` and succeeds thereafter.
- **Scenario**:
  1. Bank-fund `addr_1`.
  2. Configure the harness SDK to point at the proxy.
  3. Issue a transfer.
- **Assertions**:
  - Wallet returns `Ok(...)` despite the transient 5xx (assuming policy is to retry; if the policy is "fail fast and surface to caller", invert the assertion and document that contract).
  - Final on-chain state shows the transfer applied exactly once (proxy's request log shows two POSTs — one 503, one 200; chain shows one ST).
  - On the proof-fetch failure variant (DAPI succeeds on broadcast, 5xx on proof fetch): wallet either retries proof fetch, or returns a `BroadcastedAwaitingProof` typed result (whichever the contract defines).
- **Negative variants**:
  - DAPI returns 5xx persistently → typed `NetworkError` after exhausted retries; cached wallet state unchanged.
- **Harness extensions required**: a controllable test DAPI proxy (Wave F-adjacent). This is non-trivial; mark as "blocked on test-DAPI-proxy infra" if unavailable.
- **Estimated complexity**: M
- **Rationale**: Transient 5xx is the most common production failure mode for thin-client SDKs. Without a deterministic test, retry policy drifts between "broken" and "infinite loop" and nobody notices until users complain.

#### PA-014 — Multi-output at protocol-max output count
- **Priority**: P2
- **Wallet feature exercised**: `wallet/platform_addresses/transfer.rs:31` at the protocol max-output boundary; payload-size limits in DPP / Drive.
- **DET parallel**: none.
- **Preconditions**: bank-funded test wallet with sufficient credits to fund N outputs (where N is the protocol max for `address_funds` outputs).
- **Scenario**:
  1. Discover the protocol-max output count from `platform_version.dpp.state_transitions.address_funds.max_outputs` (or the equivalent constant).
  2. Bank-fund `addr_1` with enough credits to cover N outputs of `100_000` each plus fees.
  3. Construct a transfer with exactly `max_outputs` destinations; submit. Record the result.
  4. Construct a transfer with `max_outputs + 1` destinations; submit.
- **Assertions**:
  - At `max_outputs`: transfer succeeds; all N destinations reach the expected balance.
  - At `max_outputs + 1`: wallet returns a typed `PayloadTooLarge` / `TooManyOutputs` validation error before broadcast (or, if the wallet attempts and DAPI rejects, the SDK error class is mapped to a typed wallet error). Pin which side enforces.
- **Negative variants**: none.
- **Harness extensions required**: ability to read `max_outputs` from the active platform version; a pool of `max_outputs + 1` distinct destination addresses (likely already available via `next_unused_address` on a fresh wallet).
- **Estimated complexity**: M
- **Rationale**: The wallet's only multi-output coverage today is "5 outputs". The actual upper limit is unmeasured; a protocol-version bump that changes `max_outputs` would silently shift behaviour, with regressions surfacing only in production state-transitions that are mysteriously rejected.

### Identity (ID)

#### ID-001 — Register identity funded from platform addresses
- **Priority**: P0
- **Wallet feature exercised**: `wallet/identity/network/register_from_addresses.rs:65` (`IdentityWallet::register_from_addresses`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/identity_create.rs:13` (`test_create_identity`) — DET uses asset-lock; we use the address-funded variant explicitly.
- **Preconditions**: bank-funded test wallet; identity-signer harness extension landed.
- **Scenario**:
  1. Derive `addr_1`, bank-fund with `60_000_000`, wait for balance.
  2. Build a placeholder `Identity` with one `MASTER` ECDSA key and one `HIGH` ECDSA key derived via DIP-9 (identity index `0`).
  3. Call `IdentityWallet::register_from_addresses(identity, {addr_1: 50_000_000}, output: None, identity_index: 0, identity_signer, address_signer, settings: None)`.
  4. Wait for the identity to appear on-chain by `sdk.fetch::<Identity>(identity.id())`.
- **Assertions**:
  - Returned `Identity::id()` is non-zero and equals the on-chain fetched identity.
  - On-chain identity public-keys count == 2.
  - Identity balance == `50_000_000 - identity_create_fee` (`identity_create_fee > 0`).
  - `addr_1` residual balance == `60_000_000 - 50_000_000 - tx_fee`.
  - `IdentityManager::known_identities()` lists exactly this identity.
- **Negative variants**:
  - `inputs` is empty → wallet returns `PlatformWalletError::InvalidIdentityData("At least one input address is required")` (already enforced at `register_from_addresses.rs:78`; assert exact message stability).
  - Insufficient funds in input → SDK error class.
  - Placeholder `Identity` with zero keys → identity-create transition rejection.
- **Harness extensions required**:
  - `Signer<IdentityPublicKey>` impl — Wave A (see §4).
  - `TestWallet::register_identity_from_addresses(funding: Credits) -> Identity` helper that wraps the placeholder build + call.
  - `wait_for_identity_balance(identity_id, expected, timeout)` helper.
- **Estimated complexity**: L (multi-file harness extension)
- **Rationale**: Highest-leverage Identity test. The address-funded path is currently exercised by no test anywhere in the workspace — FFI binds the asset-lock variant only. ID-001 is the gateway: every other Identity case (ID-002+) inherits the placeholder-Identity setup it builds.

#### ID-002 — Top-up identity from platform addresses
- **Priority**: P0
- **Wallet feature exercised**: `wallet/identity/network/top_up_from_addresses.rs:37`.
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/identity_tasks.rs:63` (`step_top_up_from_platform_addresses`).
- **Preconditions**: ID-001 setup helper; identity registered with starting balance.
- **Scenario**:
  1. Register identity per ID-001 (helper).
  2. Capture `pre_balance = identity.balance()` (post-registration).
  3. Bank-fund `addr_2` (a freshly derived address) with `30_000_000`.
  4. Call `top_up_from_addresses({addr_2: 25_000_000}, identity_id, …)`.
  5. Sync identity.
- **Assertions**:
  - `post_balance == pre_balance + 25_000_000 - top_up_fee`
  - `top_up_fee > 0`
  - `addr_2` residual == `30_000_000 - 25_000_000 - tx_fee`.
- **Negative variants**:
  - Top-up to non-existent identity id → typed error.
  - Top-up with empty `inputs` map → typed validation error.
- **Harness extensions required**: same as ID-001 — Wave A.
- **Estimated complexity**: M
- **Rationale**: Validates the partner of ID-001. Together they cover the entire address-funded identity lifecycle entry surface.

#### ID-003 — Identity-to-identity credit transfer
- **Priority**: P0
- **Wallet feature exercised**: `wallet/identity/network/transfer.rs:74` (`transfer_credits_with_external_signer`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/identity_tasks.rs:238` (`step_transfer_credits`).
- **Preconditions**: ID-001 helper × 2 (two registered identities, both funded from same test wallet).
- **Scenario**:
  1. Register `identity_a` and `identity_b` (sequential ID-001 invocations on different addresses).
  2. Capture pre-balances.
  3. Transfer `10_000_000` credits from `identity_a` to `identity_b`.
- **Assertions**:
  - `post_a == pre_a - 10_000_000 - transfer_fee`, `transfer_fee > 0`
  - `post_b == pre_b + 10_000_000`
  - `IdentityManager` reflects both new balances after sync.
- **Negative variants**:
  - Transfer amount exceeds sender balance → typed error.
  - Transfer to self (`identity_a -> identity_a`) → typed error.
- **Harness extensions required**: Wave A only (everything inherits ID-001).
- **Estimated complexity**: M
- **Rationale**: Confirms identity-balance bookkeeping in `ManagedIdentity` is bidirectional and idempotent. Pairs with ID-002 to cover the symmetric "credit increase" + "credit decrease" code paths.

#### ID-004 — Identity update: add and disable a key
- **Priority**: P1
- **Wallet feature exercised**: `wallet/identity/network/update.rs:89` (`update_identity_with_external_signer`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/identity_tasks.rs:188` (`step_add_key`) and `tc_020_identity_mutation_lifecycle`.
- **Preconditions**: ID-001 helper.
- **Scenario**:
  1. Register identity with MASTER + HIGH keys (purpose AUTHENTICATION).
  2. Build a new HIGH ECDSA key (purpose AUTHENTICATION) — derive via identity-key derivation Wave A helper.
  3. Issue an `IdentityUpdateTransition` adding the new key.
  4. Issue a second update disabling the original HIGH key.
  5. Refresh identity from chain.
- **Assertions**:
  - After step 3: identity has 3 keys, the new key is `is_disabled == false`.
  - After step 4: original HIGH key has `disabled_at != None`; new HIGH key still active.
  - MASTER key is untouched.
- **Negative variants**:
  - Disable last MASTER key → typed error (CRITICAL/MASTER class invariant).
  - Add key signed by non-MASTER → typed error.
- **Harness extensions required**: Wave A; plus a `derive_identity_key(identity_index, key_index, purpose, security_level)` test helper.
- **Estimated complexity**: L
- **Rationale**: Identity-update pathways have multiple silent failure modes (key-class restrictions, MASTER signing requirements). Recent commit `844eef74e8` ("token transitions require a CRITICAL signing key") shows this surface is actively changing — coverage prevents future regressions.

#### ID-005 — Transfer credits from identity to platform addresses
- **Priority**: P1
- **Wallet feature exercised**: `wallet/identity/network/transfer_to_addresses.rs:66`.
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/identity_tasks.rs:291` (`step_transfer_to_addresses`).
- **Preconditions**: ID-001 helper.
- **Scenario**:
  1. Register identity with `≥ 60_000_000` credits (ID-001 with larger funding).
  2. Derive `dest_addr` on the test wallet.
  3. Call `transfer_credits_to_addresses_with_external_signer(identity_id, {dest_addr: 20_000_000}, signer, settings: None)`.
  4. Sync test wallet balances.
- **Assertions**:
  - `balances[dest_addr] == 20_000_000`
  - Identity balance decreased by `20_000_000 + transfer_fee`.
  - Returned `Credits` value equals on-chain transferred amount (the wallet returns the post-fee `Credits` — assert matches `20_000_000`).
- **Negative variants**:
  - Transfer to malformed `PlatformAddress` (P2SH that the harness cannot sign for is fine here — it's the destination, not the source) → SDK accepts it; assert balance shows up.
  - Insufficient identity balance → typed error.
- **Harness extensions required**: Wave A only.
- **Estimated complexity**: M
- **Rationale**: Closes the ID surface — combined with ID-002 (addresses → identity) and ID-005 (identity → addresses), this exercises the full money-flow loop that wallets actually need to demo.

#### ID-006 — Refresh and load identity by index
- **Priority**: P1
- **Wallet feature exercised**: `wallet/identity/network/loading.rs:28` (`load_identity_by_index`); `loading.rs:162` (`refresh_identity`); `discovery.rs:79` (`discover`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/identity_tasks.rs:350` (`tc_025_refresh_identity`); `identity_tasks.rs:420` (`tc_027_load_identity`); `identity_tasks.rs:585` (`tc_031_incremental_address_discovery`).
- **Preconditions**: ID-001 helper.
- **Scenario**:
  1. Register identity via ID-001 at `identity_index = 0`.
  2. Drop the test-wallet handle; rebuild a fresh `TestWallet` from the same seed.
  3. Call `discover()` to walk identity indices 0..n until none found.
  4. Call `load_identity_by_index(0)`.
  5. Mutate something off-band (e.g. issue a top-up via ID-002) and call `refresh_identity`.
- **Assertions**:
  - `discover()` returns exactly the registered identity.
  - `load_identity_by_index(0)` populates the local `IdentityManager` with id, balance, and key set matching the on-chain identity.
  - Post-`refresh_identity`, the cached balance reflects the top-up.
- **Negative variants**:
  - `load_identity_by_index(1)` for a non-existent identity at that index → returns `Ok(None)` (assert) or typed `NotFound` (whichever the contract specifies — this case will surface that contract).
- **Harness extensions required**: Wave A; helper to rebuild a `TestWallet` from a stored seed (the registry already stores `seed_hex`).
- **Estimated complexity**: M
- **Rationale**: Wallet restart / identity rediscovery is the most-hit path in mobile apps and the most-broken-by-protocol-bumps. ID-006 catches discovery regressions deterministically.

#### ID-001c — Non-default `StateTransitionSettings`
- **Priority**: P2
- **Wallet feature exercised**: `wallet/identity/network/register_from_addresses.rs:65`'s `settings: Option<StateTransitionSettings>` argument; non-default values (e.g. `wait_for_proof = false`, fee multiplier override, signing-key override).
- **DET parallel**: none.
- **Preconditions**: ID-001 helper.
- **Scenario**: register an identity exactly as ID-001 except pass a non-default `StateTransitionSettings`. Run two sub-cases:
  1. `settings: Some(StateTransitionSettings { wait_for_proof: false, .. })`. Expect the call to return as soon as broadcast succeeds, without blocking on proof.
  2. `settings: Some(StateTransitionSettings { fee_multiplier: <non-default>, .. })`. Expect the on-chain fee to scale by the configured multiplier.
- **Assertions**:
  - Sub-case (1): the call's wall-clock duration is bounded below by network RTT and above by a `proof_wait_timeout` it should not have hit; cached identity is "broadcasted, awaiting proof"; on next sync the proof is observed and the change-set finalised.
  - Sub-case (2): observed on-chain fee scales as documented (within rounding).
- **Negative variants**: none.
- **Harness extensions required**: Wave A; a "did we wait for proof?" hook on the harness SDK (or a wall-clock-bound check).
- **Estimated complexity**: M
- **Rationale**: Every existing Identity / DPNS / DashPay test passes `settings: None`. The `Some` branch is entirely uncovered; without ID-001c, settings-related fields can be silently misrouted.

#### ID-005b — `transfer_credits_to_addresses` with empty outputs
- **Priority**: P2
- **Wallet feature exercised**: `wallet/identity/network/transfer_to_addresses.rs:66` validation gate.
- **DET parallel**: none.
- **Preconditions**: ID-001 helper; identity with non-zero balance.
- **Scenario**:
  1. Register an identity per ID-001 with starting balance `≥ 50_000_000`.
  2. Call `transfer_credits_to_addresses_with_external_signer(identity_id, {}, signer, None)` — empty output map.
- **Assertions**:
  - Returns a typed validation error of "at least one output is required" shape (mirror the ID-001 negative-variant message style; pin the exact variant or message).
  - No state-transition broadcast.
  - Identity balance unchanged.
- **Negative variants**: none — this case IS the empty-input variant.
- **Harness extensions required**: Wave A only.
- **Estimated complexity**: S
- **Rationale**: ID-001 already pins the empty-`inputs` error message exactly. ID-005b mirrors that pin on the empty-`outputs` side, which is currently uncovered.

#### ID-006b — Identity-key derivation index boundary
- **Priority**: P2
- **Wallet feature exercised**: identity-key derivation under `wallet/identity/network/identity_handle.rs::derive_ecdsa_identity_auth_keypair_from_master` at `key_index` boundaries.
- **DET parallel**: none direct.
- **Preconditions**: ID-001 helper.
- **Scenario**:
  1. Register an identity with `key_index = 0`. Verify on-chain that the registered HIGH key matches `derive_identity_key(.., key_index = 0, ..)`.
  2. Register a second identity (or `update_identity` add-key on the same identity) with `key_index = DEFAULT_GAP_LIMIT - 1`. Verify the registered key matches the corresponding derivation.
  3. Optionally: attempt `key_index = DEFAULT_GAP_LIMIT` and pin the contract (rejected vs gap grown).
- **Assertions**: each sub-case asserts that the on-chain key bytes match the off-chain DIP-9 derivation at the boundary index.
- **Negative variants**: none.
- **Harness extensions required**: Wave A's `derive_identity_key` helper exposed for `key_index` (in addition to `identity_index`).
- **Estimated complexity**: M
- **Rationale**: ID-006 covers `identity_index` boundaries; `key_index` is the parallel axis and currently uncovered.

### Tokens (TK)

The wallet has token operations on the API surface
(`wallet/tokens/wallet.rs` + `wallet/identity/network/tokens/*`). They all
require an existing on-testnet token contract and an authorised identity.
Without a contract-registry strategy, only TK-001/TK-002 (operations on
existing balances) are achievable in P0/P1.

#### TK-001 — Token transfer between two identities
- **Priority**: P1
- **Wallet feature exercised**: `wallet/identity/network/tokens/transfer.rs:21` (`token_transfer_with_signer`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/token_tasks.rs:359` (`step_transfer`).
- **Preconditions**: ID-001 helper; **a known testnet token contract** (env-driven `PLATFORM_WALLET_E2E_TOKEN_CONTRACT_ID` + `_TOKEN_POSITION`); the registered identity must already hold a non-zero balance of that token (operator pre-funds via the same flow used to fund the bank).
- **Scenario**:
  1. Register `identity_a` and `identity_b` per ID-001.
  2. Pre-condition: operator pre-funds `identity_a` with `≥ 100` tokens of the configured contract (one-time setup, similar to bank funding).
  3. Call `token_transfer_with_signer(identity_a, contract_id, token_position, identity_b, amount=50)`.
  4. Sync token balances on both.
- **Assertions**:
  - `identity_a` token balance decreased by exactly `50`.
  - `identity_b` token balance increased by exactly `50`.
  - `identity_a` credit balance decreased by `transfer_fee` (token transfer pays in credits, not in tokens).
- **Negative variants**:
  - Transfer amount exceeds sender token balance → typed error.
  - Transfer with wrong `token_position` → contract-validation error.
- **Harness extensions required**:
  - Wave A (Identity signer).
  - `Config::token_contract_id` + `token_position` env vars.
  - `TestWallet::token_balance(identity_id, contract_id, token_pos)` helper.
  - Operator documentation: how to pre-fund tokens (one-time, sibling of bank pre-funding).
- **Estimated complexity**: L
- **Rationale**: Most-used token op. Catches token-amount underflow bugs and credit-fee accounting bugs in one shot.

#### TK-001b — Token transfer of amount 0
- **Priority**: P2
- **Wallet feature exercised**: `wallet/identity/network/tokens/transfer.rs:21` zero-amount boundary.
- **DET parallel**: none.
- **Preconditions**: TK-001 setup (two identities with non-zero token balance on `identity_a`).
- **Scenario**: call `token_transfer_with_signer(identity_a, contract_id, token_position, identity_b, amount=0)`.
- **Assertions**: pin one contract:
  - **(a) Reject**: typed validation error of "amount must be positive" shape; no broadcast; balances unchanged.
  - **(b) Accept**: broadcast succeeds; both token balances unchanged; only `identity_a` credit balance decreased by `transfer_fee`.
- **Negative variants**: none.
- **Harness extensions required**: TK-001 extensions.
- **Estimated complexity**: S
- **Rationale**: Zero-amount transfers may be valid no-ops or invalid per contract. Either contract needs an asserted test.

#### TK-002 — Token claim (perpetual / pre-programmed distribution)
- **Priority**: P2
- **Wallet feature exercised**: `wallet/identity/network/tokens/claim.rs:18` (`token_claim_with_signer`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/token_tasks.rs:702` (`tc_064_estimate_perpetual_rewards`) and `step_*` token lifecycle.
- **Preconditions**: TK-001 setup + a token contract that grants the registered identity claim rights.
- **Scenario**:
  1. Register identity per ID-001.
  2. Wait for the perpetual-distribution interval to advance.
  3. Call `token_claim_with_signer`.
- **Assertions**:
  - Token balance increases by the documented per-interval claim amount (operator-supplied env `PLATFORM_WALLET_E2E_TOKEN_CLAIM_AMOUNT`).
  - Second claim within the same interval returns a typed "already claimed" error.
- **Negative variants**: claim with no rights → typed error.
- **Harness extensions required**: TK-001 extensions + interval-aware sleep helper (10–60 s).
- **Estimated complexity**: L
- **Rationale**: Perpetual-distribution bugs are silent — balance just doesn't increase. Adding claim coverage is the only way to surface those.

#### TK-003 — Token mint (authorised identity)
- **Priority**: P2 (gated)
- **Wallet feature exercised**: `wallet/identity/network/tokens/mint.rs:19`.
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/token_tasks.rs:305` (`step_mint`).
- **Preconditions**: TK-001 setup + the registered identity is on the contract's mint allow-list.
- **Scenario**: mint `100` of token to self; sync.
- **Assertions**: identity token balance increased by `100`; total supply increased.
- **Negative variants**: mint without authority (TK-001's `identity_b`) → unauthorised error (DET parallel: `tc_065_mint_unauthorized` at `token_tasks.rs:756`).
- **Harness extensions required**: TK-001 extensions.
- **Estimated complexity**: M
- **Rationale**: Mint-without-authority is the canonical token authz failure mode.

#### TK-004 — Token burn
- **Priority**: P2
- **Wallet feature exercised**: `wallet/identity/network/tokens/burn.rs` (mod-level fn at `tokens/mod.rs`).
- **DET parallel**: `token_tasks.rs:330` (`step_burn`).
- **Preconditions**: TK-001 setup with non-zero balance.
- **Scenario**: burn `25` tokens; sync.
- **Assertions**: identity token balance decreased by `25`; total supply decreased.
- **Negative variants**: burn more than balance → typed error.
- **Harness extensions required**: TK-001 extensions.
- **Estimated complexity**: M
- **Rationale**: Symmetric partner of TK-003; together they validate supply bookkeeping.

### Core / SPV (CR)

All Core cases are gated on Task #15 (SPV stabilisation). They are spec'd here
so that when SPV lands, the test bodies can be written without further design.

#### CR-001 — SPV mn-list sync readiness
- **Priority**: P1 (post-Task #15)
- **Wallet feature exercised**: `manager::accessors::spv()` returning a started `SpvRuntime`; mn-list sync internals.
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/spv_wallet.rs:14` (`test_spv_sync_and_create_wallet`).
- **Preconditions**: SPV enabled in `harness::E2eContext::build` (uncomment block at `harness.rs:200-218`).
- **Scenario**:
  1. Wait `<= 180s` for `spv::wait_for_mn_list_synced` to return.
  2. Read mn-list height.
- **Assertions**: mn-list height > 0; SPV runtime reports `Ready` state.
- **Negative variants**: zero peers reachable → harness fails fast with explicit error (not a silent infinite wait).
- **Harness extensions required**: re-enable `SpvContextProvider` swap; add a `SpvHealth::status() -> Enum` accessor to the manager.
- **Estimated complexity**: M
- **Rationale**: Foundation for every other Core test — guarantees the SPV layer is alive before any Core operation runs.

#### CR-002 — Core wallet receive address derivation
- **Priority**: P1 (post-Task #15)
- **Wallet feature exercised**: `wallet/core/wallet.rs:59` (`next_receive_address_for_account`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/core_tasks.rs:14` (`test_tc001_refresh_wallet_info_core_only`).
- **Preconditions**: CR-001 ready.
- **Scenario**: derive 5 receive addresses on account `0`; assert distinctness; assert `network() == bank.network()`.
- **Assertions**: 5 distinct `Address`es; consistent network prefix.
- **Negative variants**: derive on non-existent account → typed error.
- **Harness extensions required**: SPV-backed `TestCoreWallet` helper.
- **Estimated complexity**: M
- **Rationale**: Catches Core-account derivation regressions independently of broadcast/sync.

#### CR-003 — Asset-lock-funded identity registration (full path)
- **Priority**: P2 (post-Task #15)
- **Wallet feature exercised**: `wallet/asset_lock/build.rs:39` + `wallet/identity/network/registration.rs:240` (`register_identity_with_signer`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/core_tasks.rs:132` (`test_tc004_create_registration_asset_lock`).
- **Preconditions**: CR-001 + a Core-funded test wallet (operator funds via testnet faucet).
- **Scenario**: build asset-lock tx; wait for instant-lock; register identity.
- **Assertions**: identity exists on-chain; asset-lock recorded in `tracked_asset_locks`; Core balance decreased by lock amount + fee.
- **Negative variants**: insufficient Core balance; chain re-org of asset-lock tx (P2 — manual).
- **Harness extensions required**: faucet adapter; Core-funded wallet helper.
- **Estimated complexity**: L
- **Rationale**: Mirrors DET's existing canonical Identity-create coverage. Lower priority than ID-001 because address-funded is the path with no other coverage in the workspace.

### Contracts (CT)

#### CT-001 — Document put: deploy a fixture data contract
- **Priority**: P1
- **Wallet feature exercised**: `wallet/identity/network/contract.rs:124` (`create_data_contract_with_signer`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/fetch_contract.rs` (read side); DET writes via `register_contract.rs` backend task.
- **Preconditions**: ID-001 helper; fixture contract JSON at `tests/fixtures/contracts/minimal.json`.
- **Scenario**:
  1. Register identity per ID-001.
  2. Load contract JSON (one document type, two scalar fields).
  3. Call `create_data_contract_with_signer(contract, identity_id, signer)`.
  4. Fetch contract via `sdk.fetch::<DataContract>(contract.id())`.
- **Assertions**:
  - On-chain contract id matches local id.
  - Document-type schema round-trips byte-equal (canonical CBOR).
  - Identity credit balance decreased by `contract_create_fee > 0`.
- **Negative variants**: re-deploy the same contract → typed "already exists" error.
- **Harness extensions required**: Wave A; `tests/fixtures/contracts/minimal.json`.
- **Estimated complexity**: M
- **Rationale**: Establishes the contract-fixture pattern. CT-002/003 build on it.

#### CT-002 — Document put / replace lifecycle
- **Priority**: P2
- **Wallet feature exercised**: `dash_sdk::platform::Document::{put,replace}` invoked via the SDK directly (the wallet doesn't wrap document put).
- **DET parallel**: DET's `backend_task::document.rs`.
- **Preconditions**: CT-001 contract deployed; identity from ID-001.
- **Scenario**: put a document; mutate one field; replace; fetch.
- **Assertions**: replaced document version increments; field value matches.
- **Negative variants**: replace with wrong revision → typed error.
- **Harness extensions required**: thin SDK-direct helper (no wallet API).
- **Estimated complexity**: M
- **Rationale**: Documents are the actual user-facing primitive — coverage of put/replace catches schema-validation regressions in DPP.

#### CT-003 — Contract update (add document type)
- **Priority**: P2
- **Wallet feature exercised**: `update_data_contract` flow via SDK + identity signer.
- **DET parallel**: DET's `backend_task::update_data_contract.rs`.
- **Preconditions**: CT-001 contract deployed.
- **Scenario**: update contract to add a second document type; fetch and verify.
- **Assertions**: contract version incremented; new document type queryable.
- **Negative variants**: incompatible schema change (remove required field) → typed validation error.
- **Harness extensions required**: contract-update SDK helper.
- **Estimated complexity**: M
- **Rationale**: Contract-update validation is a known sharp edge — explicit coverage prevents subtle DPP changes from breaking deployed contracts silently.

### DPNS

#### DPNS-001 — Register and resolve a `.dash` name
- **Priority**: P0
- **Wallet feature exercised**: `wallet/identity/network/dpns.rs:176` (`register_name_with_external_signer`); `dpns.rs:281` (`resolve_name`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/register_dpns.rs:14` (`test_register_dpns_name`).
- **Preconditions**: ID-001 helper; identity has `≥ 100_000_000` credits (DPNS register fee + headroom).
- **Scenario**:
  1. Register identity with sufficient balance.
  2. Generate random name `e2e-<8 random hex>.dash`.
  3. Call `register_name_with_external_signer(name, identity_id, signer, settings: None)`.
  4. Wait for `resolve_name(name)` to return `Some(identity_id)`.
- **Assertions**:
  - `resolve_name` returns the registering identity's id.
  - `sync_dpns_names()` lists the name on the identity.
  - Identity credit balance decreased by `dpns_fee > 0`.
- **Negative variants**:
  - Re-register the same name → typed `AlreadyExists` error.
  - Register a name not ending in `.dash` → typed validation error.
  - Register a name shorter than 3 chars or longer than 63 → typed validation error.
- **Harness extensions required**: Wave A; random-name helper (cryptographic RNG, lower-case alphanumeric).
- **Estimated complexity**: M
- **Rationale**: DPNS register is the most user-visible Platform feature after Identity. DPNS-001 is also the gateway to Dashpay (DP-001 needs a DPNS name).

#### DPNS-001b — Name-length boundary quartet (2 / 3 / 63 / 64 chars)
- **Priority**: P2
- **Wallet feature exercised**: DPNS name-length validation at `wallet/identity/network/dpns.rs:176`.
- **DET parallel**: none.
- **Preconditions**: ID-001 helper; identity with sufficient credits to register a DPNS name.
- **Scenario**: four sub-cases, each with a fresh DPNS-eligible identity (or the same identity if the wallet permits multiple names):
  1. Name length **2** chars (`xy.dash` — 2-char label). Expect typed validation error.
  2. Name length **3** chars (`xyz.dash`). Expect contested-name flow OR success (depends on protocol; pin which).
  3. Name length **63** chars (max-allowed label, all alphanumeric). Expect success.
  4. Name length **64** chars. Expect typed validation error.
- **Assertions**: each sub-case nails accept/reject and the typed error variant on rejection.
- **Negative variants**: none — this case IS the boundary set.
- **Harness extensions required**: Wave A; the random-name helper extended to take an explicit length.
- **Estimated complexity**: M
- **Rationale**: DPNS-001's negative variants list "shorter than 3 or longer than 63" but never pin the exact boundaries. Off-by-one at name-length is the canonical DPNS bug class.

#### DPNS-001c — DPNS name with a multibyte character
- **Priority**: P2
- **Wallet feature exercised**: DPNS name validation / canonicalisation at `wallet/identity/network/dpns.rs:176`.
- **DET parallel**: none.
- **Preconditions**: ID-001 helper; identity with sufficient credits.
- **Scenario**: register a name containing a multibyte character (e.g. `naive.dash` with `i` replaced by `ï`, or `cafe.dash` with `e` → `é`). Submit. Pin the contract:
  - **(a) Accept-and-canonicalise**: name normalised to ASCII (e.g. via Punycode / IDN-ASCII); subsequent `resolve_name` returns the canonical form.
  - **(b) Reject**: typed validation error of "ASCII-only" / "invalid character" shape.
- **Assertions**: nail one of (a) or (b). If (a), assert the canonical form matches the documented rule; if (b), assert the error variant.
- **Negative variants**: none.
- **Harness extensions required**: Wave A.
- **Estimated complexity**: S
- **Rationale**: Whichever contract the wallet implements, an explicit pin prevents future protocol-version drift from silently flipping it.

#### DPNS-002 — Resolve a known external name (negative-only assertion)
- **Priority**: P2
- **Wallet feature exercised**: `dpns.rs:281` (`resolve_name`).
- **DET parallel**: `register_dpns.rs` resolve-side.
- **Preconditions**: none beyond network reachability.
- **Scenario**: resolve a fixed never-registered name `definitely-does-not-exist-<random>.dash`.
- **Assertions**: returns `None` (not an error).
- **Negative variants**: malformed name (no `.dash` suffix) → typed validation error.
- **Harness extensions required**: none (DPNS-001's signer setup not required here).
- **Estimated complexity**: S
- **Rationale**: Confirms DPNS resolve handles the "name doesn't exist" path without surfacing it as a hard error — easy to regress when DPNS schema evolves.

### Dashpay (DP)

#### DP-001 — Set DashPay profile
- **Priority**: P1
- **Wallet feature exercised**: `wallet/identity/network/profile.rs:237` (`create_profile_with_external_signer`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/dashpay_tasks.rs:48` (`tc_032_update_profile`).
- **Preconditions**: ID-001 + DPNS-001 (identity has a DPNS name).
- **Scenario**: create profile with `display_name = "Marvin"` and `public_message`; sync profile back.
- **Assertions**: profile fetched from chain has matching `display_name` and `public_message`; profile timestamp non-zero.
- **Negative variants**: profile `display_name` exceeding length limit → typed validation error.
- **Harness extensions required**: Wave A.
- **Estimated complexity**: M
- **Rationale**: Profile is the simplest DashPay write — establishes the pattern other DashPay operations (DP-002, DP-003) reuse.

#### DP-001b — Profile with optional fields `None` vs `Some`
- **Priority**: P2
- **Wallet feature exercised**: `wallet/identity/network/profile.rs:237` partial-profile semantics.
- **DET parallel**: none direct.
- **Preconditions**: ID-001 + DPNS-001.
- **Scenario**: two sub-cases on the same identity (or on two identities if the wallet enforces single-profile-per-identity):
  1. Create profile with `display_name = None, public_message = Some("hello")`. Sync; fetch.
  2. Create profile with `display_name = Some("Marvin"), public_message = None`. Sync; fetch.
- **Assertions**:
  - Fetched profile preserves the `None`/`Some` distinction byte-for-byte (a `None` field comes back as absent, not as empty string `""`).
  - Sub-case (1) post-sync: `display_name == None`, `public_message == Some("hello")`.
  - Sub-case (2) post-sync: `display_name == Some("Marvin")`, `public_message == None`.
- **Negative variants**: none.
- **Harness extensions required**: Wave A.
- **Estimated complexity**: M
- **Rationale**: DashPay profile is a partial-update primitive in production; conflating `None` with `Some("")` would silently break all clients that use either default presentation.

#### DP-001c — Profile `display_name` containing emoji / RTL text
- **Priority**: P2
- **Wallet feature exercised**: `wallet/identity/network/profile.rs:237` UTF-8 round-trip.
- **DET parallel**: none.
- **Preconditions**: ID-001 + DPNS-001.
- **Scenario**: create a profile with `display_name = "Marvin 🤖"` (emoji) and an additional sub-case with an RTL string (e.g. Hebrew or Arabic text). Sync; fetch.
- **Assertions**:
  - Fetched `display_name` is byte-equal to the input (including the emoji code-points and any RTL embedding marks).
  - No silent normalisation that loses information.
  - Length validation operates on grapheme clusters or bytes (whichever the contract specifies); pin which.
- **Negative variants**: none.
- **Harness extensions required**: Wave A.
- **Estimated complexity**: S
- **Rationale**: UTF-8 round-trip in user-displayed fields is a quiet hazard — losing emoji or RTL marks bricks user-presented identity strings without surfacing as an error.

#### DP-002 — Send and accept a contact request
- **Priority**: P1
- **Wallet feature exercised**: `contact_requests.rs:91` (`send_contact_request_with_external_signer`); `contact_requests.rs:466` (`accept_contact_request_with_external_signer`).
- **DET parallel**: `dashpay_tasks.rs:546` (`tc_037_dashpay_contact_lifecycle`).
- **Preconditions**: two registered identities (ID-001 × 2); DPNS names on both (DPNS-001 × 2); both have profiles (DP-001 × 2).
- **Scenario**:
  1. From `identity_a`: send contact request to `identity_b`.
  2. From `identity_b`: list contact requests; accept the inbound request.
  3. Sync established contacts on both sides.
- **Assertions**:
  - `identity_a.sent_contact_requests()` lists the request.
  - `identity_b.sync_contact_requests()` returns the inbound request.
  - After acceptance, `established_contacts()` on both identities includes the other.
- **Negative variants**:
  - Send contact request to non-existent identity → typed error.
  - Accept already-accepted request → typed `AlreadyExists` or idempotent success (assert which contract the wallet defines).
  - Send self-contact request → typed validation error.
- **Harness extensions required**: Wave A; helper to spin up two identities in one `setup()`.
- **Estimated complexity**: L
- **Rationale**: Most non-trivial multi-identity flow on the wallet. Catches handshake regressions in `contact_requests.rs` end-to-end.

#### DP-003 — Send a DashPay payment
- **Priority**: P2
- **Wallet feature exercised**: `wallet/identity/network/payments.rs:92` (`send_payment`).
- **DET parallel**: covered indirectly by `dashpay_tasks.rs::tc_041_load_payment_history_empty` and DET's payment broadcast tests.
- **Preconditions**: DP-002 (two contacts established).
- **Scenario**: send a Dashpay payment from `identity_a` to `identity_b`'s contact-derived address; sync `identity_b`.
- **Assertions**: `identity_b.try_record_incoming_payment(...)` returns `Some` for the corresponding tx; payment amount matches sent.
- **Negative variants**: payment to a stranger (no contact relationship) → typed error.
- **Harness extensions required**: DP-002 setup; Wave A.
- **Estimated complexity**: L
- **Rationale**: End-to-end DashPay payment flow. Without this, payment-derivation regressions only surface in production.

### Contested Names (CN)

Contested-name auctions span minutes-to-hours on testnet and require multiple
identities voting in lockstep. Both factors push them into P2 (or "deferred to
DET parity") rather than P0/P1. Two cases are stubbed for completeness.

#### CN-001 — Initiate a contested DPNS name (premium / 3-char)
- **Priority**: P2
- **Wallet feature exercised**: `dpns.rs:176` register pathway with a contested name; `dpns.rs:425` (`contest_vote_state`).
- **DET parallel**: DET `backend_task::contested_names`.
- **Preconditions**: DPNS-001 + identity with extra credits.
- **Scenario**: register a 3-character name (`xy.dash`); query `contest_vote_state`; assert state is `Active` with the registering identity as a contender.
- **Assertions**: contest state is `Active`; registering identity present in contender list.
- **Negative variants**: query `contest_vote_state` on a non-contested name → returns `None` / `Closed`.
- **Harness extensions required**: Wave A; long-timeout polling helper.
- **Estimated complexity**: L
- **Rationale**: Smoke-tests the contest entry point without committing to the full multi-day auction flow.

#### CN-002 — Cast a masternode vote on a contested name (DEFERRED)
- **Priority**: P2 (out-of-scope today)
- **Reason for deferral**: requires a masternode signer and operator-controlled mn-list participation; harness has no way to drive that today.
- **Action**: keep this row as a placeholder; revisit when a regtest-with-masternodes harness is in scope.

### Harness self-tests (Harness)

Cases in this subsection exercise the test harness itself (registry
serialisation, async cancellation safety, workdir isolation), not the wallet.
They live here because their failures masquerade as wallet bugs and the only
sane place to pin the harness contract is alongside the wallet contract.

#### Harness-G1a — Corrupted registry JSON: refuse to overwrite
- **Priority**: P2
- **Wallet feature exercised**: `framework/registry.rs` parse + lock-file flow.
- **DET parallel**: none.
- **Preconditions**: clean workdir; ability to seed the registry file with arbitrary bytes before harness startup.
- **Scenario**:
  1. Pre-seed `registry.json` with valid JSON for one entry, followed by trailing garbage (`\n}}}`).
  2. Start the harness (e.g. invoke `setup()`).
- **Assertions**:
  - Harness returns a typed `RegistryError::ParseError { path, byte_offset }` (pin the variant; `byte_offset` should be near the trailing garbage).
  - Harness does **not** overwrite the on-disk registry file (preserve user data; assert file bytes unchanged after the failed start).
  - The lock-file (`.lock`) is released cleanly so a subsequent run that fixes the file can proceed.
- **Negative variants**: none.
- **Harness extensions required**: a typed parse-error variant on `framework/registry.rs` (likely already there; confirm name); a test setup that seeds the registry file before harness start.
- **Estimated complexity**: M
- **Rationale**: When the registry serialisation format changes, stale registry files in CI shouldn't silently corrupt user data. Harness-G1a pins refuse-to-overwrite as the contract.

#### Harness-G1b — Registry forward-compatible unknown field
- **Priority**: P2
- **Wallet feature exercised**: `framework/registry.rs` deserialisation tolerance.
- **DET parallel**: none.
- **Preconditions**: clean workdir; ability to pre-seed registry contents.
- **Scenario**:
  1. Pre-seed `registry.json` with a valid entry that includes a future-version field (e.g. `"unknown_field": "future-value"`).
  2. Start the harness; let it perform a normal write that round-trips the registry.
- **Assertions**:
  - Harness loads the registry without error.
  - On rewrite, the `unknown_field` is preserved byte-equal (forward-compatible: don't strip fields the current code doesn't understand).
  - Tests that depend on the entry continue to operate.
- **Negative variants**: none.
- **Harness extensions required**: registry serde must use `#[serde(other)]` / a catch-all field, or otherwise round-trip unknown keys. Confirm or implement.
- **Estimated complexity**: S
- **Rationale**: Without forward-compat, the moment two CI workers run different versions of the harness against a shared registry, fields get silently stripped.

#### Harness-G4 — Drop `wallet.transfer` future mid-flight, recover on next sync
- **Priority**: P2
- **Wallet feature exercised**: cancellation safety of `wallet/platform_addresses/transfer.rs:31`; on-next-sync recovery in `wallet/platform_addresses/sync.rs:24`.
- **DET parallel**: none.
- **Preconditions**: bank-funded test wallet.
- **Scenario**:
  1. Bank-fund `addr_1` with `40_000_000`.
  2. Wrap `wallet.transfer({addr_2: 5_000_000})` in a `tokio::select!` against a controllable cancellation token.
  3. Trigger cancellation **after** the broadcast call returns (i.e. ST hit DAPI) but **before** the proof-fetch completes. Confirm the future is dropped via the cancellation token.
  4. Call `wallet.sync_balances()`.
- **Assertions**:
  - Internal wallet state is consistent after the drop: no half-applied change-set, no orphaned in-flight marker that would block the next call.
  - Post-`sync_balances`, the wallet observes the broadcasted transfer and records the change-set correctly: `balances[addr_2] == 5_000_000`, `addr_1` decreased by `5_000_000 + fee`.
  - A subsequent `wallet.transfer({addr_3: 1_000_000})` succeeds — no duplicate broadcast of the previous transfer, no nonce collision.
- **Negative variants**:
  - Cancellation **before** broadcast: assert no broadcast occurred and balances unchanged.
- **Harness extensions required**: a way to inject a cancellation point between broadcast and proof-fetch (likely a test-only hook on the harness SDK or a `select!` wrapper on the wallet call). This is the most invasive of the Harness-G cases; mark as "blocked on cancellation hook" if not yet plumbed.
- **Estimated complexity**: L
- **Rationale**: `tokio::select!` cancellation safety is a documented Tokio footgun. Without an asserted contract, the wallet may corrupt internal state on user-initiated cancellation (e.g. mobile app foregrounding/backgrounding) and only surface as "wallet shows wrong balance after I closed the app".

---

## 4. Harness extension roadmap

Aggregating "Harness extensions required" across §3 and proposing a build
order. Each wave unlocks the cases listed.

### Wave A — Identity signer + identity setup helpers
- Add `SeedBackedIdentitySigner` implementing `Signer<IdentityPublicKey>` in `framework/signer.rs` (DIP-9 derivation per `derive_ecdsa_identity_auth_keypair_from_master` at `wallet/identity/network/identity_handle.rs:143`).
- Add `derive_identity_key(seed_bytes, network, identity_index, key_index, purpose, security_level) -> IdentityPublicKey` test helper.
- Add `TestWallet::register_identity_from_addresses(funding: Credits) -> Identity` helper that builds the placeholder, calls `register_from_addresses`, and waits for on-chain visibility.
- Add `wait_for_identity_balance(identity_id, expected, timeout)` in `framework/wait.rs`.
- **Unlocks**: ID-001, ID-001c, ID-002, ID-003, ID-004, ID-005, ID-005b, ID-006, ID-006b, DPNS-001, DPNS-001b, DPNS-001c, DPNS-002 (partial), CT-001, DP-001, DP-001b, DP-001c, DP-002, DP-003, TK-001, TK-001b, TK-002, TK-003, TK-004, CN-001.

### Wave B — Multi-identity per setup
- Extend `setup()` to accept `setup_with_n_identities(n: u32) -> SetupGuard { test_wallet, identities: Vec<RegisteredIdentity> }`.
- **Unlocks**: ID-003, DP-002, DP-003.
- **Cost**: Wave A pre-requisite; ~150 LoC.

### Wave C — Contract fixture loader
- `tests/fixtures/contracts/` directory + `framework::fixtures::load_contract(name)` helper.
- One canonical `minimal.json` (one doc type, two scalar fields).
- **Unlocks**: CT-001, CT-002, CT-003.

### Wave D — Token contract operator config
- `Config::token_contract_id`, `Config::token_position`, optional `Config::token_claim_amount`.
- Operator pre-funds tokens to the bank-derived identity (one-time, README'd next to bank pre-funding).
- **Unlocks**: TK-001, TK-001b, TK-002, TK-003, TK-004.

### Wave E — SPV re-enablement (Task #15)
- Uncomment SPV block in `harness.rs:200-218`; swap `TrustedHttpContextProvider` → `SpvContextProvider`.
- Add `SpvHealth::status()` accessor to manager.
- Add Core-funded test wallet helper (faucet integration).
- **Unlocks**: CR-001, CR-002, CR-003.

### Wave F — Test-only utility helpers
- `TestWallet::transfer_with_inputs` (PA-002 negative variant; PA-004b exact-balance setup).
- `TestWallet::transfer_capturing_st_bytes` (PA-006, PA-006b).
- `TestWallet::estimate_transfer_fee` (PA-002b).
- `Bank::total_credits` accessor exposed (already exists, just lift to public re-export if not).
- `Bank::with_balance_for_test` constructor (PA-010).
- `TestRegistry::get_status(wallet_id)` (PA-004).
- `FUNDING_MUTEX` instrumentation hook (PA-008c).
- "Did we broadcast?" hook on the harness SDK (PA-004c, PA-013).
- Cancellation-point hook between broadcast and proof-fetch (Harness-G4).
- Test DAPI proxy / `httpmock` adapter (PA-013).
- **Unlocks**: PA-002 (negative), PA-002b, PA-004 (full assertions), PA-004b, PA-004c, PA-006, PA-006b, PA-008c, PA-009, PA-010, PA-011, PA-012, PA-013, Harness-G1a, Harness-G1b, Harness-G4.
- **Cost**: ~200-400 LoC across multiple commits; the test-DAPI-proxy and cancellation-hook items are non-trivial and can land late.

**Recommended build order**: Wave A first (highest leverage — unblocks 25+ cases), then Wave F's cheap helpers (estimate-fee, transfer-with-inputs, registry status, FUNDING_MUTEX hook) which unblock most P2 PA cases, then Wave C, then Wave B as ID-003/DP-002 land. Wave F's expensive items (test DAPI proxy, cancellation hook) and Waves D/E are independent and can run in parallel with the others once a champion is assigned.

### Wallet-API gap notes (follow-up issues)

While drafting §3 the following minor public-API gaps were noted. None block
the spec but each would simplify a test if filed as a follow-up issue:

1. **No `PlatformWallet::fee_paid` accessor** — every PA case derives the fee from `Σ funded - Σ received - Σ remaining`. A first-class `last_transfer_fee()` (or a `fee` field on `PlatformAddressChangeSet`) would let assertions read the fee directly. Currently noted as a comment in `cases/transfer.rs:142-147`.
2. **No public sync-watermark getter on `PlatformAddressWallet`** — PA-007 needs to read the provider's `last_known_recent_block` to assert monotonicity. The field is internal; exposing a `pub fn sync_watermark() -> Option<RecentBlock>` would unblock cleanly.
3. **`IdentityManager::known_identities()` shape** — needed by ID-001's "exactly one identity registered" assertion. If the manager exposes only `BTreeMap<u32, ManagedIdentity>` without a length convenience, the test must pull internals; a `.len()` / `.identity_ids()` helper would be cleaner.
4. **Token-balance accessor by `(identity, contract, position)`** — `wallet/tokens/wallet.rs:248` already has `balance(...)`; confirm signature matches what TK-001 needs (`balance_for(identity_id, contract_id, position)`) and add the convenience if not.
5. **DPNS `register_name_with_external_signer` lacks a "wait for visibility" partner** — Wave A would benefit from a `wait_for_dpns_name_visible(name, timeout)` helper, ideally co-located with `wait_for_balance` in `framework/wait.rs`.
6. **No protocol-version accessor for `min_input_amount` / `max_outputs`** — PA-009 and PA-014 need to read these from the active `PlatformVersion`; expose a thin test-friendly getter.

---

## 5. Out-of-scope register

Explicit list of what this suite WILL NOT cover, with reasons. Each entry
prevents future scope creep arguments.

1. **Shielded transfers** — entire `wallet/shielded/` surface. Reason: prover, viewing-key derivation, and note-selection are a parallel system; coverage belongs in a dedicated suite. Re-evaluate when shielded ships to mainnet.
2. **Credit withdrawals** (`wallet/identity/network/withdrawal.rs`, `wallet/platform_addresses/withdrawal.rs`) — withdrawal verification requires Layer-1 observation of the withdrawal tx. Blocked on Task #15 (SPV stabilisation). Defer.
3. **Token contract deployment** — no testnet contract registry; the suite assumes pre-deployed contracts via env config (Wave D).
4. **Asset-lock-funded identity registration** — the bank holds Platform credits, not Core UTXOs. The address-funded variant (ID-001) covers this need from the wallet's perspective; full asset-lock coverage stays with DET (`dash-evo-tool/tests/backend-e2e/identity_create.rs`).
5. **DAPI Core path** (`tx_is_ours`, mn-list diffs, peer behaviour) — DET territory; this suite tests the wallet against DAPI, not DAPI itself.
6. **Cross-process bank concurrency** — README §"Multi-process safety" documents the operator-side requirement; not a test concern.
7. **Mainnet runs** — config supports `network=mainnet` but the suite's bank-funded model is testnet-by-policy. Mainnet runs require an explicit operator review; out-of-scope for automation.
8. **CN-002 (masternode voting)** — needs a regtest-with-masternodes harness that doesn't exist today.
9. **Non-BIP-39 mnemonic / seed sources** — see §1.2. Mnemonics must be drawn from the BIP-39 English wordlist; raw-entropy and arbitrary-UTF-8 paths are out of scope.
10. **Clock-skew / wall-clock-dependent assertions** — testnet runners are assumed to have NTP. Tests that rely on chain timestamps assume the runner's wall clock is within a few seconds of chain time. Cases that need to assert behaviour under arbitrary skew belong in a unit-test layer below this suite.

---

## 6. Open questions for product owner

Each question's answer changes the spec; numbered for reference.

1. **Token contract registry** — do we maintain one canonical testnet token contract for TK-001..TK-004, or do we rely on operators to provide their own via env? (Answer changes Wave D scope.)
2. **Contested-name coverage** — should CN-001 be promoted to P1, or do we accept DET parity and leave it P2/deferred?
3. **Long-running tests** — PA-005 (16 funding round-trips, ~3 min) is borderline. Do we accept multi-minute tests in the default `cargo test --test e2e` run, or gate them behind a `slow-tests` cargo feature?
4. **Identity withdrawal coverage** — once SPV (Task #15) lands, do we want withdrawal coverage here, or is that DET's exclusive territory?
5. **Mainnet smoke** — should the suite ever support a single, opt-in mainnet smoke case (e.g. PA-001 with a tiny `1_000`-credit transfer) for release-gate validation?
6. **Fee-bound numbers** — PA-003 asserts `fee_5 - fee_1 < 1_000_000`. Should we baseline empirical fee numbers and tighten these bounds in a follow-up, or keep them loose and rely on protocol-version bumps to reset them?
7. **Deterministic fixture network** — testnet is shared and noisy. Is there appetite to maintain a regtest-with-Drive cluster for CI exclusively, or do we accept testnet flakiness as the operating constraint?
8. **Test DAPI proxy infra** — PA-013 and the broadcast-retry contract require a controllable test DAPI proxy. Build it bespoke (`httpmock`-based), reuse an existing harness from elsewhere in the workspace, or defer the case until the proxy lands?
9. **Cancellation-hook plumbing** — Harness-G4 needs a test-only injection point between broadcast and proof-fetch. Acceptable to add a `cfg(test)` hook on the wallet, or must this stay external (wrap the future in a `select!` from the test side and accept coarser cancellation granularity)?

---

<sub>Catalogued by Marvin (QA), with the resigned competence of someone who has read every line of this code twice. Edge-case expansion by Trillian, who knows that the difference between "tested" and "tested at the boundary" is the difference between "ships" and "ships back".</sub>
