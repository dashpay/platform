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

## Evo-tool coupling — the DB layer is stable, the persister is not

Phase 9a-2 (unify `PlatformWalletChangeSet`) immediately breaks
evo-tool's `SqliteWalletPersister` at
`dash-evo-tool/src/changeset/sqlite.rs` because it imports every
deleted type (`ChainChangeSet`, `TransactionChangeSet`,
`TransactionEntry`, `UtxoChangeSet`, `AccountChangeSet`, …). Evo-tool
also has backend-task code in `register_identity.rs`,
`top_up_identity.rs`, and `contact_requests.rs` that constructs those
types directly.

### What stays — the DB schema and query modules

Research against evo-tool `v1.0-dev` (the canonical stable branch —
no feat/platform-wallet2 persistence work) confirms the **DB schema
is already rich enough** to hold the new native changeset types:

- **`utxos`** (`src/database/initialization.rs`, `src/database/utxo.rs`)
  — has `txid BLOB, vout INTEGER, address TEXT NOT NULL, value INTEGER NOT NULL, script_pubkey BLOB NOT NULL, network TEXT NOT NULL`.
  The current persister writes empty-string placeholders for
  `address` / `script_pubkey`; the columns themselves are sufficient
  for native `key_wallet::Utxo` data. Missing columns for
  `is_coinbase`, `is_confirmed`, `is_instantlocked`, `block_height`
  — these are either recomputed on load or would need a schema
  migration.
- **`wallet_transactions`** — has
  `seed_hash, txid, network, timestamp, height, block_hash, net_amount, fee, label, is_ours, raw_transaction, status`.
  Every column that the native `TransactionRecord` needs already
  exists. `input_details` / `output_details` / direction /
  transaction_type are recomputed from the deserialized
  `raw_transaction` on load.
- **`wallet`** (`src/database/wallet.rs`) — has `seed_hash`, balance
  columns, `last_terminal_block`, network metadata. Matches
  `ChainChangeSet` + `BalanceChangeSet` needs.
- **`wallet_addresses`** — `seed_hash, address, derivation_path, path_reference, path_type`.
  Maps to address pools but the relationship to key-wallet's
  per-pool `highest_used` needs translation.
- **`platform_address_balances`** — `seed_hash, address, balance, nonce, updated_at, last_full_sync_balance`.
  Matches `PlatformAddressChangeSet` 1:1 (with an extra `nonce`
  column that the native platform_address_balances BTreeMap doesn't
  carry — that's a gap, either drop the nonce column or add a nonce
  field to the runtime state).
- **`asset_lock_transaction`** — has every field
  `AssetLockEntry` carries (transaction_data, amount, instant_lock_data,
  chain_locked_height, identity_id, account_index, funding_type,
  identity_index, proof_data).
- **`identity`**, **`top_up`**, **`dashpay_*`** — identity metadata,
  top-ups, contacts. Match the existing `IdentityChangeSet` /
  `ContactChangeSet` shapes (with known gaps: identity timestamps,
  contact metadata).

### What was introduced by the persister and will be deleted

- **`wallet_account_state`** — not in v1.0-dev. Created inline by
  `SqliteWalletPersister::persist_accounts` via
  `CREATE TABLE IF NOT EXISTS`. Schema:
  `(seed_hash, account_index, path_reference, last_revealed, network)`.
  **Needs migration decision:** either promote into a first-class
  table in `initialization.rs` (keyed by the new per-account shape —
  `(account_index, pool_type)` instead of `(account_index, path_reference)`)
  or fold the reveal watermark into `wallet_addresses` derivation_path.
- **`wallet_identity_dpns_names`** — not in v1.0-dev. Created inline
  by the persister. Schema: `(identity_id, name, network)`. Only
  stores string names; `DpnsNameInfo` metadata from the runtime
  `ManagedIdentity.dpns_names: Vec<DpnsNameInfo>` is dropped.

### Direct writes — the persister doesn't own everything

The current persister **does not serialize all wallet writes**.
Direct writes that bypass the persister still exist at call sites in
SPV processing and backend tasks, calling domain writer functions
like `Database::insert_utxo`, `drop_utxo`, `replace_wallet_transactions`,
`store_wallet`, `update_wallet_balances`, `store_identities`, etc.

