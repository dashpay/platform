---
title: "Persistence Redesign — BDK-style changesets across key-wallet and platform-wallet"
type: refactor
status: in-progress
date: 2026-04-08
updated: 2026-04-10
---

# Persistence Redesign

Living plan for the BDK-style changeset persistence work that spans
`rust-dashcore/key-wallet` and `platform/packages/rs-platform-wallet`.
Supersedes the "PR-22 ChangeSet-based persistence" row in `PLAN.md` — that
was the first attempt, which grew stale and has been partially rewritten.

Branches (all called `feat/platform-wallet2` in their respective repos):

- **rust-dashcore** `feat/platform-wallet2` — base `v0.42-dev`
- **platform** `feat/platform-wallet2` — base `feat/platform-wallet`

## Why

The original `feat/platform-wallet` attempt modified dashcore's
`WalletManager` to use per-wallet `Arc<RwLock<T>>` locks and a
`WalletPersistence` trait baked into the manager. It caused 7× SPV
slowdown from lock contention.

`feat/platform-wallet2` takes a different approach:

- Keep dashcore's `WalletManager` **identical to `v0.42-dev`** — no
  per-wallet locks, no baked-in persistence.
- Build persistence on top of a **BDK-style changeset API** that
  every wallet mutation emits, and that an external persister consumes
  at its own pace.
- The changeset carries the wallet's **native types** (`Utxo`,
  `TransactionRecord`, `ManagedIdentity`) rather than flattened
  persistence-friendly entries. Persistence backends translate at
  the storage layer.

## Shape of the design (BDK pattern)

The mental model is BDK's `apply_block` / `apply_changeset` — mutation
methods return a `ChangeSet` describing what changed; apply methods
consume one to restore state during load.

**Key-wallet side (`rust-dashcore/key-wallet`):**

```
WalletChangeSet
├── account_keys : Option<AccountKeyChangeSet>     // HD accounts added
├── chain        : Option<ChainChangeSet>           // synced height / block hash
├── balance      : Option<BalanceChangeSet>         // cached balance delta
└── per_account  : BTreeMap<AccountType, AccountChangeSet>
                                                    // one bucket per ManagedCoreAccount

AccountChangeSet
├── addresses_used      : BTreeSet<Address>
├── highest_used        : BTreeMap<AddressPoolType, u32>
├── utxos_added         : BTreeMap<OutPoint, Utxo>    // native type, not flattened
├── utxos_spent         : BTreeSet<OutPoint>
├── utxos_instant_locked: BTreeSet<OutPoint>
└── transactions        : BTreeMap<Txid, TransactionRecord>
```

**Mutation API:** every per-account mutation method on `ManagedCoreAccount`
returns `(value, WalletChangeSet)`. The method writes into
`cs.per_account[self.account_type.to_account_type()]` directly — no
address-based routing at accumulation time.

**Apply API:** `ManagedWalletInfo::apply_changeset(wallet, cs)` delegates
per-bucket to `ManagedCoreAccount::apply_changeset(account_type, bucket)`
via a direct `get_by_account_type_mut` lookup. No scanning, no fallbacks.

**Invariants:**
- Idempotent — applying the same changeset N times = applying it once.
- Monotonic on state flags — `is_confirmed` / `is_instantlocked` OR'd
  on replay; never regress.
- No re-emission — apply doesn't return a new changeset.
- Best-effort routing — buckets with unknown `AccountType` are silently
  skipped with a `tracing::warn!`.
- Loud on `account_keys` failures — a missing account cascades, so
  errors bubble up.

**Platform-wallet side (`packages/rs-platform-wallet`):**

`PlatformWalletInfo` wraps a `ManagedWalletInfo` inside `core_wallet` and
adds four platform-specific fields:

- `identity_manager`    — registered identities, DPNS names, top-ups
- `tracked_asset_locks` — asset lock lifecycle (created → IS-locked → used)
- `platform_address_balances` — credit balances per platform address
- `token_balances` / `token_watched` — Platform token balances

