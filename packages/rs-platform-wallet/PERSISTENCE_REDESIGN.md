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

## Write-path architecture — one write, one path

The goal is **one write path per wallet state type**:

```
mutation API on PlatformWalletInfo  (or core_wallet)
    │  returns PlatformWalletChangeSet
    ▼
orchestrator merges changesets into a buffer
    │
    ▼
SqliteWalletPersister flushes buffer → Database::* writer methods → SQLite
```

No wallet-state writes bypass the persister. Direct `Database::*`
writes are allowed **only** for tables that have no corresponding
in-memory field on `PlatformWalletInfo` — lifecycle, settings,
caches, and UI metadata.

### Catalogue of wallet-state tables and their PlatformWalletInfo mapping

**Covered today (Category A):**

| Table | PlatformWalletInfo field | Current write status |
|-------|--------------------------|----------------------|
| `utxos` | `core_wallet.per_account[..].{utxos_added,utxos_spent,utxos_instant_locked}` | **Persister-only**: `insert_utxo` has 0 non-test callers; `drop_utxo` has 1 non-persister caller in `model/wallet/utxos.rs::remove_selected_utxos` (tx building) |
| `wallet_transactions` | `core_wallet.per_account[..].transactions` | **Persister-only**: `replace_wallet_transactions` has 0 callers anywhere — dead code |
| `wallet_addresses` | `core_wallet.per_account[..].addresses_used` + `highest_used` | Persister writes subset; some direct writes via address balance updates |
| `wallet.(balance, last_terminal_block)` | `core_wallet.balance`, `core_wallet.metadata.synced_height` | `update_wallet_balances` called directly from backend tasks — duplicate |
| `identity` | `identity_manager.identities[id].identity` | **Duplicate**: `insert_local_qualified_identity` / `update_local_qualified_identity` have ~25 direct call sites across `backend_task/identity/*` AND `sync_identity_to_platform_wallet` mirrors to `IdentityManager` afterwards. Wrong direction — should be mutation-first, persister-second. |
| `top_up` | `identity_manager.identities[id].top_ups` | Same as identity — direct write, then sync |
| `wallet_identity_dpns_names` | `identity_manager.identities[id].dpns_names` (type mismatch: `Vec<String>` vs runtime `Vec<DpnsNameInfo>`) | Persister-only; table created inline by persister (not in v1.0-dev) |
| `asset_lock_transaction` | `tracked_asset_locks` | Direct writes from `backend_task/identity/register_identity.rs`, `top_up_identity.rs` + persister path — **duplicate** |
| `platform_address_balances` | `platform_address_balances` | `set_platform_address_info` / `delete_platform_address_info` called directly — need to go through changeset |
| `identity_token_balances` | `token_balances` | `insert_token_identity_balance` called directly + persister path — **duplicate** |
| `token` | `token_watched` (partial — metadata NOT in wallet) | Direct writes for token metadata (ticker, name, decimals). Only `token_watched: BTreeSet<Identifier>` is in the wallet; full metadata stays direct. |
| `dashpay_contacts` | `identity_manager.identities[id].established_contacts` | Direct writes from `backend_task/dashpay/*` — should emit contact changeset |
| `dashpay_contact_requests` | `identity_manager.identities[id].{sent,incoming}_contact_requests` | Direct writes from `backend_task/dashpay/*` — should emit contact changeset |

**Out of scope (Category C — legitimately direct):**

No in-memory field exists, no reason to add one. These stay as
direct writes:

- `settings` — theme, password, network, UI state (not wallet state)
- `wallet` lifecycle columns — encrypted seed, salt, nonce, alias, is_main, password_hint, uses_password, core_wallet_name (create/rename/delete wallet)
- `dashpay_profiles` — avatar bytes, display name, public message (**gap candidate**: could add `profile: Option<DashPayProfile>` to `ManagedIdentity` — defer, documented below)
- `dashpay_payments` — payment history (**gap candidate**: per-contact payment log — defer)
- `dashpay_contact_address_indices` — derivation indices for DashPay contact payments
- `dashpay_address_mappings` — address → contact runtime cache
- `contact_private_info` — encrypted local contact metadata (UI layer)
- `contested_name`, `contestant` — DPNS voting state
- `contract` — data contract metadata cache
- `proof_log` — audit trail
- `scheduled_votes` — voting scheduler
- `shielded_notes` — Zcash shielded integration (separate subsystem)
- `single_key_wallet` — legacy migration path
- `identity_order`, `token_order` — UI sort order