This means: **the adapter doesn't need to replace all writes —
only the ones the current persister handles.** Direct writes keep
working as-is. The persister becomes a translation layer between
`PlatformWalletChangeSet` and a subset of the existing
`database::*` writer methods.

### Strategy — thin adapter, not raw SQL

The current `SqliteWalletPersister` (~1000 LOC) writes raw SQL that
duplicates logic already present in `database/wallet.rs`,
`database/utxo.rs`, `database/identities.rs`, etc. The **right
approach** is to rewrite it as a **thin translation layer**:

```rust
// Pseudocode for the new SqliteWalletPersister::flush_one:
fn flush_one(&self, wallet_id: WalletId, cs: PlatformWalletChangeSet) -> Result<()> {
    let seed_hash = wallet_id; // evo-tool uses WalletId == seed_hash

    // 1. Core wallet deltas from cs.core
    if let Some(core) = &cs.core {
        // 1a. Chain state → UPDATE wallet.last_terminal_block
        if let Some(chain) = &core.chain {
            if let Some(h) = chain.synced_height {
                self.db.set_wallet_terminal_block(seed_hash, h, ...)?;
            }
        }
        // 1b. Balance delta → recompute absolute + UPDATE wallet.(confirmed|unconfirmed|total)_balance
        //    (requires reading current balance first — not pure delta)
        // 1c. Per-account buckets → iterate and delegate
        for (account_type, bucket) in &core.per_account {
            // UTXOs added — delegate to Database::insert_utxo per entry
            for (outpoint, utxo) in &bucket.utxos_added {
                self.db.insert_utxo(
                    outpoint.txid.as_byte_array(),
                    outpoint.vout,
                    &utxo.address,     // real Address, not placeholder!
                    utxo.txout.value,
                    utxo.txout.script_pubkey.as_bytes(),
                    network,
                )?;
            }
            // UTXOs spent — delegate to Database::drop_utxo
            for outpoint in &bucket.utxos_spent {
                self.db.drop_utxo(outpoint, network)?;
            }
            // Transactions → Database::replace_wallet_transactions (or insert path)
            //    — the native TransactionRecord has everything the
            //      existing wallet_transactions row needs
            // highest_used → wallet_account_state (if we keep this table)
            //   OR update wallet_addresses based on the bucket's
            //   addresses_used + pool-type discriminator
        }
    }

    // 2. Platform-specific sub-changesets — existing delegation points
    if let Some(asset_locks) = &cs.asset_locks {
        for entry in asset_locks.asset_locks.values() {
            self.db.store_asset_lock_transaction(seed_hash, entry, ...)?;
        }
    }
    if let Some(identities) = &cs.identities {
        self.db.store_identities(seed_hash, identities.identities.values(), ...)?;
    }
    // ... etc

    Ok(())
}
```

**Key property:** every SQL statement lives in `database/*.rs`; the
persister is just translation + routing. Much smaller (~200 LOC vs
~1000), no raw SQL duplication, and writes go through the same
paths as the "direct writes" that bypass the persister — which
means the two paths can't diverge.

### Gaps that need resolution before landing

1. **Balance delta tracking.** The native `BalanceChangeSet` is a
   *delta* (signed). The existing `wallet` table stores *absolute*
   values. The adapter must either:
   - Read the current balance, apply the delta, write the absolute.
     (Racey if another writer races — but we hold the WalletManager
     write lock at this point, so OK.)
   - Store deltas-over-time, compute absolute on load. (Bigger
     change, not worth it now.)
   - **Or punt**: balance recompute happens on every
     `update_balance()` call which re-derives from UTXOs. The
     persister doesn't need to write balance at all — the next load
     will recompute from the persisted UTXO set.
2. **`wallet_account_state` — keep or fold in?** The new per-account
   shape is keyed by `AccountType` (enum) + `AddressPoolType` per
   pool. The existing `wallet_account_state` uses
   `(account_index, path_reference)`. Decision: **keep the table,
   migrate the schema** to `(account_type_discriminant,
   account_index, pool_type, last_revealed)`. One commit, additive.