Each gets its own sub-changeset. The top-level `PlatformWalletChangeSet`
will look like:

```rust
pub struct PlatformWalletChangeSet {
    pub core: Option<key_wallet::changeset::WalletChangeSet>,  // delegated to core_wallet
    pub identities: Option<IdentityChangeSet>,
    pub contacts: Option<ContactChangeSet>,
    pub platform_addresses: Option<PlatformAddressChangeSet>,
    pub asset_locks: Option<AssetLockChangeSet>,
    pub token_balances: Option<TokenBalanceChangeSet>,  // NEW
}
```

`PlatformWalletInfo::apply_changeset` will delegate `core` to
`ManagedWalletInfo::apply_changeset` and apply platform-specific
sub-changesets in-place.

## What's done

| Phase | Commit | What |
|-------|--------|------|
| **1+2** | `8384ae88` (dashcore) | `changeset` module skeleton, `Merge` trait, `AddressPool::set_highest_used`, `ManagedCoreAccount::{insert,remove}_utxo`, `Wallet::add_account` made idempotent, first round of tests |
| **3+4+5+6** | `66e6462d` (dashcore) | BDK-style mutate-and-return API: every `ManagedCoreAccount` / `ManagedWalletInfo` mutation method returns `(value, WalletChangeSet)`. First review pass flagged: field rename drift, routing gaps, update_utxos state-flag bug. |
| **P3 review fix** | `1d6b6160` (dashcore) | Rename `last_revealed` → `highest_used`; monotonic UTXO state flags on replay; single-address routing strategy in apply; ApplyChangeSet trait extraction |
| **7** | `e018f773` (dashcore) | `ManagedWalletInfo::apply_changeset` restore path — address-routed |
| **7 review fix** | `1d6b6160` (dashcore) | (same commit as above) |
| **7.5 (BDK)** | `c07207a0` (dashcore) | **Structural:** `WalletChangeSet` restructured to BDK-style `per_account: BTreeMap<AccountType, AccountChangeSet>`. Deletes ~5 routing helpers. Mutation methods pre-route at emission time. `AccountType` gets `Ord/PartialOrd/Hash` derives. |
| **7.5 review fix** | `6111c8fb` (dashcore) | `transactions: Vec → BTreeMap<Txid, TransactionRecord>` (free last-write-wins on merge, kills O(n²) dedup loop); `update_utxos` inline style; `mark_utxos_instant_send` extend-not-assign bug fix; `PlatformPayment` replay regression fix via `contains_account_type`; `tracing::warn!` on unknown bucket; `apply` → `apply_changeset` rename; ApplyChangeSet trait deleted (unused); `_match` → `_matching` accessor rename; Ord landmine doc comment; stronger mixed-bucket test; stale doc fixes |
| **7.5 (platform wire-through)** | `a193fa3c` (platform) | `PlatformWalletInfo::update_balance` / `mark_instant_send_utxos` return `WalletChangeSet` — trait signature match |
| **8** | `45a9b126` (dashcore) + `c89f8412` (platform) | Drop `add_to_state` flag from address generation — every real caller was passing `true`, the one internal `false` was vestigial, the `false` branch produced ghost addresses. Net -14 LOC. **Also being extracted as a standalone PR off base branches** (see "Open PRs" below) because it's conceptually independent. |

## Commit chain

```
c89f8412 (platform)   refactor(platform-wallet): drop add_to_state arg
45a9b126 (dashcore)   refactor(key-wallet): drop add_to_state flag
6111c8fb (dashcore)   fix(key-wallet): Phase 7.5 review pass
c07207a0 (dashcore)   refactor(key-wallet): BDK-style per-account changeset shape
1d6b6160 (dashcore)   fix(key-wallet): Phase 7 review pass on apply_changeset
e018f773 (dashcore)   feat(key-wallet): apply_changeset for idempotent restore
66e6462d (dashcore)   feat(key-wallet): BDK-style mutate-and-return changesets
8384ae88 (dashcore)   feat(key-wallet): changeset module + prep work
b8ffa26f (dashcore)   feat(key-wallet-manager): insert_wallet + get_wallet_and_info_mut
                      (preceded by v0.42-dev head: dda1db7a)
```