**Gap candidates (Category D — should be in PlatformWalletInfo but aren't):**

These have existing DB tables but nothing in `PlatformWalletInfo`
tracks them. They'd be legitimate category A once added. Deferred
out of scope for Phase 9a:

1. **DashPay profiles** → add `profile: Option<DashPayProfile>` on
   `ManagedIdentity`. Fields: display_name, public_message,
   avatar_hash, avatar_bytes (optional), created_at.
2. **DashPay payments** → add `payments: BTreeMap<(Identifier, Txid), PaymentEntry>`
   on `ManagedIdentity` or top-level. Tracks per-contact payment
   history with amounts, direction, timestamps.
3. **DashPay contact address derivation indices** → track per-contact
   receive index for BIP44 contact payment paths. Currently in
   `dashpay_contact_address_indices`.

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
NOT build until 9a-4 lands.

### 9a-2 platform-wallet mutation methods return changesets

**Where:** platform-wallet, across:
- `src/wallet/identity/manager.rs` — `IdentityManager::add_identity`,
  `remove_identity`, `set_label`, `set_last_scanned_index`,
  `set_primary_identity`. And the per-identity mutations on
  `ManagedIdentity`: `add_dpns_name`, `add_top_up`,
  `update_balance_block_time`, `update_keys_sync_block_time`.
- `src/wallet/identity/managed_identity/contact_requests.rs` —
  `add_sent_contact_request`, `add_incoming_contact_request`,
  `remove_sent_contact_request`, `remove_incoming_contact_request`.
- `src/wallet/asset_lock/manager.rs` — `record_asset_lock`,
  `update_status` (created → IS-locked → chain-locked).
- `src/wallet/platform_addresses/wallet.rs` — direct mutations of
  `platform_address_balances` map.
- `src/wallet/tokens/*.rs` — `add_watched_token`,
  `update_token_balance`.

**Goal:** each mutation method returns `PlatformWalletChangeSet`
(or a narrower sub-changeset) instead of `()`. Callers
`result.changeset.merge(cs)` into an accumulator, same pattern as
key-wallet's `ManagedCoreAccount::mark_address_used` etc.

**Not in scope:** wiring the accumulated changesets to the
persister. That's 9a-5.

Commit on platform-wallet `feat/platform-wallet2`. Platform-wallet
builds and its unit tests pass. Evo-tool still broken until 9a-4.

### 9a-3 `PlatformWalletInfo::apply_changeset` (restore path) ✅

Landed in two commits:
- `a48aeb3064` — prereq: carry full `EstablishedContact` in
  `ContactChangeSet.established` (latent 9a-2 schema bug — auto-establish
  paths emitted only the `(owner, contact)` pair, losing the underlying
  `ContactRequest`s and making faithful replay impossible).
- (this commit) — `src/wallet/apply.rs` with the canonical
  `PlatformWalletInfo::apply_changeset` plus the `apply_identity_entry`
  helper on `IdentityManager` and the `apply_*_contact_request` /
  `apply_established_contact` helpers on `ManagedIdentity`.

Final shape:

```rust
pub fn apply_changeset(
    &mut self,
    wallet: &mut Wallet,
    cs: &PlatformWalletChangeSet,
) -> Result<(), ApplyError>
```

Sequencing: `cs.core` → identities (insert + remove + primary fixup +
scan watermark) → contacts (sent / incoming inserts → tombstone removes
→ established promotions, each routed to the owning `ManagedIdentity` by
`(owner, contact)` key) → platform addresses (insert + tombstone) →
asset locks (insert with `amount_duffs` → `amount` rename + tombstone) →
token balances (balance updates + watched/unwatched/removed_balances) →
`update_balance()` recompute and mirror into the lock-free `Arc<WalletBalance>`.

Invariants:
- Idempotent (9 unit tests cover insert/remove/double-apply for every
  sub-changeset).
- No re-emission — apply returns `Result<(), ApplyError>`.
- Best-effort routing for contacts: orphan owners are
  `tracing::warn!`-ed and skipped.
- Loud on core failures: `cs.core` failures (HD account derivation
  cascade) propagate as `ApplyError::CoreApply(String)`.

Apply-side helpers added:
- `IdentityManager::apply_identity_entry(&IdentityEntry)` — in-place
  update if the identity already exists, or fresh insert. Mirrors the
  merge policy on `IdentityChangeSet` (revision-gated `identity` blob,
  union for dpns/top_ups/key_storage). First-inserted identity becomes
  primary if no primary is set.
- `IdentityManager::apply_remove(&Identifier) -> bool`.
- `ManagedIdentity::apply_sent_contact_request`,
  `apply_incoming_contact_request`, `apply_removed_sent`,
  `apply_removed_incoming`, `apply_established_contact` — raw
  inserts/removes that skip the auto-establish fast path (it was
  already captured at mutation time).

Deletions:
- The stale `PlatformWallet::apply` (only handled asset locks, had TODOs
  for everything else) is now a thin async wrapper that takes the
  WalletManager write lock via `get_wallet_mut_and_info_mut` and
  delegates to `PlatformWalletInfo::apply_changeset`.
- `AssetLockManager::restore_from_changeset_blocking` — its only caller
  was the old `PlatformWallet::apply`, gone.

**Open follow-up (cross-cutting, both repos) — `apply_changeset` should
consume the changeset by value.** Today both
`PlatformWalletInfo::apply_changeset` (platform-wallet) and
`ManagedWalletInfo::apply_changeset` (rust-dashcore key-wallet) take
`cs: &…ChangeSet`. Every `insert` then has to clone owned data out of
borrowed data:

- platform-wallet side: `Identity` blobs, `KeyStorage`, `dpns_names`,
  `ContactRequest`s, `EstablishedContact`s, `Transaction`s inside
  `AssetLockEntry`.
- key-wallet side (bigger blast radius): every `Utxo` and every
  `TransactionRecord` in
  `ManagedCoreAccount::apply_changeset` is `.clone()`-ed before
  insert (`managed_account/mod.rs:745, 768`). The
  `Transaction` clone inside each `TransactionRecord` is the heaviest
  item in any sync replay.

For the typical persister-load case (deserialize → apply once → drop)
all of those clones are pure waste — the persister already produced
owned data, we could move it directly into the wallet maps.

Fix in both crates:

1. `key_wallet`: switch
   `ManagedWalletInfo::apply_changeset(&mut self, wallet, cs:
   WalletChangeSet)`. Inside
   `ManagedCoreAccount::apply_changeset(account_type, bucket:
   AccountChangeSet)` use `into_iter()` / `drain` on each map. The
   monotonic-flag merge on UTXOs (`is_confirmed |= existing`) becomes
   a `match` on `entry(*outpoint)`: if occupied, take the existing
   flags then `into_mut()` or `insert(merged)`; otherwise insert by
   value with no clone.
2. `platform-wallet`: switch
   `PlatformWalletInfo::apply_changeset(&mut self, wallet, cs:
   PlatformWalletChangeSet)`. Same `into_iter()` / `drain` pattern
   for every sub-changeset. `apply_identity_entry` and the
   `apply_*_contact_request` helpers grow `_owned` variants that
   take the entry / request by value.
3. Single-variant API. No `apply_changeset` + `apply_changeset_owned`
   overload — the borrow form should be deleted, not co-existed.
   Hidden clones are not OK.

Order: key-wallet first (it's the heavier path and platform-wallet
calls into it via `cs.core`), then platform-wallet. Both land before
9a-5 because the persister adapter is the one caller that benefits
most from the move.

### 9a-4 round-trip tests on platform-wallet ✅

Landed in `apply.rs` as 10 new tests on top of the 9 synthesized-data
tests from 9a-3. Each round-trip test mutates `info_a` via the new
mutation API (which now returns sub-changesets), wraps the captured
sub-changeset into a `PlatformWalletChangeSet`, applies it to a
sibling `info_b`, and asserts convergence. This verifies the round-
trip contract: emitted changesets are faithful enough to rebuild
state via apply.

Coverage (sync mutation surface):
- `round_trip_add_identity` — IdentityManager::add_identity
- `round_trip_remove_identity_reselects_primary` —
  IdentityManager::remove_identity (verifies the primary re-selection
  fixup in apply matches the mutation-side selection)
- `round_trip_set_label` — IdentityManager::set_label
- `round_trip_last_scanned_index_watermark` —
  IdentityManager::set_last_scanned_index
- `round_trip_dpns_name_and_top_up` — ManagedIdentity::add_dpns_name
  + record_top_up via snapshot_changeset
- `round_trip_block_time_updates` — ManagedIdentity timestamp
  updates
- `round_trip_sent_contact_request_no_auto_establish` — plain
  insert path on add_sent_contact_request
- `round_trip_auto_establish_contact` — incoming + sent →
  auto-establish, verifies both pending sets drained on B and the
  established contact rebuilt from the carried `EstablishedContact`
- `round_trip_remove_contact_request` — tombstone replay
- `round_trip_double_apply_is_idempotent` — multi-changeset replay
  applied twice on B with no divergence

`core` path: covered by key-wallet's own apply tests; the platform-
wallet integration with `cs.core` is delegated and doesn't need
duplicate coverage here.

**Deferred — async / SDK-dependent mutation paths:**
The following mutation methods require an `Sdk`, broadcaster, and
`Notify` to construct the manager, so they can't run as plain unit
tests. Their round-trip coverage will land as integration tests in a
9a-4 follow-up:
- `AssetLockManager::{track_asset_lock, advance_asset_lock_status,
  remove_asset_lock}` — apply side already covered by the
  synthesized-data tests in `apply.rs`
- `TokenWallet::{watch, unwatch, unwatch_identity, sync}`
- `PlatformAddressWallet::{sync_balances, transfer, withdraw,
  fund_from_asset_lock}`

The synthesized-data tests in `apply.rs` already cover the apply
side for all of these — the gap is verifying the *mutation side*
emits a faithful changeset, which only matters once the persister
adapter is wired in 9a-5 / 9a-6 and the data path is exercised
end-to-end.

Test count: 19 in apply.rs (9 synthesized + 10 round-trip) + 9
contact workflow integration tests + 81 other lib tests, all green.

Commit on platform-wallet `feat/platform-wallet2`.

### 9a-5 rewrite evo-tool `SqliteWalletPersister` as thin adapter

**Where:** dash-evo-tool `feat/platform-wallet2`,
`src/changeset/sqlite.rs`.

**Goal:** replace ~1000 LOC of raw SQL with a ~200 LOC translation
layer that delegates to existing `Database::*` writer methods.

**Scope:**
- Update imports to the new `PlatformWalletChangeSet` shape.
- Rewrite `flush_one` (thin adapter style):
  - `cs.core.chain.synced_height` → update `wallet.last_terminal_block`
    via existing or new `Database::set_wallet_terminal_block`.
  - `cs.core.balance` — **punt.** Balance is recomputed from UTXOs on
    load via `update_balance()`. Persister doesn't need to touch
    balance columns.
  - `cs.core.per_account[..].utxos_added` → `Database::insert_utxo`
    with the native `Utxo` fields (real address + script, not
    placeholders).
  - `cs.core.per_account[..].utxos_spent` → `Database::drop_utxo`.
  - `cs.core.per_account[..].transactions` → collect and call
    `Database::replace_wallet_transactions` once per wallet. Map
    the native `TransactionRecord` to evo-tool's `WalletTransaction`
    shape (timestamp, height, block_hash, net_amount, fee, label,
    is_ours, raw_transaction, status).
  - `cs.core.per_account[..].highest_used` → write to
    `wallet_account_state` table. **Migrate schema** to key on
    `(seed_hash, account_type_discriminant, account_index, pool_type)`
    → `last_revealed`.
  - `cs.core.per_account[..].addresses_used` → update
    `wallet_addresses` (mark addresses used, extend derivation
    state).
  - `cs.identities` → `Database::insert_local_qualified_identity` /
    `update_local_qualified_identity`. This works because evo-tool's
    `QualifiedIdentity` type wraps the dpp `Identity` that
    `IdentityEntry` carries.
  - `cs.contacts` → `Database::save_contact_request` /
    `save_dashpay_contact` per-entry.
  - `cs.platform_addresses` → `Database::set_platform_address_info`
    per-address.
  - `cs.asset_locks` → `Database::store_asset_lock_transaction` per-entry.
  - `cs.token_balances` → `Database::insert_token_identity_balance`
    per-entry.
- Rewrite `load`:
  - Read from the same tables. Return `PlatformWalletChangeSet`
    with `core` populated using native key-wallet types.
- Delete the persister-created `wallet_identity_dpns_names` table and
  its writer helpers. DPNS names now live inside the persisted
  identity blob via `IdentityEntry.dpns_names: Vec<DpnsNameInfo>`.
- Migrate `wallet_account_state` schema as above (add columns or
  drop-and-recreate via evo-tool's migration system).

**Commit on evo-tool `feat/platform-wallet2`.**

### 9a-6 rip out duplicate direct writes from evo-tool

**Where:** dash-evo-tool, across `src/backend_task/`, `src/context/`,
`src/model/wallet/`.

**Goal:** every write path that currently goes direct to
`Database::*` for a Category A table (from the catalogue above)
instead mutates `PlatformWalletInfo` and relies on the persister
to catch the emitted changeset.

**Specific call sites to rewire:**

1. **Identity write-through** (~25 call sites):
   - `context/identity_db.rs::insert_local_qualified_identity` —
     flip the order: mutate `platform_wallet.identity_manager.add_identity`
     first (which emits a changeset), then feed the changeset to
     the persister. The direct `self.db.insert_local_qualified_identity`
     call goes away.
   - Same for `update_local_qualified_identity`.
   - Delete `sync_identity_to_platform_wallet` — the mutation IS
     the sync.
   - All ~25 backend-task call sites now call the mutation API
     (which already returns a changeset) and queue it.

2. **Asset lock writes** (`backend_task/identity/register_identity.rs`,
   `top_up_identity.rs`):
   - Currently call `store_asset_lock_transaction` directly.
   - Flip to `platform_wallet.asset_locks.record_asset_lock` which
     returns an `AssetLockChangeSet`. Queue for the persister.

3. **Platform address balance updates**
   (`backend_task/wallet/fund_platform_address_from_wallet_utxos.rs` etc.):
   - Currently call `set_platform_address_info` directly.
   - Flip to the platform wallet's platform-address mutation API.

4. **Token balance updates** (`backend_task/tokens/mint_tokens.rs`,
   `burn_tokens.rs`, etc.):
   - Currently call `insert_token_identity_balance` directly.
   - Flip to `platform_wallet.tokens.update_balance` which emits a
     `TokenBalanceChangeSet`.

5. **Transaction building UTXO reservation**
   (`model/wallet/utxos.rs::remove_selected_utxos`):
   - Currently drops the UTXO from the DB directly when a tx is
     being built, to prevent double-spend.
   - Flip to a platform-wallet mutation that marks the UTXO as
     "reserved" or drops it from the in-memory set, emitting a
     changeset.

6. **SPV integration** — verify: does the SPV event handler
   currently feed `TransactionCheckResult.changeset` to the
   persister? If not, wire it. If it does, also stop any parallel
   direct writes it might be doing.

**Lifecycle writes stay direct** (Category C):
- `Database::store_wallet` / `remove_wallet` / `set_wallet_alias` /
  `set_wallet_core_wallet_name` — wallet existence and metadata,
  not state.
- All settings writes.
- All DashPay profile / payment / address mapping writes
  (category D, deferred).
- All contract / contested_name / proof_log / scheduled_votes /
  shielded_notes / single_key_wallet writes.

**Commit on evo-tool `feat/platform-wallet2`.**

### 9a-7 wire persister queue/flush lifecycle

**Where:** evo-tool, in the platform wallet load/unload path and
SPV event loop.

**Goal:** every emitted `PlatformWalletChangeSet` from a mutation
call site ends up in `SqliteWalletPersister` via `store(wallet_id, cs)`.
Flush strategy:

- Immediate flush on `SyncComplete` SPV events.
- Debounced flush 30s after the last `store()`.
- Explicit flush on wallet unload / app shutdown.
- **Never** flush during cold SPV sync — the batching keeps the
  buffer merged and writes happen at checkpoints.

Replace the current `FlushStrategy::Immediate` default with a
scheduled flush driven from SPV events.

**Commit on evo-tool `feat/platform-wallet2`.**

### 9a-8 verify + review pass + commit

cargo check + test across key-wallet, key-wallet-manager,
key-wallet-ffi, platform-wallet, dash-evo-tool. Everything green.
Review pass on the persister rewrite and the write-path rewiring:
rust-quality, simplicity, pattern reviewers in parallel.

**9a scope does NOT include:** the Category D gaps (DashPay
profiles, payments, contact address indices). Those stay as
direct writes for now and get picked up in a follow-up phase
when the platform wallet grows the corresponding in-memory fields.

## Phase 9b — Close the Category D gaps (future)

After 9a lands, a handful of DashPay/wallet-adjacent tables still
have legitimate direct writes because `PlatformWalletInfo` has no
in-memory representation for them. Phase 9b grows the platform
wallet to cover these tables, then migrates their writes through
the changeset flow exactly like Phase 9a does for the Category A
tables.

Each sub-phase follows the same 5-step pattern:

1. Add the in-memory field to `PlatformWalletInfo` or `ManagedIdentity`.
2. Add a sub-changeset to `PlatformWalletChangeSet`.
3. Add mutation methods that return a changeset.
4. Extend `PlatformWalletInfo::apply_changeset` to handle the new sub.
5. Extend evo-tool's `SqliteWalletPersister` adapter to translate
   the new sub-changeset to the existing DB tables.
6. Flip evo-tool's direct writes to route through the mutation API.

### 9b-1 — DashPay profiles

**Table:** `dashpay_profiles` (avatar_bytes, display_name,
public_message, created_at)

**In-memory:** add `profile: Option<DashPayProfile>` on
`ManagedIdentity`.

**Writers to flip:**
- `Database::save_dashpay_profile` (dashpay.rs:223)
- `Database::save_dashpay_profile_avatar_bytes` (dashpay.rs:262)

### 9b-2 — DashPay payment history

**Tables:** `dashpay_payments` (owner_identity_id, contact_identity_id,
txid, amount)

**In-memory:** add `payments: BTreeMap<(Identifier, Txid), PaymentEntry>`
on `ManagedIdentity`, where `PaymentEntry` carries amount, direction,
timestamp, status.

**Writers to flip:**
- `Database::save_payment` (dashpay.rs:521)
- `Database::update_payment_status` (dashpay.rs:552)

### 9b-3 — DashPay contact address derivation indices

**Tables:** `dashpay_contact_address_indices`
(owner_identity_id, contact_identity_id, highest_receive_index,
bloom_registered_count)

**In-memory:** add per-contact derivation state to
`ManagedIdentity.established_contacts`. Matches the key-wallet
`highest_used` pattern but scoped to BIP44 DashPay contact payment
paths.

**Writers to flip:**
- `Database::update_highest_receive_index` (dashpay.rs:741)
- `Database::update_bloom_registered_count` (dashpay.rs:769)

### 9b-4 — DashPay address mapping cache (maybe)

**Table:** `dashpay_address_mappings` (address, contact_identity_id,
network, path_type)

**Decision needed:** is this a pure runtime cache (ephemeral, never
persisted → out of scope entirely) or a persistence concern? If
the former, drop the table and recompute at load. If the latter,
route via a new sub-changeset.

**Writers to flip (if persisted):**
- `Database::save_dashpay_address_mapping` (dashpay.rs:827)
- `Database::delete_dashpay_address_mappings_for_contact` (dashpay.rs:919)

### 9b scope and ordering

9b-1 through 9b-4 are **independent** and can land in any order
after 9a is complete. Each sub-phase is a self-contained commit
chain (add field → changeset → mutation → apply → evo-tool
persister adapter → direct-write rip-out).

Estimated size: each sub-phase is smaller than a Phase 9a step
because the plumbing (changeset module, persister, evo-tool
integration points) already exists. The work is almost entirely
mechanical once the pattern is established by 9a.

## Phase 10+ — Open questions, not planned

- **Token metadata cache** (`token` table: ticker, name, decimals,
  contract ID). Currently direct-write Category C. Could be
  promoted to a changeset-driven field if token metadata becomes
  wallet-scoped rather than global.
- **Data contract cache** (`contract` table). Similar question —
  probably stays global/app-level, not wallet-scoped.
- **Wallet lifecycle via changesets?** Creating and deleting
  wallets (the `wallet` table encrypted seed / alias columns) is
  currently direct. Arguable whether that's worth changing —
  wallet existence isn't really state, it's identity. Probably
  leave alone.

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