3. **Identity timestamps** (`last_updated_balance_block_time`,
   `last_synced_keys_block_time`). Not persisted today; they're
   ephemeral cache on `ManagedIdentity`. Either add columns to
   `identity` or accept them as runtime-only. **Decision: runtime
   only.** They're just "don't hammer the network" timestamps, not
   authoritative state.
4. **DPNS names as `Vec<String>` vs `Vec<DpnsNameInfo>`.** Pre-existing
   data-shape mismatch. The current persister stores only strings,
   dropping `DpnsNameInfo` metadata (expires_at, document_id).
   **Decision: fix during this pass** — update `IdentityEntry.dpns_names`
   to `Vec<DpnsNameInfo>` and the persister to store the metadata.
5. **Contacts stored per-identity at runtime, at wallet level in
   changeset.** Pre-existing data-shape mismatch. `ContactChangeSet`
   uses `(from_identity, to_identity)` tuples keyed at the wallet
   level, but `ManagedIdentity` holds `established_contacts`,
   `sent_contact_requests`, `incoming_contact_requests` per-identity.
   **Decision: fix during this pass** — route contact entries into
   the owning `ManagedIdentity` on apply; change `ContactChangeSet`
   to key by `(owner_identity_id, contact_identity_id)` matching
   the DB tables.

## What's next — Phase 9 breakdown

Ordering matters here because platform-wallet and evo-tool share types
via path-dep. Breaking evo-tool's build is OK between sibling commits
on the same branch, but we want to land fixes paired so CI can go
green within a few commits of each other.

### 9a-1 ✅ platform-wallet restructure (committed as `44d15fac`)

Unified `PlatformWalletChangeSet`:
- Embedded `core: Option<key_wallet::changeset::WalletChangeSet>`.
- Deleted duplicate `ChainChangeSet`, `TransactionChangeSet`,
  `TransactionEntry`, `UtxoChangeSet`, `AccountChangeSet`, and
  `PlatformAddressEntry` (latter folded into
  `PlatformAddressChangeSet.addresses: BTreeMap<PlatformAddress, Credits>`
  matching runtime shape).
- Added `TokenBalanceChangeSet`.
- Bridged platform-wallet's richer `Merge` trait to key-wallet's
  via a one-off impl on `WalletChangeSet`.
- platform-wallet builds and its 72 unit tests pass.

**Status:** landed. Evo-tool's current `feat/platform-wallet2` does
NOT build until 9a-2 lands.

### 9a-2 rewrite evo-tool SqliteWalletPersister as a thin adapter ⬅ NEXT

**Where:** `dash-evo-tool` on `feat/platform-wallet2`,
`src/changeset/sqlite.rs`.

**Goal:** delete ~1000 LOC of raw SQL in the persister and replace
with a thin translation layer (~200 LOC) that calls existing
`database::*` writer methods.