Tests at each step: key-wallet 491 passing, key-wallet-manager 39 passing,
key-wallet-ffi compiles clean, platform-wallet compiles clean.

## Where we are right now

Mid-Phase-9a. The decision fork on `PlatformWalletChangeSet` was
resolved in favour of **Option A — unify** (the platform-wallet-local
sub-changeset types are legacy from the evo-tool migration and should
be deleted in favour of embedding `key_wallet::changeset::WalletChangeSet`).

**Survey findings** (completed task P9a-1):

`PlatformWalletChangeSet` at
`packages/rs-platform-wallet/src/changeset/changeset.rs:368` currently
has its own copies of:

- `ChainChangeSet`         (line 37)   — duplicates `key_wallet::changeset::ChainChangeSet`
- `TransactionChangeSet` + `TransactionEntry` (lines 69, 92)
  — duplicates key-wallet's, but with a flattened `TransactionEntry`
  that drops most of the native `TransactionRecord` fields (direction,
  input/output details, transaction_type, …)
- `UtxoChangeSet`          (line 115)  — **lossy**: stores
  `BTreeMap<OutPoint, u64>` (just value!), drops address/script/coinbase/confirmed/etc
- `AccountChangeSet`       (line 251)  — uses
  `BTreeMap<(u32, DerivationPathReference), u32>` keys vs key-wallet's
  per-account `BTreeMap<AddressPoolType, u32>`

All four predate the key-wallet `WalletChangeSet` work. The comment at
the top of the file even says *"Sub-changesets are modelled after the
real types used in `key-wallet`"* — they were always intended as
stand-ins.

Genuinely platform-specific (keep as-is):
- `IdentityChangeSet` + `IdentityEntry`
- `ContactChangeSet` + `ContactRequestEntry`
- `PlatformAddressChangeSet` + `PlatformAddressEntry`
- `AssetLockChangeSet` + `AssetLockEntry`

Missing (needs to be added):
- `TokenBalanceChangeSet` — for `PlatformWalletInfo.token_balances` / `token_watched`

**`PlatformWalletPersistence` trait** at
`packages/rs-platform-wallet/src/changeset/traits.rs:23` already exposes
`store` / `flush` / `load`. The existing skeleton
`PlatformWallet::apply()` at `platform_wallet.rs:236` only wires
`asset_locks.restore_from_changeset_blocking(...)` with TODOs for the
rest — it'll be rewritten to become
`PlatformWalletInfo::apply_changeset`.

The `queue_persist` / `flush_persist` / `load_persisted` methods on
`PlatformWallet` carry a `TODO: What these methods for? can we remove?`
comment — they're dead plumbing that predates this redesign and will
be replaced by the proper emit/apply flow in Phase 9c/9d.

**`ApplyChangeSet` trait:** deleted from key-wallet in Phase 7.5-fix
(it was unused outside a single `WalletManager<T>` wrapper). No stale
references remain in platform-wallet.

## What's next — Phase 9 breakdown

### 9a — `PlatformWalletInfo::apply_changeset` (in progress)

Split into:

- **9a-2a** ✅ audit: identify all callers of the deleted types in
  platform-wallet src. Done — all uses are confined to
  `changeset/changeset.rs`, `changeset/mod.rs`, and `lib.rs` re-exports.
  No external consumers.
- **9a-2b** restructure `PlatformWalletChangeSet`:
  - Delete `ChainChangeSet`, `TransactionChangeSet`, `TransactionEntry`,
    `UtxoChangeSet`, `AccountChangeSet` definitions (lines 31–273 of
    `changeset.rs`).
  - Replace the four top-level fields (`chain`, `accounts`,
    `transactions`, `utxos`) with a single
    `pub core: Option<key_wallet::changeset::WalletChangeSet>`.
  - Keep `identities`, `contacts`, `platform_addresses`, `asset_locks`.
  - Add `token_balances: Option<TokenBalanceChangeSet>` (define the
    type too — mirrors `PlatformWalletInfo.token_balances: BTreeMap<(Identifier, Identifier), TokenAmount>`).
  - Update `Merge` impl and `is_empty`.
