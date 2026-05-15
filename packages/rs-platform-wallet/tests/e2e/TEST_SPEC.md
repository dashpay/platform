# `rs-platform-wallet` e2e — Test Case Specification

Brain the size of a planet, and here I am cataloguing test cases. Right then.
This document enumerates the work to do; another document, somewhere, will
presumably enumerate the joy of doing it.

---

## Changelog

- **v3.1-dev (2026-05-15, TK-001 / TK-014 setup-gate Found-025 hardening)** — TK-001 and TK-014 `green` → `red-real-fail` (v53; PASS in v47), then hardened. Both timed out in the **setup funding gate before any token logic ran** — TK-001 at `tk_001_token_transfer.rs:67` (`setup_with_token_and_two_identities`), TK-014 at `tk_014_token_group_action.rs:109` (`setup_with_per_identity_funding`, three identities). In both, `bank.fund_address` chain-confirmed the funding (nonce streak 2/2) *before* the wait, then the rs-sdk address-sync silently discarded the fetched balance update because the target address was not yet in `pending_addresses` — **Found-025** (L273), amplified by 14-thread concurrency (TK-014's 3-way funding churn is the peak-pressure case). Not production defects: transfer / group-action / co-sign code never executed, and siblings (TK-001b/TK-001c, TK-009/TK-010/TK-012) were green in the same run. **One shared fix:** the single funding chokepoint `framework/mod.rs::setup_with_per_identity_funding` previously gated on `wait_for_balance`, whose proof-verified hand-off only runs *after* the Found-025-poisoned local sync map (`balances().get(addr)`) first reaches target — so under Found-025 the proof gate was never reached and the budget expired in the local-view branch. It now observes funding directly via the proof-verified `AddressInfo::fetch` path (`wait_for_address_balance_chain_confirmed_n`, `CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES`) — the same chain-state read the validator itself walks and the same family PA-009c adopted — bypassing the poisoned map entirely; the existing strong `wait_for_address_known_to_platform` gate is unchanged. Only the funding-observation mechanism changed: no funding amounts, identity counts, contract publish, propose/co-sign, or token/identity assertions altered. The fix is deterministic and concurrency-independent, so it hardens the whole setup-helper blast radius (all 22 TK-* / ID-* / CR-003 / DPNS-001 cases routing through `setup_with_per_identity_funding`). No new Found-NNN pin and no upstream issue (Found-025 already owns the root cause). A TK-wave serialization / worker-pool cap remains a documented fallback only — not implemented, since the proof-verified read-back structurally bypasses the poisoned map. Live re-validation deferred to the combined v54 run (bank-funded node unavailable in the fix environment; verified by inspection + compilation + clippy).

- **v3.1-dev (2026-05-15, PA-009c deterministic on-chain read-back)** — PA-009 sub-case C fixed (QA-014 resolved). The post-teardown observation no longer re-derives the gone wallet and trusts its recent-zone sync watermark (a watermark-less re-derived wallet's `sync_balances(AddressSyncConfig{ full_rescan_after_time_s: 0 })` resolved to a recent-zone-only query that returned `0` for `addr_1`, even though the dust was never swept — a non-deterministic harness gap, not a production defect). It now reads `addr_1` straight from the chain via the proof-verified `AddressInfo::fetch` gate (`wait_for_address_balance_chain_confirmed`, the same path the funding step already uses successfully) and asserts the residual is still exactly `TARGET_RESIDUAL`. All three pinned invariants are preserved and strengthened: (a) below-`min_input` dust is abandoned with no sweep broadcast, (b) the gate value equals `PlatformVersion::latest().dpp.state_transitions.address_funds.min_input_amount` and is positive (sub-cases A/B, untouched), (c) `addr_1`'s residual remains on chain at exactly `TARGET_RESIDUAL`. C is no longer QA-014-blocked and is no longer "degenerate against the testnet fee market" (that caveat only ever applied to the AT/JUST-ABOVE sub-cases the spec omits, never to the BELOW-gate C). `#[ignore]` is retained (network-gated, the standard for all on-chain e2e cases here; suite runs `--include-ignored`).

- **v3.1-dev (2026-05-15, PA-005b Fix-B rebaseline)** — PA-005b `blocked` → `IMPLEMENTED — passing`. The triplet is rebaselined onto the real eager-pool starting state instead of an empty-pool premise production never reaches. Production's `AddressPool::new` eagerly fills indices `0..=gap_limit-1` (`highest_generated = Some(gap_limit-1)`) and the QA-002 hook marks index 0 used, leaving one slot of helper headroom. A test-scoped precondition `open_full_gap_window` marks the highest eager index (`gap_limit-1`) used — modelling a wallet that has cycled its first DIP-17 gap window — shifting the ceiling up by `gap_limit` to open a genuine `gap_limit`-wide fresh window. A/B then batch-derive `gap_limit-1` / `gap_limit` distinct addresses; C requests `gap_limit+1`, asserts `Exceeded` with every field (`requested`, `available`, `gap_limit`, `highest_used`, `highest_generated`) pinned against the live post-mark watermarks, then a boundary retry proves non-mutation. Shared helper math at `framework/gap_limit.rs:188-207` is unchanged (confirmed correct); only the test + its precondition changed. The three-way mismatch noted in the 2026-05-14 triage entry below is resolved by this rebaseline (resolution path: a fourth — rebaseline to real state — rather than the three open options listed there).

- **v3.1-dev (2026-05-15, PA-003 fee-scaling re-pin)** — PA-003 → `green`: measures the real chain-time fee via pre/post balance accounting under single-input isolation, with symmetric pre-markers so both shapes hit address-funds UPDATE ops (no CREATE skew); restored `fee_5>fee_1`, sub-linear `fee_5<fee_1*5`, and `FEE_DELTA_CEILING` guards. `addr_src` funding sized to cover the six markers plus both measured transfers.

- **v3.1-dev (2026-05-14, Found-019 / Found-020 deletion)** — both entries removed: already-fixed pins with closed contracts. Found-019 (`SeedBackedIdentitySigner` ECDSA_HASH160 re-hash) fix landed at `tests/e2e/framework/signer.rs:148-154` in commit `59cba08af5` (PR #3563) — `identity_key_lookup` branches on `key.key_type()`, uses `key.data()` as-is for ECDSA_HASH160. Production `packages/simple-signer/src/signer.rs` does NOT have the bug shape (different storage models). Found-020 (`output_change_address` spec/impl drift) resolved via spec realignment in PR #3609 — PA-001b rewritten to match implicit-change semantics. Knowledge preserved in memcan; spec clutter dropped.

- **v3.1-dev (2026-05-14, Found-012 / Found-023 unification)** — Found-012 / Found-023 unified and filed downstream: dashpay/platform#3642 (5 hard-coded BIP-44 lookups in `proof.rs` + `recovery.rs`; downstream fix via `all_funding_accounts()` iteration, no upstream change required; SPV-side tracking verified comprehensive across all account types). Cross-link `TODO(dashpay/platform#3642)` comments added at each of the 5 sites.

- **v3.1-dev (2026-05-14, QA-901 CR-004 retarget)** — QA-901 retargets CR-004 from red-by-design (dash-evo-tool#845 pin) to passing-as-regression. TRACE run confirmed test-side dust-threshold mismatch (test assumed 2,730 duffs; upstream `transaction_builder.rs:294` uses 546). Headroom changed from 2,500 → 700; test now pins symmetric BIP-32 spent-marking via `check_core_transaction` (confirmed symmetric across TransactionRouter, ManagedAccountCollection, check_transaction_for_match, update_utxos) and the upstream sub-dust fold contract.

- **v3.1-dev (2026-05-14, Found-008 spec correction)** — AL-001 / Found-008: spec drift corrected. Bug confirmed platform-internal (not upstream rust-dashcore). Filed dashpay/platform#3641 with full repro + root-cause. Verified PR #3634 does NOT fix this — it fixes a different "no events at all" bug (FFI knob), while Found-008 is the "events arrive but get dropped in the race window" bug. AL-001 will continue to fail under N concurrent asset-lock builds until #3641 lands.

- **v3.1-dev (2026-05-14 triage, post-v47)** — three reclassifications, one upstream issue filed, two spec-drift fixes:
  - PA-003 reclassified `green` → `red-real-fail (test-bug)`. Root cause: the five-marker pre-funding loop (`pa_003_fee_scaling.rs:146-166`) writes `address_funds` storage rows for each future `dests[i]` before the 5-output transfer runs. Chain-time fee (Drive's `validate_fees_of_event/v0/mod.rs:195` driving the cost off real drive ops, not the static `state_transition_min_fees` floor) therefore pays a cheap UPDATE per 5-output recipient while the 1-output transfer pays the one-time CREATE; observed Δfee ≈ 536k matches one absent create. The asserted "more bytes ⇒ larger fee" invariant silently bakes in a "no pre-existing outputs" assumption that the marker-derivation trick violates. No production regression — the test contract is misformulated for the chosen address-derivation strategy.
  - PA-005b spec drift resolved → truth is `blocked`. Both prior `PASS` claims (detailed body at line ~534 and changelog "PR #3609 merged" entry) were stale: they landed in PR #3609 / commit `5c6baabd8f` on 2026-05-11 without re-running PA-005b against the QA-002 setup hook (`consume_platform_address_index_zero`, `wallet_factory.rs:1106-1140`) that had landed seven days earlier on 2026-05-04 (commit `94902be73b`). The failure is a three-way contract mismatch: QA-002's hook marks index 0 used while the DIP-17 platform-payment pool eagerly generates indices `0..=19` in `AddressPool::new` (rust-dashcore pinned rev `53130869e5`, `address_pool.rs:351-368`), and the headroom helper at `framework/gap_limit.rs:188-207` measures fresh-past-`highest_generated` rather than any-unused-below-ceiling — so `available` is permanently 1 from the first call regardless of the request. Test-side defect, not production.
  - PA-008b reclassified `green / IMPLEMENTED — passing` → `red-real-fail (concurrency-only)`. Isolation re-run on 2026-05-14 with `cargo test … --test-threads=1` passes in 158s; the 14-thread suite hits the canonical 120s `wait_for_balance` timeout on the first marker funding (`pa_008b_cross_wallet_funding.rs:59`, before the six-way `tokio::join!` fan-out). Suspected race in `PlatformAddressWallet::next_unused_receive_address` (`platform_addresses/wallet.rs:223-270`) vs concurrent BLAST syncs from sibling tests: a freshly derived receive address may not be promoted into the unified provider's pending set in time, so the next `sync_balances` BLAST sweep at `platform_addresses/sync.rs:24-86` returns `current=0` for the funded address indefinitely. Pinned as **Found-026** in §3 Found-bug pins.
  - Found-006 — upstream issue filed: **dashpay/rust-dashcore#762** — *Add `top_up_index` field to `CreditOutputFunding::IdentityTopUp` (DIP-9 conformance gap)*. Wallet-side TODO in `wallet/identity/network/top_up.rs` updated to reference the issue; once it lands, drop the `_` prefix on `topup_index` and forward it through the derivation path.
  - **Found-026** added — `PlatformAddressWallet::next_unused_receive_address` pool-cursor bump may not enqueue address into BLAST sync provider's pending set under concurrent load. P2, MEDIUM, suspected (pinned by PA-008b). Symmetric `rs-sdk`-side gap is already pinned as Found-025.

- **v3.1-dev (SHA `cf9b6d2ba4`, v47 audit)** — 34 PASS / 4 FAIL on 38 tests; Wave G (tokens) complete:
  - Wave G token harness (`framework/tokens.rs`) fully implemented; all TK-001 through TK-014 test files present and running — reclassified from `blocked` to `green` (except TK-007, network flake in v47).
  - DPNS-001 file implemented and running — reclassified from `blocked` to `green`.
  - ID-002b file implemented — reclassified from `not implemented` to `blocked` (prereq: Core funding).
  - AL-001 file implemented — reclassified from `not implemented` to `red-real-fail` (UTXO visibility under concurrent load; fix tracked at task #382).
  - CR-004 reclassified from `failing` to `red-by-design`: Layer 1 fixed at `1c4c8a76f4`; Layer 2 (dash-evo-tool#845 UTXO-mutation) is the genuine production bug pin; test fails deterministically as designed.
  - Found-006 reclassified `not implemented` → `red-by-design` (test file present, runs RED as designed).
  - Found-008 reclassified `not implemented` → `red-by-design` (inverted pin: Cargo PASS = bug confirmed = intentionally RED-by-design).
  - Found-025 reclassified `not implemented` → `red-by-design — pending upstream test-hook surface`. The earlier "unit test" at `tests/e2e/cases/found_025_address_sync_silent_discard.rs` asserted on a locally-built `HashMap` that the SDK never touches (Found-022 disease per `/tmp/marvin-redbyd-sweep.md`). Pin deleted; file now a stub documenting the upstream `rs-sdk` surface (`sync_address_balances` transport seam / inner-fn extraction / `AddressProvider` refresh hook) the retarget needs.
  - Found-004, Found-012, Found-013 reclassified `not implemented` → `blocked` (test files present, `#[ignore]`d on harness extension prereq).
  - Status legend expanded: `red-by-design` and `passing-as-regression` formalized; terminology normalized.
  - v47 trajectory entry added; count line recomputed.

- **v3.1-dev (commit `16636f01c0`)** — V27-007 fixed; Found-024 regression pin added:
  - V27-007 (`PlatformAddressWallet::transfer` ledger pollution — foreign output balances written to source wallet) fixed with ownership guard `account.contains_platform_address(&p2pkh)` at `transfer.rs:160`. Defensive identical guard added to `withdrawal.rs`. Canonical pattern already present at `fund_from_asset_lock.rs:77`.
  - PA-004b and PA-009: `#[ignore]` removed; both are now passing green.
  - Found-024 added to Found-bug-pins matrix (P1, passing-as-regression) as the regression pin for V27-007.

- **v3.1-dev (PR #3609 merged)** — TEST_SPEC reflects post-V20 state:
  - TK-013, PA-001b: previously failing or blocked → PASS after fix. (PA-005b also recorded as PASS in this entry; that claim was stale — see the 2026-05-14 triage entry above. Truth at that time was already `blocked` because the QA-002 setup hook had landed on 2026-05-04 without a follow-up PA-005b re-run.)
  - TK-002, CR-003: stabilised
  - CR-004: failing — two test-side defects (see §3 CR-004 detail): Layer 1 (`next_unused` idempotency) fixed at `1c4c8a76f4` via `next_receive_addresses(count=2, advance=true)`; Layer 2 (dust-threshold math wrong at line 214, `dash-evo-tool#845` reference cargo-culted) pending (QA-008)
  - `bank.fund_address` now waits for chain-confirmed nonce before releasing `FUNDING_MUTEX` (DAPI replica lag — upstream issue #3611)
  - Parallelism: PA-002, PA-008c, Harness-ID-1 (`id_sweep`) made parallel-safe
  - SPV: enabled by default (v17/v18/v19/v21 all validated SPV-on); `PLATFORM_WALLET_E2E_DISABLE_SPV=1` is an escape hatch for ChainLock-cycle outages (rust-dashcore #470), not the operating mode

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
assumptions for any P0/P1 case (Core-feature tests depend on SPV, which is now
enabled by default — see §3 "Core / SPV" preamble).

The spec is implementation-agnostic. Authors should consume it, not migrate it
verbatim from `dash-evo-tool` (DET) — DET parallels are cited only to anchor
intent and to surface battle-tested edge cases. The harness lives on top of
`PlatformWalletManager<NoPlatformPersistence>` and a `SpvContextProvider` (SPV
enabled; see §4 Wave E). Anything requiring asset locks, shielded notes, or
fresh contract deployment is explicitly deferred (see §5).

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
to their parent; new top-level edge cases get fresh dense IDs (e.g. `PA-009`).
No existing case ID is renumbered; new cases slot in adjacent to their parent.

### 1.2 Mnemonic / seed source

Mnemonics used by the harness (bank wallet, every `TestWallet`) MUST be drawn
from the BIP-39 English wordlist. Out-of-band entropy paths — raw entropy,
non-BIP-39 wordlists, or arbitrary UTF-8 strings fed as "mnemonic" — are out
of scope for this suite. Any test that generates a seed does so via the
BIP-39 mnemonic generator already used by `framework/wallet_factory.rs`. Cases
that exercise non-ASCII content (e.g. Unicode display names) do so on
downstream fields, not on the seed.

### 1.3 Known issues / operator notes

**Known issue: dash-spv mn-list QRInfo stall.** When the workdir's
`masternodestate.json` cache is missing (first run or after wipe), and
the test starts near a testnet quorum rotation boundary, dash-spv's
QRInfo retry loop may hard-cap at 3 attempts with the error
`Required rotated chain lock sig at h - 0 not present`. The engine
then stops trying to advance mn-list. `wait_for_mn_list_synced` now
surfaces this immediately as `dash-spv reported ManagerError before
mn-list synced` (event-driven path) or as a no-forward-progress stall
after 120 s (heuristic backstop), instead of waiting the full 600 s
cold-cache floor.

Operator workaround: wait 10–20 min for the next testnet ChainLock
cycle, then retry. If the issue persists, wipe
`${TMPDIR}/dash-platform-wallet-e2e/spv-data/` and retry from a clean
state.

---

## 2. Harness capability matrix

Honest snapshot of what PR #3549 can drive today vs. what each test area still
needs. "Wallet API exists" reflects what `packages/rs-platform-wallet/src/`
already exposes; "Harness ready" reflects whether
`packages/rs-platform-wallet/tests/e2e/framework/` can drive it without code
changes.

| Area | Wallet API exists | Harness ready | Gaps to fill | Out of scope (and why) |
|------|-------------------|---------------|--------------|------------------------|
| Platform Addresses | yes (`platform_addresses/{transfer,sync,withdrawal,fund_from_asset_lock}`) | yes for transfer/sync; partial for withdrawal | needs `wait_for_balance_eq` (exact-equality variant), needs explicit-input transfer helper, needs withdrawal Core-balance verification stub | `withdraw` end-to-end (Layer-1 observation, deferred — see §5 item 2); `fund_from_asset_lock` (Core UTXO needed, bank holds credits not coins) |
| Identity | yes (`identity/network/{register_from_addresses,top_up_from_addresses,registration,update,transfer,transfer_to_addresses,withdrawal}`) | no | `Signer<IdentityPublicKey>` impl, identity-key derivation helper, `TestWallet::register_identity_from_addresses`, `wait_for_identity_balance` | asset-lock-funded identity **registration** (DET territory; bank holds credits — see CR-003); asset-lock-funded top-up now has spec coverage (ID-002b); identity withdrawal (Layer-1 observation) |
| Tokens | yes (`tokens/wallet.rs` and `identity/network/tokens/*`) | no | `Signer<IdentityPublicKey>`, identity setup, contract-token discovery helper, `TestTokenContract` fixture pointer | fresh contract deployment (no testnet contract registry); group-action workflows that need multi-identity coordination outside one harness |
| Core / SPV | yes (`core/{wallet,balance,broadcast,balance_handler}`) | yes — SPV enabled (Task #15 complete, Wave E landed) | `wait_for_core_balance` implemented; faucet helper ready | broadcast tests (deferred P2); tx-is-ours flag tests (DET parity, P2) |
| Asset Lock | yes (`asset_lock/{build,manager,sync,tracked,lock_notify_handler}`) | no | needs Core-UTXO funded test wallet (SPV runtime is now available), `wait_for_asset_lock`; AL-001 concurrent-build case added | sequential single-build path already covered by CR-003 and ID-002b; concurrent-build gap closed by AL-001 |
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

<!-- merge note: kept theirs' Status-column structure (legend + Status column + ID-007 row + expanded TK list TK-001c..TK-014). Re-added the CR-004 row from cr004-spec; upstream audit reclassified CR-004 from failing-by-design to failing (test-design issue, not production bug). -->
Status legend: **green** = test file present, body has real assertions, runnable end-to-end on testnet today (subject to operator env vars). **blocked** = test file or spec entry exists but cannot run end-to-end yet — the body panics on a missing helper / prereq, the `#[ignore]` reason names an unmet prereq, or the spec body marks the entry `STUB` / `BLOCKED`. **red-by-design** = test exists, is `#[ignore]`'d, and is expected to fail (Cargo reports FAIL or, for inverted pins, PASS) until a specific upstream fix lands; the failure mode documents the bug contract. **red-real-fail** = test exists and runs but fails for a reason that is NOT a designed pin — a genuine regression or a concurrent-load/SPV gap under active investigation. **passing-as-regression** = test exists and passes today, pinning the contract that a now-fixed bug must not recur; a future regression flips it RED. **not implemented** = spec entry exists but no `<id>_*.rs` file under `tests/e2e/cases/` yet. The Status column reflects the spec body's `Status:` line where present; otherwise it is derived from the test file. (Retired terms: `failing` and `failing-by-design` — use `red-by-design` instead.)

| ID | Title | Priority | Status | Complexity |
|----|-------|----------|--------|------------|
| PA-001 | Multi-output platform-address transfer | P0 | green | S |
| PA-002 | Partial-fund + change handling | P0 | green | S |
| PA-004 | Sweep-back: drain test wallet, observe bank credit | P0 | green | S |
| PA-003 | Fee scaling: one-output vs. five-output | P1 | green — real chain-time fee under single-input isolation; symmetric pre-markers put both shapes on address-funds UPDATE ops; strict + sub-linear + ceiling guards | M |
| PA-005 | Address rotation: gap-limit + observed-used cursor | P1 | green | M |
| PA-006 | Replay safety: same outputs, second submission rejected | P1 | green | M |
| PA-007 | Sync watermark idempotency | P1 | green | M |
| PA-008 | Concurrent funding from bank: serialised | P1 | green | S |
| PA-002b | Zero-change exact-equality (`Σ outputs + fee == input balance`) | P1 | green | S |
| PA-001b | Transfer with `output_change_address: None` vs `Some(addr)` | P2 | blocked | S |
| PA-001c | Zero-credit single-output transfer | P2 | green | S |
| PA-004b | Sweep dust threshold boundary triplet | P2 | green | M |
| PA-004c | Sweep with exactly zero balance | P2 | green | S |
| PA-005b | `DEFAULT_GAP_LIMIT` triplet (19 / 20 / 21 unused) | P2 | IMPLEMENTED — passing | M |
| PA-006b | Two concurrent broadcasts of identical ST bytes | P2 | green | M |
| PA-007b | Two concurrent `sync_balances` on one wallet | P2 | green | M |
| PA-008b | Two `TestWallet`s × three concurrent funders each | P2 | red-real-fail (concurrency-only) — full-suite 14-thread FAIL on first marker `wait_for_balance` (120s timeout); `--test-threads=1` isolation PASS in 158s; suspected provider-pending promotion race in `next_unused_receive_address` | M |
| PA-008c | Observable serialisation of `FUNDING_MUTEX` | P2 | green | M |
| PA-009 | `min_input_amount` boundary triplet for cleanup | P2 | green | M |
| PA-011 | Workdir slot exhaustion at `MAX_SLOTS + 1` | P2 | not implemented | M |
| PA-012 | `sync_balances` racing with `transfer` | P2 | not implemented | M |
| PA-013 | Broadcast retry under transient DAPI 5xx | P2 | not implemented | M |
| PA-014 | Multi-output at protocol-max output count | P2 | not implemented | M |
| ID-001 | Register identity funded from platform addresses | P0 | green | L |
| ID-002 | Top-up identity from platform addresses | P0 | green | M |
| ID-002b | Asset-lock-funded top-up of existing identity | P1 | blocked — test file present; `#[ignore]`d on bank Core (Layer-1) funding prereq | L |
| ID-003 | Identity-to-identity credit transfer | P0 | green | M |
| ID-004 | Identity update: add and disable a key | P1 | not implemented | L |
| ID-005 | Transfer credits from identity to platform addresses | P1 | green | M |
| ID-006 | Refresh and load identity by index | P1 | not implemented | M |
| ID-001b | `setup_with_n_identities(N)` multi-identity helper | P1 | not implemented | M |
| ID-001c | Non-default `StateTransitionSettings` (`wait_for_proof = false`) | P2 | not implemented | M |
| ID-003b | Concurrent identity-to-identity transfers serialise on identity nonce | P2 | not implemented | M |
| ID-005b | `transfer_credits_to_addresses` with empty outputs | P2 | not implemented | S |
| ID-006b | Identity-key derivation index boundary (`0` and `DEFAULT_GAP_LIMIT - 1`) | P2 | not implemented | M |
| ID-007 | Identity-auth addresses are intentionally NOT monitored (pins intended architecture) | P2 | green | M |
| TK-001 | Token transfer between two identities | P1 | red-real-fail — network flake in v53 (setup-gate `wait_for_balance` timeout; root cause Found-025 + testnet latency under 14-thread concurrency; hardened — see changelog) | L |
| TK-001b | Token transfer of amount 0 | P2 | green | S |
| TK-001c | Token transfer across re-issued identity (signer rotation) | P2 | green | M |
| TK-002 | Token claim (perpetual — long-runtime nightly) | P2 | green | L |
| TK-003 | Register token contract (deploy via `create_data_contract_with_signer`) | P0 | green | L |
| TK-004 | Token transfer fee accounting & balance round-trip | P0 | green | M |
| TK-005 | Token mint + total-supply assertion | P1 | green | M |
| TK-005b | Mint with `recipient_id != self` | P2 | green | S |
| TK-006 | Token burn + total-supply decrement | P1 | green | M |
| TK-007 | Freeze identity for token (admin action) | P1 | red-real-fail — network flake in v47 (wait_for_balance timeout; root cause Found-025 + testnet latency) | M |
| TK-008 | Unfreeze identity for token | P1 | green | S |
| TK-009 | Destroy frozen funds | P1 | green | M |
| TK-010 | Pause and resume token (emergency action) | P1 | green | M |
| TK-011 | Set price + direct purchase round-trip | P1 | green | L |
| TK-012 | Update token config (single ChangeItem mutation) | P2 | green | M |
| TK-013 | Token claim from pre-programmed distribution | P2 | green | L |
| TK-014 | Group-action gateway: queue a mint, list pending, co-sign | P2 | red-real-fail — network flake in v53 (setup-gate `wait_for_balance` timeout; root cause Found-025 + testnet latency under 14-thread concurrency; hardened — see changelog) | L |
| CR-001 | SPV mn-list sync readiness | P1 | green | M |
| CR-002 | Core wallet receive address derivation | P1 | not implemented | M |
| CR-003 | Asset-lock-funded identity registration (full path) | P2 | green | L |
| CR-004 | Legacy BIP32 account: balance + UTXO state updates after spend | P1 | passing-as-regression — Layer 1 (next_unused idempotency) fixed at `1c4c8a76f4`; Layer 2 test-side dust-threshold mismatch fixed in QA-901 (2026-05-14); now pins the BIP-32 spent-marking + sub-dust-fold contract | M |
| AL-001 | Concurrent asset-lock builds from same wallet | P1 | red-real-fail (shifted-failure-mode) — coin-selection race closed by `403d29c3c8` + PR #3585 `OutpointReservations`; current failure is `FinalityTimeout` at `:299` (task 1 IS-lock wakeup missed); blocked on Found-008 (`LockNotifyHandler::notify_waiters` at `packages/rs-platform-wallet/src/wallet/asset_lock/lock_notify_handler.rs:30`, tracked: dashpay/platform#3641); identical fingerprint v48/v49/v50 | L |
| CT-001 | Document put: deploy a fixture data contract | P1 | not implemented | M |
| CT-002 | Document put / replace lifecycle | P2 | not implemented | M |
| CT-003 | Contract update (add document type) | P2 | not implemented | M |
| DPNS-001 | Register and resolve a `.dash` name | P0 | green | M |
| DPNS-001b | Name-length boundary quartet (2 / 3 / 63 / 64 chars) | P2 | not implemented | M |
| DPNS-001c | DPNS name with a multibyte character | P2 | not implemented | S |
| DPNS-002 | Resolve a known external name (negative-only) | P2 | not implemented | S |
| DP-001 | Set DashPay profile | P1 | not implemented | M |
| DP-001b | Profile with optional fields `None` vs `Some` | P2 | not implemented | M |
| DP-001c | Profile `display_name` containing emoji / RTL text | P2 | not implemented | S |
| DP-002 | Send and accept a contact request | P1 | not implemented | L |
| DP-003 | Send a DashPay payment | P2 | not implemented | L |
| CN-001 | Initiate a contested DPNS name (premium / 3-char) | P2 | not implemented | L |
| CN-002 | Cast a masternode vote on a contested name | DEFERRED | not implemented | — |
| Harness-G1a | Corrupted registry JSON: refuse to overwrite | P2 | not implemented | M |
| Harness-G1b | Registry forward-compatible unknown field | P2 | not implemented | S |
| Harness-G4 | Drop `wallet.transfer` future mid-flight, recover on next sync | P2 | not implemented | L |
| Harness-ID-1 | `sweep_identities` regression: registered identities surrender credits at teardown | P0 | green | S |

#### Found-bug pins

| ID | Title | Priority | Status | Complexity |
|----|-------|----------|--------|------------|
| Found-001 | `auto_select_inputs_for_withdrawal` ignores `min_input_amount` floor | P2 | not implemented | S |
| Found-002 | `auto_select_inputs_for_withdrawal` skips fee-target headroom check | P2 | not implemented | M |
| Found-003 | `addresses_with_balances` and `total_credits` only see the first platform-payment account | P2 | not implemented | S |
| Found-004 | `transfer` / `withdraw` / `fund_from_asset_lock` silently fall back to `address_index = 0` on lookup miss | P2 | blocked — test file present; `#[ignore]`d on harness extension (fine-grained address seeding) | S |
| Found-005 | `register_from_addresses` / `top_up_from_addresses` discard SDK-returned address balances and nonces | P2 | not implemented | M |
| Found-006 | `top_up_identity_with_funding` requires pre-created `IdentityTopUp { registration_index }` HD slot; absence yields confusing "not found" error | P2 | red-by-design — test file present; fails deterministically until `CreditOutputFunding` gains `top_up_index` (upstream `key-wallet`) | S |
| Found-007 | `PlatformAddressSyncManager::start` lacks a generation guard so a fast `start()` → `stop()` → `start()` can spawn parallel sync threads | P2 | not implemented | M |
| Found-008 | `LockNotifyHandler` uses `notify_waiters()` so a lock event arriving in the check / wait gap of `wait_for_proof` is dropped (tracked: dashpay/platform#3641) | P2 | red-by-design — inverted pin: Cargo PASS = bug confirmed = intentionally RED until `LockNotifyHandler` migrates off `notify_waiters()` | M |
| Found-009 | wallet-event adapter swallows `RecvError::Lagged` events without compensating recovery | P2 | not implemented | M |
| Found-010 | `PlatformAddressChangeSet::apply` ignores `funds.nonce` so persister-only nonce state can drift behind balance | P2 | not implemented | S |
| Found-011 | `IdentityChangeSet::merge` documents commutativity but `insert + tombstone` for the same key resolves to "removed" regardless of submission order | P2 | not implemented | S |
| Found-012 | `validate_or_upgrade_proof` and `wait_for_proof` only consult `standard_bip44_accounts`, missing CoinJoin / non-BIP-44 funding accounts | P2 | blocked — test file present; `#[ignore]`d on harness extension (non-BIP-44 account setup); tracked at dashpay/platform#3642 | M |
| Found-013 | `recover_asset_lock_blocking` swallows every error and returns `()` — silent recovery failure | P2 | blocked — test file present; `#[ignore]`d on harness extension (Core Layer-1 setup for asset lock recovery path) | S |
| Found-014 | `transfer_credits_with_external_signer` never updates the receiver's local balance even when the receiver is wallet-owned | P2 | not implemented | S |
| Found-015 | `load_from_persistor` leaves a partially registered wallet in `wallet_manager` when `wallet_id` mismatches | P2 | not implemented | M |
| Found-016 | `remove_wallet` removes from `self.wallets` then `self.wallet_manager` non-atomically, leaving a window where readers see only one of the two | P2 | not implemented | M |
| Found-017 | `register_wallet` registers wallet in memory even when persister `store` returns `Err` — vanishes on next launch | P2 | not implemented | S |
| Found-018 | `PlatformAddressChangeSet::merge` documents fee semantics as "fee paid by the transfer that produced this changeset" but actually accumulates fees across merged changesets | P2 | not implemented | S |
| Found-021 | `TransactionRecord::update_context` silently drops `InstantLock` state when tx transitions `InstantSend` → `InBlock` | P2 | red-by-design — pure unit test pins the merging invariant; fails deterministically until upstream `key-wallet` retains the IS-lock across `InBlock` promotion | M |
| Found-022 | `AssetLockBuilder::build` bumps `monitor_revision` on the BIP-44 funds account before `build_asset_lock` can fail, contradicting the doc-comment "no addresses consumed on failure" guarantee | P2 | red-by-design — test forces coin-selection failure on a UTXO-less wallet, snapshots `account.monitor_revision()` before the call, and asserts it is unchanged after; fails today (bumps by 1) because `set_funding` calls `next_change_address(..., add_to_state=true)` (which always invokes `bump_monitor_revision`) before `build_signed` can fail | S |
| Found-023 | `ManagedAccountCollection` lacks a `find_transaction_record(&Txid)` helper — every consumer rolls its own incomplete loop | P2 | not implemented; actionable fix downstream at dashpay/platform#3642 (Found-012 surface) | S |
| Found-024 | `PlatformAddressWallet::transfer` writes foreign output-address balances to local ledger (no ownership check) | P1 | passing-as-regression | S |
| Found-025 | `rs-sdk` address sync silently discards balance update when address is not yet in `pending_addresses` snapshot (TK-suite flake root cause) | P1 | red-by-design — pending upstream test-hook surface; prior pin was Found-022-style fake (asserted on a local `HashMap` the SDK never touches) and has been deleted. Retarget blocked on `rs-sdk` exposing a transport seam, inner-fn extraction, or post-phase `key_to_tag` refresh hook for `sync_address_balances` | M |
| Found-026 | `PlatformAddressWallet::next_unused_receive_address` pool-cursor bump may not enqueue address into BLAST sync provider's pending set (concurrent-load race) | P2 | suspected — pinned by PA-008b concurrency-only failure (full-suite FAIL, `--test-threads=1` PASS); needs TRACE instrumentation at the pool-bump + provider-enqueue boundary to confirm | M |

<!-- merge note: theirs' counts already reflect the expanded TK section + ID-007 (93 total; 74 baseline). cr004-spec adds the CR-004 row (P1, env-gated FAILING-by-design), so P1 and total each +1: P1: 25, baseline: 75, total: 94. Kept theirs' "post-Task #15" annotations and noted CR-004 as the env-gated entry under P1. ID-002b (P1, not implemented) added: P1: 26, baseline: 76, total: 95. AL-001 (P1, not implemented) added: P1: 27, baseline: 77, total: 96. upstream-audit adds Found-021, Found-022, Found-023 (P2, not implemented): P2 +3, Found-bug pins 18→21, total 96→99. Found-024 (P1, passing-as-regression): P1 +1, Found-bug pins 21→22, total 99→100. Found-025 (P1, not implemented): P1 +1, Found-bug pins 22→23, total 100→101. v47 audit adds Found-019, Found-020 (P2): P2 +2, Found-bug pins 23→25, total 101→103. -->
Counts by priority: **P0: 10**, **P1: 29** (incl. CR-004 passing-as-regression + ID-002b + AL-001 + Found-024 + Found-025), **P2: 64** (incl. 24 P2 Found-bug pins), **DEFERRED: 1** (104 total index entries; 77 baseline + 26 Found-bug pins + 1 deferred placeholder).

**Status at v47 (SHA `55472a3e79`, run date 2026-05-12):**
- 34 GREEN / 4 RED on 38 tests in `--ignored` cohort
- RED breakdown: 2 red-by-design (cr\_004 — dash-evo-tool#845; found\_006 — upstream CreditOutputFunding) + 1 network flake (tk\_007 — wait\_for\_balance timeout; root cause Found-025) + 1 real fail (al\_001 — SPV UTXO visibility under concurrent load; fix tracked at task #382)
- found\_008: inverted pin — Cargo PASS = bug confirmed (missed-wakeup under controlled timing)
- Found-024: passing-as-regression (V27-007 production fix confirmed)
- V27-007 production fix shipped; PA-004b + PA-009 now green; pa\_009/c FIXED in v47

**Status at HEAD (SHA `cf9b6d2ba4`, post-v47):**
- CR-004 retargeted (QA-901, 2026-05-14): reclassified `red-by-design (dash-evo-tool#845)` → `passing-as-regression`. The deterministic failure was a test-side dust-threshold mismatch (assumed 2,730; upstream gate at `transaction_builder.rs:294` is 546). Headroom changed `2_500 → 700`; test now pins the symmetric BIP-32 spent-marking + upstream sub-dust fold contracts.
- Found-025 prior pin retargeted: the v47-era unit test asserted on a local `HashMap` (Found-022 disease) and has been deleted in favour of a documented stub. Status remains `red-by-design — pending upstream test-hook surface`; no Cargo test is emitted today. See `/tmp/marvin-redbyd-sweep.md` and the file-level docstring at `cases/found_025_address_sync_silent_discard.rs`.
- 24 Found-bug pins total; 2 red-by-design with live Cargo tests (Found-006, Found-008), 1 red-by-design pending upstream test-hook surface (Found-025; pin deleted), 1 passing-as-regression (Found-024 V27-007 fix), 3 blocked-scaffold (Found-004, Found-012, Found-013), 1 suspected concurrency-only race (Found-026, pinned by PA-008b), 16 not implemented (Found-019/020 deleted 2026-05-14 — fixes confirmed; knowledge in memcan)

### Platform Addresses (PA)

#### PA-001 — Multi-output platform-address transfer (one tx, N outputs)
- **Priority**: P0
- **Status**: IMPLEMENTED — passing (testnet; gated by `cargo test -p platform-wallet --tests` plus operator env vars per `tests/e2e/README.md`).
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
  - `0 < fee < 5_000_000` (fee scales sub-linearly with output count — guards regression of fee strategy). **Implementation note (post-Status update):** the active test pins `0 < fee < 30_000_000` because platform issue #3040 leaves chain-time fees ~20M for 1in/2out (vs the static `state_transition_min_fees` floor ~6.5M). The 5M ceiling is restored once #3040 lands and `calculate_min_required_fee` reflects chain-time reality.
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
- **Status**: IMPLEMENTED — passing (parallel-safe). Cross-bank-balance asserts (`bank_pre` / `bank_post` comparison) were dropped — sibling test traffic pollutes the bank balance under parallel execution, making those bounds non-deterministic. The per-address balance invariants (`balances[addr_1]`, `balances[addr_2]`, `fee > 0`) are the real contract and remain.
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
- **Status**: IMPLEMENTED — passing.
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
- **Status**: `green` — measures the real chain-time fee (`Σ gross outputs − Σ destination balance deltas`) for two self-transfers that draw inputs exclusively from one source address. Every destination, including the 1-output `dest_1`, is pre-markered so both shapes hit address-funds UPDATE ops — output count is the sole varied factor. Asserts `fee_5 > fee_1`, sub-linear `fee_5 < 5 × fee_1`, and the `FEE_DELTA_CEILING` linear-schedule tripwire.
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
- **QA-003 investigation (2026-05-14)**: Root cause is a test-bug, not a production fee-strategy regression. The marker pre-funding loop at `cases/pa_003_fee_scaling.rs:146-166` issues five sequential 1-output marker transfers of 30M each into `dests[0..5]` to advance `next_unused_address`. Side effect: each `dest_i` already has an `address_funds` storage row before the 5-output transfer runs, so those outputs become cheap UPDATE operations. The 1-output transfer's `dest_1` is brand-new and pays the one-time CREATE. Chain-time fee at `rs-drive-abci/.../validate_fees_of_event/v0/mod.rs:195` is derived from real drive operation costs (storage create/update asymmetry), not from the static `state_transition_min_fees` floor at `rs-platform-version/.../v1.rs:14-15` (`output_cost = 6_000_000`). Observed Δfee ≈ 536k ≪ the static `output_cost`, consistent with exactly one absent create on the 5-output side. The "more bytes ⇒ larger fee" invariant at line 235 silently bakes in a "no pre-existing outputs" assumption that the marker-derivation trick violates. Suggested resolution: either compare two never-funded vs two never-funded transfers (create vs create), or assert against a marker baseline rather than `fee_1`. Auto-selector input-count drift was ruled out (`build_auto_select_candidates` at `transfer.rs:399-413` is balance-descending; both transfers resolve to a single `addr_src` input). PR #3554 fee-path changes ruled out — `select_inputs_reduce_output` bails before chain fee is computed.

#### PA-005 — Address rotation: gap-limit + observed-used cursor
- **Priority**: P1
- **Status**: IMPLEMENTED — passing (4 of spec's 16 rounds; runtime budget compromise, sustained-rotation property at 16+ rounds untested).
- **Wallet feature exercised**: `wallet/platform_addresses/wallet.rs:180` (`next_unused_receive_address`); `provider::PerAccountPlatformAddressState`.
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/wallet_tasks.rs:19` (`tc_012_generate_receive_address`).
- **Preconditions**: bank-funded test wallet; `DEFAULT_GAP_LIMIT = 20`.
- **Scenario**:
  1. Call `next_unused_address()` three times back-to-back BEFORE any sync. All three must return the same address (cursor is parked until first observed-used).
  2. Bank-fund the address; wait for balance.
  3. Call `next_unused_address()` once more. Must return a different address.
  4. Repeat steps 2-3 three more times (4 rounds total), funding each new address in turn.
- **Assertions**:
  - First three calls return the same `PlatformAddress` (cursor not advanced).
  - Each post-funding call advances the cursor: all 5 observed addresses (initial + 4 advances) are pairwise distinct.
  - Every funded address holds at least `FUND_FLOOR` credits after a final balance sync (no misrouted funding).
- **Negative variants**:
  - Derive 21+ unused addresses without funding — expect either gap-limit growth or a typed "gap exceeded" error (whichever the wallet contract defines; this case will surface that contract).
- **Harness extensions required**: none.
- **Estimated complexity**: M (bookkeeping ≈ 150 LoC; 4 funding round-trips are comfortably within P1 runtime budget).
- **Rationale**: The fix in commit `60f7850ab0` ("sort auto-select candidates by balance descending") is one of several invariants in the address provider that needs a regression test. PA-005 also documents the "cursor advances on observed-used" property that bit Wave 8 in PR #3549 (see `cases/transfer.rs:91-97`). The original spec called for 16 rounds (chain RTT × 16 ≈ 8 min); trimmed to 4 rounds as a P1-tier runtime compromise (QA-007). Sustained rotation through the full DIP-17 gap window remains untested at this tier — tracked for a dedicated slow-test variant. The previously listed assertion `signer.cached_key_count() >= 17` was struck (QA-008): `SimpleSigner` exposes no such accessor; the reference was to an unrelated `SeedBackedIdentitySigner` method.

#### PA-006 — Replay safety: same outputs, second submission rejected
- **Priority**: P1
- **Status**: IMPLEMENTED — passing.
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
- **Status**: IMPLEMENTED — passing (positive path only). The negative variant ("disconnect from DAPI, expect typed network error, balances unchanged") is NOT covered by the current test file; it requires a per-test SDK with a swappable DAPI URL, but the harness today shares one `Sdk` across the process via `E2eContext::sdk`. Tracked as a follow-up: tightening would mean either a `TestWallet::with_sdk_override(bogus_url)` helper or a controllable DAPI proxy (sibling of PA-013). Out of scope for this PR.
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
- **Status**: IMPLEMENTED — passing.
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
- **Status**: IMPLEMENTED — passing.
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

#### PA-001b — Transfer with implicit change: `Σ inputs == Σ outputs` canonical contract
- **Priority**: P2
- **Status**: PASS — spec realigned to match production semantics in PR #3609. `PlatformAddressWallet::transfer` has no `output_change_address` parameter; change is implicit. Sub-case A: `transfer_with_change_address(None)` — only `TRANSFER_CREDITS` are declared as outputs; the undeclared residual (`FUNDING_CREDITS - TRANSFER_CREDITS`) remains on the input address as implicit change. The Σ inputs == Σ outputs + fee invariant holds across both sub-cases.
- **Wallet feature exercised**: `wallet/platform_addresses/transfer.rs:31`; implicit-change (residual-on-input) semantics.
- **DET parallel**: none — exercises the implicit-change contract that existing PA cases never explicitly assert.
- **Preconditions**: bank-funded test wallet.
- **Scenario**:
  1. Bank-fund `addr_1` with `60_000_000`.
  2. Transfer `{addr_2: 5_000_000}` from `addr_1`. Only `5_000_000` is declared as output.
  3. Sync `addr_1` post-transfer.
- **Assertions**:
  - `balances[addr_2] == 5_000_000`
  - `balances[addr_1] == 60_000_000 - 5_000_000 - fee` (residual stays on source address)
  - `fee > 0`; `Σ inputs == Σ outputs + fee`
- **Negative variants**:
  - Transfer where `TRANSFER_CREDITS == FUNDING_CREDITS - fee` (exact sweep); assert residual on `addr_1` is `0 ± epsilon`.
- **Harness extensions required**: none.
- **Estimated complexity**: S
- **Rationale**: Pins the implicit-change contract so "residual silently goes to a sink" regressions become visible. (Prior spec/impl drift on a non-existent `output_change_address` parameter was resolved by this realignment in PR #3609; entry deleted from the Found section 2026-05-14.)

#### PA-001c — Zero-credit single-output transfer
- **Priority**: P2
- **Status**: IMPLEMENTED — passing.
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
- **Status**: IMPLEMENTED — passing (BELOW-gate sub-case only). The AT/JUST-ABOVE sub-cases collapse onto "broadcast attempted, broadcast failed" against the testnet fee market (chain-time fee ~`15_000_000` ≫ active gate of `100_000`); pinning them would leave a permanently-stuck testnet orphan with no recovery path. PA-004 already covers the well-above-fee path with `100_000_000`. The ACTIVE sweep gate is `min_input_amount` (`100_000`), not the `SWEEP_DUST_THRESHOLD = 5_000_000` referenced in the original scenario text — corrected at the implementation site. Note: this test was previously blocked by V27-007 (`PlatformAddressWallet::transfer` ledger pollution), which caused `total_credits()` to return the bank's full balance on the BELOW-gate wallet. V27-007 fixed at `16636f01c0`; pinned as Found-024.
- **Wallet feature exercised**: `framework/cleanup.rs` sweep gate at `min_input_amount` (active value: `100_000` credits via `PlatformVersion::latest().dpp.state_transitions.address_funds.min_input_amount`).
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
- **Status**: IMPLEMENTED — passing with caveats. Spec asks for a `Skipped` registry status assertion but `framework/registry.rs::EntryStatus` exposes only `Active` / `Failed` (no `Skipped` variant). Spec also asks for a "no DAPI broadcast call made" counter or "absence of nonce consumption on the bank"; neither hook is wired in the harness today (broadcast counter would need an SDK instrumentation, and the test wallet — not the bank — is the one that would broadcast a sweep). Resolution: the test pins `Ok(()) + registry entry removed`, which together with `total_credits == 0` precondition is the strongest contract observable on the current harness; tightening to a positive "no broadcast" proof requires an SDK-level instrumentation hook that's out of scope for this PR.
- **Wallet feature exercised**: `framework/cleanup.rs` sweep path with empty inputs.
- **DET parallel**: none.
- **Preconditions**: bank-funded harness; test wallet seeded but never funded (or fully drained before cleanup).
- **Scenario**:
  1. Create a fresh `TestWallet`. Do not fund it.
  2. Call `setup_guard.teardown()`.
- **Assertions**:
  - Cleanup returns `Ok(())`.
  - Registry entry is removed after teardown (the dust-gate skip path completes the lifecycle even though the sweep isn't broadcast). The fictional `Skipped` registry status is a spec drift — see Status above.
  - No broadcast attempted — observable today via the wallet's `total_credits == 0` precondition (combined with `cleanup.rs:171-178`'s explicit "skipping platform sweep" branch when total < dust_gate). A direct broadcast-counter assertion would require an SDK instrumentation hook.
- **Negative variants**: none.
- **Harness extensions required**: a "did we broadcast?" hook on the harness SDK, or a registry status accessor.
- **Estimated complexity**: S
- **Rationale**: A no-op cleanup must not throw. Without this case a refactor that moves the empty-input check could regress to `Err(InsufficientFunds)` and the test suite would never notice.

#### PA-005b — `DEFAULT_GAP_LIMIT` triplet (19 / 20 / 21 unused)
- **Priority**: P2
- **Status**: `IMPLEMENTED — passing` — rebaselined onto the real eager-pool starting state (Fix-B, 2026-05-15). Production's DIP-17 platform-payment pool is built by the eager `AddressPool::new` (upstream `key-wallet/src/managed_account/address_pool.rs:351-368`), which fills indices `0..=gap_limit-1` so `highest_generated = Some(gap_limit-1)`; the QA-002 setup hook `consume_platform_address_index_zero` (`framework/wallet_factory.rs:1106-1140`) then marks index 0 used. From that real state the batch helper's fresh-past-`highest_generated` headroom (`framework/gap_limit.rs:188-207`, semantics confirmed correct and unchanged) is `highest_used + 1` = `1` — no `gap_limit`-wide window exists at construction. The earlier expectation of a full empty-pool window was the test-side defect; production never reaches that state. The test-scoped precondition `open_full_gap_window` marks index `gap_limit-1` used (modelling a wallet that has cycled its first gap window), shifting the ceiling up by `gap_limit` to open a genuine `gap_limit`-wide window, then pins the same DIP-17 boundary from that real state. Not a production bug.
- **Wallet feature exercised**: `wallet/platform_addresses/wallet.rs:180` gap-limit enforcement at `DEFAULT_GAP_LIMIT = 20`.
- **DET parallel**: none direct; PA-005 covers cursor rotation but not the gap-limit boundary.
- **Preconditions**: bank-funded test wallet.
- **Scenario**: three sub-cases run on separate `TestWallet` instances. Each first calls `open_full_gap_window` to mark index `gap_limit-1` used (real precondition: a wallet that has cycled its first DIP-17 gap window), opening a genuine `gap_limit`-wide fresh-unused run past `highest_generated`:
  1. Request **`gap_limit-1`** (19) fresh unused addresses. Assert the batch succeeds and all are distinct.
  2. Request **`gap_limit`** (20) — exactly on the boundary. Assert the batch succeeds and all are distinct.
  3. Request **`gap_limit+1`** (21). Assert `GapLimitError::Exceeded` with every field pinned against the live post-mark watermarks (`requested=21`, `available=gap_limit`, `gap_limit`, `highest_used=Some(gap_limit-1)`, `highest_generated=Some(gap_limit-1)`); then a follow-up boundary request (`gap_limit`) must still succeed, proving the rejection did not mutate the pool.
- **Assertions**: each sub-case nails the wallet's contract at the `DEFAULT_GAP_LIMIT` boundary from the real eager-pool state — bounded success, structured ceiling rejection with concrete watermark-derived fields, and non-mutating rejection.
- **Negative variants**: none — this case is the boundary.
- **Harness extensions required**: a way to derive without funding (already supported via `next_unused_address` repeatedly; confirm cursor doesn't auto-park).
- **Estimated complexity**: M
- **Rationale**: PA-005's "21+ unused addresses" line is exploratory; PA-005b promotes it to an asserted boundary on each side of `DEFAULT_GAP_LIMIT`.
- **QA-005b spec-drift resolution (2026-05-14)**: The prior `PASS` claim on this entry (and the matching changelog line under "PR #3609 merged") was stale. PR #3609 / commit `5c6baabd8f` (2026-05-11) recorded `PASS — uses live pool_gap_limit` without re-running the test against the QA-002 setup hook that had landed seven days earlier on 2026-05-04 (`94902be73b`, `consume_platform_address_index_zero` in `framework/wallet_factory.rs:1106-1140`). On a fresh run that day all three sub-cases panicked with `available: 1` — the three-way mismatch then documented. Three resolution paths were listed:
  1. Short-circuit `consume_platform_address_index_zero` for pool-introspection tests like PA-005b (keeps QA-002 contract for normal-funded tests).
  2. Switch the helper's semantics from "fresh-past-`highest_generated`" to "any unused below ceiling" (needs audit of every caller for behavioural assumptions).
  3. Stop the pool from eagerly generating `gap_limit` addresses in `AddressPool::new` — requires upstream key-wallet change; out of scope here.

  **Resolved 2026-05-15 via Fix-B (a fourth path):** rebaseline the triplet onto the real eager-pool state. Rather than suppressing eager fill (paths 1/3) or changing shared helper semantics (path 2, rejected — the helper math is correct), the test models a real wallet that has cycled its first gap window: `open_full_gap_window` marks index `gap_limit-1` used, opening a genuine `gap_limit`-wide window, and the triplet pins the same DIP-17 boundary from the production starting state. Helper math at `framework/gap_limit.rs:188-207` is unchanged. Cargo pin verified: rust-dashcore `5297d61ac13b4bdfc85aef683e3c46e0597e6741` (`Cargo.toml:52-60`) still has the eager-fill in `AddressPool::new` (`address_pool.rs:351-368`).

#### PA-006b — Two concurrent broadcasts of identical ST bytes
- **Priority**: P2
- **Status**: IMPLEMENTED — passing.
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
- **Status**: IMPLEMENTED — passing.
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
- **Status**: `red-real-fail (concurrency-only)` — full-suite 14-thread cohort FAILS deterministically at the first marker `wait_for_balance` (panic site `cases/pa_008b_cross_wallet_funding.rs:59`, helper `derive_three_distinct` lines 51-74, BEFORE the six-way `tokio::join!` fan-out at lines 82-89). Isolation re-run with `--test-threads=1` PASSES in 158s. Suspected root cause: `PlatformAddressWallet::next_unused_receive_address` pool-cursor bump may not enqueue the freshly derived address into the unified `provider`'s pending set in time, so concurrent BLAST syncs from sibling tests snapshot stale `pending_addresses` and never surface the new address in `result.found`. Pinned as Found-026 below.
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
- **QA-008b isolation re-run (2026-05-14)**: 14-thread suite cohort hits the canonical 120s `wait_for_balance` timeout on the very first marker funding (`fund_address` for marker-a on wallet A, P2pkh `f961...830d`, 30M credits). Captured trace: bank broadcast accepted at 09:35:18.535 (seq=30, elapsed 2.4s); `wait_for_address_nonces_chain_confirmed` cleared in 682ms via the nonce-streak heuristic at 09:35:21.883; then `wait_for_balance` polled the recipient 71 times across 120s with every poll observing `current=0`, `first_observed=Some(0)`, `any_balance_change_observed=false` — i.e., the wallet's local view of the freshly derived address never moved despite the chain-time broadcast landing. The test never reaches the six-way `tokio::join!` fan-out. The 1-thread isolation re-run (`cargo test … --test-threads=1`) PASSES in 158s — single-threaded, no sibling-test interference. PA-008 (preceding test in the same cohort) and PA-008c (parallel-safe) both passed in the same failing run, biasing the diagnosis toward "cross-test BLAST-sync interference on this wallet's freshly derived address" rather than DAPI lag or bank-funding regression. Pinned as Found-026 below for upstream investigation.

#### PA-008c — Observable serialisation of `FUNDING_MUTEX`
- **Priority**: P2
- **Status**: IMPLEMENTED — passing (parallel-safe). Harness instrumentation lives in `framework/bank.rs` (`FundingMutexHistoryEntry`, `BankWallet::funding_mutex_history`); each `fund_address` call records `(seq, entry_ns, exit_ns)` under the lock so the test asserts pairwise non-overlap of the critical sections. The strict `history.len() == 3` assertion is relaxed to `history.len() >= 3` — under parallel test execution, sibling calls may contribute additional entries; per-address non-overlap (the real serialisation invariant) is the binding assertion.
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
- **Status**: IMPLEMENTED — passing (all three sub-cases). A/B are pure version-source asserts: the cleanup gate value equals `PlatformVersion::latest().dpp.state_transitions.address_funds.min_input_amount` and is positive — the unique contribution vs PA-004b. C exercises the BELOW-gate teardown end-to-end and reads `addr_1` straight from the chain via the proof-verified `AddressInfo::fetch` gate (`wait_for_address_balance_chain_confirmed`), asserting the residual is still exactly `TARGET_RESIDUAL` — i.e. the sub-`min_input` dust was abandoned and no sweep transition was broadcast. The earlier QA-014 block (a watermark-less re-derived wallet's recent-zone sync returning `0` for `addr_1`) is resolved by reading the chain directly instead of trusting the re-derived local view. The AT/JUST-ABOVE sub-cases the spec literally asks for remain degenerate against the testnet fee market (see PA-004b status); that caveat never applied to the BELOW-gate C. Note: previously also blocked by V27-007 (same root cause as PA-004b); fixed at `16636f01c0` (Found-024).
- **Wallet feature exercised**: `framework/cleanup.rs::min_input_amount`, sourced from `platform_version.dpp.state_transitions.address_funds.min_input_amount`. Test reads it via the new `framework/cleanup.rs::cleanup_dust_gate` accessor.
- **DET parallel**: none.
- **Preconditions**: bank-funded harness; test wallet × 3, each with a precisely tuned balance.
- **Scenario**: read `min` = `platform_version.dpp.state_transitions.address_funds.min_input_amount`. Three sub-cases, each its own top-level test on a fresh wallet:
  1. (A) Assert `cleanup_dust_gate(version)` == `version.dpp.state_transitions.address_funds.min_input_amount` — pins the gate to the protocol field, not a stale constant.
  2. (B) Assert the gate is `> 0` — a zero would silently sweep every wallet.
  3. (C) Fund `addr_1`, trim it to `TARGET_RESIDUAL` (`1_000`, well below `min`), teardown, then read `addr_1` directly from the chain via the proof-verified `AddressInfo::fetch` gate and assert it still equals exactly `TARGET_RESIDUAL` — the dust was abandoned, no sweep transition was broadcast. (The literal `min-1`/`min`/`min+1` triplet is not implemented: the AT/JUST-ABOVE points are degenerate against the testnet chain-time fee market — see the test module docs and PA-004b status.)
- **Assertions**: A/B pin the gate's version-source and positivity; C pins the BELOW-gate no-broadcast contract via a deterministic on-chain residual read.
- **Negative variants**: none.
- **Harness extensions required**: PA-004b's exact-balance setup helper; a way to read `min_input_amount` from the active `PlatformVersion` inside the test.
- **Estimated complexity**: M
- **Rationale**: `min_input_amount` is currently entirely uncovered. A protocol-version bump that changes the value would silently shift cleanup behaviour, with no failing test to flag the shift.

#### PA-011 — Workdir slot exhaustion at `MAX_SLOTS + 1`
- **Priority**: P2
- **Status**: STUB — placeholder for follow-up PR (no test file in `tests/e2e/cases/` yet; needs sub-process orchestration or in-process `flock` simulation).
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
- **Status**: STUB — placeholder for follow-up PR (no test file in `tests/e2e/cases/` yet).
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
- **Status**: BLOCKED — needs harness refactor: a controllable test DAPI proxy (httpmock-style) able to inject transient 5xx on `/broadcastStateTransition`. No test file yet.
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
- **Status**: STUB — placeholder for follow-up PR (no test file yet; trivial once the `max_outputs` constant is read off `PlatformVersion`).
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
- **Status**: Pass — `tests/e2e/cases/id_001_register_identity_from_addresses.rs` (drives `register_identity_from_addresses` and pins on-chain key count + balance bounds + post-fee residual).
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

#### ID-001b — `setup_with_n_identities(N)` multi-identity helper
- **Priority**: P1
- **Wallet feature exercised**: harness helper `setup_with_n_identities(n, funding_per)` chained over `IdentityWallet::register_from_addresses` for `n` consecutive DIP-9 identity indices.
- **DET parallel**: none direct.
- **Preconditions**: ID-001 helper landed; bank funded for `n × (funding_per + register_fee_headroom)`.
- **Scenario**:
  1. `let guard = setup_with_n_identities(3, 30_000_000).await?;`
  2. For each `i` in `0..3`, fetch `Identity::fetch(sdk, guard.identities[i].id)`.
- **Assertions**:
  - The three `Identifier`s are pairwise distinct.
  - The three `identity_index` values are `0`, `1`, `2` in registration order.
  - Each fetched identity has `balance >= funding_per / 2` (post-fee threshold).
  - The three identities' MASTER public keys are pairwise distinct (DIP-9 fan-out, not a copy-paste of slot 0).
  - Bank's `total_credits()` decreased by `[n × funding_per, n × funding_per + n × fund_fee_upper_bound]`.
- **Negative variants**:
  - `n == 0` → typed validation error.
- **Harness extensions required**: Wave A only.
- **Estimated complexity**: M
- **Rationale**: Multi-identity setup is the gateway for ID-003 / ID-008 and any future contact-graph or DashPay test. Pins the helper's nonce-discipline against `register_from_addresses`'s nonce-cache TODO regressing.

#### ID-002 — Top-up identity from platform addresses
- **Priority**: P0
- **Status**: Pass — `tests/e2e/cases/id_002_top_up_identity.rs` (post-top-up identity balance fetched on-chain, fee derived from delta, second-address residual asserted).
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

#### ID-002b — Asset-lock-funded top-up of existing identity
- **Priority**: P1
- **Status**: Not implemented. New test file `tests/e2e/cases/id_002b_asset_lock_top_up.rs` (TBC).
- **Wallet feature exercised**: `wallet/identity/network/top_up.rs:60` (`top_up_identity_with_funding` with `TopUpFundingMethod::FundWithWallet { amount_duffs }`). Internally drives `wallet/asset_lock/build.rs` → `create_funded_asset_lock_proof` — the same build path CR-003 exercises for identity registration.
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/identity_tasks.rs:27` (`step_top_up` — uses `TopUpIdentityFundingMethod::FundWithWallet` to top-up an existing identity via wallet UTXOs). This is a live DET coverage path; ID-002b brings parity to the rs-platform-wallet suite.
- **Preconditions**: CR-001 (SPV ready) + a Core-funded test wallet with at least `TEST_WALLET_CORE_FUNDING + CORE_TX_FEE_RESERVE` duffs on BIP-44 account 0 (same funding floor as CR-003) + a registered identity. The registration can use the address-funded path (ID-001 helper); the top-up source does not need to match the registration source.
- **Scenario**:
  1. `setup_with_core_funded_test_wallet(TEST_WALLET_CORE_FUNDING)` — land `TEST_WALLET_CORE_FUNDING` duffs on BIP-44 account 0 (mirror CR-003 setup).
  2. Register an identity via `register_from_addresses` (Platform-side, simpler — reuse ID-001 helper). Capture `identity_id` and `pre_balance`.
  3. Define `TOP_UP_ASSET_LOCK_AMOUNT = 100_000_000` (100 M duffs ≈ 0.001 DASH) plus fee headroom as the top-up amount.
  4. Call `IdentityWallet::top_up_identity_with_funding(identity_id, TopUpFundingMethod::FundWithWallet { amount_duffs: TOP_UP_ASSET_LOCK_AMOUNT }, _topup_index = 1, None)`.
  5. Wait for IS-lock / ChainLock on the asset-lock tx (same primitive CR-003 uses for registration).
  6. Fetch the identity's chain balance via `Identity::fetch(sdk, identity_id)`.
- **Assertions**:
  - `post_balance == pre_balance + (TOP_UP_ASSET_LOCK_AMOUNT × CREDITS_PER_DUFF) - top_up_fee`, where `CREDITS_PER_DUFF = 1000`.
  - `top_up_fee > 0`.
  - The asset-lock tx appears in the wallet's `tracked_asset_locks` registry with state Used/Consumed.
  - The test wallet's confirmed Core balance decreased by `(TOP_UP_ASSET_LOCK_AMOUNT + asset_lock_fee + core_send_fee)` duffs relative to its post-setup balance.
- **Negative variants (defer to follow-up)**:
  - Top-up of a non-existent `identity_id` → typed error.
  - `amount_duffs = 0` → typed validation error.
  - Insufficient Core balance on the test wallet → typed `PlatformWalletError::Wallet` error.
- **Notes / risks**:
  - Requires the same `PLATFORM_WALLET_E2E_BANK_CORE_GATE` env var that CR-003 uses (default-on, 900 s deadline). An under-funded Core address surfaces as `FrameworkError::Bank` with the bank's Core address embedded — identical operator-actionable error contract to CR-003.
  - Core-sweep teardown should return Core residuals to the bank (mirror CR-003 teardown); teardown failure is best-effort: log and skip rather than fail the test.
  - Found-006 (matrix line, §3) notes that `top_up_identity_with_funding` ignores the caller-supplied `_topup_index` — the parameter is currently a no-op (`_topup_index` in the production signature). Pass `1` for ID-002b to distinguish from the registration asset lock; cover the `topup_index` routing discrepancy separately under Found-006's test entry.
- **Harness extensions required**: same as CR-003 — `setup_with_core_funded_test_wallet`, `wait_for_asset_lock`; plus Wave A identity setup helpers already needed by ID-001.
- **Estimated complexity**: L (Core-funded wallet setup + asset-lock orchestration — same shape as CR-003; the top-up call itself is simpler than registration but the harness scaffolding is equivalent)
- **Rationale**: `top_up_identity_with_funding` with `FundWithWallet` is a complete production primitive with zero positive test coverage in this suite. ID-002 covers the address-funded top-up path; this case covers the Core/asset-lock-funded path — the two together give full positive coverage of the identity top-up surface.

#### ID-003 — Identity-to-identity credit transfer
- **Priority**: P0
- **Status**: Pass — `tests/e2e/cases/id_003_identity_to_identity_transfer.rs` (uses `setup_with_n_identities(2, …)`; pins receiver-side exact gain + sender-side loss > amount + non-zero fee).
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

#### ID-003b — Concurrent identity-to-identity transfers serialise on identity nonce
- **Priority**: P2
- **Wallet feature exercised**: `transfer_credits_with_external_signer` under concurrent invocation from the same source identity.
- **DET parallel**: none.
- **Preconditions**: ID-001b helper (multi-identity setup).
- **Scenario**:
  1. `let guard = setup_with_n_identities(3, 60_000_000).await?;`
  2. Spawn two `tokio::spawn` tasks from `guard.identities[0]` — task 1 transfers `5_000_000` to `guard.identities[1]`; task 2 transfers `7_000_000` to `guard.identities[2]`.
  3. `tokio::join!` on both. Record each task's `Result`.
- **Assertions**:
  - Either both tasks succeed, OR exactly one task succeeds and the other returns a typed nonce-collision error from DAPI. Pin which contract the wallet implements.
  - `post_sender == pre_sender - successful_amounts_total - successful_fees_total`.
  - Sender identity revision is monotonic: `post_revision == pre_revision + count(successful transfers)` (no skipped, no duplicate).
- **Negative variants**: foreign signer signing for `sender`'s transition is covered by QA-001's regression test in `signer.rs`.
- **Harness extensions required**: Wave A; ID-001b helper.
- **Estimated complexity**: M
- **Rationale**: The identity-side parallel of PA-008b. Surface-discovery: pins whichever serialisation contract the wallet exposes today rather than asserting an aspirational one.

#### ID-004 — Identity update: add and disable a key
- **Priority**: P1
- **Status**: Not implemented — deferred to a follow-up PR. The harness's `SeedBackedIdentitySigner` only pre-derives keys for `key_index ∈ 0..DEFAULT_GAP_LIMIT`; signing the next transition with a freshly-issued key needs a `derive_identity_key`-driven cache-injection helper that does not exist yet (mirrors the `ID-flow-009` Blocked entry).
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
- **Status**: Pass — `tests/e2e/cases/id_005_identity_to_addresses_transfer.rs` (pins exact destination-address gain + identity loss > amount + on-chain post-balance equals wallet-returned `Credits`).
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
- **Status**: Not implemented — deferred to a follow-up PR. The "rebuild a fresh `TestWallet` from the same seed and run discovery" path needs a `TestWallet::from_seed_bytes` helper that does not exist today; `load_identity_by_index` itself is exercised by the orphan-recovery branch of `cleanup::sweep_identities_with_seed` but not by a dedicated assertion-bearing test.
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
- **Status**: STUB — P2 deferred. The harness has no "did we wait for proof?" hook today; ID-001c is the right place to add one but lands after the P0/P1 bring-up.
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
- **Status**: STUB — P2 deferred; pins the empty-`outputs` validation error message after the P0/P1 cohort lands.
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
- **Status**: STUB — P2 deferred; needs the `derive_identity_key` helper exposure for `key_index` (sibling of ID-004's blocked helper).
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

#### ID-007 — Identity-auth addresses are intentionally NOT monitored
- **Priority**: P2
- **Status**: Pass — `tests/e2e/cases/id_007_identity_auth_addresses_not_monitored.rs`
  pins the intentional architecture that DIP-9 identity-authentication
  subfeature paths (subfeature `0..3`,
  `m/9'/coinType'/5'/{0,1,2,3}'/identity_index'/key_index'`) are NOT in
  `WalletAccountCreationOptions::Default` and therefore NOT in
  `PlatformWalletInfo::monitored_addresses()`. Sending Core duffs to
  one of those addresses does NOT increase the wallet's Core balance,
  and the UTXO set never observes such a send. `#[ignore]`-tagged so a
  default `cargo test` stays green; `cargo test -- --ignored` runs it
  end-to-end and is expected to PASS. Documents the intended
  architecture; closed PR `dashpay/rust-dashcore#554` was a speculative
  attempt to change this and was correctly rejected. End-to-end runs
  are gated on **operator pre-funding the bank's Core (Layer-1) receive
  address** with at least `100_000 + fee` duffs of testnet DASH (the
  address is logged at framework init under target
  `platform_wallet::e2e::bank`).
- **Wallet feature exercised**: `PlatformWalletInfo::monitored_addresses` (`wallet/platform_wallet_traits.rs:93`) projection for DIP-9 identity-authentication addresses derived via `derive_ecdsa_identity_auth_keypair_from_master` (`wallet/identity/network/identity_handle.rs:143`). Concretely: the `m/9'/coinType'/5'/0'/identity_index'/key_index'` subfeature path, which is intentionally excluded from `WalletAccountCreationOptions::Default` because identity-auth keys are pure key material, not funds-bearing addresses.
- **DET parallel**: `dash-evo-tool/src/backend_task/account_summary.rs:226-229` — explicitly states identity-auth addresses "usually hold zero balance"; `receive_address()` returns BIP-44 paths only and DET's UI hides them outside developer-mode "Identity System" view.
- **Preconditions**:
  - SPV runtime enabled (Task #15 — gates `CR-001` too).
  - ID-001 helper landed (Wave A).
  - Bank wallet that holds **Core coins**, not just credits — same prerequisite as `CR-003`.
- **Scenario**:
  1. `let id = setup_with_n_identities(1, 30_000_000).await?.identities[0];`
  2. Compute `auth_addr = P2PKH(derive_ecdsa_identity_auth_keypair_from_master(master, network, identity_index = 0, key_index = 0).public_key)`.
  3. Snapshot `wallet.monitored_addresses()` *before* sending anything.
  4. Send `100_000` duffs from the Core-funded bank to `auth_addr` on Layer-1.
  5. Snapshot `wallet.monitored_addresses()` *after* the broadcast.
  6. Wait up to `30s` for the wallet's Core balance to reflect the incoming UTXO; expect it does NOT.
- **Assertions** (pin the **intended** contract — green when the architecture is intact):
  - `auth_addr` is **NOT** in `monitored_addresses()` both before and after step 4.
  - The wallet's Core balance does **NOT** increase to `pre_balance + 1` within the negative window after step 6 (the `wait_for_core_balance` call is expected to time out).
  - The wallet's UTXO set does **NOT** contain a `100_000`-duff UTXO at `auth_addr`.
  - When this test starts FAILING, a regression has happened: either `WalletAccountCreationOptions::Default` started including `BlockchainIdentities*` `AccountType`s, or some other code path has begun monitoring these addresses without architecture review. Investigate before flipping.
- **Variants** (covered inline in the same test — registration status is irrelevant, the derivation is pure; same architecture applies):
  - Compute `auth_addr` for `identity_index = 1` (an unregistered slot) — the address must remain unmonitored regardless of registration state.
  - Repeat for the BLS subfeature path (`m/9'/coinType'/5'/2'/identity_index'/key_index'`) once `derive_*_bls_identity_auth_keypair_from_master` lands; same intended-contract assertions apply. (Deferred — TODO comment in the test body.)
- **Harness extensions required**:
  - SPV runtime re-enabled (Task #15 — same prerequisite as `CR-001`).
  - Core-funded bank wallet helper (same prerequisite as `CR-003`).
  - `wait_for_core_balance(wallet, expected_min, timeout)` — landed in `framework/wait.rs` alongside this case (parallel of `wait_for_balance` for Layer-1 balance instead of credits).
  - Wave A's `SeedBackedIdentitySigner` (already needed for `ID-001`).
- **Estimated complexity**: M (test body is short — most of the cost is the prerequisite SPV + Core-faucet bring-up that `CR-001` and `CR-003` already require).
- **Funding budget**: `100_000` Core duffs (~0.001 DASH) per run for the Layer-1 send; rounding for Core-tx fee. Negligible compared to the credit budget of any P0/P1 case.
- **Rationale**: Pins the **intentional** architecture for "which DIP-9 subfeatures get monitored?" Identity-auth addresses are pure key material — they sign identity state transitions, they don't receive Layer-1 Dash. dash-evo-tool (the canonical Platform client) treats them this way: `account_summary.rs:226-229` explicitly notes they "usually hold zero balance"; `receive_address()` returns BIP-44 paths only; the UI hides them outside developer-mode "Identity System" view. No standard flow sends Layer-1 Dash to these addresses. The closed PR `dashpay/rust-dashcore#554` was a speculative attempt to change this for a hypothetical use case, not a fix for any active bug — its rejection was correct. ID-007 pins the not-monitored contract so any accidental regression — or any deliberate architecture shift — surfaces loudly.
- **Operator notes**: First cold-cache run takes ~15 minutes because SPV walks compact filters from genesis (~1.47M testnet blocks). Subsequent runs reuse the on-disk cache and complete in seconds. The harness gates init on `PLATFORM_WALLET_E2E_BANK_CORE_GATE` — **default-on with a 900s deadline**, waiting for the bank's confirmed Core balance to become non-zero so ID-007 doesn't race a cold-cache scan and see `core_balance_confirmed=0` mid-scan for an already-funded address. Set the var to `0` (or `disabled` / `false` / `off`) to opt out for Platform-only suites; set a positive integer to override the timeout in seconds. Set `RUST_LOG=info,platform_wallet::e2e::wait=info` to see scan-progress lines (`scan_height` vs `scan_tip`) every 30s.
- **Notes**:
  - Today `derive_ecdsa_identity_auth_keypair_from_master` is the only DIP-9 subfeature `rs-platform-wallet` exposes (subfeature 0, ECDSA). Adding the BLS / Hash160 variants is contingent on the upstream `key-wallet` API gaining BLS derivation helpers.
  - This is a **defensive pin of intentional behavior**, in the same family as `Found-003` / `Found-004`: green = architecture intact, red = something changed and needs review. The change might be a real architecture shift (in which case flip the assertions in the same PR that wires the change) or an accident (in which case revert the breakage).

### Tokens (TK)

The wallet has token operations on the API surface
(`wallet/tokens/wallet.rs` + `wallet/identity/network/tokens/*`). The earlier
plan rested on an operator-pre-funded testnet token contract; that approach
is superseded. The current plan deploys a fresh token contract per CI run via
`create_data_contract_with_signer` (the wallet already accepts a
`tokens_schema_json` argument — `wallet/identity/network/contract.rs:124`),
shared across most TK cases via a OnceCell fixture and re-built fresh only
where a non-default contract config is required (pre-programmed distribution,
groups, paused-on-create). Wave A (Identity signer harness) and Wave G
(token-contract bootstrap helpers, see §4) are both complete. What were previously tracked
as `Gap-T1..Gap-T6` (wallet-API surface gaps) are now resolved: Wave G
delivers framework-level SDK-wrapper helpers for each, living in
`packages/rs-platform-wallet/tests/e2e/framework/tokens.rs`. No new wallet
public API is required; tests compose the SDK directly through those helpers.
All TK cases ran in v47 (SHA `55472a3e79`); TK-001 through TK-014 PASS except TK-007
(network flake — `wait_for_balance` timeout; see TK-007 entry below).

#### TK-001 — Token transfer between two identities
- **Priority**: P1
- **Status**: red-real-fail — `tests/e2e/cases/tk_001_token_transfer.rs` (Wave 2-α; `#[ignore]`-tagged, runs on demand against testnet). PASS in v47; FAIL in v53 with `wait_for_balance timed out after 120s` at `tk_001_token_transfer.rs:67` (`setup_with_token_and_two_identities`) — funding chain-confirmed before the wait, then the SDK address-sync silently discarded the update. Root cause: Found-025 (L273) address-sync silent-discard amplified by 14-thread concurrency; not a `token_transfer` regression (sibling TK-001b/TK-001c green same run). Hardened: the shared per-identity funding gate (`framework/mod.rs::setup_with_per_identity_funding`) now observes funding via the proof-verified `AddressInfo::fetch` path instead of the Found-025-poisoned local sync map. Live re-validation deferred to the combined v54 run.
- **Wallet feature exercised**: `wallet/identity/network/tokens/transfer.rs:21` (`token_transfer_with_signer`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/token_tasks.rs:359` (`step_transfer`).
- **Preconditions**: Wave A signer + Wave G token-contract bootstrap (TK-003 helper); two registered identities (`identity_a`, `identity_b`); `identity_a` holds a non-zero token balance from an in-test mint (TK-005 helper).
- **Scenario**:
  1. `setup_with_token_and_two_identities()` returns `(token_fixture, identity_a, identity_b)` (the shared OnceCell-cached contract).
  2. `identity_a` mints `≥ 100` tokens to self via the harness `mint_to` shortcut.
  3. Call `token_transfer_with_signer(identity_a, contract_id, token_position=0, identity_b, amount=50, …)`.
  4. Sync token balances on both via `token_balance_of`.
- **Assertions**:
  - `identity_a` token balance decreased by exactly `50`.
  - `identity_b` token balance increased by exactly `50`.
  - `identity_a` credit balance decreased by `transfer_fee` (token transfer pays in credits, not in tokens).
- **Negative variants**:
  - Transfer amount exceeds sender token balance → typed error.
  - Transfer with wrong `token_position` → contract-validation error.
- **Harness extensions required**: Wave A; Wave G's `setup_with_token_and_two_identities`, `mint_to`, `token_balance_of`.
- **Estimated complexity**: L
- **Rationale**: Most-used token op. Catches token-amount underflow bugs and credit-fee accounting bugs in one shot. TK-004 is the upgraded round-trip variant with explicit fee separation; TK-001 stays as the canonical happy path.

#### TK-001b — Token transfer of amount 0
- **Priority**: P2
- **Status**: green — `tests/e2e/cases/tk_001b_token_transfer_zero.rs` (Wave 2-α; `#[ignore]`-tagged, runs on demand; PASS in v47).
- **Wallet feature exercised**: `wallet/identity/network/tokens/transfer.rs:21` zero-amount boundary.
- **DET parallel**: none.
- **Preconditions**: TK-001 setup (in-test deployed token + two identities with non-zero balance on `identity_a` via in-test mint).
- **Scenario**: call `token_transfer_with_signer(identity_a, contract_id, token_position=0, identity_b, amount=0, …)`.
- **Assertions**: pin one contract:
  - **(a) Reject**: typed validation error of "amount must be positive" shape; no broadcast; balances unchanged.
  - **(b) Accept**: broadcast succeeds; both token balances unchanged; only `identity_a` credit balance decreased by `transfer_fee`.
- **Negative variants**: none.
- **Harness extensions required**: TK-001 extensions.
- **Estimated complexity**: S
- **Rationale**: Zero-amount transfers may be valid no-ops or invalid per contract. Either contract needs an asserted test.

#### TK-001c — Token transfer across re-issued identity (signer rotation)
- **Status**: green — `tests/e2e/cases/tk_001c_token_transfer_after_reissue.rs` (Wave 2-α; `#[ignore]`-tagged; PASS in v47. Note: `#[ignore]` reason flags a possible `wait_for_balance` flake shared with Found-025; the test body itself is correct).
- **Priority**: P2
<!-- merge note: HEAD's content here was misplaced TK-002 prose (perpetual claim / InvalidTokenClaimNoCurrentRewards) — the surrounding TK-001c heading + scenario describe key rotation, so theirs is the correct content for this entry. TK-002 already carries its own STUB status downstream; no relocation needed. -->
- **Wallet feature exercised**: `wallet/identity/network/tokens/transfer.rs:21` after the sender's signing key has been rotated (add new key, disable old key, transfer with new key).
- **DET parallel**: none direct.
- **Preconditions**: TK-003 helper + ID-004 helpers; identity with a minted token balance from an in-test mint.
- **Scenario**:
  1. Setup token + identity with mint balance.
  2. Add a fresh AUTHENTICATION key via `update_identity` (ID-004 path), disable the old one.
  3. Transfer tokens using the **new** key as the signer.
- **Assertions**:
  - Transfer succeeds with the new key.
  - Transfer with the disabled key would fail with a typed "key not found / disabled" error (sub-case).
- **Negative variants**: covered above.
- **Harness extensions required**: depends on Wave A + ID-004 chain; TK-003 helper.
- **Estimated complexity**: M
- **Rationale**: Token operations don't hard-code a signing key — they accept a `signing_key: &IdentityPublicKey` parameter and rely on the identity's current key set. Pinning that "the wallet picks the right active key after rotation" prevents a quiet "still uses the old key" regression.

#### TK-002 — Token claim (live perpetual distribution — long-runtime, nightly only)
- **Priority**: P2
- **Status**: green — `tests/e2e/cases/tk_002_token_claim_perpetual.rs` (Wave 2-α; `#[ignore]`-tagged, nightly only; PASS in v47). Note: long-runtime (~4 min wall clock); `#[ignore]` reason flags a possible `wait_for_balance` flake shared with Found-025. Demoted to nightly because perpetual intervals run on testnet block time (~3 s) and a meaningful claim window is 30–60 s of wall clock; the synchronous CI tier covers the same surface via TK-013's pre-programmed-distribution variant.
- **Wallet feature exercised**: `wallet/identity/network/tokens/claim.rs:18` (`token_claim_with_signer`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/token_tasks.rs:702` (`tc_064_estimate_perpetual_rewards`) and `step_*` token lifecycle (DET tests only the *estimate* path).
- **Preconditions**: TK-003 helper extended to deploy a token with live perpetual distribution; identity holding claim rights.
- **Scenario**:
  1. Deploy the token with perpetual distribution rules (interval = block-based, minimum testnet interval).
  2. Wait for the perpetual-distribution interval to advance (~30–60 s wall clock).
  3. Call `token_claim_with_signer`.
- **Assertions**:
  - Token balance increases by the per-interval claim amount documented in the contract.
  - Second claim within the same interval returns a typed "already claimed" / "no claimable amount" error.
- **Negative variants**: claim with no rights → typed error.
- **Harness extensions required**: TK-003 extensions + interval-aware sleep helper (30–60 s).
- **Estimated complexity**: L
- **Rationale**: Perpetual-distribution bugs are silent — balance just doesn't increase. TK-013 covers the synchronous path; TK-002 keeps the live-time variant in scope behind a `slow-tests` cargo feature (cf. §6 Q3). Without it, a regression that breaks perpetual-distribution event scheduling never surfaces.

#### TK-003 — Register token contract (deploy via `create_data_contract_with_signer`)
- **Status**: green — `tests/e2e/cases/tk_003_register_token_contract.rs` (Wave 2-β; `#[ignore]`-tagged; PASS in v47). Signs with `RegisteredIdentity::master_key` (MASTER, KeyID 0); if testnet rejects MASTER with `InvalidSignatureError` that surfaces as a hard `panic!` in the test body.
- **Priority**: P0 (gateway for every other TK-NNN entry)
- **Wallet feature exercised**: `wallet/identity/network/contract.rs:124` (`create_data_contract_with_signer`) with non-empty `tokens_schema_json`.
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/token_tasks.rs:78` (`tc_045_register_token_contract`); fixture at `tests/backend-e2e/framework/fixtures.rs:111`; helper at `tests/backend-e2e/framework/token_helpers.rs:33`.
- **Preconditions**: ID-001 helper; identity has ≥ `1_000_000_000` credits (contract-create fee + headroom).
- **Scenario**:
  1. Register identity via ID-001.
  2. Build a permissive owner-only token-config JSON (mirror DET's `build_register_token_task`: 8 decimals, max supply 1e15, no perpetual distribution, owner-only ChangeControlRules across mint/burn/freeze/unfreeze/destroy/emergency/max-supply/conventions/marketplace, `start_paused = false`, `allow_transfers_to_frozen_identities = false`, `marketplace_trade_mode = 1`).
  3. Call `create_data_contract_with_signer(owner, documents="{}", tokens=Some(config), …)`.
  4. `sdk.fetch::<DataContract>(returned.id())`.
- **Assertions**:
  - Returned contract id matches the on-chain fetch.
  - `contract.tokens()` is non-empty; token at position 0 has the configured name / decimals / max supply.
  - Identity credit balance decreased by `> 0` (contract-create fee).
- **Negative variants**:
  - Re-deploy with same id (contrived — id is owner+nonce-derived) → `AlreadyExists` SDK error class.
  - Token config with `max_supply < base_supply` → typed validation error.
- **Harness extensions required**: `setup_with_token_contract(...)` helper (§4 Wave G); contract fixture JSON template at `tests/fixtures/contracts/permissive_token.json`. The TK-003 happy path runs against the shared OnceCell-cached contract; the negative variants opt into a fresh deploy.
- **Estimated complexity**: L (the JSON template assembly is the long pole; per-test harness orchestration is M)
- **Rationale**: Without an asserted register-side case, every other TK-NNN entry rests on an unasserted assumption. This case exercises the `register_token_contract_via_sdk` helper from Wave G (previously tracked as Gap-T1).

#### TK-004 — Token transfer fee accounting & balance round-trip
- **Status**: green — `tests/e2e/cases/tk_004_token_transfer_round_trip.rs` (Wave 2-β; `#[ignore]`-tagged, runs on demand against testnet; PASS in v47).
- **Priority**: P0
- **Wallet feature exercised**: `wallet/identity/network/tokens/transfer.rs:21` (`token_transfer_with_signer`).
- **DET parallel**: `token_tasks.rs:359` (`step_transfer`).
- **Preconditions**: TK-003 + a minted balance on `identity_a` (mint via `token_mint_with_signer` — itself covered in TK-005). Two identities (`identity_a`, `identity_b`).
- **Scenario**:
  1. `setup_with_token_and_two_identities()` returns `(token, owner=A, peer=B)` (shared OnceCell-cached contract).
  2. Owner mints `100_000` to self.
  3. Owner transfers `40_000` to B with `public_note = Some("e2e-tk006")`.
  4. Wait for sync; read both balances; read owner's credit balance.
- **Assertions**:
  - `token_balance(A, contract, 0) == 60_000` exactly (mint − transfer).
  - `token_balance(B, contract, 0) == 40_000` exactly.
  - `A.credit_balance` decreased by `transfer_fee > 0` only (token transfer pays fees in credits, not in tokens).
  - Returned `TransferResult` carries `actual_fee > 0`.
- **Negative variants**:
  - Transfer amount exceeds balance → typed insufficient-tokens error.
  - Transfer to self (A → A) → pin contract: either accepted as a no-op (still pays fee) or rejected as "self-transfer disallowed".
  - Wrong `token_position` (e.g. position 7 on a single-token contract) → typed contract-validation error.
- **Harness extensions required**: `setup_with_token_and_two_identities`, `token_balance_of` helper (Wave G SDK-wrapper).
- **Estimated complexity**: M
- **Rationale**: Most-used token op. Pins the credit-fee vs. token-amount accounting separation that any refactor of the fee model would silently break.

#### TK-005 — Token mint + total-supply assertion
- **Status**: green — `tests/e2e/cases/tk_005_token_mint.rs` (Wave 2-γ; `#[ignore]`-tagged, runs on demand; PASS in v47).
- **Priority**: P1
- **Wallet feature exercised**: `wallet/identity/network/tokens/mint.rs:19` (`token_mint_with_signer`).
- **DET parallel**: `token_tasks.rs:305` (`step_mint`).
- **Preconditions**: TK-003; owner identity with ≥ `100_000_000` credits.
- **Scenario**:
  1. `setup_with_token()` returns `(token, owner)` (shared OnceCell-cached contract).
  2. Read pre-mint `token_supply(contract, 0)` (== 0 for a base-supply-zero token).
  3. Owner mints `500_000` to self with `recipient_id: None`.
  4. Owner mints `50_000` to self with `recipient_id: Some(owner_id)` (explicit-recipient sub-case).
  5. Read post-mint supply and owner balance.
- **Assertions**:
  - `token_supply(contract, 0) == 550_000` after both mints.
  - `token_balance(owner, contract, 0) == 550_000`.
  - Both `MintResult.actual_fee > 0`.
- **Negative variants**:
  - Unauthorised mint (non-owner identity attempts) → typed authorisation error. **DET parallel: `token_tasks.rs:756` (`tc_065_mint_unauthorized`).**
  - Mint with `amount = 0` → pin contract (reject with "amount must be positive" vs. accept as fee-only no-op).
  - Mint that would exceed `max_supply` → typed error.
  - Mint to a non-existent identity (`recipient_id: Some(garbage)`) → typed error.
- **Harness extensions required**: TK-003 helpers; `register_extra_identity` for the unauthorised sub-case; supply accessor.
- **Estimated complexity**: M
- **Rationale**: Pins both the supply bookkeeping and the authorisation gate (TC-065 in DET is one of the few negative tests that already exists; we mirror it).

#### TK-005b — Mint with `recipient_id != self`
- **Status**: green — `tests/e2e/cases/tk_005b_token_mint_to_other.rs` (Wave 2-γ; `#[ignore]`-tagged, runs on demand; PASS in v47).
- **Priority**: P2
- **Wallet feature exercised**: `wallet/identity/network/tokens/mint.rs:19` `recipient_id: Some(other)` branch.
- **DET parallel**: tested implicitly in DET via `mint_to: Some(identity.id)`; the cross-identity case isn't exercised explicitly.
- **Preconditions**: TK-003 helper with `minting_allow_choosing_destination = true`; owner + second identity.
- **Scenario**:
  1. Setup token (`allow_choose_destination = true`); register second identity.
  2. Owner mints `100` with `recipient_id: Some(second.id)`.
- **Assertions**:
  - `token_balance(second, contract, 0) == 100`.
  - `token_balance(owner, contract, 0) == 0` (mint went to the recipient, not owner).
  - Total supply == `100`.
- **Negative variants**:
  - Mint with `recipient_id` on a contract that has `allow_choose_destination = false` → typed validation error (build a separate token contract with this rule for the negative — fresh contract, opt out of the shared OnceCell).
- **Harness extensions required**: TK-003 helpers; `register_extra_identity`; supply accessor.
- **Estimated complexity**: S
- **Rationale**: Pins the cross-identity destination contract (an Option-branch the DET tests don't split).

#### TK-006 — Token burn + total-supply decrement
- **Status**: green — `tests/e2e/cases/tk_006_token_burn.rs` (Wave 2-γ; `#[ignore]`-tagged, runs on demand; PASS in v47). Note: `#[ignore]` reason flags a possible `wait_for_balance` flake shared with Found-025.
- **Priority**: P1
- **Wallet feature exercised**: `wallet/identity/network/tokens/burn.rs:19` (`token_burn_with_signer`).
- **DET parallel**: `token_tasks.rs:330` (`step_burn`).
- **Preconditions**: TK-003; owner with `≥ 1_000` token balance (mint inside the test).
- **Scenario**:
  1. `setup_with_token()`; owner mints `1_000`.
  2. Read pre-burn supply.
  3. Owner burns `100`.
  4. Read post-burn supply and balance.
- **Assertions**:
  - Owner balance: `1_000 → 900`.
  - Total supply: `1_000 → 900`.
  - `BurnResult.actual_fee > 0`.
- **Negative variants**:
  - Burn more than balance → typed insufficient-tokens error.
  - Burn `amount = 0` → pin contract.
  - Burn without authority (when ChangeControlRules disallow caller) → typed error. (Note: DET's permissive contract has `manual_burning_rules: ContractOwner` — non-owner burn fails. This sub-case uses the second identity.)
- **Harness extensions required**: TK-003 helpers.
- **Estimated complexity**: M
- **Rationale**: Symmetric partner of TK-005. Together they validate supply conservation across mint+burn pairs.

#### TK-007 — Freeze identity for token (admin action)
- **Status**: red-real-fail — `tests/e2e/cases/tk_007_token_freeze.rs` (Wave 2-δ; `#[ignore]`-tagged). PASS in v46; FAIL in v47 with `wait_for_balance timed out after 120s` during two-identity token setup. Root cause: network latency / testnet flake (possibly Found-025 race under parallelism). Not a code regression from v47. `#[ignore]` reason also notes the Found-025 upstream race.
- **Priority**: P1
- **Wallet feature exercised**: `wallet/identity/network/tokens/freeze.rs:18` (`token_freeze_with_signer`).
- **DET parallel**: `token_tasks.rs:389` (`step_freeze`).
- **Preconditions**: TK-003 with two identities (owner = admin, target = peer); peer has a non-zero token balance (transfer some over before freeze).
- **Scenario**:
  1. Setup token + two identities; mint to owner; owner transfers `200` to peer.
  2. Owner calls `token_freeze_with_signer(contract, 0, owner_id, peer_id, …)`.
  3. Wait for sync.
  4. Peer attempts `token_transfer_with_signer(contract, 0, peer, owner, 50, …)`.
- **Assertions**:
  - Step 4 fails with a typed "frozen balance / cannot transfer" error class.
  - Peer's token balance unchanged after the failed transfer.
  - `token_frozen_balance_of(peer, fixture) == Some(200)` (via Wave G helper).
  - `FreezeResult.actual_fee > 0`.
- **Negative variants**:
  - Non-admin attempts to freeze → typed authorisation error.
  - Freeze an already-frozen identity → pin contract (idempotent vs. typed "already frozen" error).
- **Harness extensions required**: TK-003 helpers; `register_extra_identity`.
- **Estimated complexity**: M
- **Rationale**: Freeze is the canonical regulatory primitive. Without explicit coverage, a regression that turns freeze into a no-op would only surface as "users complain transfers work after we froze them".

#### TK-008 — Unfreeze identity for token
- **Status**: green — `tests/e2e/cases/tk_008_token_unfreeze.rs` (Wave 2-δ; `#[ignore]`-tagged; PASS in v47).
- **Priority**: P1
- **Wallet feature exercised**: `wallet/identity/network/tokens/unfreeze.rs:18` (`token_unfreeze_with_signer`).
- **DET parallel**: `token_tasks.rs:419` (`step_unfreeze`).
- **Preconditions**: TK-007 setup, post-freeze state.
- **Scenario**:
  1. Re-use TK-007's frozen state.
  2. Owner calls `token_unfreeze_with_signer(contract, 0, owner_id, peer_id, …)`.
  3. Peer retries the transfer that was rejected in TK-007.
- **Assertions**:
  - Step 3 succeeds; peer balance decremented; owner balance incremented.
  - `UnfreezeResult.actual_fee > 0`.
  - `token_frozen_balance_of(peer, fixture)` is `None` or `0` (via Wave G helper).
- **Negative variants**:
  - Unfreeze an identity that was never frozen → pin contract (idempotent vs. typed error).
  - Non-admin unfreeze → typed auth error.
- **Harness extensions required**: same as TK-007.
- **Estimated complexity**: S (composes with TK-007)
- **Rationale**: Round-trip pin: freeze + unfreeze must restore exactly the pre-freeze state.

#### TK-009 — Destroy frozen funds
- **Status**: green — `tests/e2e/cases/tk_009_token_destroy_frozen.rs` (Wave 2-δ; `#[ignore]`-tagged; PASS in v47).
- **Priority**: P1
- **Wallet feature exercised**: `wallet/identity/network/tokens/destroy_frozen_funds.rs:20` (`token_destroy_frozen_funds_with_signer`).
- **DET parallel**: `token_tasks.rs:452` (`step_destroy_frozen`).
- **Preconditions**: TK-007 frozen state; total supply recorded.
- **Scenario**:
  1. Compose with TK-007: peer has frozen balance `200`.
  2. Owner calls `token_destroy_frozen_funds_with_signer(contract, 0, owner_id, peer_id, …)` — note no `amount` parameter; the call destroys the full frozen balance.
  3. Read post-destroy supply, peer balance, and frozen balance.
- **Assertions**:
  - Peer balance == `0`.
  - Total supply decreased by exactly `200`.
  - `DestroyFrozenFundsResult.actual_fee > 0`.
  - Subsequent unfreeze would have nothing to unfreeze (`token_frozen_balance_of` returns `None`).
- **Negative variants**:
  - Destroy on a not-frozen identity → typed error.
  - Non-admin destroy → typed auth error.
- **Harness extensions required**: TK-003 + TK-007 chain.
- **Estimated complexity**: M
- **Rationale**: Destroy-frozen-funds is the irreversible "burn the rule-breaker's bag" action — the negative-supply consequence must be pinned.

#### TK-010 — Pause and resume token (emergency action)
- **Status**: green — `tests/e2e/cases/tk_010_token_pause_resume.rs` (Wave 2-ε; `#[ignore]`-tagged, runs on demand; PASS in v47). Uses the shared OnceCell-cached contract; the `start_paused = true` variant (TK-paused-on-create) remains deferred.
- **Priority**: P1
- **Wallet feature exercised**: `wallet/identity/network/tokens/pause.rs:19`, `wallet/identity/network/tokens/resume.rs:18`.
- **DET parallel**: `token_tasks.rs:501` (`step_pause`), `token_tasks.rs:529` (`step_resume`).
- **Preconditions**: TK-003 with two identities; both have a non-zero token balance.
- **Scenario**:
  1. Setup token + two identities; mint to owner; transfer some to peer.
  2. Owner calls `token_pause_with_signer(contract, 0, owner_id, …)`.
  3. Owner attempts `token_transfer_with_signer(...)` — should be rejected.
  4. Owner calls `token_resume_with_signer(contract, 0, owner_id, …)`.
  5. Owner retries the transfer.
- **Assertions**:
  - Step 3 fails with typed "token paused" error class.
  - Step 5 succeeds.
  - Both `EmergencyActionResult.actual_fee > 0`.
  - `token_is_paused_of(fixture) == true` after pause, `false` after resume (via Wave G helper).
- **Negative variants**:
  - Pause an already-paused token → pin contract (idempotent vs. typed error).
  - Non-admin pause → typed auth error.
- **Harness extensions required**: TK-003 helpers; second identity.
- **Estimated complexity**: M
- **Rationale**: Pause is the kill switch. Pinning both directions (pause-blocks, resume-restores) catches the "resume forgot to clear the flag" regression class.

#### TK-011 — Set price + direct purchase round-trip
- **Status**: green — `tests/e2e/cases/tk_011_token_price_purchase.rs` (Wave 2-ε; `#[ignore]`-tagged, runs on demand; PASS in v47). Note: `#[ignore]` reason flags a possible `wait_for_balance` flake shared with Found-025.
- **Priority**: P1
- **Wallet feature exercised**: `wallet/identity/network/tokens/set_price.rs:26` (`token_set_price_with_signer`); `wallet/identity/network/tokens/purchase.rs:25` (`token_purchase_with_signer`).
- **DET parallel**: `token_tasks.rs:557` (`step_set_price`); `token_tasks.rs:588` (`step_purchase`).
- **Preconditions**: TK-003; owner with mintable supply; buyer identity (= second identity) with `≥ 50_000_000` credits.
- **Scenario**:
  1. Setup token; owner mints `1_000` to self.
  2. Owner sets pricing schedule to `Some(SinglePrice(1_000))` (1 000 credits per token).
  3. Buyer calls `token_purchase_with_signer(contract, 0, buyer_id, amount=10, total_agreed_price=10_000, …)`.
  4. Read post-purchase balances on owner and buyer.
- **Assertions**:
  - Buyer's token balance: `0 → 10`.
  - Owner's token balance: `1_000 → 990` (purchase reduces seller stock).
  - Buyer's credit balance decreased by `10_000 + purchase_fee`.
  - Owner's credit balance increased by `10_000` (purchase price arrives as credits, minus protocol fees per the pricing-schedule spec).
  - `SetPriceResult.actual_fee > 0`; `DirectPurchaseResult.actual_fee > 0`.
- **Negative variants**:
  - Buyer submits `total_agreed_price` lower than chain pricing → typed price-mismatch / over-budget error (this is the on-chain race-protection contract).
  - Purchase before any price is set → typed "no pricing schedule" error.
  - Set price to `None` (clear schedule) then buyer attempts purchase → typed "no pricing schedule" error.
- **Harness extensions required**: TK-003 helpers; second identity with credits.
- **Estimated complexity**: L (two related transitions, two-side balance bookkeeping, on-chain price race assertion).
- **Rationale**: Direct purchase is the only money-flow primitive on the wallet that crosses two identities AND moves both credits and tokens in one transition. Pricing-race protection (`total_agreed_price` mismatch) is the headline correctness property.

#### TK-012 — Update token config (single ChangeItem mutation)
- **Status**: green — `tests/e2e/cases/tk_012_token_update_config.rs` (Wave 2-ε; `#[ignore]`-tagged, runs on demand; PASS in v47). Single-ChangeItem mutation against a fresh deploy to keep the shared OnceCell fixture immutable.
- **Priority**: P2
- **Wallet feature exercised**: `wallet/identity/network/tokens/update_config.rs:20` (`token_update_config_with_signer`).
- **DET parallel**: `token_tasks.rs:617` (`step_update_config`).
- **Preconditions**: TK-003; owner identity. Note the shared OnceCell contract caches `max_supply` for cross-test reads — this case uses a fresh deploy to avoid mutating the shared fixture under other tests.
- **Scenario**:
  1. Setup token (fresh deploy) with `max_supply = Some(1_000_000_000_000_000)`.
  2. Owner calls `token_update_config_with_signer(contract, 0, owner, ChangeItem::MaxSupply(Some(2_000_000_000_000_000)), …)`.
  3. Re-fetch the contract; read the token's `max_supply`.
- **Assertions**:
  - Returned contract reflects the new `max_supply`.
  - Contract version (or token-config version, whichever DPP increments) advanced.
  - `ConfigUpdateResult.actual_fee > 0`.
- **Negative variants**:
  - Update with `MaxSupply(Some(< current_supply))` → typed error.
  - Update with a `ChangeItem` variant disallowed by ChangeControlRules → typed auth error.
  - Non-admin update → typed auth error.
- **Harness extensions required**: TK-003 helpers (fresh-deploy variant); helper to re-fetch the contract bytes after the change.
- **Estimated complexity**: M
- **Rationale**: `TokenConfigurationChangeItem` is open-ended (DPP grows it over time). One pinned variant (`MaxSupply`) catches schema-drift across DPP bumps; specific high-risk variants get their own follow-up cases.

#### TK-013 — Token claim from pre-programmed distribution
- **Status**: green — `tests/e2e/cases/tk_013_token_claim_pre_programmed.rs` (Wave 2-ζ; `#[ignore]`-tagged, runs on demand; PASS in v47). Uses a fresh deploy with `distribution_rules` override (not the shared OnceCell), since the distribution config is per-test.
- **Priority**: P2
- **Wallet feature exercised**: `wallet/identity/network/tokens/claim.rs:18` (`token_claim_with_signer`).
- **DET parallel**: `token_tasks.rs:702` (`tc_064_estimate_perpetual_rewards`) — DET only tests the *estimate* path because their `shared_token` has no perpetual; the actual claim flow is uncovered in DET. We propose to cover it.
- **Preconditions**: a token deployed with pre-programmed distribution: epoch 0 at a past timestamp granting `100` tokens to the configured beneficiary identity (= owner).
- **Scenario**:
  1. `setup_with_token_and_pre_programmed_distribution()` returns `(token, owner)` with a distribution event already eligible.
  2. Owner calls `token_claim_with_signer(contract, 0, owner_id, distribution_type=PreProgrammed, …)`.
  3. Read post-claim balance.
- **Assertions**:
  - Owner balance increased by exactly the documented per-epoch payout (`100`).
  - `ClaimResult.actual_fee > 0`.
  - Second claim within the same epoch returns a typed "already claimed" / "no claimable amount" error.
- **Negative variants**:
  - Identity with no distribution rights claims → typed error.
  - Claim on a contract with no distribution configured → typed error.
- **Harness extensions required**: TK-003 helpers extended with a `with_pre_programmed_distribution(epoch_zero_at, payout)` variant; `token_balance_of` helper (Wave G SDK-wrapper).
- **Estimated complexity**: L (the contract config is the non-trivial part — pre-programmed distribution JSON shape).
- **Rationale**: Claim is silent on failure — the balance just doesn't move. Pre-programmed-distribution variant dodges the live-time perpetual-distribution wait, putting the test inside CI runtime budget. The live-perpetual sibling (TK-002) stays out of the synchronous tier.

#### TK-014 — Group-action gateway: queue a mint, list pending, co-sign
- **Status**: red-real-fail — `tests/e2e/cases/tk_014_token_group_action.rs` (Wave 2-ζ; `#[ignore]`-tagged, runs on demand). PASS in v47; FAIL in v53 with `wait_for_balance timed out after 120s` at `tk_014_token_group_action.rs:109` (`setup_with_per_identity_funding`, three identities) — funding chain-confirmed before the wait, then the SDK address-sync silently discarded the update; group-action / co-sign code never ran. Root cause: same Found-025 (L273) address-sync silent-discard as TK-001, amplified by 14-thread concurrency (TK-014's 3-way funding churn is the peak-pressure case); not a group-action regression (sibling TK-009/TK-010/TK-012 green same run). Hardened by the same shared-gate proof-verified-read-back fix as TK-001 (see changelog). Live re-validation deferred to the combined v54 run. Once green, the test uses a fresh deploy with `main_control_group` and `groups` populated; spins three identities (proposer + two co-signers) and asserts the proposer's mint is non-final, that pending lists it, and that the co-sign produces the synchronous group MintResult.
- **Priority**: P2
- **Wallet feature exercised**: `wallet/identity/network/tokens/mint.rs:19` (`token_mint_with_signer`) with `group_info: Some(...)`; read-side `wallet/tokens/group_queries.rs::pending_group_actions_external` and `group_action_signers_external`.
- **DET parallel**: none direct in `tests/backend-e2e/token_tasks.rs` (DET's contract uses `groups: BTreeMap::new()`); coverage exists in DET production code.
- **Preconditions**: token contract with `mint_rules` requiring group action and `groups` populated with a group containing three identities.
- **Scenario**:
  1. Identity A proposes a mint via `token_mint_with_signer(..., group_info: Some(NewGroupAction(...)))`.
  2. Read `pending_group_actions_external(...)` — assert one entry, status `Open`, params == proposed mint.
  3. Identity B co-signs by re-issuing `token_mint_with_signer(..., group_info: Some(ExistingGroupAction(action_id)))`.
  4. Read `pending_group_actions_external(...)` — status now `Closed`/`Approved`; mint applied; supply increased.
- **Assertions**:
  - After step 1: pending list contains the proposal; recipient balance unchanged.
  - After step 3: pending list shows action closed; recipient balance increased by minted amount; total supply increased.
  - `MintResult.actual_fee > 0` on both proposer and co-signer.
- **Negative variants**:
  - Co-sign by a non-member → typed auth error.
  - Co-sign with a parameter mismatch (different amount) → typed mismatch error.
- **Harness extensions required**: TK-003 with group config; `setup_three_identities` helper; group-discovery accessor wiring.
- **Estimated complexity**: L
- **Rationale**: Group-gated actions are an entire class of bug surface (sign-thresholds, parameter binding). One pinned end-to-end case unlocks the rest as cheap variants in a follow-up.

### Core / SPV (CR)

SPV is **enabled by default** in the harness (Task #15 / Wave E complete: `SpvContextProvider`
is wired in `harness.rs`, `SpvHealth::status()` accessor is available). The suite has been
validated SPV-on since v17; v21 (current) runs SPV-on. The env var
`PLATFORM_WALLET_E2E_DISABLE_SPV=1` is an **escape hatch only** for testnet ChainLock-cycle
outages (rust-dashcore #470) — it is NOT the operating mode. Any documentation or config that
implies SPV-off is the default is incorrect.

#### CR-001 — SPV mn-list sync readiness
- **Priority**: P1
- **Status**: Pass — `tests/e2e/cases/cr_001_spv_mn_list_sync_readiness.rs`
- **Wallet feature exercised**: `manager::accessors::spv()` returning a started `SpvRuntime`; mn-list sync internals.
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/spv_wallet.rs:14` (`test_spv_sync_and_create_wallet`).
- **Preconditions**: SPV enabled in `harness::E2eContext::build` (block at `harness.rs:200-218` is active).
- **Scenario**:
  1. Wait `<= 180s` for `spv::wait_for_mn_list_synced` to return.
  2. Read mn-list height.
- **Assertions**: mn-list height > 0; SPV runtime reports `Ready` state.
- **Negative variants**: zero peers reachable → harness fails fast with explicit error (not a silent infinite wait).
- **Harness extensions required**: `SpvContextProvider` swap is done; `SpvHealth::status() -> Enum` accessor is available.
- **Estimated complexity**: M
- **Rationale**: Foundation for every other Core test — guarantees the SPV layer is alive before any Core operation runs.

#### CR-002 — Core wallet receive address derivation
- **Priority**: P1
- **Status**: Not implemented — TBD test file.
- **Wallet feature exercised**: `wallet/core/wallet.rs:59` (`next_receive_address_for_account`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/core_tasks.rs:14` (`test_tc001_refresh_wallet_info_core_only`).
- **Preconditions**: CR-001 ready.
- **Scenario**: derive 5 receive addresses on account `0`; assert distinctness; assert `network() == bank.network()`.
- **Assertions**: 5 distinct `Address`es; consistent network prefix.
- **Negative variants**: derive on non-existent account → typed error.
- **Harness extensions required**: `TestCoreWallet` helper (SPV runtime is now available).
- **Estimated complexity**: M
- **Rationale**: Catches Core-account derivation regressions independently of broadcast/sync.

#### CR-003 — Asset-lock-funded identity registration (full path)
<!-- merge note: both sides flip CR-003 to PASS. Kept theirs' detailed Status (file path, env-gate behaviour, funding-amount + operator-actionable error contract, bank Core address logging) and theirs' more precise Wallet-feature exercise list (build.rs:39 + build.rs:285 + registration.rs:59). Folded in HEAD's distinct Core-sweep-teardown-best-effort fact as a separate Status sentence so it isn't lost. -->
- **Priority**: P2 (post-Task #15)
- **Status**: Pass — `tests/e2e/cases/cr_003_asset_lock_funded_registration.rs` (`#[ignore]`-tagged; harness init blocks on the **default-on** `PLATFORM_WALLET_E2E_BANK_CORE_GATE`). Builds the asset-lock tx via `setup_with_core_funded_test_wallet(TEST_WALLET_CORE_FUNDING)`, waits for the IS-lock, registers the identity, and pins on-chain identity existence + `tracked_asset_locks` recording + Core-balance decrement (lock amount + fee, in duffs). End-to-end runs require the bank's Core (Layer-1) primary receive address to hold at least `TEST_WALLET_CORE_FUNDING + CORE_TX_FEE_RESERVE` (≈ 200_010_000 duffs ≈ 2.0001 DASH testnet); under-funded surfaces as `FrameworkError::Bank` with the bank's Core address embedded so the operator-actionable "top up at &lt;addr&gt;" message reaches the test log unchanged. The bank Core address is logged once per process at framework init under the `platform_wallet::e2e::bank` target. Core-sweep teardown is best-effort: any teardown sweep failure is logged and skipped rather than failing the test.
- **Wallet feature exercised**: `wallet/asset_lock/build.rs:39` (`build_asset_lock_transaction`) + `wallet/asset_lock/build.rs:285` (`create_funded_asset_lock_proof`) + `wallet/identity/network/registration.rs:59` (`register_identity_with_funding_external_signer` driving `IdentityFundingMethod::FundWithWallet`).
- **DET parallel**: `dash-evo-tool/tests/backend-e2e/core_tasks.rs:132` (`test_tc004_create_registration_asset_lock`).
- **Preconditions**: CR-001 + a Core-funded test wallet (operator funds via testnet faucet).
- **Scenario**: build asset-lock tx; wait for instant-lock; register identity.
- **Assertions**: identity exists on-chain; asset-lock recorded in `tracked_asset_locks`; Core balance decreased by lock amount + fee.
- **Negative variants**: insufficient Core balance; chain re-org of asset-lock tx (P2 — manual).
- **Harness extensions required**: faucet adapter; Core-funded wallet helper.
- **Estimated complexity**: L
- **Rationale**: Mirrors DET's existing canonical Identity-create coverage. Lower priority than ID-001 because address-funded is the path with no other coverage in the workspace.
- **Operator notes**: First cold-cache run takes ~15 minutes because SPV walks compact filters from genesis (~1.47M testnet blocks). Subsequent runs reuse the on-disk cache and complete in seconds. The harness gates init on `PLATFORM_WALLET_E2E_BANK_CORE_GATE` — **default-on with a 900s deadline**, waiting for the bank's confirmed Core balance to become non-zero so CR-003 doesn't race a cold-cache scan and see `core_balance_confirmed=0` mid-scan. Set the var to `0` (or `disabled` / `false` / `off`) to opt out for Platform-only suites; set a positive integer to override the timeout in seconds. Set `RUST_LOG=info,platform_wallet::e2e::wait=info` to see scan-progress lines (`scan_height` vs `scan_tip`) every 30s.

#### CR-004 — Legacy BIP32 account: balance + UTXO state updates after spend

- **Priority**: P1 — pins symmetric BIP-32 spent-marking + upstream sub-dust fold
- **Status**: passing-as-regression — Layer 1 (next_unused idempotency) fixed at `1c4c8a76f4`; Layer 2 test-side dust-threshold mismatch fixed in QA-901 (2026-05-14). The test now pins (a) post-broadcast `check_core_transaction` correctly marks every consumed BIP-32 UTXO spent (symmetric with the BIP-44 path through TransactionRouter → ManagedAccountCollection → check_transaction_for_match → update_utxos), and (b) the upstream sub-dust fold at `transaction_builder.rs:294` (rev `5313086…`, threshold `546` duffs) prevents emitting a stray change UTXO so the send-all truly drains the account.
- **Root cause history** (from Marvin's cr_004, QA-008, and QA-901 investigations): two distinct test-side defects, both now fixed.
- **Two layered fixes**:

  **Layer 1 (fixed at `1c4c8a76f4`):** `key-wallet::AddressPool::next_unused` is **idempotent by design** — it returns the same "current unused frontier" address until something external marks that address used. The upstream unit test `address_pool.rs:test_next_unused` explicitly asserts `addr1 == addr2` on two consecutive calls to `next_unused` on a freshly seeded pool; advancement requires an intervening `mark_used`. CR-004 originally called `next_receive_address` twice on a fresh wallet WITHOUT an intervening spend and asserted the two addresses differ — inverting the documented upstream contract. Fix: use the multi-variant `next_receive_addresses(count=2, advance=true)` call (the upstream `next_unused_multiple` path via `ManagedCoreFundsAccount::next_receive_addresses`) to satisfy the idempotent-by-design contract. Ref: `key-wallet/src/managed_account/address_pool.rs:521–540` and `:1196–1214`, audited at SHA `d6dd5da`.

  **Layer 2 (fixed in QA-901, 2026-05-14):** The test previously asserted
  `bip32_count_post == 0` while sending with a `2_500`-duff headroom under the false
  belief that the upstream P2PKH dust threshold was `2_730`. TRACE re-investigation
  confirmed the actual upstream gate at
  `rust-dashcore/key-wallet/src/wallet/managed_wallet_info/transaction_builder.rs:294`
  (rev `5313086…`) is `if change_amount > 546`. With observed testnet fees in
  `[226, 500]` duffs for a 2-in/2-out P2PKH transaction, a `2_500`-duff headroom left
  change in `[2_000, 2_274]` duffs — well above `546`, so the builder correctly
  emitted a change UTXO and the assertion fired. Fix: headroom `2_500 → 700`. New
  change range is `[200, 474]` duffs — fully sub-dust across the observed fee range —
  so the builder folds it into the fee and the BIP-32 account is truly drained.

  **Note on dash-evo-tool#845 reference:** The original CR-004 framing pinned
  dash-evo-tool#845 (stale-UTXO production bug after BIP-32 send-all). QA-901's TRACE
  run on the 2026-05-14 codebase confirms the symmetric BIP-32 spent-marking path
  (TransactionRouter → ManagedAccountCollection → check_transaction_for_match →
  update_utxos) is working correctly — the deterministic failure attributed to #845
  was actually the dust-threshold mismatch above. The test contract has been retargeted
  to "pin the symmetric BIP-32 spent-marking + upstream sub-dust fold" — both are
  invariants any downstream consumer (DET, SwiftExampleApp, Rust-SDK UIs) relies on.

- **Wallet feature exercised**: `wallet/core/wallet.rs:54` (`CoreWallet::balance`); `wallet/core/broadcast.rs:185` (`check_core_transaction` post-broadcast state mutation on `standard_bip32_accounts`).
- **Bug repro (upstream)**: [dashpay/dash-evo-tool#845](https://github.com/dashpay/dash-evo-tool/issues/845) — historical reference; the originally-reported "send all leaves stale UTXOs" surface on `rs-platform-wallet` does not reproduce in the current codebase per QA-008 (2026-05-12) and QA-901 (2026-05-14) TRACE runs. The symmetric BIP-32 spent-marking path works correctly. Any remaining DET-side surface lives in dash-evo-tool's own UI refresh path, outside this suite's contract surface. This test now pins the BIP-32 spent-marking + sub-dust fold contracts in `rs-platform-wallet` as a passing-as-regression guard against future drift.
- **DET parallel**: none yet — DET is the affected consumer; this test pins the contract on the rs-platform-wallet side so a fix becomes verifiable from a single repository.
- **Preconditions**: CR-001 + a Core-funded BIP32 legacy account (derivation path `m/44'/1'/0'`, `StandardAccountType::BIP32Account` at index `0`, stored under `wallet.accounts.standard_bip32_accounts`).
- **Scenario**:
  1. Create a wallet whose primary accounts include a **legacy BIP32 account** (`StandardAccountType::BIP32Account`). Fund it with at least 2 distinct UTXOs from the bank's Core funding helper so coin selection has more than one input to consider.
  2. Sync until `core_balance_confirmed > 0` for the legacy account.
  3. Build a "send all" Core transfer via `CoreWallet::send_to_addresses(StandardAccountType::BIP32Account, 0, outputs)` using the **advanced (explicit input selection)** path that consumes every UTXO on the legacy account; broadcast and wait for instant-lock or confirmation.
  4. Read the wallet's balance for the legacy account immediately after broadcast completes (re-use `wait_for_core_balance` from CR-003 with target `== 0`).
  5. Issue a second small transfer on the same legacy account via `send_to_addresses`.
- **Assertions**:
  - After step 3 + sync, the legacy account's confirmed balance equals `0` (or fee-only residue if the helper deducts the fee from outputs rather than inputs).
  - `standard_bip32_accounts[0].spendable_utxos(current_height)` returns an empty set — no entry that is confirmed and unspent.
  - The second `send_to_addresses` at step 5 fails with `PlatformWalletError::TransactionBuild` whose message identifies no spendable inputs, NOT with a stale-UTXO selection on already-spent outputs.
- **Negative variants**:
  - Mid-spend reorg of the broadcast (P2 — manual / mocked).
  - Send-all on a legacy account that is itself sourced from a watch-only descriptor (P2 — separate ticket if it diverges from the keyed path).
- **Harness extensions required**:
  - `setup_with_legacy_bip32_funded_account(funding_duffs, utxo_count)` helper analogous to the existing `setup_with_core_funded_test_wallet`, but using `StandardAccountType::BIP32Account` at index `0` (path `m/44'/1'/0'`).
  - `assert_no_unspent_utxos(account)` reusable assertion (or open-coded inline for now).
  - `wait_for_core_balance` already exists from CR-003 — re-use with `target == 0`.
- **Estimated complexity**: M
- **Rationale**: Pins the spend → state-update contract of the Core wallet for the legacy BIP32 account path. Without it, any future regression in `check_core_transaction`'s handling of `standard_bip32_accounts` (which dash-evo-tool, the SwiftExampleApp, and Rust-SDK-driven UIs all depend on) ships silently to consumers and is caught only when downstream consumers file issues. The bug is currently open upstream, so the test fails at first run — exactly the "pin invariants, including currently-broken ones" pattern used throughout this spec.
- **Operator notes**: Same SPV cold-cache caveat as CR-003 (~15 min on first run). The `PLATFORM_WALLET_E2E_BANK_CORE_GATE` default-on still applies. The legacy BIP32 account derivation must NOT cross-contaminate the wallet's default Core account UTXO set — assertions read `standard_bip32_accounts` slot state directly, not the wallet-aggregate balance.

### Asset Lock (AL)

This section covers primitive-level correctness of `AssetLockManager` — the internal component that coordinates asset-lock transaction building, UTXO selection, IS-lock waiting, and proof correlation. Asset-lock-funded feature flows (identity registration, identity top-up) are tested at the feature level under CR-003 and ID-002b respectively; the AL category pins the manager's invariants that those feature-level tests do not exercise, particularly concurrent-build behaviour. AL tests require a Core-funded test wallet and SPV, so they share the Wave E prerequisite with CR-003.

#### AL-001 — Concurrent asset-lock builds from same wallet

- **Priority**: P1
- **Status**: red-real-fail (shifted-failure-mode) — failure fingerprint `FinalityTimeout(<txid>)` at `al_001_concurrent_asset_lock_builds.rs:299` (task 1). Identical across v48, v49, v50 — no run-to-run drift. Blocked on Found-008 (platform-internal — tracked at dashpay/platform#3641).
- **Failure site**: `tests/e2e/cases/al_001_concurrent_asset_lock_builds.rs:299` — the `wait_for_asset_lock` / IS-lock poll on task 1's broadcast transaction.
- **Blocker**: Found-008 (`LockNotifyHandler::notify_waiters` at `packages/rs-platform-wallet/src/wallet/asset_lock/lock_notify_handler.rs:30`; see detail section below).
- **Wallet feature exercised**: `wallet/asset_lock/manager.rs::AssetLockManager` (concurrent-build path); transitively `wallet/asset_lock/build.rs::build_asset_lock_transaction` and `wallet/asset_lock/build.rs::create_funded_asset_lock_proof`. Driver: `wallet/identity/network/top_up.rs::top_up_identity_with_funding`.
- **DET parallel**: None — DET does not drive concurrent asset-lock builds from a single wallet.
- **Historical failure mode (coin-selection race — now closed)**:
  - Before `403d29c3c8`: concurrent tasks raced to grab UTXOs. The losing task would observe a balance-updated-but-UTXO-index-stale window and fail with `"Coin selection error: No UTXOs available for selection"` (v47 trace). In the worst case, both tasks obtained the same UTXO and produced a double-spend.
  - `403d29c3c8` applied a two-phase gate (balance check + spendable-UTXO count check). PR #3585's `OutpointReservations` system (integrated via `02cb61b30d`) closes the race definitively at the architecture level: concurrent callers filter spendable snapshots against an `Arc<Mutex<HashSet<OutPoint>>>` reservation set; the second caller short-circuits with `NoSpendableInputs` before build.
  - This surface is confirmed closed. Marvin's v50 audit found the failure fingerprint identical to v49 (pre-`02cb61b30d`-merge), validating that PR #3585 is orthogonal to AL-001's remaining gate.
- **Current failure mode (IS-lock notification race)**:
  1. Coin selection succeeds for both tasks (reservation guard working as intended).
  2. Task 1's asset-lock transaction broadcasts to mempool.
  3. Task 1 waits for the IS-lock (`InstantSend`) notification confirming quorum acceptance.
  4. The IS-lock event arrives at `LockNotifyHandler` but task 1's wait future never wakes. It times out at `FinalityTimeout`.
  - Root cause: `LockNotifyHandler::notify_waiters()` (in `packages/rs-platform-wallet/src/wallet/asset_lock/lock_notify_handler.rs:30`) calls `tokio::sync::Notify::notify_waiters()`. That method signals all currently-registered waiters but does NOT store a permit for waiters that register after the signal fires. If the IS-lock event arrives in the narrow window before task 1's wait future registers with the handler, the wakeup is permanently lost. There is no second IS-lock event for the same transaction.
  - This is **Found-008** — see the Found-008 detail section for the full spec.
- **Fix path** (in this repo — at `packages/rs-platform-wallet/src/wallet/asset_lock/lock_notify_handler.rs:30` and/or `packages/rs-platform-wallet/src/wallet/asset_lock/sync/proof.rs:367-418`):
  - **Option A** (minimal): change `Notify::notify_waiters()` → `Notify::notify_one()`. `notify_one` stores a pending permit when no waiter is currently registered; the next `notified().await` claim it immediately without waiting for a new event.
  - **Option B** (thorough): replace `Notify` with a `tokio::sync::broadcast` channel that retains the event for late subscribers within a bounded window.
  - After the fix lands (tracked at dashpay/platform#3641), AL-001 should turn green without any test-side changes.
- **Preconditions**:
  - CR-001 (SPV ready).
  - Core-funded test wallet. Implementation uses `N = 3` concurrent tasks, per-lock amount `100_000_000` duffs (0.001 DASH); Core funding floor ≈ 500_000_000 duffs (5 DASH testnet). Same `PLATFORM_WALLET_E2E_BANK_CORE_GATE` env gate as CR-003.
  - N pre-registered identities (each via address-funded `register_from_addresses` from the ID-001 helper). Concurrent top-ups target different identities to avoid colliding on Found-006.
- **Scenario**:
  1. `setup_with_core_funded_test_wallet(CONCURRENT_LOCK_FUNDING_TOTAL)` lands Core funds on the test wallet.
  2. Register N identities via the address-funded path (ID-001 helper); capture `identity_ids[N]` and `pre_balances[N]`.
  3. Spawn N concurrent tasks via `tokio::spawn` (NOT a sequential `for` loop):
     ```rust
     let handles: Vec<_> = identity_ids
         .iter()
         .map(|id| {
             let wallet = wallet.clone();
             let signer = signer.clone();
             tokio::spawn(async move {
                 wallet.top_up_identity_with_funding(
                     id.clone(),
                     IdentityFundingMethod::FundWithWallet { amount_duffs: LOCK_AMOUNT },
                     &signer,
                     None,
                 ).await
             })
         })
         .collect();
     ```
  4. `try_join_all(handles).await` — collect all N task outputs (fails today at step 4, task 1, with `FinalityTimeout` at `:299`).
  5. Fetch all N identities' chain balances post-top-up.
  6. Fetch the test wallet's Core balance.
  7. Read the `tracked_asset_locks` registry — collect the N asset-lock txids that landed.
- **Assertions** (expected post-fix):
  - All N task results are `Ok(_)` — every concurrent build succeeded.
  - The N asset-lock txids are all distinct (no `AssetLockManager` collision; `OutpointReservations` guards this).
  - `post_balances[i] >= pre_balances[i] + (LOCK_AMOUNT * 1000) - top_up_fee_max` for all `i` (where `1000` is `CREDITS_PER_DUFF`).
  - Test wallet's Core balance decreased by approximately `N × (LOCK_AMOUNT + asset_lock_fee + top_up_fee)` (within fee tolerance).
  - No `tracked_asset_locks` entry in `Failed` state.
  - No UTXO double-spend: input sets of the N asset-lock transactions are pairwise disjoint.
- **Why AL-001 stays in the spec**:
  - When the Found-008 fix lands (dashpay/platform#3641), AL-001 turns green with zero test-side changes. Acts as the canary.
  - Documents the historical coin-selection race surface: if a future refactor accidentally reopens the UTXO double-spend window, AL-001 will fail in a different way and flag it before production code is affected.
- **Negative variants (defer to follow-up AL-* cases)**:
  - `N >> available_utxos`: assert graceful `Wallet::InsufficientFunds`, not a double-spend.
  - One task panics mid-build: assert remaining tasks complete (no shared-state poisoning via `AssetLockManager`).
  - Concurrent build while a fourth task calls `recover_asset_lock_blocking`: assert no deadlock.
- **Notes / risks**:
  - Found-008 is the current gate. Found-012 (account-type tunnel vision in `validate_or_upgrade_proof`) is also on the path for non-BIP-44-funded builds.
  - Upstream `next_private_key` is non-idempotent (`mark_index_used` called before return at `managed_account_trait.rs:480`), so concurrent builds do not collide on one-time-key derivation. Confirmed clean by Marvin's upstream audit.
  - Requires `PLATFORM_WALLET_E2E_BANK_CORE_GATE` (same as CR-003, default-on, 900 s deadline).
- **Harness extensions required**: same as CR-003 — `setup_with_core_funded_test_wallet`, `wait_for_asset_lock`; plus Wave A identity setup helpers (ID-001).
- **Estimated complexity**: L (~300 LOC including multi-identity setup + concurrent orchestration + multi-assertion validation).
- **Rationale**: `AssetLockManager` is critical-path code that every asset-lock-funded registration and top-up goes through, and it has never been exercised under concurrent load in a green test. CR-003's sequential single-build path does not validate the manager's locking, UTXO-reservation, or proof-correlation logic under concurrent callers. Any app driving concurrent top-ups or multi-identity registrations hits this path in production. AL-001 pins the contract those applications depend on, and documents both the historical UTXO-race surface (now closed) and the remaining IS-lock wakeup gap (Found-008, platform-internal — dashpay/platform#3641).

### Contracts (CT)

#### CT-001 — Document put: deploy a fixture data contract
- **Priority**: P1
- **Status**: STUB — placeholder for follow-up PR (Wave A + Wave C — contract fixture loader).
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
- **Status**: STUB — placeholder for follow-up PR (Wave A + Wave C).
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
- **Status**: STUB — placeholder for follow-up PR (Wave A + Wave C).
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
- **Status**: green — implemented in `cases/dpns_001_register_name.rs`; `#[ignore]`-gated, run with `cargo test -- --ignored`; PASS in v47.
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
- **Status**: STUB — placeholder for follow-up PR (Wave A + DPNS helpers).
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
- **Status**: STUB — placeholder for follow-up PR (Wave A + DPNS helpers).
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
- **Status**: STUB — placeholder for follow-up PR (no identity needed; resolver-only). Trivial once a DPNS resolution helper lands.
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
- **Status**: STUB — placeholder for follow-up PR (Wave A).
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
- **Status**: STUB — placeholder for follow-up PR (Wave A).
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
- **Status**: STUB — placeholder for follow-up PR (Wave A).
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
- **Status**: STUB — placeholder for follow-up PR (Wave A + Wave B for two identities).
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
- **Status**: STUB — placeholder for follow-up PR (Wave A + Wave B).
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
- **Status**: STUB — placeholder for follow-up PR (Wave A + DPNS contest helpers).
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
- **Status**: BLOCKED — needs harness refactor: masternode signer + operator-controlled mn-list participation. Re-evaluate once a regtest-with-masternodes harness is in scope.
- **Reason for deferral**: requires a masternode signer and operator-controlled mn-list participation; harness has no way to drive that today.
- **Action**: keep this row as a placeholder; revisit when a regtest-with-masternodes harness is in scope.

### Harness self-tests (Harness)

Cases in this subsection exercise the test harness itself (registry
serialisation, async cancellation safety, workdir isolation), not the wallet.
They live here because their failures masquerade as wallet bugs and the only
sane place to pin the harness contract is alongside the wallet contract.

#### Harness-G1a — Corrupted registry JSON: refuse to overwrite
- **Priority**: P2
- **Status**: STUB — placeholder for follow-up PR (pure-harness unit test on `framework/registry.rs`; no chain access required).
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
- **Status**: STUB — placeholder for follow-up PR (pure-harness unit test on `framework/registry.rs`).
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
- **Status**: STUB — placeholder for follow-up PR (cancellation-safety probe; needs structured `select!`-based cancellation harness).
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

#### Harness-ID-1 — `sweep_identities` regression: registered identities surrender credits at teardown
- **Priority**: P0
- **Status**: IMPLEMENTED — passing (parallel-safe). The `bank_gain <= pre_sweep_balance` upper-bound assertion is dropped — under parallel execution, sibling test sweeps flow into the bank concurrently, making the upper bound non-deterministic. The binding assertion is the lower-bound recovery check combined with the "no registry entry after teardown" guarantee.
- **Wallet feature exercised**: `tests/e2e/framework/cleanup.rs::sweep_identities` (was a no-op stub on `feat/rs-platform-wallet-e2e-cases`; implementation lands on the identity-tests-and-sweep branch).
- **DET parallel**: none.
- **Preconditions**: ID-001 helper available; bank identity configured for the sweep destination (per `bank_identity` env-var contract).
- **Scenario**:
  1. `let bank_pre = guard.base.ctx.bank().total_credits();`
  2. `let guard = setup_with_n_identities(2, 30_000_000).await?;`
  3. Do not issue any extra transfers. Capture `identity_a_pre` / `identity_b_pre` balances.
  4. `guard.teardown().await?`.
- **Assertions**:
  - For each registered identity, post-teardown `Identity::fetch(...).balance()` is `0` or below `min_input_amount` (pin whichever shape the `sweep_identities` implementation adopts; document the choice in the test comment).
  - `bank_post >= bank_pre - 2 * 30_000_000 - register_fees - sweep_fees - slack` (sweep recovers most of what was funded; no double-credit).
  - The persistent test-wallet registry has no entry for `guard.base.test_wallet.id()` after teardown.
- **Negative variants**:
  - Bank identity not configured → typed `IdentitySweepNoBank` error from teardown; registry entry retained for next-startup retry.
- **Harness extensions required**: `sweep_identities` lands on a sibling branch (this PR); this entry pins its contract on merge.
- **Estimated complexity**: S
- **Rationale**: Without a regression pin, a future refactor that reverts `sweep_identities` to `Ok(())` would slip past CI and identity credits would leak across runs until the bank starves.

### Found-bug pins (Found-NNN)

Bug-pin cases discovered during a QA-mindset audit of `packages/rs-platform-wallet/src/`.
Each entry names the contract violation, the proof shape that would catch it,
and what the fix should look like. The author of the production fix is a
separate concern; these entries pin the expected behaviour so the regression
becomes a test failure rather than a silent drift.

#### Found-001 — `auto_select_inputs_for_withdrawal` ignores `min_input_amount` floor
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `wallet/platform_addresses/withdrawal.rs:170` (`auto_select_inputs_for_withdrawal`).
- **Suspected bug**: The withdrawal-side auto-selector iterates every funded address (`balance > 0`) and inserts each into the selected map. Unlike `transfer.rs::auto_select_inputs` (which filters out balances `< min_input_amount`), the withdrawal helper has no `min_input_amount` floor. An address holding fewer credits than the protocol's per-input minimum will be selected, and the resulting transition trips `InputBelowMinimumError` at `validate_structure` time.
- **Preconditions**: a platform payment account holds at least one address with balance `> 0` but `< min_input_amount` (e.g. an address that absorbed dust on a prior partial sync).
- **Scenario**:
  1. Seed account with two funded addresses: `addr_A.balance = 100_000_000`, `addr_B.balance = min_input_amount - 1`.
  2. Call `withdraw(account_index, InputSelection::Auto, ..., DeductFromInput(0))`.
- **Assertions** (the proof shape):
  - The selector returns an `Err(PlatformWalletError::AddressOperation(_))` whose message references `min_input_amount`, OR the selector returns `Ok(map)` where every value is `>= min_input_amount`.
  - In NEITHER case does it return `Ok(map)` containing `addr_B → (min_input_amount - 1)`.
- **Expected** (after fix): mirror the transfer-side filter — exclude candidates below `min_input_amount` before constructing the input map; if the survivors don't cover the requested fee, error with a descriptive message.
- **Actual** (current code): the function selects `addr_B` unconditionally; the broadcast then fails with a generic protocol-validation error that doesn't name the cause.
- **Severity**: HIGH (per-input minimum is a hard protocol gate; user gets an opaque rejection instead of a clear wallet-side error)
- **Harness extensions required**: `auto_select_inputs_for_withdrawal` is a private helper; the test exercises it indirectly via `withdraw(InputSelection::Auto, ...)` and seeded balances. Needs a way to seed individual platform-payment addresses with a sub-minimum balance — likely via direct `set_address_credit_balance` on `ManagedPlatformAccount` for the test setup.
- **Estimated complexity**: S
- **Rationale**: The transfer path was hardened against this exact failure mode (see `auto_select_inputs` filter). Withdrawal silently drifted out of parity. Real-world trigger: a dust-tier address arrives mid-sync and the user attempts an "auto-select" withdrawal — the wallet builds an unspendable transition.

#### Found-002 — `auto_select_inputs_for_withdrawal` skips fee-target headroom check
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `wallet/platform_addresses/withdrawal.rs:170-235`.
- **Suspected bug**: The transfer-side `select_inputs_deduct_from_input` performs an explicit "fee target retains ≥ estimated_fee" check (Phase 3) before returning. The withdrawal-side helper checks only the aggregate `accumulated < estimated_fee` — i.e. that the *sum* of all inputs covers the fee. Under `[DeductFromInput(0)]` the fee is taken from the lex-smallest input's *remaining balance*, not the aggregate, so a selection where the lex-smallest input is fully consumed but other inputs cover the difference passes the helper's gate yet fails on chain — the same failure pattern PA-002b / commits `9ea9e7033c` and `687b1f86cd` pinned for transfer.
- **Preconditions**: a withdrawal account with at least one small input that becomes the lex-smallest "fee target" after BTreeMap insertion.
- **Scenario**:
  1. Seed account with `addr_A` (lex-smallest, balance == small amount equal to its own consumption with no fee headroom) and `addr_B` (large balance covering the rest).
  2. Call `withdraw(..., InputSelection::Auto, ..., DeductFromInput(0))`.
- **Assertions** (the proof shape):
  - The selector errors with a "fee headroom" message, OR after broadcast `validate_fees_of_event` would return `fee_fully_covered = false` (provable in a unit test by feeding the helper output to `deduct_fee_from_outputs_or_remaining_balance_of_inputs` exactly as PA-006 does for transfer).
- **Expected** (after fix): adopt the transfer helper's Phase-3 headroom check — confirm `lex-smallest-input.balance - lex-smallest-input.consumed >= estimated_fee` before returning.
- **Actual** (current code): the helper performs only an aggregate check; the chain-time deduction misdirects to an empty-remaining input.
- **Severity**: HIGH (drives users into the same chain-time `AddressesNotEnoughFundsError` class as platform #3040)
- **Harness extensions required**: same as Found-001 — fine-grained seeding of platform-payment account balances. A protocol-level reproduction (analogous to `pre_fix_buggy_selector_output_is_rejected_by_protocol_fee_deduction` in transfer's tests) is the simplest proof shape.
- **Estimated complexity**: M
- **Rationale**: Withdrawal lags transfer's hardening; the same regression class will silently re-emerge in withdrawal until the contract is pinned.

#### Found-003 — `addresses_with_balances` and `total_credits` only see the first platform-payment account
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `wallet/platform_addresses/wallet.rs:233` (`addresses_with_balances`), `wallet/platform_addresses/wallet.rs:271` (`total_credits`).
- **Suspected bug**: Both methods reach for `first_platform_payment_managed_account()` and return data from that single account. The doc comments make no mention of the "first account only" restriction (`addresses_with_balances` says "all platform addresses", `total_credits` says "total platform credits across all addresses"). Wallets with multiple platform-payment accounts (DIP-17 supports this) silently undercount.
- **Preconditions**: a wallet with two or more `PlatformPayment` accounts, each holding a non-zero balance on at least one address.
- **Scenario**:
  1. Construct a wallet with `WalletAccountCreationOptions` that yields two PlatformPayment accounts (account `0` and account `1`).
  2. Fund one address on account `0` with `40_000_000`; fund one address on account `1` with `60_000_000`.
  3. Read `wallet.platform().addresses_with_balances().await` and `wallet.platform().total_credits().await`.
- **Assertions** (the proof shape):
  - `addresses_with_balances` returns at least two entries (one from each account).
  - `total_credits == 100_000_000` (sum across both accounts).
- **Expected** (after fix): iterate `core_wallet.platform_payment_managed_accounts()` (or equivalent multi-account accessor) and aggregate.
- **Actual** (current code): returns only account-0 data; second account's `60_000_000` is invisible from these accessors.
- **Severity**: MEDIUM (UI-facing; the user sees a "wrong balance" without any error indication)
- **Harness extensions required**: a test wallet builder that requests multiple PlatformPayment accounts at creation. The existing `wallet_factory` defaults to one; a `WalletAccountCreationOptions` variant or test-only setup is needed.
- **Estimated complexity**: S
- **Rationale**: The "first account only" restriction is a load-bearing implicit assumption that nothing in the public API surface tells callers about. Multi-account support is documented at the wallet-creation layer; the readback must match.

#### Found-004 — `transfer` / `withdraw` / `fund_from_asset_lock` silently fall back to `address_index = 0` on lookup miss
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `wallet/platform_addresses/transfer.rs:157-167`, `wallet/platform_addresses/withdrawal.rs:142-152`, `wallet/platform_addresses/fund_from_asset_lock.rs:130-140`.
- **Suspected bug**: All three call sites build a `PlatformAddressBalanceEntry` whose `address_index` is computed via a `find_map(...).unwrap_or(0)` over the account's address pool. If the address truly is not in the pool (defensive case — e.g. caller passed an address that doesn't belong to the account), the entry persists with `address_index = 0`, mis-attributing the balance update to whichever address actually sits at index 0. The persister then writes the wrong row.
- **Preconditions**: an account containing at least one address at index `0`. A subsequent operation references an address NOT in the pool (e.g. via `Explicit` input that's foreign to this account).
- **Scenario**:
  1. Build account `A` with addresses `addr_at_0`, `addr_at_1`, `addr_at_2`.
  2. Construct a transfer / withdrawal / fund call referencing a `PlatformAddress` that is NOT in any of the account's pools but is otherwise well-formed.
  3. Inspect the returned `PlatformAddressChangeSet`.
- **Assertions** (the proof shape):
  - The changeset must NOT contain an entry with `(address: foreign_addr, address_index: 0)` — that's a corrupted persistence row.
  - Either the operation rejects with a typed error before producing a changeset entry, OR the entry omits the foreign address entirely.
- **Expected** (after fix): on `find_map(...) == None`, log + skip the entry instead of attributing it to index 0; or fail the call with a typed error pointing at the unknown address.
- **Actual** (current code): the entry is attributed to index 0 and written to the persister.
- **Severity**: MEDIUM (silent data corruption in the persister's address table; downstream readers think `addr_at_0`'s balance is whatever the SDK reported for the foreign address)
- **Harness extensions required**: a way to drive the call site with a foreign `PlatformAddress`. The transfer / fund paths accept `Explicit*` input maps so this is straightforward; the withdrawal path is per-account so requires a similar input-construction helper.
- **Estimated complexity**: S
- **Rationale**: `unwrap_or(0)` on a derivation-index lookup is the canonical "should have been a typed error" pattern. With three call sites identical, the regression class is broad.

#### Found-005 — `register_from_addresses` / `top_up_from_addresses` discard SDK-returned address balances and nonces
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `wallet/identity/network/register_from_addresses.rs:87-122`, `wallet/identity/network/top_up_from_addresses.rs:58`.
- **Suspected bug**: Both call sites pattern-match the SDK return as `(_address_infos, ...)` and drop the address-info map. `transfer()` and `withdraw()` (in `platform_addresses/`) consume this same map to update local balances + nonces. The TODO comment in `register_from_addresses.rs:139-143` admits the gap. As a result, addresses' cached `(balance, nonce)` go stale immediately after these calls — until the next BLAST sync round resolves them. A second operation against the same address before the sync uses a stale nonce and is rejected.
- **Preconditions**: a platform-funded address with a known nonce. Run two consecutive operations against it.
- **Scenario**:
  1. Fund `addr_A` on test wallet with `60_000_000`. Note the address's nonce (post-funding).
  2. Call `register_from_addresses({addr_A: 30_000_000}, ...)` — this consumes part of addr_A's balance and bumps its nonce on chain.
  3. Without an intervening BLAST sync, immediately call a second operation against `addr_A` (e.g. another `register_from_addresses` or a `transfer`).
- **Assertions** (the proof shape):
  - After step 2, `wallet.platform().addresses_with_balances()` reflects `addr_A`'s post-call balance (i.e. NOT the pre-call `60_000_000`).
  - The cached nonce for `addr_A` matches the chain-time nonce post-step-2.
  - Step 3 succeeds (would fail with a stale-nonce error today).
- **Expected** (after fix): mirror the `transfer()` pattern — walk `address_infos` and update each address's cached `AddressFunds` + emit a `PlatformAddressChangeSet` so the persister sees the updated nonce.
- **Actual** (current code): the map is dropped; local cache stays at pre-call values.
- **Severity**: MEDIUM (causes "spam-click" failures and surprises power users; not silent corruption but slow-to-recover staleness)
- **Harness extensions required**: a way to issue two back-to-back operations against the same input address with no sync between them.
- **Estimated complexity**: M (needs identity-signer + DPNS-style identity setup, then two consecutive identity-funding calls)
- **Rationale**: The TODO comment in the source admits the gap; a test pins it so the comment doesn't outlive the next refactor that touches these files.

#### Found-006 — `top_up_identity_with_funding` requires pre-created `IdentityTopUp { registration_index }` HD slot; absence yields confusing "not found" error
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `wallet/identity/network/top_up.rs:60-106`.
- **Two layered bugs** (QA-006 investigation, 2026-05-12):

  **Bug A — the visible precondition failure (what this pin actually tests):**
  `top_up_identity_with_funding` calls `create_funded_asset_lock_proof` with
  `AssetLockFundingType::IdentityTopUp { identity_index }`. Internally,
  `peek_next_funding_address` (`key-wallet/src/wallet/asset_lock/build.rs:163`,
  audited at SHA `d6dd5da`) does
  `wallet_info.accounts.identity_topup.get_mut(&identity_index)` and returns
  `Err("Identity top-up account for index N not found")` when the BTreeMap has no
  entry for that index. The map is populated only via
  `Wallet::add_account(AccountType::IdentityTopUp { registration_index: N }, None)`,
  which `WalletAccountCreationOptions::Default` never calls — `Default` delegates to
  `create_special_purpose_accounts` (`key-wallet/src/wallet/helper.rs:524-549`),
  which creates `IdentityRegistration`, `IdentityInvitation`,
  `IdentityTopUpNotBoundToIdentity`, and provider-key accounts, but no
  `IdentityTopUp { registration_index: N }` for any N. As a result, any test or
  caller that creates a wallet with `Default` and then calls
  `top_up_identity_with_funding` with `FundWithWallet` receives the "not found" error
  for every index, including 0. The error fires before any `topup_index`-routing logic
  is reached. This is on the critical path of ID-002b, AL-001, and this pin (Found-006)
  itself. Fix owner: upstream `key-wallet` (lazy-create the slot inside
  `peek_next_funding_address` / `build.rs:163`) OR the `rs-platform-wallet` wrapper
  (call `add_account(AccountType::IdentityTopUp { registration_index })` lazily before
  the lookup). The population point is
  `ManagedAccountCollection::insert_keys_bearing_account`
  (`key-wallet/src/managed_account/managed_account_collection.rs:307`).

  **Bug B — the original claim, blocked behind Bug A:**
  Once Bug A is fixed, `topup_index` routing to distinct HD slots is testable. The
  upstream `CreditOutputFunding` in
  `key-wallet/src/wallet/managed_wallet_info/asset_lock_builder.rs:42-49` exposes only
  `identity_index` for the `IdentityTopUp` variant; the canonical DIP-9 path
  `DerivationPath::identity_top_up_path(network, identity_index, top_up_index)` at
  `key-wallet/src/bip32.rs:1062-1077` takes a second index that the type system never
  plumbs. Whether `topup_index` actually routes to distinct HD slots cannot be confirmed
  until Bug A is fixed so the test can reach the routing logic. Bug B is retained as a
  follow-up assertion within this pin.

- **Preconditions**: an identity registered on testnet via the wallet; **the test must
  pre-provision `IdentityTopUp { registration_index }` HD slots before calling
  `top_up_identity_with_funding`** (v44-discovered prerequisite that Bilby is
  implementing test-side). Concretely, call
  `wallet.add_account(AccountType::IdentityTopUp { registration_index: 0 }, None)` and
  `wallet.add_account(AccountType::IdentityTopUp { registration_index: 1 }, None)` after
  wallet creation and before the first top-up attempt. Alternatively, use
  `WalletAccountCreationOptions::AllAccounts` or `SpecificAccounts` with those slots
  included.
- **Scenario**:
  1. Register identity `I` via `register_identity_with_funding_external_signer`.
  2. Pre-provision HD slots: `wallet.add_account(AccountType::IdentityTopUp { registration_index: 0 }, None)` and `{ registration_index: 1 }`.
  3. Call `top_up_identity(&I.id, topup_index=0, amount_duffs=A0, ...)`.
  4. Call `top_up_identity(&I.id, topup_index=1, amount_duffs=A1, ...)` — same identity, fresh `topup_index`.
- **Assertions** (the proof shape):
  - Without the precondition step, both calls fail with `"Identity top-up account for index N not found"` — this is the Bug A regression proof.
  - After precondition step, the two top-up calls produce DIFFERENT funding-output addresses (re-derived from different paths) — this is the Bug B assertion.
  - The two asset-lock transactions have different txids.
  - The doc claim about "successive top-ups for the same identity" is honoured — both calls succeed and credit the identity by `A0 + A1` total.
- **Expected** (after Bug A fix): `top_up_identity_with_funding` works without manual `add_account` preamble; after Bug B fix, `topup_index` routes to distinct HD paths.
- **Actual** (current code): any call with `WalletAccountCreationOptions::Default` fails at `build.rs:163` with a misleading "not found" error before any routing logic is reached.
- **Severity**: HIGH — Bug A is on the critical path of every asset-lock-funded top-up; affects ID-002b, AL-001, and this pin. The silent prerequisite provides no actionable guidance to callers.
- **Owner**: upstream `key-wallet` — lazy-create the `IdentityTopUp` slot inside `peek_next_funding_address` (`build.rs:163`) rather than erroring; OR `rs-platform-wallet` wrapper — call `add_account` lazily before the lookup. Population point: `ManagedAccountCollection::insert_keys_bearing_account` (`managed_account_collection.rs:307`).
- **Harness extensions required**: identity setup; access to the asset-lock transaction details (currently inside `AssetLockManager`); wallet `add_account` call prior to top-up.
- **Estimated complexity**: M
- **Rationale**: Bug A is pinned explicitly — a public API method that fails with a confusing "not found" error for a precondition that no default initialisation path satisfies is a DX contract violation. Bug B (the original `topup_index` ignored claim) is retained as the follow-up assertion once Bug A is fixed.

#### Found-007 — `PlatformAddressSyncManager::start` lacks a generation guard so a fast `start()` → `stop()` → `start()` can spawn parallel sync threads
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `manager/platform_address_sync.rs:189-224` (`start`).
- **Suspected bug**: `start()` checks `guard.is_some()` and bails early, then installs a fresh cancel token. On loop exit the spawned thread unconditionally writes `*guard = None;`. There is no generation counter (unlike `IdentitySyncManager::start`, which does have one). Trace: `start()` spawns thread A → `stop()` cancels A → `start()` spawns thread B (guard now Some(B)) → thread A's loop finally exits and overwrites `guard = None`. Thread B is still running, but `is_running()` reports `false` and a third `start()` will spawn thread C. Multiple sync threads can run concurrently against the same `wallets` map, each issuing GRPC calls to DAPI.
- **Preconditions**: a manager whose `start()` returns quickly enough to interleave a `stop()` and another `start()` before the original thread observes cancellation.
- **Scenario**:
  1. Build a manager with one registered wallet and a reachable DAPI endpoint.
  2. Call `start()`.
  3. Immediately call `stop()`.
  4. Immediately call `start()` again (before thread A's first sync round completes).
  5. Wait for thread A to observe its cancel token (it will, eventually) and clean up.
  6. Inspect `is_running()` and the actual thread count.
- **Assertions** (the proof shape):
  - At every moment after step 4, AT MOST one platform-address-sync thread is running.
  - `is_running() == true` for the entire window between step 4 and a later `stop()`.
  - After thread A exits in step 5, `is_running()` does NOT drop to `false` (because thread B is still active).
- **Expected** (after fix): adopt `IdentitySyncManager`'s generation-counter pattern — the spawned thread only clears the guard if its own generation matches the latest installed one.
- **Actual** (current code): thread A unconditionally clears the guard on exit, masking thread B's existence to `is_running()`.
- **Severity**: MEDIUM (parallel sync threads cause duplicate DAPI calls, write contention on the wallet manager lock, and inflated rate-limit usage; not data corruption but operationally noisy)
- **Harness extensions required**: a way to count active "platform-address-sync" threads (`std::thread::Builder::name`) or to wedge a sync iteration so cancellation is observable but slow. The simplest proof shape is a counter that the sync routine increments per pass; if two threads run concurrently the counter advances faster than the interval.
- **Estimated complexity**: M
- **Rationale**: `IdentitySyncManager` already has the right pattern. The asymmetry between the two managers is the bug.

#### Found-008 — `LockNotifyHandler` uses `notify_waiters()` so a lock event arriving in the check / wait gap of `wait_for_proof` is dropped
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `wallet/asset_lock/lock_notify_handler.rs:30` (`notify_waiters()`); `wallet/asset_lock/sync/proof.rs:287-337` (`wait_for_proof`'s check-then-await loop).
- **Tracking issue**: dashpay/platform#3641
- **Suspected bug**: `LockNotifyHandler::on_sync_event` calls `Notify::notify_waiters()`, which wakes only currently-registered waiters and produces no permit. `wait_for_proof` runs a check-then-await loop: read state under a read lock, drop the lock, then call `lock_notify.notified().await`. If a lock event fires in the gap between the state check and the registration of the next `notified()` future, no waiter is currently registered, the notification is discarded, and the waiter sleeps until the next event or the timeout.
- **Preconditions**: SPV emits exactly one `InstantLockReceived` for the watched outpoint at a precise moment.
- **Scenario**:
  1. Tracked asset lock `OL` is in `Broadcast` state.
  2. Test thread calls `wait_for_proof(&OL.out_point, timeout=300s)`.
  3. The sequence (deterministic for the test):
     - Wait for `wait_for_proof` to enter the loop and complete its first state check (no proof yet, still `Broadcast`).
     - BEFORE `wait_for_proof` reaches `lock_notify.notified()`, drive `LockNotifyHandler::on_sync_event(InstantLockReceived(OL))` exactly once.
     - Update the underlying `TransactionContext` to `InstantSend(lock)` AT THE SAME TIME (so a re-check would succeed).
- **Assertions** (the proof shape):
  - `wait_for_proof` returns `Ok(InstantAssetLockProof(...))` within `1s` (i.e. without waiting for the timeout).
  - Counter-assertion if buggy: it sleeps until either a follow-up notify or `FinalityTimeout`.
- **Expected** (after fix): use `Notify::notify_one()` (which keeps a permit if no waiter is registered) or call `notified()` BEFORE the state check (so the future is registered before the check happens, per Tokio's documented "intended use").
- **Actual** (current code): a single missed notification stalls the waiter.
- **Severity**: HIGH (asset-lock proof flow is on the critical path of identity registration / top-up; a stalled wait surfaces as long timeouts followed by spurious "asset lock expired" errors)
- **Upstream scope**: Confirmed purely downstream — no upstream `key-wallet` involvement. (`grep -rn 'Notify\|notify_waiters\|notify_one' key-wallet/src/` returned zero hits, audited at SHA `d6dd5da`.)
- **Not fixed by PR #3634**: PR #3634's commit `885a1be3` removed the `masternodeSyncEnabled=false` FFI knob, fixing a DIFFERENT bug (IS/CL events never reaching `LockNotifyHandler` at all when SPV managers were disabled). The Found-008 race surface — events that DO reach `LockNotifyHandler` but are dropped during `wait_for_proof`'s check/await window — is unchanged by #3634. Verified by diff inspection: `lock_notify_handler.rs:30` and the `wait_for_proof` loop in `proof.rs` are not modified.
- **Harness extensions required**: a test handle on `LockNotifyHandler` (it's already constructed with an `Arc<Notify>`); a way to drive the handler synchronously with a controlled state mutation. The wait-for-proof check uses `wallet_manager`, so the test must mutate the tracked record's `TransactionContext` before re-driving the handler.
- **Estimated complexity**: M
- **Rationale**: This is the textbook `Notify` footgun — `notify_waiters` doesn't store a permit, so check-then-await is a missed-wakeup. The asset-lock flow is exactly the place where one missed wakeup turns a 5-second proof wait into a 5-minute hang.

#### Found-009 — wallet-event adapter swallows `RecvError::Lagged` events without compensating recovery
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `changeset/core_bridge.rs:71-115` (the `tokio::select!` loop in `spawn_wallet_event_adapter`).
- **Suspected bug**: On `Err(RecvError::Lagged(n))` the loop logs a warning and continues. The dropped events are gone — `WalletEvent::TransactionDetected`, `BlockProcessed`, etc. that the broadcast channel discarded never reach the persister. Persisted state then lags reality, and there's no compensating mechanism to refetch them.
- **Preconditions**: the broadcast channel's capacity is exceeded (many events fired in a tight burst, e.g. an SPV catch-up with a lot of UTXO changes).
- **Scenario**:
  1. Configure the persister to record every `store(..., cs)` it sees.
  2. Drive the upstream broadcast channel with `(channel_capacity + 10)` distinct events in a tight burst, each with a unique `wallet_id` or `txid` so the persister can tell them apart.
  3. Wait for the loop to drain.
- **Assertions** (the proof shape):
  - The persister observes ALL injected events. Or, equivalently, at least one of: (a) the loop's recovery mechanism re-emits the dropped events (e.g. by walking `wallet_manager` state and emitting a synthetic catch-up changeset), (b) the loop returns / signals an error to the caller so the application can react. Today neither happens.
- **Expected** (after fix): on `Lagged(n)`, either re-subscribe and emit a "full state snapshot" changeset, or escalate the error (e.g. via a status channel) so the operator can issue an explicit re-sync. Silent loss is not OK because the persister diverges from chain reality with no signal.
- **Actual** (current code): events are gone, only a warning log remains.
- **Severity**: MEDIUM (losing core-wallet events causes the persister's stored state to diverge silently from the in-memory `WalletManager` state)
- **Harness extensions required**: a way to construct a small-capacity `tokio::sync::broadcast::Sender` and inject events directly; or an instrumented wallet manager that exposes the broadcast for tests.
- **Estimated complexity**: M
- **Rationale**: `Lagged` is rare but not impossible. When it happens, the wallet's persisted state silently goes wrong. Documenting the contract one way or the other (re-emit / escalate / accept loss) is the minimum bar.

#### Found-010 — `PlatformAddressChangeSet::apply` ignores `funds.nonce` so persister-only nonce state can drift behind balance
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `wallet/apply.rs:259-273` (the `platform_addresses` apply branch).
- **Suspected bug**: The apply path walks `addr_cs.addresses` and writes only `entry.funds.balance` via `set_address_credit_balance`. The `nonce` field on `entry.funds` is dropped — the comment at line 266-270 admits this and points at "evo-tool's platform_address_balances table" as the alleged consumer of the nonce. But that consumption only happens via the FFI persister callback; pure in-memory replay (e.g. tests, restart-into-memory) loses the nonce and a subsequent operation against the same address will use a stale value.
- **Preconditions**: a persister round-trip whose only consumer is `apply_changeset` (no FFI sidecar).
- **Scenario**:
  1. Source `PlatformWalletInfo` `A` has `addr_X` with `(balance=50, nonce=7)`.
  2. Snapshot `A` into a `PlatformAddressChangeSet` and apply it to a fresh `PlatformWalletInfo` `B`.
  3. Read `B`'s cached state for `addr_X`.
- **Assertions** (the proof shape):
  - `B`'s cached nonce for `addr_X == 7`.
  - Counter-assertion if buggy: `B`'s nonce reads back as `0` (the default) because apply never wrote it.
- **Expected** (after fix): persist + apply the nonce alongside the balance — extend `set_address_credit_balance` to also accept the nonce, or add a sibling write.
- **Actual** (current code): apply discards the nonce. Test harnesses replaying a changeset see balance-only state.
- **Severity**: MEDIUM (only bites pure-Rust persisters and tests; FFI consumers are unaffected because they read the changeset directly)
- **Harness extensions required**: ability to read back per-address nonce from `ManagedPlatformAccount`. If no such accessor exists today, the test would need a new one.
- **Estimated complexity**: S
- **Rationale**: The contract is "apply replays the changeset onto state". Replaying balance only is a partial replay; the silent-drop of nonce is a documentation gap that masquerades as design.

#### Found-011 — `IdentityChangeSet::merge` documents commutativity but `insert + tombstone` for the same key resolves to "removed" regardless of submission order
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `changeset/changeset.rs:336-421` (`IdentityChangeSet::merge`); `wallet/apply.rs:127-143` (the apply order: insert then remove).
- **Suspected bug**: The `Merge` trait's docstring says changesets are "commutative and associative". `IdentityChangeSet::merge` extends `identities` (inserts) and `removed` (tombstones) independently with no insert-vs-tombstone resolution. The apply order is "insert first, then remove", so a merged changeset that contains BOTH an insert and a tombstone for identity `id_X` always resolves to "removed", regardless of which side was passed first to `merge`. The latent contract violation: `A.merge(B)` then apply ≠ `B.merge(A)` then apply for the case `A = {insert id_X}`, `B = {tombstone id_X}` (both produce "removed"), but the merger has no way to express "the insert wins because it came later". The docstring on the changeset itself acknowledges the hazard ("Merge ordering hazard"); the trait-level docstring still claims commutativity. One of the two is wrong.
- **Preconditions**: two changesets that disagree on a single identity (one inserts, one removes).
- **Scenario**:
  1. Build `cs_insert` containing `identities: {id_X → entry}` only.
  2. Build `cs_remove` containing `removed: {id_X}` only.
  3. Compute state_AB by merging cs_insert into a copy, then merging cs_remove, then applying.
  4. Compute state_BA by merging cs_remove into a copy, then merging cs_insert, then applying.
- **Assertions** (the proof shape):
  - If commutativity is the contract: state_AB == state_BA AND for at least one of them id_X is present (non-vacuous). Today both end up "removed", so the contract is "tombstone wins". State the rule in the docstring.
  - If "tombstone wins" is the contract: docstring on the `Merge` trait must say so explicitly; the test pins the ordering.
- **Expected** (after fix): pick one — either `merge` resolves the conflict by last-seen (A.merge(B) ⇒ tombstone wins because it came later in `B`; B.merge(A) ⇒ insert wins because it came later in `A`), or document "tombstone always wins regardless of merge order" and remove the commutativity claim.
- **Actual** (current code): tombstone always wins and the docstring claims commutativity; one of the two is misleading.
- **Severity**: LOW (no current emitter produces both insert and tombstone for the same key in one mutation, per the in-source comment, but the latent footgun is documented as if it isn't a footgun)
- **Harness extensions required**: none — pure unit-test-shaped.
- **Estimated complexity**: S
- **Rationale**: A "commutative" claim that doesn't hold for the simplest counter-example is a documentation bug that misleads future emitters. Pinning the actual semantics in a test forces the doc to match reality.

#### Found-012 — `validate_or_upgrade_proof` and `wait_for_proof` only consult `standard_bip44_accounts`, missing CoinJoin / non-BIP-44 funding accounts
- **Priority**: P2 (bug pin — failure is the proof)
- **Tracking issue**: dashpay/platform#3642 (downstream-only fix — iterate `all_funding_accounts()` at the 5 hard-coded sites)
- **Wallet feature exercised**: `wallet/asset_lock/sync/proof.rs:43-54` (`validate_or_upgrade_proof`); `wallet/asset_lock/sync/proof.rs:289-322` (`wait_for_proof`); `wallet/asset_lock/sync/recovery.rs:104-110` (`resolve_status_from_info`).
- **Suspected bug**: All three lookups walk `info.core_wallet.accounts.standard_bip44_accounts.get(&account_index)` and bail with "Transaction not found" if the BIP-44 lookup misses. But `account_index` on the tracked lock can refer to a CoinJoin account, an identity account, or any non-BIP-44 funding source. A real CoinJoin-funded asset lock would have its tx in `coinjoin_accounts` (or wherever), not `standard_bip44_accounts`. The wallet then can't resolve the chain status, can't upgrade IS to CL, and `wait_for_proof` returns "transaction not found" even though the chain has the tx.
- **Preconditions**: an asset lock funded from a non-BIP-44 account.
- **Scenario**:
  1. Track a `TrackedAssetLock` whose `account_index` corresponds to a non-BIP-44 account containing the asset-lock tx.
  2. Call `wait_for_proof(&out_point, timeout=10s)`.
- **Assertions** (the proof shape):
  - `wait_for_proof` returns `Ok(_)` (the proof) within the timeout, OR errors with a CLEAR account-type-mismatch message — never a generic "Transaction not found in account N" message that masks the real cause.
- **Expected** (after fix): walk every account collection, not just `standard_bip44_accounts`; or carry the account *kind* alongside `account_index` on `TrackedAssetLock`.
- **Actual** (current code): non-BIP-44 funded asset locks silently fail proof discovery.
- **Severity**: MEDIUM (impacts CoinJoin / shielded users; the failure mode is "asset lock never resolves" with a misleading error)
- **Harness extensions required**: ability to register a CoinJoin or non-BIP-44 account on the test wallet and seed a tx into its `transactions` map.
- **Estimated complexity**: M
- **Rationale**: Hardcoding `standard_bip44_accounts` in three places means the bug class spans the entire asset-lock proof pipeline. Pinning the contract on at least the proof-wait path catches a future shielded / CoinJoin asset-lock effort.

#### Found-013 — `recover_asset_lock_blocking` swallows every error and returns `()` — silent recovery failure
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `wallet/asset_lock/sync/recovery.rs:36-88` (`recover_asset_lock_blocking`).
- **Suspected bug**: The function returns `()`; every failure path is a silent `return`: `wallet_id` not in manager → silent return; lock already tracked → silent return; persister `store` failure → logged and discarded inside `queue_asset_lock_changeset`. There is no signal to the caller that recovery either ran successfully or failed — the doc neither mentions success/failure nor offers a query path to check whether the lock is now tracked.
- **Preconditions**: a recovery attempt against a wallet that doesn't exist in the manager.
- **Scenario**:
  1. Construct an `AssetLockManager` whose `wallet_id` was deliberately removed from the wallet manager.
  2. Call `recover_asset_lock_blocking(...)`.
- **Assertions** (the proof shape):
  - The caller can detect the failure — either via a `Result<(), _>` return type, or a follow-up `is_tracked` check that reflects "no, the recovery did not land".
  - Today: the function returns `()`; the caller has no way to distinguish "recovery succeeded" from "wallet was missing".
- **Expected** (after fix): change the signature to `Result<(), PlatformWalletError>` (matching the rest of this module's surface), or document explicitly that the function is best-effort and provide a sibling `is_tracked` accessor for confirmation.
- **Actual** (current code): silent failure on `wallet_id` miss; the test harness can't distinguish a successful recovery from a no-op.
- **Severity**: LOW (a recovery failure should be loud; silent swallow is poor ergonomics rather than data corruption — but evo-tool / DET-style callers may rely on this contract)
- **Upstream scope**: Confirmed purely downstream — upstream `AssetLockError` exposes rich variants (`Signer`, `SigningFailed`, `UnsupportedSignerMethod`, `KeyDerivation`, etc.); the swallowing is `rs-platform-wallet`'s own flattening in `recover_asset_lock_blocking`.
- **Harness extensions required**: an `is_tracked` query on `AssetLockManager` (likely already exists via `list_tracked_locks`).
- **Estimated complexity**: S
- **Rationale**: `pub fn ... -> ()` on an operation that has multiple distinct failure modes is a documentation bug; pin the contract one way or the other.

#### Found-014 — `transfer_credits_with_external_signer` never updates the receiver's local balance even when the receiver is wallet-owned
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `wallet/identity/network/transfer.rs:74-138`.
- **Suspected bug**: The SDK call returns `(sender_balance, receiver_balance)`; the wallet uses only `sender_balance` and pattern-matches the receiver as `_receiver_balance`. If the receiver identity is also owned by this wallet (a wallet hosting two identities is the canonical case), its local cached balance falls out of sync until the next identity sync round.
- **Preconditions**: a wallet hosting two identities `I_send` and `I_recv`. Both are managed by the local `IdentityManager`.
- **Scenario**:
  1. Register both `I_send` and `I_recv` against the same wallet.
  2. Record both identities' cached balances pre-transfer.
  3. Call `transfer_credits_with_external_signer(I_send, I_recv, amount, ...)`.
  4. Read both cached balances post-call (no intervening sync).
- **Assertions** (the proof shape):
  - `I_send.cached_balance` decreased by `amount + fee` (call returns `sender_balance`, so this side updates).
  - `I_recv.cached_balance` increased by `amount` exactly.
  - Counter-assertion if buggy: `I_recv.cached_balance` is unchanged from its pre-call value.
- **Expected** (after fix): if `I_recv` is in the local `IdentityManager`, write `set_balance(receiver_balance)` for it too and emit a snapshot changeset.
- **Actual** (current code): receiver-side cache is stale until the next sync; UI reads show the wrong balance for the receiver.
- **Severity**: MEDIUM (UI staleness for self-transfers; not data corruption, but a contract violation since the SDK explicitly reports the receiver balance and the wallet has it on hand)
- **Harness extensions required**: identity setup with two wallet-owned identities (Wave A blocker).
- **Estimated complexity**: S
- **Rationale**: The SDK pattern-binds the receiver balance specifically so the wallet can use it. Discarding it via `_receiver_balance` is a small but precise contract miss.

#### Found-015 — `load_from_persistor` leaves a partially registered wallet in `wallet_manager` when `wallet_id` mismatches
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `manager/load.rs:69-85`.
- **Suspected bug**: The load loop calls `wm.insert_wallet(wallet, platform_info)` which yields an internally-recomputed `wallet_id`. Immediately afterwards the code compares against `expected_wallet_id` and returns an `Err` if they differ. But by that point the wallet has already been inserted into `self.wallet_manager`. The error-return short-circuits any subsequent rollback, so the manager ends up holding a wallet whose id doesn't match the persisted record — and the `self.wallets` map (the public registry) doesn't have it. Subsequent reads via `wallets.get(...)` return `None` while sync paths see the stale entry.
- **Preconditions**: a persister whose load returns a `(expected_wallet_id, wallet_state)` pair where `expected_wallet_id` != `Wallet::compute_id(wallet_state.wallet)`. (Trivially constructible in tests.)
- **Scenario**:
  1. Build a `ClientStartState` with `wallets[expected_id] = state` where `state.wallet`'s recomputed id is `actual_id != expected_id`.
  2. Call `manager.load_from_persistor()` and observe the error.
  3. Inspect `manager.wallet_manager` (count of wallets) and `manager.wallets` (count of public-registered wallets).
- **Assertions** (the proof shape):
  - On error from `load_from_persistor`, both `wallet_manager` and `self.wallets` contain ZERO wallets — neither was partially populated.
  - Counter-assertion if buggy: `wallet_manager` contains ONE wallet (the partial insert) while `self.wallets` is empty.
- **Expected** (after fix): roll back the `wm.insert_wallet` (call `wm.remove_wallet(wallet_id)`) before returning the error, or perform the id check BEFORE inserting.
- **Actual** (current code): the manager is left in a half-loaded state where the inner manager and the outer registry disagree.
- **Severity**: MEDIUM (only triggered by corrupted persisted state, but when it triggers the wallet manager is operationally inconsistent)
- **Harness extensions required**: a stub persister that returns a malformed `ClientStartState`.
- **Estimated complexity**: M
- **Rationale**: Half-loaded states lead to the worst class of bug — the manager's internal invariant ("every entry in `wallet_manager` has a matching `Arc<PlatformWallet>` in `self.wallets`") is silently broken.

#### Found-016 — `remove_wallet` removes from `self.wallets` then `self.wallet_manager` non-atomically, leaving a window where readers see only one of the two
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `manager/wallet_lifecycle.rs:322-337`.
- **Suspected bug**: The function takes the `self.wallets` write lock, removes the wallet, drops the lock, then takes the `self.wallet_manager` write lock and removes from there. Between the two operations, a concurrent task can read `self.wallet_manager` (via e.g. a sync routine) and find the wallet still present, while `self.wallets` no longer has it. The sync routine then queries provider state for a wallet it can't find via the public registry — which manifests as `WalletNotFound` deep inside an unrelated callsite.
- **Preconditions**: at least one concurrent reader on `self.wallet_manager` while `remove_wallet` is in progress.
- **Scenario**:
  1. Register a wallet `W` with the manager.
  2. Spawn task `T1`: in a tight loop, take `wallet_manager.read()` and check whether `W` is present; record both that result and the result of `self.wallets.read()` for the same wallet.
  3. From the main task, call `manager.remove_wallet(&W.id)`.
  4. Stop `T1`.
- **Assertions** (the proof shape):
  - For every observation `T1` made: either both registries report present, or both report absent. Never one-of-two.
  - Counter-assertion if buggy: at least one observation shows `wallet_manager` present, `self.wallets` absent.
- **Expected** (after fix): perform both removes under a coordinated lock or document the transient inconsistency window. Operations that depend on cross-registry consistency must guard against it.
- **Actual** (current code): a small but real window of inconsistency.
- **Severity**: MEDIUM (race window is small but the resulting `WalletNotFound` errors look like spontaneous failures at unrelated call sites)
- **Harness extensions required**: a way to wedge a concurrent reader with deterministic interleaving (e.g. a `tokio::sync::Barrier` injected for tests).
- **Estimated complexity**: M
- **Rationale**: Two-registry models (here, the inner `WalletManager` plus the outer `Arc<PlatformWallet>` registry) are a classic source of inconsistency windows. The fix is invariant-driven; the test pins the invariant.

#### Found-017 — `register_wallet` registers wallet in memory even when persister `store` returns `Err` — vanishes on next launch
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `manager/wallet_lifecycle.rs:238-244`, `manager/wallet_lifecycle.rs:296-298`.
- **Suspected bug**: The persister is invoked to store the registration changeset (metadata + per-account specs + per-pool snapshots). On failure the code logs and proceeds to insert the wallet into `self.wallets`. The wallet is fully usable in the current process but on next launch the persister has no record of it — the user-visible effect is "I imported my wallet, used it, restarted the app, and the wallet is gone".
- **Preconditions**: a persister whose `store` returns an error for the registration round.
- **Scenario**:
  1. Build a manager with a stub persister that fails (`store(...) → Err(_)`) on its first call.
  2. Call `create_wallet_from_mnemonic(...)`.
  3. Inspect the result and the manager state.
- **Assertions** (the proof shape):
  - EITHER `create_wallet_from_mnemonic` returns `Err(_)` so the caller knows the wallet won't survive a restart, AND the manager state is rolled back (no entry in `self.wallets`, no entry in `self.wallet_manager`).
  - OR the function succeeds AND the persister failure is exposed via a status / event channel the caller can subscribe to. A silent log isn't sufficient.
- **Expected** (after fix): treat the registration `store` as load-bearing — fail the registration and roll back the in-memory state on persister error.
- **Actual** (current code): the registration silently proceeds; the user discovers the loss only on next launch.
- **Severity**: HIGH (data loss class — a successful-looking wallet import that doesn't survive restart)
- **Harness extensions required**: a stub persister with a configurable failure mode.
- **Estimated complexity**: S
- **Rationale**: The current code path assumes the persister is "best-effort". For the registration-round changeset specifically, this assumption is wrong — without that record, the wallet is unrecoverable.

#### Found-018 — `PlatformAddressChangeSet::merge` documents fee semantics as "fee paid by the transfer that produced this changeset" but actually accumulates fees across merged changesets
- **Priority**: P2 (bug pin — failure is the proof)
- **Wallet feature exercised**: `changeset/changeset.rs:586-635` (`PlatformAddressChangeSet::fee_paid`, `Merge::merge`).
- **Suspected bug**: The `fee` field's docstring says "Fee paid by the transfer that produced this changeset, in credits." (singular). `fee_paid()` returns `self.fee`. But `merge` does `self.fee = self.fee.saturating_add(other.fee)` — so a merged changeset's `fee_paid()` returns the sum of fees across multiple transfers. A consumer that calls `fee_paid()` on a merged changeset and expects "the fee for ONE transfer" gets a misleading number with no way to tell.
- **Preconditions**: two changesets, each with a non-zero `fee`.
- **Scenario**:
  1. Build `cs_a` with `fee = 100_000`.
  2. Build `cs_b` with `fee = 200_000`.
  3. Compute `cs_a.merge(cs_b)`.
  4. Read `cs_a.fee_paid()`.
- **Assertions** (the proof shape):
  - Pick one — and document the choice:
    - (a) `fee_paid()` on a merged changeset is the sum: `300_000`. Then rename / re-document the field to "total fee paid across operations in this batch".
    - (b) `fee_paid()` is the fee of a single transfer; `merge` should preserve it via last-write-wins or refuse to merge non-zero fees. Then document and enforce.
  - Today: `fee_paid()` returns `300_000` while the docstring says "fee paid by the transfer that produced this changeset" — internally inconsistent.
- **Expected** (after fix): rename the docstring or change the merge policy. The two are at war.
- **Actual** (current code): consumers reading `fee_paid()` on a merged changeset can mis-count the per-transfer fee.
- **Severity**: LOW (only callers reading the fee accessor on a merged changeset are affected; the changeset is mostly consumed pre-merge)
- **Harness extensions required**: none — pure unit-test.
- **Estimated complexity**: S
- **Rationale**: Two facts in the source disagree (docstring vs merge behaviour). One of them is wrong. A test pins which.

#### Found-021 — `TransactionRecord::update_context` silently drops `InstantLock` state when tx transitions `InstantSend` → `InBlock`
- **Priority**: P2 (bug pin — failure is the proof)
- **Severity**: HIGH (silent data loss on the critical path; an `InstantLock` is proof material that vanishes on block confirmation)
- **Owner: upstream `key-wallet` (rust-dashcore)**
- **Status**: red-by-design. Test pin at `tests/e2e/cases/found_021_instant_lock_dropped_on_context_promotion.rs` (commit `e85d558bbe`). Pure `#[test]` — no harness, no network. Asserts the IS-lock is still accessible after `update_context(InBlock(info))`; fails today because `self.context = context` unconditionally replaces the prior `InstantSend(lock)`. The defect line moved during the resolved rust-dashcore rev (`5313086`) — `TransactionRecord::new` now requires an `AccountType` second-position argument — the contract is identical.
- **Wallet feature exercised**: `wallet/asset_lock/sync/proof.rs` (any path that reads `TransactionContext` to recover an IS-lock as proof material after block confirmation).
- **Suspected bug** (upstream `key-wallet`, SHA `d6dd5da`): `TransactionRecord::update_context` at `key-wallet/src/managed_account/transaction_record.rs:181-184` is a naive replace — `self.context = context`. When a transaction is first observed as `TransactionContext::InstantSend(InstantLock)` and a later `InBlock(BlockInfo)` event arrives, the IS-lock is overwritten and gone. Any downstream consumer that reads back the `TransactionRecord` after block confirmation to use the IS-lock as proof material (e.g. to construct an `InstantAssetLockProof`) will find the lock field absent. The `update_utxos` path at `:201-202` sets `utxo.is_instantlocked` for the current call but does not preserve the lock across context promotions.
- **Preconditions**: a tracked asset-lock transaction that receives both an `InstantSend(lock)` context update AND a subsequent `InBlock(info)` update before the caller reads the record.
- **Scenario**:
  1. Broadcast an asset-lock transaction and wait for SPV to emit `InstantLockReceived`.
  2. Let `update_context(InstantSend(lock))` run — verify `record.context` holds the lock.
  3. Wait for block confirmation — let `update_context(InBlock(info))` run.
  4. Read `record.context` and attempt to extract the `InstantLock`.
- **Assertions** (the proof shape):
  - After step 3, `record.context` EITHER is `InBlock(info)` with the original `InstantLock` preserved alongside (e.g. via `InBlockWithInstantLock { info, lock }`) OR a dedicated `record.instant_lock` field retains the lock independently of `context`.
  - Counter-assertion if buggy (today's behaviour): `record.context == InBlock(info)` with no lock accessible — `InstantLock` has been silently dropped.
- **Expected** (after upstream fix): promote `update_context` to a merging operation that retains the IS-lock when transitioning to `InBlock`/`InChainLockedBlock`. One approach: extend `TransactionContext` with an `InBlockWithInstantLock { info, lock }` variant; another: store the most recent `InstantLock` on `TransactionRecord` independently and document the merge rules.
- **Actual** (current upstream code): `self.context = context` — IS-lock is unconditionally replaced.
- **Harness extensions required**: direct access to `TransactionRecord` after context promotion; a mock or real SPV event driver that can inject both context updates in order.
- **Estimated complexity**: M (upstream change required before downstream test can pass; test itself is M once the API is in place).
- **Rationale**: Asset-lock proof flows commonly observe InstantSend first, then block confirmation. The IS-lock is the proof material until the block becomes chain-locked. Dropping it silently on block arrival means any proof consumer that is not racing to read before block confirmation loses its proof. Filed from Marvin's upstream audit (audit Finding #2, MEDIUM — re-classified HIGH here because the downstream impact is silent data loss on the critical proof path).

#### Found-022 — `AssetLockBuilder::build` bumps `monitor_revision` on the BIP-44 funds account before `build_asset_lock` can fail, contradicting the doc-comment "no addresses consumed on failure" guarantee
- **Priority**: P2 (bug pin — failure is the proof)
- **Severity**: MEDIUM (silent funds-account mutation when build fails; the doc-comment's "no addresses consumed" guarantee is misleading and `monitor_revision` consumers see a phantom advance)
- **Owner: upstream `key-wallet` (rust-dashcore)**
- **Status**: red-by-design. Test pin at `tests/e2e/cases/found_022_asset_lock_builder_consumes_change_index_on_failure.rs`. `#[tokio_shared_rt::test(shared)]` constructs a UTXO-less wallet, snapshots `account.monitor_revision()` on BIP-44 account 0, calls `build_asset_lock` (which fails at coin selection with `NoUtxosAvailable`), and asserts the snapshot is unchanged. Fails today because the snapshot advances by one across the failed build — the upstream `TransactionBuilder::set_funding` call path mutates the funds account before `build_signed` can fail.
- **Wallet feature exercised**: `wallet/asset_lock/build.rs` (any path through `build_asset_lock_transaction` that exercises the upstream builder).
- **Suspected bug** (upstream `key-wallet`, rev `5313086`): The doc-comment on `build_asset_lock` claims "The transaction is built first, and keys are only derived after a successful build — so no addresses are consumed if the build fails." This is misleading. `TransactionBuilder::set_funding` at `key-wallet/src/wallet/managed_wallet_info/transaction_builder.rs:79-83` runs BEFORE `build_signed` can perform coin selection:
  ```rust
  pub fn set_funding(mut self, funds_acc: &mut ManagedCoreFundsAccount, acc: &Account) -> Self {
      self.inputs = funds_acc.utxos.values().cloned().collect();
      self.change_addr = funds_acc.next_change_address(Some(&acc.account_xpub), true).ok();
      self
  }
  ```
  `funds_acc.next_change_address(..., add_to_state=true)` always invokes `self.keys.bump_monitor_revision()` at `key-wallet/src/managed_account/managed_core_funds_account.rs:540` regardless of whether the eventual build succeeds. When `build_signed` then errors with `NoUtxosAvailable`, the funds account has already been mutated and no transaction was produced.
- **Observability footnote** — only `monitor_revision` is mutated under realistic test setup. The internal `AddressPool` is NOT visibly drifted because `WalletAccountCreationOptions::Default` pre-populates the BIP-44 internal pool with a full gap-limit window (30 derived addresses, indices 0..=29). `AddressPool::next_unused` at `key-wallet/src/managed_account/address_pool.rs:521-540` first scans `0..=highest_generated` for an unused entry and short-circuits to index 0 without calling `generate_address_at_index`. So neither `addresses.len()` nor `highest_generated` change; only the unconditional `bump_monitor_revision()` call leaves a footprint. Earlier framings of this finding ("change-pool `highest_used == None`" / "phantom address leaked into `addresses`") do not bite in practice — see the test module-doc for the diagnostic chain.
- **Preconditions**: a build attempt that fails on `build_asset_lock` (e.g. coin selection fails) after `set_funding` has already run.
- **Scenario**:
  1. Construct a fresh `Wallet` + `ManagedWalletInfo` with default accounts and zero UTXOs.
  2. Snapshot `account.monitor_revision()` on BIP-44 account 0 via `ManagedAccountTrait`.
  3. Call `ManagedWalletInfo::build_asset_lock`; expect `Err(Builder(CoinSelection(NoUtxosAvailable)))`.
  4. Re-read `monitor_revision()` and compare against the snapshot.
- **Assertions** (the proof shape):
  - After the failed build, `monitor_revision` is unchanged from the pre-build snapshot — no funds-account mutation occurred on the failure path.
  - Counter-assertion if buggy (today): `monitor_revision` has advanced by one — the funds account was mutated even though no transaction was produced.
- **Expected** (after upstream fix): either (a) defer `next_change_address` until after `build_signed` succeeds; or (b) teach `set_funding` to peek the change address without bumping (`add_to_state=false` and no `bump_monitor_revision` on the failure path).
- **Actual** (current upstream code): `set_funding` calls `next_change_address(..., add_to_state=true)` eagerly; the call unconditionally bumps `monitor_revision`; `build_signed` then fails on the empty UTXO set.
- **Harness extensions required**: none — `ManagedAccountTrait::monitor_revision()` is public and reachable from the test crate via `key_wallet::account::ManagedAccountTrait`.
- **Estimated complexity**: S (test is a self-contained unit test; the upstream fix is S-M).
- **Rationale**: A doc-comment that promises "no addresses consumed on failure" and a code path that silently mutates the funds account is a broken contract. Consumers that watch `monitor_revision` for "did the account change?" signaling (cache invalidation, persistence triggers, monitor diffs) will see phantom bumps that don't correspond to any actual transaction. Filed from Marvin's upstream audit (audit Finding #3, MEDIUM); retargeted from the original `highest_used` formulation after empirical diagnosis showed the visible footprint is `monitor_revision`, not pool state.

#### Found-023 — `ManagedAccountCollection` lacks a `find_transaction_record(&Txid)` helper — every consumer rolls its own incomplete loop
- **Priority**: P2 (bug pin — failure is the proof)
- **Severity**: LOW (ergonomic footgun; the symptom is "transaction not found" for CoinJoin / BIP-32-funded asset locks, not data corruption)
- **Tracking issue**: dashpay/platform#3642 — actionable fix is downstream (Found-012's surface in `rs-platform-wallet`), not the upstream `key-wallet` helper. The 5 hard-coded BIP-44 lookups can be replaced with `all_funding_accounts()` iteration today, without waiting on the upstream `find_transaction_record` helper.
- **Owner: upstream `key-wallet` (rust-dashcore)**
- **Wallet feature exercised**: `wallet/asset_lock/sync/proof.rs` (`validate_or_upgrade_proof`); `wallet/asset_lock/sync/recovery.rs` (`recover_asset_lock_blocking`); any path that looks up a transaction record by `Txid` across account types.
- **Suspected bug** (upstream `key-wallet`, SHA `d6dd5da`): `ManagedAccountCollection` at `key-wallet/src/managed_account/managed_account_collection.rs:1057-1143` exposes broad iteration helpers (`all_accounts`, `all_funding_accounts`) but no focused "find a transaction record by `Txid` across all funds-bearing accounts" helper. Every downstream consumer that wants to confirm an asset-lock transaction must either (a) know which account collection the funding came from (typically impossible, since CoinJoin / BIP-32 funding is opaque) or (b) hand-roll `all_funding_accounts()` + `transactions.get(&txid)`. In practice consumers hard-code `standard_bip44_accounts` (as Found-012 in `rs-platform-wallet` documents), and CoinJoin / BIP-32-funded asset locks return "transaction not found". A `fn find_transaction_record(&self, txid: &Txid) -> Option<(AccountType, &TransactionRecord)>` on `ManagedAccountCollection` would close this cliff.
- **Preconditions**: an asset-lock transaction funded from a non-BIP-44 account (e.g. CoinJoin or BIP-32).
- **Scenario**:
  1. Fund an asset-lock via a CoinJoin or BIP-32 account (not the default `standard_bip44_accounts`).
  2. Call any downstream path that looks up the transaction record by `Txid` (e.g. `validate_or_upgrade_proof`).
- **Assertions** (the proof shape):
  - The lookup succeeds regardless of which account type funded the transaction.
  - Counter-assertion if buggy (today's behaviour): the lookup returns `None` / "transaction not found" for non-BIP-44-funded locks — surfaces as "asset lock not tracked" errors in the platform wallet.
- **Expected** (after upstream fix): add `find_transaction_record(&self, txid: &Txid) -> Option<(AccountType, &TransactionRecord)>` (and `_mut` variant) on `ManagedAccountCollection`, walking every funds-bearing collection. Document that callers must prefer it over per-collection lookups.
- **Actual** (current upstream code): no such helper exists; consumers write per-collection loops and miss CoinJoin / BIP-32 accounts (Found-012 in `rs-platform-wallet` is exactly this).
- **Harness extensions required**: a way to force a CoinJoin or BIP-32-funded asset-lock build (currently the harness always uses the default BIP-44 account); access to `ManagedAccountCollection` to verify lookup results.
- **Estimated complexity**: S (a short upstream addition; the downstream test is also S once the upstream helper exists).
- **Rationale**: Every consumer of the asset-lock proof flow needs this lookup. Without a collection-wide helper, the default "just use BIP-44" shortcut is both the obvious pattern and the wrong one for CoinJoin / BIP-32-funded wallets. A missing ergonomic helper is a footgun that becomes a bug in every downstream consumer that doesn't know to iterate all account types. Filed from Marvin's upstream audit (audit Finding #5, LOW).

#### Found-024 — `PlatformAddressWallet::transfer` writes foreign output-address balances to local ledger (no ownership check)
- **Priority**: P1 (real shipped bug; blocked PA-004b and PA-009c in CI; surfaces in production wallets sending credits to any foreign Platform address)
- **Severity**: HIGH (local ledger corruption; `total_credits()` returns an arbitrarily inflated value; downstream sweep and dust-gate paths act on bad data)
- **Owner**: `rs-platform-wallet` (downstream wrapper bug — not upstream `key-wallet` or SDK)
- **Status**: passing-as-regression. Fix landed at `16636f01c0` (V27-007). Tests PA-004b and PA-009 unblocked; `#[ignore]` removed at the same commit.
- **Wallet feature exercised**: `src/wallet/platform_addresses/transfer.rs:160` — the post-broadcast ledger-update loop inside `PlatformAddressWallet::transfer`. Canonical sibling guard (the pattern this fix mirrors): `src/wallet/platform_addresses/fund_from_asset_lock.rs:77`.
- **Suspected bug** (now confirmed, fixed at `16636f01c0`):
  - The Dash Platform SDK returns post-transition state for every address touched by an `IdentityCreditTransferToAddresses` transition — both inputs (source) and outputs (recipients), regardless of which wallet owns them.
  - `transfer.rs` iterated `address_infos` and called `account.set_address_credit_balance` for every entry, with no ownership check.
  - When a source wallet transferred credits to a foreign address (e.g. the bank wallet's primary receive address), the response included that foreign address's post-credit balance.
  - Without the ownership guard, the source wallet staged that foreign balance into its own local `address_balances` ledger.
  - `wallet.total_credits()` then returned the sum of the source wallet's own balances plus the foreign address's balance — inflated by up to the foreign wallet's full credit holdings.
  - Marvin's investigation chain: QA-V40-004 → QA-V42-004 → QA-V43-001 → QA-V44-004 → QA-V45-010 (the version this commit closes). See also V27-007 in §7 (Known Issues).
- **Preconditions**: source wallet has at least one platform address with a credit balance; at least one recipient address in the transfer is NOT in any of the source wallet's platform-address pools.
- **Scenario (regression-test shape)**:
  1. Construct a `PlatformAddressWallet` with a single owned address holding `1_000` credits.
  2. Mock the SDK (or use the post-broadcast path's pre-guard predicate directly) to return a transfer response that includes a foreign address — e.g. the bank's primary receive address — with `9_680_000_000_000` post-credit balance.
  3. Call `transfer(...)` to send `500` credits to that foreign address.
  4. After the call returns, query `wallet.total_credits()` and `wallet.address_credit_balance(&bank_addr)`.
- **Assertions**:
  - `wallet.total_credits()` ≈ `500` (source balance of `1_000` minus the `500` sent minus fee). NOT `9_680_000_000_500` or any value incorporating the foreign address's balance.
  - `wallet.address_credit_balance(&bank_addr) == None` — the bank's address was never in this wallet's pool and must not appear in its local ledger.
  - **Today's behaviour (PASS)**: assertions hold because `account.contains_platform_address(&p2pkh)` gates the `set_address_credit_balance` call.
  - **Pre-fix behaviour (FAIL — what this regression pin tests against)**: `total_credits()` returned the sum including the foreign balance; assertions would fail. PA-004b / PA-009 saw the bank's full ~40.8 tDASH where the dust-residual wallet should have shown `1_000` credits.
- **Expected**: PASS today. FAIL if V27-007 regresses (i.e. the ownership guard is removed or the ledger-update loop is refactored without re-applying the guard).
- **Actual (post-fix)**: PASS.
- **Harness extensions required**: an SDK mock that returns a multi-address transfer response including at least one foreign address, or a direct unit test that calls the post-broadcast path's ledger-update predicate without a live SDK. The latter is the recommended shape — pure unit test, ~80 LOC, no network dependency. Defensive guard also added to `withdrawal.rs:141` for consistency; the analogous guard was already present at `fund_from_asset_lock.rs:77`.
- **Estimated complexity**: S (~80 LOC unit test).
- **Rationale**: The bug shipped in production via the FFI / Swift SDK. Transfer-to-a-foreign-Platform-address is the most common cross-wallet flow (bank to user, user to counterparty). Without this regression pin, any future refactor of the ledger-update loop is one careless line away from re-introducing the same corruption — silently, because `total_credits()` has no self-consistency check against on-chain state.

#### Found-025 — `rs-sdk` address sync silently discards balance update when address is not yet in `pending_addresses` snapshot (TK-suite flake root cause)
- **Priority**: P1 (deterministic under parallelism; affects every test that funds a fresh address)
- **Severity**: HIGH (silent data loss on the critical path of every parallel TK test; reproduced on first run of `cargo test -p platform-wallet --test e2e -- --ignored cases::tk_`)
- **Owner**: upstream `rs-sdk` (not `rs-platform-wallet`). Fix location: `packages/rs-sdk/src/platform/address_sync/mod.rs:619`.
- **Status**: red-by-design — pending upstream test-hook surface. The pin file `tests/e2e/cases/found_025_address_sync_silent_discard.rs` is a documented stub; no `#[test]` is emitted. The earlier v47-era unit test asserted on a locally-built `HashMap<Vec<u8>, (tag, address)>` that the SDK never touches — it returned `None` for any key never inserted, which is `std::collections::HashMap` semantics, not SDK behaviour. After any genuine upstream fix the assertion would still fire and falsely report regression (same disease as Found-022; see Marvin's empirical sweep at `/tmp/marvin-redbyd-sweep.md` and raw log `/tmp/marvin-redbyd-found_025.log`). Retarget to drive `sync_address_balances` with a `GrowingAddressProvider` mock is blocked: every code path past the early-return at `mod.rs:334` issues live DAPI requests with grovedb-proof verification, and neither `Sdk::new_mock()` (cannot synthesize valid grovedb proof bytes) nor the testnet bank harness (unavailable in this environment) closes the gap. Unblocking requires one of: (i) a test-only transport seam on `sync_address_balances`, (ii) an inner-fn extraction that takes pre-built `key_to_tag` + canned updates, or (iii) a post-phase `key_to_tag` refresh hook on `AddressProvider` (the fix itself). Each is a public-API change in `rs-sdk` requiring user input.
- **Wallet feature exercised**: `rs-sdk::platform::address_sync::AddressSyncProvider::incremental_catch_up` (specifically the `address_lookup.get(&addr_bytes)` filter at line 619); transitively `next_unused_receive_address` → `pending_addresses()` registration ordering in the SDK's address-monitoring provider.
- **Suspected bug**: The SDK builds `address_lookup` (a `HashMap<addr_bytes, address_tag>`) **once at sync entry** by snapshotting `provider.pending_addresses()`. If the recipient address was allocated by `next_unused_receive_address()` AFTER the snapshot but BEFORE the next sync cycle, the SDK's filter discards a perfectly-valid balance update returned by the DAPI proof. The address bytes ARE in the response payload — Marvin verified this in the live trace at log line 27750 of the Phase 3 trace log. The discard is silent: no `warn!`, no `error!`, no signal to the caller that data was dropped.
- **Preconditions**: an address freshly allocated via `next_unused_receive_address` (or sibling), followed by a funding broadcast that lands on chain BEFORE the address is registered in `pending_addresses`.
- **Scenario** (regression-pin shape):
  1. Allocate a fresh address `addr` from a wallet's HD pool via `next_unused_receive_address`.
  2. DO NOT call any sync-registration helper that would put `addr` into `pending_addresses` (the bug is that callers must remember to do this themselves; the SDK should do it for them).
  3. Fund `addr` via a real broadcast OR a synthetic balance entry that the SDK's compacted-response path would handle.
  4. Call `sync_balances`.
  5. Assert `addresses_with_balances()` shows `addr` with the funded balance.
- **Assertions**:
  - `addresses_with_balances().get(&addr) == Some(funded_amount)`.
  - **Today's behaviour (FAIL)**: `addresses_with_balances().get(&addr) == None` because the SDK's `incremental_catch_up` discarded the balance update.
  - **After fix (PASS)**: the SDK either (i) re-registers `addr` into `pending_addresses` atomically inside `next_unused_receive_address`, or (ii) `incremental_catch_up` falls back to a full re-snapshot when it sees an address it doesn't recognise, or (iii) emits a typed signal so callers can re-issue the registration before the next sync.
- **Expected**: PASS after upstream fix.
- **Actual** (today): FAIL — pin is correctly RED-by-design.
- **Harness extensions required**: none if the test drives the SDK directly via a synthetic compacted response (preferred); OR a Core-funded test wallet if the test exercises the path through `bank.fund_address` (gated under `PLATFORM_WALLET_E2E_BANK_CORE_GATE`).
- **Estimated complexity**: M (~150-200 LOC; needs SDK introspection or e2e setup).
- **Rationale**: This is the load-bearing finding of the TK-flake investigation. Without the pin, future SDK refactors can re-introduce the same race silently. The chain-confirmation gate (`wait_for_address_nonces_chain_confirmed`) is a misleading proxy — it confirms the SENDER side, not the recipient's balance visibility. Found-025 pins the actual contract: address-sync must surface balance updates for any address the SDK's HD-pool has emitted.
- **Cross-reference**: Found-024 (above) surfaced in the same bank-funding diagnostic investigation (Marvin Phase 3, SHA `5cca0fbd1a`). Found-024 is the `rs-platform-wallet` ledger-corruption side; Found-025 is the `rs-sdk` address-sync silent-discard side. See also V28-303 in §7 (Known Issues) — the `wait_for_balance` timeouts attributed there to DAPI contention are a symptom of this race under parallelism.

##### Secondary findings from Marvin Phase 3 (filed under Found-025, not as standalone entries)

**QA-P3-002 (MEDIUM) — `wait_for_address_nonces_chain_confirmed` is a false proxy for recipient balance visibility**

Location: `tests/e2e/framework/bank.rs:526-561` and `framework/wait.rs:573-650`. This is a test-harness defect, not a production bug. The helper confirms that the SENDER's nonce advanced on-chain (the funding transaction was included in a block). It does NOT confirm that the RECIPIENT's balance is visible in the SDK's address-sync layer — which is precisely the gap Found-025 exploits. Under parallelism, the nonce confirmation completes while the SDK's snapshot for that sync cycle is already stale, giving tests false confidence that funding is complete. Severity MEDIUM (affects test reliability, not production). Fix: after the nonce confirmation, also poll `addresses_with_balances()` on the recipient until the expected balance appears, with a bounded timeout. This is a framework fix, not a spec pin.

**QA-P3-003 (LOW) — one-off `path segment not found in proof layer` grovedb error logged at DEBUG instead of WARN**

Location: `rs-sdk` (production side). A GroveDB path-not-found condition during proof verification is logged at DEBUG level with no proof-height or DAPI endpoint context. Should be WARN with structured fields (`proof_height`, `endpoint`, `path`). Severity LOW (observability gap, not data corruption). Not filed as a standalone Found-* entry — too low severity to warrant a regression pin; noted here so a future observability pass can pick it up.

#### Found-026 — `PlatformAddressWallet::next_unused_receive_address` pool-cursor bump may not enqueue address into BLAST sync provider's pending set (concurrent-load race)
- **Priority**: P2
- **Severity**: MEDIUM — concurrency-only; passes deterministically under `--test-threads=1`. Would erode test signal as parallelism scales, or if production-side traffic shifts toward concurrent address-derivation + sync.
- **Owner**: `rs-platform-wallet` (this crate). Suspected fix location: `packages/rs-platform-wallet/src/platform_addresses/wallet.rs:223-270` (`PlatformAddressWallet::next_unused_receive_address`) and its provider-enqueue boundary; possibly transport-layer in `rs-sdk` if the registration is lazy in `sync_balances`.
- **Status**: suspected — pinned by PA-008b (`cases/pa_008b_cross_wallet_funding.rs:37`). 14-thread full-suite cohort FAILS on the first marker `wait_for_balance` (panic site `:59`); `--test-threads=1` isolation re-run on 2026-05-14 PASSES in 158s. Live Cargo reproducer is PA-008b itself; no dedicated unit pin yet — needs TRACE instrumentation at the pool-bump + provider-enqueue boundary to confirm the hypothesis.
- **Wallet feature exercised**: `packages/rs-platform-wallet/src/platform_addresses/wallet.rs:223-270` (`PlatformAddressWallet::next_unused_receive_address`); transitively the unified `provider`'s pending-set management — promotion from `key_wallet::AddressPool::next_unused(..., add_to_state=true)` into the SDK `AddressProvider`'s `pending_addresses` snapshot consumed by BLAST sync at `platform_addresses/sync.rs:24-86`.
- **Suspected bug**: When `next_unused_receive_address` advances the pool cursor under concurrent load, the address may not be registered with the unified `provider`'s pending set in time. Concurrent BLAST sync iterations from sibling tests then complete and report `result.found` *without* this wallet's freshly-derived address. The wallet's local `wait_for_balance` polls the (un-tracked) address state and never sees the chain-time balance even after the broadcast lands. The bank's `wait_for_address_nonces_chain_confirmed` (`framework/bank.rs:526`) cleared in 682ms in the failing run — DAPI replica lag is NOT the primary cause; this is a wallet-side address-tracking gap.
- **Preconditions**: 14-thread test parallelism with sibling tests that also drive `sync_balances()` against shared infrastructure (DAPI / Drive); fresh derivation on the affected wallet happening inside a sibling-test sync window.
- **Scenario** (regression-pin shape, once instrumentation lands):
  1. Two `TestWallet`s A, B; each derives three fresh addresses via `next_unused_receive_address` under fan-out.
  2. A sibling test is currently running its own `sync_balances()` iteration against shared DAPI.
  3. The freshly-bumped slot on A may not be enqueued in the provider before the sibling's BLAST sync snapshots `pending_addresses`.
  4. Bank funds the new slot; broadcast lands chain-time (nonce-confirmed in <1s).
  5. `wait_for_balance` polls 71× across 120s; every poll observes `current=0`.
- **Assertions** (the proof shape, once instrumented):
  - TRACE log entry at `next_unused_receive_address`'s pool-bump line shows the new slot N being registered with the provider.
  - A subsequent BLAST sync's `pending_addresses` snapshot contains slot N.
  - `wait_for_balance` observes a non-zero balance on slot N within wallclock budget.
- **Expected** (after fix): the pool-cursor bump + provider-registration is atomic w.r.t. concurrent BLAST sync snapshots, OR the provider lazily includes newly-derived slots in subsequent iterations.
- **Actual** (current code, under 14-thread parallelism): `wait_for_balance` polls 71× across 120s, observing `current=0` every poll, `any_balance_change_observed=false` — the freshly-derived address never becomes visible in BLAST sync's view, despite the broadcast landing chain-time. Same address path used by sibling PA-008 (single-wallet, seq=29) and PA-008c (parallel-safe) both pass in the same failing run — what's structurally different is the `setup_a + setup_b` two-wallet interleave at `pa_008b_cross_wallet_funding.rs:46-47`.
- **Harness extensions required**:
  - TRACE instrumentation at `next_unused_receive_address`'s pool-bump line + the provider-enqueue site.
  - A reproducer that captures the precise interleave (sibling test's `sync_balances()` window vs the wallet's bump).
  - Optionally: a unit-level pin that drives `PlatformAddressWallet::sync_balances` immediately after a single `next_unused_receive_address` and asserts the address is in the provider's pending set.
- **Estimated complexity**: M to investigate; fix complexity depends on whether the gap is in `rs-platform-wallet` (atomic-registration patch) or `rs-sdk` (provider lazy-refresh) — see Found-025 for the matching SDK-side surface.
- **Rationale**: PA-008b currently surfaces this with a 120s `wait_for_balance` timeout under the full-suite 14-thread cohort, but passes solo. Without a Found-NNN pin, the suspicion lives only in TEST_SPEC.md's narrative changelog and erodes after a few months of doc rewrites. Pinning it here gives future investigators a stable reference and signals that PA-008b's flakiness has a hypothesised root cause, not just "concurrency hates us."
- **Cross-reference**: Found-025 covers the symmetric `rs-sdk` side — `sync_address_balances` silently discarding balance updates for addresses not in the `pending_addresses` snapshot. The two pins together capture both sides of the derive-then-sync race: Found-025 is "SDK drops the update if the address isn't yet known"; Found-026 is "wallet may not register the address with the SDK in time".

---

## 4. Harness extension roadmap

Aggregating "Harness extensions required" across §3 and proposing a build
order. Each wave unlocks the cases listed.

### Wave A — Identity signer + identity setup helpers
- Add `SeedBackedIdentitySigner` implementing `Signer<IdentityPublicKey>` in `framework/signer.rs` (DIP-9 derivation per `derive_ecdsa_identity_auth_keypair_from_master` at `wallet/identity/network/identity_handle.rs:143`).
- Add `derive_identity_key(seed_bytes, network, identity_index, key_index, purpose, security_level) -> IdentityPublicKey` test helper.
- Add `TestWallet::register_identity_from_addresses(funding: Credits) -> Identity` helper that builds the placeholder, calls `register_from_addresses`, and waits for on-chain visibility.
- Add `wait_for_identity_balance(identity_id, expected, timeout)` in `framework/wait.rs`.
- **Unlocks**: ID-001, ID-001c, ID-002, ID-002b, ID-003, ID-004, ID-005, ID-005b, ID-006, ID-006b, DPNS-001, DPNS-001b, DPNS-001c, DPNS-002 (partial), CT-001, DP-001, DP-001b, DP-001c, DP-002, DP-003, TK-001, TK-001b, TK-002, CN-001.

### Wave B — Multi-identity per setup
- Extend `setup()` to accept `setup_with_n_identities(n: u32) -> SetupGuard { test_wallet, identities: Vec<RegisteredIdentity> }`.
- **Unlocks**: ID-003, DP-002, DP-003.
- **Cost**: Wave A pre-requisite; ~150 LoC.

### Wave C — Contract fixture loader
- `tests/fixtures/contracts/` directory + `framework::fixtures::load_contract(name)` helper.
- One canonical `minimal.json` (one doc type, two scalar fields).
- **Unlocks**: CT-001, CT-002, CT-003.

### Wave D — Token contract operator config (SUPERSEDED by Wave G)
- Original plan: `Config::token_contract_id`, `Config::token_position`, optional `Config::token_claim_amount`; operator pre-funds tokens to a bank-derived identity (one-time, README'd next to bank pre-funding).
- Superseded: the wallet already accepts `tokens_schema_json` on `create_data_contract_with_signer` (`wallet/identity/network/contract.rs:124`), so the suite can deploy a fresh token contract per CI run instead of relying on operator pre-funding. See Wave G below.

### Wave E — SPV re-enablement (Task #15) — COMPLETE
- SPV block in `harness.rs:200-218` is active; `SpvContextProvider` is wired (replaces `TrustedHttpContextProvider`).
- `SpvHealth::status()` accessor is available in the manager.
- Core-funded test wallet helper (faucet integration) is ready.
- **Unlocked**: CR-001 (Pass), CR-003 (Pass), CR-002 (not implemented — test body TBD), AL-001 (implemented — red-real-fail; concurrent-build fails on UTXO visibility gap, fix tracked at task #382).
- **Note**: `PLATFORM_WALLET_E2E_DISABLE_SPV=1` is an operator escape hatch for ChainLock-cycle outages (rust-dashcore #470). It is NOT the default. SPV-on has been the operating mode since v17.

### Wave G — Token harness extensions — COMPLETE
- Replaces Wave D. The wallet's `create_data_contract_with_signer` already accepts a `tokens_schema_json` argument; Wave G assembles the V1 token-config JSON from a structured `TokenContractOpts` struct so test bodies stay terse and the schema-drift surface lives in exactly one place.
- Default contract is OnceCell-cached and shared across most TK cases (mirrors PA's bank-shared / per-test-wallet split). Tests that need a non-default config (pre-programmed distribution, groups, paused-on-create) opt into a fresh deploy.
- All helpers live in `packages/rs-platform-wallet/tests/e2e/framework/tokens.rs` (new module).
- Harness helpers (~19 total — helpers 6–10 and 14–19 are SDK-wrapper helpers, replacing what were previously tracked as Gap-T1..Gap-T6 wallet-API gaps; the wallet's public API does not need new methods to support these tests):
  1. `setup_with_token_contract(harness, opts: TokenContractOpts) -> TokenContractFixture` — registers an identity (via Wave A) and deploys a permissive owner-only token contract; default opts mirror DET's `build_register_token_task` (8 decimals, max supply 1e15, owner-only ChangeControlRules, no perpetual, allow-choose-destination).
  2. `setup_with_token_and_two_identities(harness, opts) -> (TokenContractFixture, TestIdentity)` — composes (1) with `register_extra_identity` for the multi-identity TK cases.
  3. `setup_with_token_and_three_identities(harness, opts) -> (TokenContractFixture, [TestIdentity; 2])` — three-identity variant for TK-014 group co-sign.
  4. `setup_with_token_pre_programmed_distribution(harness, payout, epoch_zero_at) -> TokenContractFixture` — TK-013 variant injecting a past-timestamp epoch-zero distribution.
  5. `mint_to(wallet, fixture, recipient, amount) -> MintResult` — one-line mint shortcut for tests that need a balance on a given identity before the operation under test.
  6. `token_balance_of(identity, fixture) -> TokenAmount` — read-side accessor; wraps `TokenInfo::fetch_one` (or equivalent SDK query) directly. SDK call site: `packages/rs-sdk/src/platform/fetch_many.rs` token-info variant. (Previously tracked as Gap-T2.)
  7. `token_supply_of(fixture) -> TokenAmount` — total-supply accessor; queries SDK token-supply endpoint directly. (Previously tracked as Gap-T3.)
  8. `token_is_paused_of(fixture) -> bool` — paused-flag accessor; re-fetches the data contract via `DataContract::fetch` and reads the token-state field. (Previously tracked as Gap-T4.)
  9. `token_pricing_of(fixture) -> Option<TokenPricingSchedule>` — pricing accessor; re-fetches the data contract and extracts the pricing schedule. (Previously tracked as Gap-T5.)
  10. `token_frozen_balance_of(identity, fixture) -> Option<TokenAmount>` — frozen-balance accessor; queries the SDK freeze-state proof endpoint directly. (Previously tracked as Gap-T6.)
  11. `wait_for_token_balance(identity, fixture, expected, timeout) -> Result<()>` — polls `token_balance_of` until equal-or-timeout; mirrors the PA `wait_for_balance` shape.
  12. `permissive_owner_token_contract_json(owner_id, opts) -> String` — pure helper that assembles the V1 token-contract JSON from the opts struct + owner id; the single source of truth for "what shape DPP wants today" (mirrors DET's `build_register_token_task` payload at `dash-evo-tool/tests/backend-e2e/framework/token_helpers.rs:33-96`).
  13. `register_extra_identity(harness, funding) -> TestIdentity` — registers a fresh identity from a freshly funded test wallet; mirrors DET's `ensure_second_identity()` at `dash-evo-tool/tests/backend-e2e/token_tasks.rs:35`. Likely shared with ID-002 / ID-003 / DP-002.
  14. `register_token_contract_via_sdk(sdk, owner_key, opts) -> DataContractId` — constructs the V1 token-contract document from `TokenContractOpts` and broadcasts via `Sdk::put_data_contract` (or the equivalent state-transition method). SDK call site: `packages/rs-sdk/src/platform/put.rs`. This is the SDK-direct path that helper (12) + `create_data_contract_with_signer` compose; exposed as a standalone helper for tests that need raw control. (Previously tracked as Gap-T1.)
  15. `token_balance_raw(sdk, identity_id, contract_id, token_position) -> TokenAmount` — lower-level variant of helper (6) accepting raw ids rather than a fixture; useful for cross-contract assertions.
  16. `token_supply_raw(sdk, contract_id, token_position) -> TokenAmount` — lower-level variant of helper (7).
  17. `token_is_paused_raw(sdk, contract_id, token_position) -> bool` — lower-level variant of helper (8).
  18. `token_pricing_raw(sdk, contract_id, token_position) -> Option<TokenPricingSchedule>` — lower-level variant of helper (9).
  19. `token_frozen_balance_raw(sdk, identity_id, contract_id, token_position) -> Option<TokenAmount>` — lower-level variant of helper (10).
- **Note on Gap-T1..Gap-T6**: these were previously listed as wallet-API surface gaps requiring new methods on `PlatformWallet`. That framing is superseded. Helpers 6–10 and 14–19 above implement the same functionality as framework-level SDK wrappers. No wallet public API change is needed; the test framework calls the SDK directly.
- **Unlocks**: TK-001, TK-001b, TK-001c, TK-002, TK-003, TK-004, TK-005, TK-005b, TK-006, TK-007, TK-008, TK-009, TK-010, TK-011, TK-012, TK-013, TK-014.

### Wave F — Test-only utility helpers
- `TestWallet::transfer_with_inputs` (PA-002 negative variant; PA-004b exact-balance setup).
- `TestWallet::transfer_capturing_st_bytes` (PA-006, PA-006b).
- `TestWallet::estimate_transfer_fee` (PA-002b).
- `Bank::total_credits` accessor exposed (already exists, just lift to public re-export if not).
- `TestRegistry::get_status(wallet_id)` (PA-004).
- `FUNDING_MUTEX` instrumentation hook (PA-008c).
- "Did we broadcast?" hook on the harness SDK (PA-004c, PA-013).
- Cancellation-point hook between broadcast and proof-fetch (Harness-G4).
- Test DAPI proxy / `httpmock` adapter (PA-013).
- **Unlocks**: PA-002 (negative), PA-002b, PA-004 (full assertions), PA-004b, PA-004c, PA-006, PA-006b, PA-008c, PA-009, PA-011, PA-012, PA-013, Harness-G1a, Harness-G1b, Harness-G4.
- **Cost**: ~200-400 LoC across multiple commits; the test-DAPI-proxy and cancellation-hook items are non-trivial and can land late.

<!-- merge note: kept theirs' updated build-order paragraph (Wave G/D supersession, SDK-wrapper helper note). Appended HEAD's "Wave E is complete" as a follow-on sentence reflecting Task #15 closure (CR-003 has flipped PASS). Preserved HEAD-only "Framework notes (post-V20)" subsection — distinct content the test-branch doesn't carry. -->
**Recommended build order**: Wave A first (highest leverage — unblocks 25+ cases), then Wave F's cheap helpers (estimate-fee, transfer-with-inputs, registry status, FUNDING_MUTEX hook) which unblock most P2 PA cases, then Wave C, then Wave B as ID-003/DP-002 land. Wave G unlocks the entire TK column once Wave A is in place; the SDK-wrapper helpers in Wave G (helpers 6–10 and 14–19, previously tracked as Gap-T1..T6) land together with Wave G, not as follow-up wallet PRs. Wave F's expensive items (test DAPI proxy, cancellation hook) and Wave E are independent and can run in parallel with the others once a champion is assigned. Wave D is superseded by Wave G. Wave E is complete (Task #15 closed; CR-003 has flipped PASS, see §3 CR-003 Status).

### Framework notes (post-V20)

**`bank.fund_address` — chain-confirmed-nonce wait (PR #3609 / upstream issue #3611)**

`bank.fund_address` now waits for the chain-confirmed nonce to advance before releasing `FUNDING_MUTEX`. This prevents a race where DAPI replica round-robin lag causes the next `fund_address` call to arrive at a replica that hasn't yet indexed the previous funding transaction, producing a stale-nonce rejection. The wait is bounded; if the nonce does not advance within the timeout, the call fails with a typed `BankNonceTimeout` error. Tests relying on serial funding order (PA-008, PA-008b, PA-008c) benefit from this without any test-side changes.

### Wallet-API gap notes (follow-up issues)

While drafting §3 the following minor public-API gaps were noted. None block
the spec but each would simplify a test if filed as a follow-up issue:

1. **No `PlatformWallet::fee_paid` accessor** — every PA case derives the fee from `Σ funded - Σ received - Σ remaining`. A first-class `last_transfer_fee()` (or a `fee` field on `PlatformAddressChangeSet`) would let assertions read the fee directly. Currently noted as a comment in `cases/transfer.rs:142-147`.
2. **No public sync-watermark getter on `PlatformAddressWallet`** — PA-007 needs to read the provider's `last_known_recent_block` to assert monotonicity. The field is internal; exposing a `pub fn sync_watermark() -> Option<RecentBlock>` would unblock cleanly.
3. **`IdentityManager::known_identities()` shape** — needed by ID-001's "exactly one identity registered" assertion. If the manager exposes only `BTreeMap<u32, ManagedIdentity>` without a length convenience, the test must pull internals; a `.len()` / `.identity_ids()` helper would be cleaner.
4. **Token-balance, supply, freeze, and pricing accessors on `PlatformWallet`** — `wallet/tokens/wallet.rs:248` already has `balance(...)`; the remaining read-side accessors (supply, freeze state, pricing, paused flag) are not yet on the wallet's public API. These are now covered by the SDK-wrapper helpers in `framework/tokens.rs` (Wave G helpers 6–10 and 14–19); adding first-class wallet methods remains a desirable but non-blocking follow-up. Previously tracked as Gap-T2..Gap-T6.
5. **DPNS `register_name_with_external_signer` lacks a "wait for visibility" partner** — Wave A would benefit from a `wait_for_dpns_name_visible(name, timeout)` helper, ideally co-located with `wait_for_balance` in `framework/wait.rs`.
6. **No protocol-version accessor for `min_input_amount` / `max_outputs`** — PA-009 and PA-014 need to read these from the active `PlatformVersion`; expose a thin test-friendly getter.

---

## 5. Out-of-scope register

Explicit list of what this suite WILL NOT cover, with reasons. Each entry
prevents future scope creep arguments.

1. **Shielded transfers** — entire `wallet/shielded/` surface. Reason: prover, viewing-key derivation, and note-selection are a parallel system; coverage belongs in a dedicated suite. Re-evaluate when shielded ships to mainnet.
<!-- merge note: item 2 — kept HEAD (SPV is enabled now via Task #15; withdrawal stays out-of-scope on its own merits, not on the SPV gate). Item 3 — kept theirs (Wave D is superseded by Wave G; the suite deploys per-CI rather than relying on operator pre-funding), which reflects the current architectural decision. -->
2. **Credit withdrawals** (`wallet/identity/network/withdrawal.rs`, `wallet/platform_addresses/withdrawal.rs`) — withdrawal verification requires Layer-1 observation of the withdrawal tx. SPV is now enabled (Task #15 complete) but withdrawal coverage is deferred pending a dedicated test design — the flow is more complex than a simple SPV read and DET currently owns the canonical coverage.
3. **Operator-pre-funded testnet token contracts** — the original Wave D plan (env-config + operator-provided contract id) is superseded. The suite deploys a fresh token contract per CI run via Wave G; no operator-side registry is required and no testnet contract id is consumed from config.
4. **Asset-lock-funded identity registration** — the bank holds Platform credits, not Core UTXOs. The address-funded variant (ID-001) covers registration from the wallet's perspective; full registration asset-lock coverage stays with DET (`dash-evo-tool/tests/backend-e2e/identity_create.rs`). Asset-lock-funded **top-up** of an existing identity is now in scope: see ID-002b.
5. **DAPI Core path** (`tx_is_ours`, mn-list diffs, peer behaviour) — DET territory; this suite tests the wallet against DAPI, not DAPI itself.
6. **Cross-process bank concurrency** — README §"Multi-process safety" documents the operator-side requirement; not a test concern.
7. **Mainnet runs** — config supports `network=mainnet` but the suite's bank-funded model is testnet-by-policy. Mainnet runs require an explicit operator review; out-of-scope for automation.
8. **CN-002 (masternode voting)** — needs a regtest-with-masternodes harness that doesn't exist today.
9. **Non-BIP-39 mnemonic / seed sources** — see §1.2. Mnemonics must be drawn from the BIP-39 English wordlist; raw-entropy and arbitrary-UTF-8 paths are out of scope.
10. **Clock-skew / wall-clock-dependent assertions** — testnet runners are assumed to have NTP. Tests that rely on chain timestamps assume the runner's wall clock is within a few seconds of chain time. Cases that need to assert behaviour under arbitrary skew belong in a unit-test layer below this suite.

---

## 6. Open questions for product owner

Each question's answer changes the spec; numbered for reference.

1. **Token contract registry** — superseded: Wave G deploys a fresh token contract per CI run via the wallet's `create_data_contract_with_signer` (`tokens_schema_json` argument). No operator-side registry is required. Retained here for historical context.
2. **Contested-name coverage** — should CN-001 be promoted to P1, or do we accept DET parity and leave it P2/deferred?
3. **Long-running tests** — PA-005 (16 funding round-trips, ~3 min) is borderline. Do we accept multi-minute tests in the default `cargo test --test e2e` run, or gate them behind a `slow-tests` cargo feature?
4. **Identity withdrawal coverage** — SPV (Task #15) is now live. The question remains: do we add withdrawal coverage here, or defer to DET's exclusive territory?
5. **Mainnet smoke** — should the suite ever support a single, opt-in mainnet smoke case (e.g. PA-001 with a tiny `1_000`-credit transfer) for release-gate validation?
6. **Fee-bound numbers** — PA-003 asserts `fee_5 - fee_1 < 1_000_000`. Should we baseline empirical fee numbers and tighten these bounds in a follow-up, or keep them loose and rely on protocol-version bumps to reset them?
7. **Deterministic fixture network** — testnet is shared and noisy. Is there appetite to maintain a regtest-with-Drive cluster for CI exclusively, or do we accept testnet flakiness as the operating constraint?
8. **Test DAPI proxy infra** — PA-013 and the broadcast-retry contract require a controllable test DAPI proxy. Build it bespoke (`httpmock`-based), reuse an existing harness from elsewhere in the workspace, or defer the case until the proxy lands?
9. **Cancellation-hook plumbing** — Harness-G4 needs a test-only injection point between broadcast and proof-fetch. Acceptable to add a `cfg(test)` hook on the wallet, or must this stay external (wrap the future in a `select!` from the test side and accept coarser cancellation granularity)?

---

## 7. Known Issues

Tracked production bugs and harness gaps that affect test outcomes. Tests are
`#[ignore]`d in these cases — but **`#[ignore]` does NOT mean "never runs"**:

- `cargo test` (default): ignored tests are **skipped**.
- `cargo test -- --ignored`: runs **only** ignored tests. PA-004b and PA-009 execute under this flag and fail by design. Any failure mode other than the one documented per-entry below is a regression.

Do not modify production code in this section — these are documentation entries only.

### V27-007 — `PlatformAddressWallet::transfer` ledger pollution (production bug)

**Status**: FIXED at `16636f01c0`. Pinned as regression in Found-024 (§3 Found-bug pins). Tests `pa_004b_sweep_below_dust_gate_no_broadcast` and `pa_009_cleanup_gate_tracks_platform_version_min_input_amount` had their `#[ignore]` removed at the same commit and are now passing. The investigation chain closed at QA-V45-010.

**Historical failure mode** (PA-004b and PA-009, pre-fix): the `assert_eq!(addr_1_residual, TARGET_RESIDUAL, ...)` assertion panicked because `total_credits()` returned the bank's full balance (~40.8 tDASH) instead of the wallet's actual residual (`TARGET_RESIDUAL = 1_000`). Any recurrence of that failure pattern is a regression of V27-007 and will be caught by the Found-024 regression pin.

**Bug**: `PlatformAddressWallet::transfer` at
`packages/rs-platform-wallet/src/wallet/platform_addresses/transfer.rs:160` calls
`account.set_address_credit_balance(p2pkh, funds.balance, key_source.as_ref())`
for every address in the transition (inputs ∪ outputs), with no ownership check.
When a wallet transfers to an externally-owned address (e.g., bank's primary
receive address), the externally-owned post-balance gets staged into the source
wallet's local `address_balances` ledger.

**Symptom**: `wallet.total_credits()` after a transfer-to-external returns the
external address's balance summed in. PA-004b/PA-009 see the bank's full
~40.8 tDASH on what should be a dust-residual wallet → assertions panic.

**Same unguarded primitive** also exists at:
- `packages/rs-platform-wallet/src/wallet/platform_addresses/withdrawal.rs:141`
- `packages/rs-platform-wallet/src/wallet/platform_addresses/fund_from_asset_lock.rs:129`

Currently safe by caller behavior (those iterate only-owned addresses), but
identical shape; defense-in-depth fix should apply there too.

**Severity**:
- **Tests**: HIGH — every `total_credits()` post-transfer-to-external is a false read.
- **SDK consumers**: HIGH — anyone following `transfer → read total_credits` sees
  inflated balances and could make wrong spend decisions.
- **Production sweep path**: MEDIUM-LOW — sweep would build inputs against the
  external address, but the source wallet can't sign for it; Drive rejects the
  transition; error swallowed → no on-chain leak.

**Fix sketch** (~6 LOC, do not apply in this PR):
Filter the loop in `transfer.rs:145-160` so `set_address_credit_balance` is
called only for addresses the source account owns:

```rust
for (addr, maybe_info) in address_infos.iter() {
    let PlatformAddress::P2pkh(hash) = addr else { continue };
    let p2pkh = PlatformP2PKHAddress::new(*hash);
    // Skip addresses the source account doesn't own; address_infos covers
    // inputs ∪ outputs and outputs we don't own must not pollute the local
    // credit ledger.
    if !account.address_balances.contains_key(&p2pkh)
        && account.addresses.address_info_by_p2pkh(&p2pkh).is_none()
    {
        continue;
    }
    // ... existing set_address_credit_balance + changeset push
}
```

Defense-in-depth: apply same filter at `withdrawal.rs:141` and
`fund_from_asset_lock.rs:129`. Optionally make `set_address_credit_balance`
itself reject addresses not in the pool (wider change in `key-wallet`).

**Confirmation audit**:
- Search for any aggregate that sums `total_credits()` across multiple wallets in the manager (production code, dashboards, telemetry) — would double-count.
- Run e2e suite with the fix in place, verify PA-004b/PA-009 pass.
- Add debug assertion in `set_address_credit_balance` that the address is in the pool — every callsite that violates would surface.

**Investigated**: Bilby read-only audit, 2026-05-08, agent ID `a2d81349f872a0c6a`.

---

### V28-303 — PA-003 partial fix: deficit closed, contention timeout remains

**Status**: partial. PA-003 (`pa_003_fee_scaling`) is NOT `#[ignore]`'d — it runs in the default `cargo test` cohort. However, it is not reliably green under concurrency.

**What V28-303 did**: bumped `FUNDING_CREDITS` from 400M to 500M and `FUNDING_FLOOR` from 350M to 450M (`cases/pa_003_fee_scaling.rs`). This closed the "available 240,524,980 credits, required 250,000,000" deficit that caused a deterministic failure on the 5-output transfer leg: with 400M pre-fund, `addr_src` retained only ~200M after the 1-out transfer and five marker transfers, giving ~235M of reachable candidate balance against a 250M requirement. With 500M pre-fund, `addr_src` retains ≥300M post-setup and the auto-selector has comfortable headroom.

**What V28-303 did NOT fix**: at `threads=8` (standard CI concurrency), the `wait_for_balance` call on funding confirmation hits the 60s deadline before the balance settles. Current observed failure mode:

```
wait_for_balance timed out after 60s — addr_src balance never reached FUNDING_FLOOR (450_000_000)
```

This is a contention symptom: eight concurrent tests competing for DAPI bandwidth and bank-wallet nonce slots delay the funding broadcast confirmation beyond the per-step `STEP_TIMEOUT = Duration::from_secs(60)`.

**Note on TK-suite flakes**: Marvin's Phase 3 reproduction (SHA `5cca0fbd1a`) identified that the `wait_for_balance` timeout pattern in TK tests has a deeper root cause than pure DAPI contention. Found-025 (§3) documents the load-bearing mechanism: the `rs-sdk` `incremental_catch_up` filter at `packages/rs-sdk/src/platform/address_sync/mod.rs:619` silently discards balance updates for freshly-allocated addresses that were not in the `pending_addresses` snapshot at sync entry. The timeout is the observable symptom; the SDK's silent discard is the cause. QA-V28-403 (raise `STEP_TIMEOUT`) is still a valid mitigation for pure contention cases, but TK-suite flakes should be assumed to have the Found-025 race until the upstream fix lands.

**Claiming "V28-303 fixes PA-003" or "PA-003 first time passing" is wrong.** V28-303 narrows the failure surface (one deterministic failure mode removed) but does not green-light PA-003 in standard CI.

**Real fix path**: QA-V28-403 — raise `STEP_TIMEOUT` per step (or use a dynamic deadline tied to observed DAPI latency under load). Until that lands, PA-003 may pass in low-concurrency or low-load runs and fail under the standard 8-thread CI tier.

---


<sub>Catalogued by Marvin (QA), with the resigned competence of someone who has read every line of this code twice. Edge-case expansion by Trillian, who knows that the difference between "tested" and "tested at the boundary" is the difference between "ships" and "ships back".</sub>