**Scope:**
- Update imports to the new `PlatformWalletChangeSet` shape.
- Rewrite `flush_one`:
  - `cs.core.chain.synced_height` → `Database::set_wallet_terminal_block`
    (or equivalent — verify the exact method name during implementation)
  - `cs.core.per_account[*].utxos_added` → iterate, call
    `Database::insert_utxo(txid, vout, &utxo.address, utxo.txout.value,
    utxo.txout.script_pubkey.as_bytes(), network)` — **this is the big
    win, real address + script get persisted instead of placeholders**
  - `cs.core.per_account[*].utxos_spent` → `Database::drop_utxo`
  - `cs.core.per_account[*].transactions` → collect into
    `Vec<WalletTransaction>` (evo-tool's shape) and call
    `Database::replace_wallet_transactions` once per wallet. Map
    `TransactionRecord` fields 1:1.
  - `cs.core.per_account[*].highest_used` → write to
    `wallet_account_state` (migrate schema to
    `(account_type_discriminant, account_index, pool_type, last_revealed)`
    as part of this commit).
  - `cs.identities` → `Database::store_identities` +
    `Database::store_top_ups`.
  - `cs.contacts` → `Database::store_contact_request` / `store_contact`.
  - `cs.platform_addresses` → `Database::update_wallet_balances` or
    dedicated platform-address writer.
  - `cs.asset_locks` → `Database::store_asset_lock_transaction`.
  - `cs.token_balances` → (no existing writer; decide: add one or
    punt).
- Rewrite `load`:
  - Read from the same tables via existing `Database::*` readers
    where they exist, raw SQL where they don't.
  - Return `PlatformWalletChangeSet` with `core` populated. The
    loaded changeset is immediately handed to
    `WalletManager::apply_changeset`, so the adapter returns
    native types directly.
- Fix the fix-up call sites in
  `src/backend_task/identity/register_identity.rs`,
  `top_up_identity.rs`, `dashpay/contact_requests.rs` — these
  currently construct the deleted changeset types by hand.
- Delete the `wallet_identity_dpns_names` table (or migrate it to
  richer `DpnsNameInfo` storage — decision needed).

**Commit strategy:** ONE commit on evo-tool feat/platform-wallet2
that makes the build green against the new platform-wallet shape.
Probably ~30-min review pass with rust-quality + simplicity
reviewers since it's a large rewrite of persistence-critical code.

### 9a-3 platform-wallet `PlatformWalletInfo::apply_changeset`

**Where:** `platform-wallet`, probably new
`src/wallet/apply.rs` module mirroring key-wallet's structure.

**Goal:** `fn apply_changeset(&mut self, wallet: &mut Wallet, cs: &PlatformWalletChangeSet) -> Result<(), ApplyError>`
that restores in-memory state from a persisted changeset.

- Delegate `cs.core` → `self.core_wallet.apply_changeset(wallet, core)`.
- Apply `cs.identities` into `self.identity_manager` — needs a new
  `IdentityManager::apply_identity_entry` method that sets all the
  `ManagedIdentity` fields (label, block_time timestamps if we
  decided to keep them, dpns_names as `Vec<DpnsNameInfo>`).
- Apply `cs.contacts` by routing entries per `owner_identity_id` to
  each `ManagedIdentity`'s contact maps.
- Apply `cs.platform_addresses` into `self.platform_address_balances`.
- Apply `cs.asset_locks` into `self.tracked_asset_locks` — inline
  translation or a new `AssetLockManager::apply_changeset_entries`
  method.
- Apply `cs.token_balances` into `self.token_balances` / `token_watched`.
- Recompute cached balance via `self.update_balance()`.
- Errors only on cascade failures; silent skip with
  `tracing::warn!` on unroutable entries.

Idempotent by construction — all writes are map inserts / overwrites.

### 9a-4 round-trip tests

Build a `PlatformWalletInfo`, mutate each platform-specific field
by hand (emission code doesn't exist yet — that's 9b). Apply
changeset to a sibling. Assert state converges. Idempotent
double-apply. Cover:
- `core` path (already covered by key-wallet's apply tests; smoke
  test here to verify delegation works)
- `identities` — add/update
- `contacts` — sent, incoming, established
- `asset_locks` — create, upgrade to IS-locked
- `platform_addresses` — balance updates
- `token_balances` — new and updated

### 9a-5 verify + commit

cargo check + test across key-wallet, key-wallet-manager,
key-wallet-ffi, platform-wallet, dash-evo-tool. Everything green.
Review pass on evo-tool's persister rewrite.

**9a scope does NOT include:** wiring mutation methods to emit
changesets, or wiring the persister's queue/flush lifecycle. Those
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
- [`dash-evo-tool/CLAUDE.md`](../../../dash-evo-tool/CLAUDE.md)
  — evo-tool architecture. Key-relevant section: "Database" (single
  `Mutex<Connection>`, domain-specific writer methods in
  `src/database/*.rs`). The `v1.0-dev` branch is the canonical
  stable base; `feat/platform-wallet2` has the persister work
  being rewritten here.
- [`dash-evo-tool/src/database/initialization.rs`](../../../dash-evo-tool/src/database/initialization.rs)
  — authoritative `CREATE TABLE` statements for wallet state.
- [`dash-evo-tool/src/changeset/sqlite.rs`](../../../dash-evo-tool/src/changeset/sqlite.rs)
  — current `SqliteWalletPersister`. Being rewritten in Phase 9a-2.