- **9a-2c** update changeset tests for the new shape.
- **9a-2d** update the `lib.rs` and `changeset/mod.rs` re-exports —
  drop the deleted types from the public surface.
- **9a-2e** `cargo check -p platform-wallet` passes.
- **9a-3** implement `PlatformWalletInfo::apply_changeset(&mut self,
  wallet: &mut Wallet, cs: &PlatformWalletChangeSet) -> Result<(),
  ApplyError>`:
  - Delegate `cs.core` → `self.core_wallet.apply_changeset(wallet, core)`.
  - Apply `cs.identities` into `self.identity_manager`.
  - Apply `cs.contacts` into `self.identity_manager.contacts` (or wherever
    DashPay contacts live — survey to confirm).
  - Apply `cs.platform_addresses` into `self.platform_address_balances`.
  - Apply `cs.asset_locks` into `self.tracked_asset_locks`.
  - Apply `cs.token_balances` into `self.token_balances` / `token_watched`.
  - Recompute cached balance.
  - Error only on cascade failures (e.g. `core_wallet.apply_changeset`
    fails on account key derivation); silently skip unroutable
    platform-specific entries with a `tracing::warn!`.
- **9a-4** round-trip tests: build a `PlatformWalletInfo`, mutate each
  platform-specific field, capture the changeset (by hand for now —
  emission is Phase 9c), apply to a sibling, assert state converges.
  Idempotent double-apply test.
- **9a-5** `cargo check` + test on all affected crates. Commit.

**9a scope does NOT include:** wiring mutation methods to emit
changesets, wiring the persister to store them, or SQLite work. Those
are 9b+.

### 9b — Mutation methods emit platform-specific changesets

Make the platform-specific mutations on `IdentityManager`,
`AssetLockManager`, DashPay, Platform address registration, and Token
tracking all return `PlatformWalletChangeSet` (or a narrower type that
merges in) instead of `()`.

- List the mutation methods per field (survey needed).
- For each method: mirror the pattern from
  `ManagedCoreAccount::mark_address_used` — emit into
  `cs.<sub>.entry(...)` directly.
- Update orchestration code (e.g. SPV callback paths) to accumulate
  the returned changesets into a single `PlatformWalletChangeSet` per
  operation.

### 9c — Persister emit path

- Remove the stale `queue_persist` / `flush_persist` / `load_persisted`
  on `PlatformWallet`.
- Define the emission flow: SPV block callback → mutate → capture
  `PlatformWalletChangeSet` → feed into an in-memory buffer inside
  `PlatformWalletPersistence` implementation.
- Flush strategy: **"always Incremental + smart flush"** —
  `store(cs)` merges into the buffer (microseconds), flush triggers
  are:
  - Event-driven: on `SyncComplete`
  - Debounced: 30s after the last `store()` call
  - Explicit: on wallet unload / shutdown
- Never flush during cold sync.

### 9d — Persister apply (restore) path

On wallet load:
1. `PlatformWalletPersistence::load(wallet_id)` reads the persisted
   `PlatformWalletChangeSet` from storage.
2. Construct a fresh `Wallet` + empty `PlatformWalletInfo`.
3. Call `PlatformWalletInfo::apply_changeset(wallet, &cs)`.
4. Insert the restored pair into `WalletManager` via `insert_wallet`.

### 9e — SQLite persister implementation

Translate `PlatformWalletChangeSet` → SQLite rows.
- Global `Mutex<BTreeMap<WalletId, PlatformWalletChangeSet>>` buffer
  (1–5 wallets in practice — DashMap locks per key, not worth it).
- `store()` merges into the buffer inside the Mutex.
- `flush()` drains the buffer and writes to SQLite.
- Tables: one per logical entity family (`utxos`, `transactions`,
  `identities`, `asset_locks`, `platform_addresses`, `token_balances`,
  `accounts_revealed`, `chain_state`). Keyed by `wallet_id`.

### 9f — evo-tool integration

The only consumer outside platform-wallet today. Update evo-tool's
wallet-load path to use `PlatformWalletPersistence::load` + apply
flow instead of its current eager-serialization approach.

## Open PRs

- **dashcore `refactor/drop-add-to-state-flag`** (being extracted by
  background agent as of 2026-04-10) — Phase 8 refactor, standalone
  PR off `v0.42-dev`. No dependency on the rest of this branch; safe
  to land independently.
- **platform `refactor/drop-add-to-state-flag`** (same agent) —
  companion PR off `feat/platform-wallet`, depends on the dashcore
  PR merging first.

## Review policy

After each substantive commit, run three reviewer agents in parallel:

- `rust-quality-engineer` — Rust correctness, unwraps, idempotency,
  edge cases
- `code-simplicity-reviewer` — YAGNI, dead code, over-engineering
- `pattern-recognition-specialist` — consistency across mutation sites,
  naming, doc comments

The Phase 3/4/7/7.5 commits all had a review pass that caught real
bugs (the `update_utxos` state-flag gap, the `mark_utxos_instant_send`
assignment clobber, the `PlatformPayment` replay regression). Don't
skip the review round just because the commit "looks small."

## Decisions log

Key design calls and why, so we don't relitigate them:

- **BDK-style mutate-and-return, not compute-and-apply.** BDK's pattern
  is simpler at call sites (no two-phase dance) and avoids the
  temptation to hide mutations behind a "pure" compute function that
  still reads heaps of state. Mutation methods mutate and return a
  delta; no separate `compute_*` methods.
- **Native types in changesets, not flattened entries.** The
  persister translates native → storage at its layer. Keeps the
  changeset API lossless and matches mutation-site output 1:1.
- **Per-account bucketing in `WalletChangeSet::per_account`, not flat
  top-level fields.** Pre-routes at emission time. Kills the
  multi-account transaction collision bug (same txid in two accounts
  with different per-account views). Also kills ~5 routing helpers.
- **`WalletChangeSet` returned from every mutation, not
  `AccountChangeSet`.** Uniform return type — one merge pattern at
  every call site. Mutation methods know their own `account_type` and
  write into `cs.account_bucket(ty)` directly.
- **`ApplyChangeSet` trait deleted.** Unused outside a single
  `WalletManager<T>` wrapper. Concrete
  `impl WalletManager<ManagedWalletInfo>` is fine; if
  `PlatformWalletInfo` needs a similar wrapper later, it can get its
  own concrete impl block.
- **No `peek_*` address generation variants.** YAGNI — nothing in the
  codebase needed pure-read address derivation. The old `add_to_state`
  flag was dead weight.
- **`transactions: BTreeMap<Txid, TransactionRecord>`, not `Vec`.** The
  map encoding makes "at most one record per txid per account" a type
  invariant; merge dedup is free via `BTreeMap::extend`.
- **UTXO state flags are monotonic on replay.** `is_confirmed` /
  `is_instantlocked` OR'd with existing entry, never overwritten.
  Prevents stale-replay regressions when a live mutation has already
  advanced state.
- **Option A on `PlatformWalletChangeSet`** (this document's current
  task): embed `key_wallet::changeset::WalletChangeSet` via a `core`
  field, delete the legacy evo-tool-migrated duplicate types. The
  platform-wallet types were stand-ins from before key-wallet had a
  changeset; carrying both forward would be translation-layer tax
  forever.

## Links

- [`PLAN.md`](PLAN.md) — the full platform-wallet project plan.
  This document is a focused plug-in covering the persistence
  redesign that supersedes the "PR-22 ChangeSet-based persistence"
  row.
- [`key-wallet/CLAUDE.md`](../../../rust-dashcore/key-wallet/CLAUDE.md)
  — key-wallet architectural overview.
