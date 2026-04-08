---
title: "feat: Platform Wallet — Complete Implementation & Evo Tool Integration"
type: feat
status: active
date: 2026-03-13
updated: 2026-04-08
---

# feat: Platform Wallet — Complete Implementation & Evo Tool Integration

## Current Status (2026-04-08)

### What's done

**All core implementation is complete.** 22 PRs merged, covering the full platform wallet library and evo-tool integration:
- PRs 1–19 ✅: Full library + evo-tool migration (sub-wallets, signing, asset locks, DashPay, tokens, identity, SPV lifecycle)
- PR-22 ✅: ChangeSet-based persistence

**Recent locking refactoring (committed, all tests passing):**
- Collapsed 7+ independent `Arc<RwLock<...>>` into a single `Arc<RwLock<PlatformWalletInfo>>`
- Sub-wallets (CoreWallet, IdentityWallet, etc.) now hold `Arc<RwLock<PlatformWalletInfo>>` instead of separate locks
- State getters moved from CoreWallet to PlatformWallet (e.g., `wallet.state().balance()` instead of `wallet.core().state().balance()`)
- CoreWallet cleanup: removed broadcaster field, removed dead methods
- Evo-tool fully migrated to new single-lock API

**Test results:**
- 76 platform-wallet lib tests: **PASS**
- 347 evo-tool lib tests: **PASS**
- Backend E2E tests (testnet): **PASS** (cleanup_only in ~7s, 2026-04-08)

### Next steps (immediate)

**PR-30: Switch to dashcore WalletManager** — see detailed spec below.

### Remaining PRs (future)

| PR | Description | Status |
|----|-------------|--------|
| PR-20 | ~~Complete identity/asset lock lifecycle~~ Core API done (one-call methods + IS→CL fallback in IdentityWallet). Leftovers in PR-31. | Done |
| PR-21 | ~~Remove remaining duplication~~ TransactionBuilder already unified. Asset lock changeset restore leftover in PR-31. | Done |
| PR-23 | ~~Merge Wallet + ManagedWalletInfo in key-wallet~~ **Superseded** by PR-30 | Superseded |
| PR-24 | Comprehensive test suite + FFI update + final cleanup | Planned |
| PR-25 | Switch asset lock broadcast from DAPI to SPV | Planned |
| PR-26 | ~~Fix lock ordering deadlock~~ **RESOLVED** by single-lock refactoring | Done |
| PR-27 | ~~Merge SpvRuntime + SpvWalletAdapter~~ **Superseded** by PR-30 | Superseded |
| PR-28 | Full SPV replacement — migrate evo-tool SpvManager to PlatformWalletManager | Planned |
| PR-29 | Asset lock test coverage | Planned |
| PR-30 | Switch to dashcore WalletManager — delete SpvWalletAdapter, use BalanceUpdated events | **Next** |
| PR-31 | Leftovers from PR-20/21: evo-tool identity + asset lock cleanup | Planned |

---

## PR-30: Switch to dashcore WalletManager

### Goal

Replace platform-wallet's custom `SpvWalletAdapter` (~330 lines), `SpvSyncState` (~55 lines), and `PlatformWalletInfoWriteGuard` (Drop-based balance update) with dashcore's `WalletManager<T>`. This eliminates duplicated multi-wallet iteration logic, duplicated sync height tracking, and the Drop-based balance workaround.

### Why

`SpvWalletAdapter` reimplements exactly what `WalletManager` already does: iterate all wallets for each block/mempool transaction, call `check_core_transaction`, track synced heights, and (in WalletManager's case) emit `BalanceUpdated` events. `DashSpvClient` already accepts `Arc<RwLock<W: WalletInterface>>`, and `WalletManager<T>` implements `WalletInterface`. We can pass it directly.

### Architecture

```
PlatformWalletManager
  ├─ wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>
  │     └─ wallets: BTreeMap<WalletId, Arc<RwLock<PlatformWalletInfo>>>
  │
  ├─ spv_client: DashSpvClient<WalletManager<PlatformWalletInfo>, ..., SpvEventForwarder>
  │     └─ wallet: Arc<RwLock<WalletManager<...>>>  (same Arc as above)
  │     └─ handler: SpvEventForwarder  (on_wallet_event fires automatically)
  │
  ├─ wallets: BTreeMap<WalletId, PlatformWallet>  (handles for consumers)
  │     └─ each holds clone of Arc<RwLock<PlatformWalletInfo>> from wallet_manager
  │
  ├─ event_tx: broadcast::Sender<PlatformWalletEvent>
  └─ sdk: Arc<Sdk>
```

**Lock hierarchy during block processing:**
1. DashSpvClient acquires `Arc<RwLock<WalletManager>>` write lock
2. WalletManager iterates `wallets` map (`&mut self` access)
3. For each wallet: acquires `Arc<RwLock<PlatformWalletInfo>>` write lock
4. `check_core_transaction` runs → mutates state → persists changeset → releases per-wallet lock
5. WalletManager emits `BalanceUpdated` events via broadcast channel
6. DashSpvClient releases WalletManager write lock

Sub-wallets (CoreWallet, IdentityWallet) go directly to their `Arc<RwLock<PlatformWalletInfo>>` — skip the manager lock.

**Event flow:**
```
WalletManager.event_sender (broadcast) → spawn_broadcast_monitor task
  → SpvEventForwarder.on_wallet_event() → PlatformWalletEvent::Wallet(WalletEvent)
  → consumers (evo-tool balance updater, asset lock manager, etc.)
```

### dashcore changes (rust-dashcore repo)

**1. New `ManagedWalletState` struct** — bundles Wallet + ManagedWalletInfo + Persister.
`ManagedWalletInfo` stays unchanged (pure UTXO/balance/account state).

```rust
pub struct ManagedWalletState<P: WalletPersistence = NoPersistence> {
    pub wallet: Wallet,
    pub wallet_info: ManagedWalletInfo,
    pub persister: P,
}
impl<P: WalletPersistence> WalletInfoInterface for ManagedWalletState<P> {
    // All ~25 methods delegate to self.wallet_info
}
```

**2. `WalletPersistence` trait** — `store(changeset)`, `flush()`. `NoPersistence` for default/tests.

**3. `WalletInfoInterface` gains `wallet()` / `wallet_mut()`** — so WalletManager can access
the Wallet through T without knowing the concrete type.

**4. Remove `wallet: &mut Wallet` param from `check_core_transaction`** — T provides its
own wallet. Extract existing logic into `ManagedWalletInfo::check_core_transaction_with_wallet(&mut self, wallet: &Wallet, ...)` helper. `ManagedWalletState` impl calls helper with `&self.wallet` (disjoint field borrow, no borrow-checker issue). Persists changeset synchronously inside the method.

**5. WalletManager struct change** — single map with per-wallet locks:
```rust
pub struct WalletManager<T: WalletInfoInterface = ManagedWalletState> {
    wallets: BTreeMap<WalletId, Arc<RwLock<T>>>,  // was: two separate maps
    // synced_height, filter_committed_height, event_sender unchanged
}
```

**6. Update all WalletManager methods** — wallet creation inserts `Arc::new(RwLock::new(T::from_wallet(&wallet)))`. `check_transaction_in_all_wallets` acquires per-wallet write locks. `get_receive_address`/`get_change_address` extract xpub before mutable borrow. Accessors rewritten for single map.

### platform-wallet changes

**1. `PlatformWalletInfo` implements `WalletInfoInterface`** — delegates to `self.wallet_info`. Persister moves from `PlatformWallet` into `PlatformWalletInfo`. `check_core_transaction` calls `self.wallet_info.check_core_transaction_with_wallet(&self.wallet, ...)` and persists `PlatformWalletChangeSet` synchronously.

**2. Delete `SpvWalletAdapter`** (~330 lines) — replaced by WalletManager's WalletInterface impl.

**3. Delete `SpvSyncState`** (~55 lines) — WalletManager tracks heights internally.

**4. Delete `PlatformWalletInfoWriteGuard`** (~25 lines) — balance atomics updated via `BalanceUpdated` events through `SpvEventForwarder.on_wallet_event()`.

**5. Update `SpvRuntime`** — `DashSpvClient<WalletManager<PlatformWalletInfo>, ...>`.

**6. Restructure `PlatformWalletManager`** — holds `wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>`, `wallets: BTreeMap<WalletId, PlatformWallet>` (handles sharing same Arc), `spv_client`.

**7. Wire `BalanceUpdated` events** — add `update_from_parts(spendable, unconfirmed, immature, locked)` to `WalletBalance`. Event bridge updates atomics.

### evo-tool changes

- Update `SpvEventBridge` to handle `PlatformWalletEvent::Wallet(BalanceUpdated{...})`
- Update E2E test harness for new API surface

### What gets deleted (~410 lines)

| File | Lines |
|------|-------|
| `spv/wallet_adapter.rs` | ~330 |
| `spv/sync_state.rs` | ~55 |
| `PlatformWalletInfoWriteGuard` | ~25 |

### Implementation sequence

Phase 1 (dashcore): Add wallet()/wallet_mut() to trait → extract check_core_transaction helper → create ManagedWalletState + WalletPersistence → change WalletManager to single Arc<RwLock<T>> map → update all methods/tests/FFI.

Phase 2 (platform-wallet): Move persister into PlatformWalletInfo → implement WalletInfoInterface → delete SpvWalletAdapter/SpvSyncState/WriteGuard → update SpvRuntime → restructure PlatformWalletManager → wire events.

Phase 3 (evo-tool): Update event bridge → update E2E tests.

---

## PR-31: Evo-tool identity + asset lock cleanup (leftovers from PR-20/21)

### Goal

Clean up remaining gaps from PR-20 (identity lifecycle) and PR-21 (asset lock duplication). Two concrete issues:

### 1. Evo-tool uses low-level `_with_signer` instead of one-call identity APIs

**Problem**: Evo-tool's `RegisterIdentityTask` and `TopUpIdentityTask` call the low-level `register_identity_with_signer()` / `top_up_identity_with_signer()` methods and manually implement IS→CL fallback (~40 lines each). Platform-wallet's `IdentityWallet` already has one-call methods (`register_identity_with_funding`, `top_up_identity_with_funding`, `funded_register_identity`, `funded_top_up_identity`) that handle IS→CL fallback internally.

**Fix**: Switch evo-tool tasks to use the one-call APIs. Delete manual IS→CL fallback code in:
- `dash-evo-tool/src/backend_task/identity/top_up_identity.rs` (lines ~112-190)
- `dash-evo-tool/src/backend_task/identity/register_identity.rs` (lines ~255-298, ~394-430)

### 2. Asset lock changeset restore is not implemented

**Problem**: `PlatformWallet::apply()` calls `self.asset_locks.restore_from_changeset_blocking(asset_lock_cs)` but this method doesn't exist. Asset lock changesets are written to the persister (evo-tool's SQLite) but never loaded back. Evo-tool works around this with `register_with_asset_lock_manager()` bridge code that scans the DB and manually re-registers locks with the manager.

**Fix**:
- Implement `AssetLockManager::restore_from_changeset_blocking()` in platform-wallet — reconstruct `tracked_asset_locks` from `AssetLockChangeSet`
- Verify `PlatformWallet::apply()` actually calls it correctly on wallet load
- Once changeset restore works, simplify evo-tool's `recover_asset_locks.rs` — the bridge code `register_with_asset_lock_manager()` becomes unnecessary since locks are restored from persistence automatically
- Update UI screens (`by_using_unused_asset_lock.rs`) to read from `AssetLockManager.list_tracked_locks()` instead of querying DB directly

### Files to modify

**platform-wallet:**
- `src/wallet/asset_lock/manager.rs` — implement `restore_from_changeset_blocking`
- `src/wallet/platform_wallet.rs` — verify `apply()` works end-to-end

**evo-tool:**
- `src/backend_task/identity/top_up_identity.rs` — switch to one-call API
- `src/backend_task/identity/register_identity.rs` — switch to one-call API
- `src/backend_task/core/recover_asset_locks.rs` — simplify once changeset restore works
- `src/ui/identities/add_new_identity_screen/by_using_unused_asset_lock.rs` — read from manager
- `src/ui/identities/top_up_identity_screen/by_using_unused_asset_lock.rs` — read from manager

---

## Overview

**Goal**: Replace `dash-evo-tool`'s self-written wallet and duplicated DashPay crypto with `rs-platform-wallet`, building and integrating iteratively — one vertical slice at a time.

**Approach**: Each PR implements a feature in `rs-platform-wallet` **and** immediately wires it into `evo-tool`, replacing the corresponding old code. Both repos share a feature branch pair (`feat/platform-wallet` in each), linked via `path` dependency in Cargo.toml. No "build everything first, integrate later" — integration is part of every PR.

**Branch setup**:
- `platform` repo: `feat/platform-wallet` (feature branch, merges to `v3.1-dev` via PRs)
- `dash-evo-tool` repo: `feat/platform-wallet` (feature branch, merges to `v1.0-dev` via PRs)
- `Cargo.toml` in evo-tool: `platform-wallet = { path = "../../platform/packages/rs-platform-wallet" }`

---

## Architecture (current — single-lock design, post-refactoring)

```
key-wallet (rust-dashcore) — reused types
├── Wallet                       ← mutable key store (mnemonic, xprv, accounts added during sync)
├── ManagedWalletInfo            ← mutable UTXO state, accounts, balance, address pools
├── ManagedAccountCollection     ← BIP44 + DashPay + PlatformPayment + Identity accounts
├── TransactionRouter            ← transaction classification + checking
├── WalletTransactionChecker     ← trait for tx matching (impl on ManagedWalletInfo)
├── TransactionContext           ← Mempool | InstantSend | InBlock(BlockInfo) | InChainLockedBlock(BlockInfo)
└── BlockInfo                    ← { height, block_hash, timestamp } (all required)

rs-platform-wallet
├── PlatformWalletInfo           ← SINGLE struct behind Arc<RwLock<PlatformWalletInfo>>
│   ├── wallet: Wallet
│   ├── wallet_info: ManagedWalletInfo
│   ├── identity_manager: IdentityManager
│   ├── tracked_asset_locks: BTreeMap<OutPoint, TrackedAssetLock>
│   ├── platform_address_balances: BTreeMap<PlatformAddress, Credits>
│   ├── token_watched: BTreeMap<Identifier, BTreeSet<Identifier>>
│   └── token_balances: BTreeMap<(Identifier, Identifier), TokenAmount>
│
├── PlatformWallet               ← cheaply cloneable handle to shared state
│   ├── wallet_id: WalletId
│   ├── sdk: Arc<Sdk>
│   ├── core: CoreWallet                       ← balance, UTXOs, addresses, tx building
│   ├── identity: IdentityWallet               ← register, discover, top-up, withdraw, transfer, DPNS
│   ├── dashpay: DashPayWallet                 ← send/accept contact requests, sync contacts
│   ├── platform: PlatformAddressWallet        ← DIP-17 sync, transfer, withdraw
│   ├── tokens: TokenWallet                    ← per-identity registry, sync, transfer, mint, burn
│   ├── asset_locks: Arc<AssetLockManager>     ← build, broadcast, track, proof lifecycle
│   ├── event_tx: broadcast::Sender<PlatformWalletEvent>
│   ├── persister: WalletPersister
│   └── state: Arc<RwLock<PlatformWalletInfo>> ← THE SINGLE LOCK (all sub-wallets share this)
│   
│   State access:
│   ├── wallet.state() → RwLockReadGuard<PlatformWalletInfo>  (async read)
│   ├── wallet.state_mut() → PlatformWalletInfoWriteGuard     (async write, auto-updates balance)
│   └── Sub-wallets also hold state: Arc<RwLock<PlatformWalletInfo>>
│
├── Sub-wallets (all hold Arc<RwLock<PlatformWalletInfo>> + Arc<Sdk>)
│   ├── CoreWallet               ← state: Arc<RwLock<PlatformWalletInfo>>
│   ├── IdentityWallet           ← state: Arc<RwLock<PlatformWalletInfo>>
│   ├── DashPayWallet            ← state: Arc<RwLock<PlatformWalletInfo>>
│   ├── PlatformAddressWallet    ← state: Arc<RwLock<PlatformWalletInfo>> + Signer<PlatformAddress>
│   └── TokenWallet              ← state: Arc<RwLock<PlatformWalletInfo>>
│
├── PlatformWalletManager        ← multi-wallet + SPV coordinator (feature-gated: manager)
│   ├── sdk: Sdk
│   ├── wallets: Arc<RwLock<BTreeMap<WalletId, PlatformWallet>>>
│   ├── event_tx: broadcast::Sender<PlatformWalletEvent>
│   └── spv: SpvRuntime
│
├── SpvRuntime (src/spv/runtime.rs)  ← SPV lifecycle
│   ├── wallets: Arc<RwLock<BTreeMap<WalletId, PlatformWallet>>>
│   ├── event_tx: broadcast::Sender<PlatformWalletEvent>
│   ├── synced_height: AtomicU32
│   ├── monitor_revision: Arc<AtomicU64>
│   ├── finality_waiters: Mutex<BTreeMap<Txid, Option<AssetLockProof>>>
│   └── client: RwLock<Option<SpvClient>>
│
├── SpvWalletAdapter (src/spv/wallet_adapter.rs)   ← multi-wallet WalletInterface
│   └── Iterates ALL wallets for process_block/process_mempool_transaction
│
├── SpvEventForwarder (src/spv/event_forwarder.rs) ← EventHandler impl
│
├── Signing
│   ├── IdentitySigner           ← Signer<IdentityPublicKey>
│   ├── ManagedIdentitySigner    ← key_storage + IdentitySigner fallback
│   └── PlatformAddressWallet    ← Signer<PlatformAddress>
│
├── Events
│   ├── PlatformWalletEvent      ← Wallet(WalletEvent) | Spv(SpvEvent)
│   └── TransactionStatus        ← Unconfirmed | InstantSendLocked | Confirmed | ChainLocked
│
└── [ShieldedWallet]             ← PR-15: feature-gated Orchard/Halo2

evo-tool integration (current state):
├── Wallet struct embeds Arc<PlatformWallet> — no duplicate fields
├── SPV: evo-tool's SpvManager still runs (old system), PlatformWalletManager bridges events
├── All UI reads go through wallet.state() (lock-free WalletBalance for hot path)
└── 347 lib tests passing
```

**Key design decisions:**
- **Single lock**: All mutable state in one `Arc<RwLock<PlatformWalletInfo>>` — eliminates deadlocks from PR-26. Sub-wallets share the same Arc. `PlatformWalletInfoWriteGuard` auto-updates `WalletBalance` on drop.
- **No WalletHandle**: `PlatformWallet.clone()` is cheap (few atomic increments). A clone is a shared handle to the same state.
- **State access pattern**: `wallet.state()` for async read, `wallet.state_mut()` for async write. Sub-wallets use `self.state.read().await` / `self.state.write().await` internally.
- **Lock ordering eliminated**: With one lock, there's no ordering problem. The old multi-lock design had confirmed deadlock risks between wallet/wallet_info/tracked locks.
- **SPV dual-system**: Evo-tool still runs its own SpvManager alongside PlatformWalletManager. Full SPV replacement is PR-28.

---

## PR History (completed)

1. **PR-1** ✅: Project scaffold + `PlatformWallet` + `PlatformWalletManager` + `CoreWallet` + evo-tool bridge
2. **PR-2** ✅: CoreWallet deep integration — `Signer<PlatformAddress>`, per-address data, asset locks, transaction sending
3. **PR-3** ✅: `IdentityWallet` — register, discover, top-up, withdraw, transfer, `IdentitySigner`
4. **PR-4** ✅: `DashPayWallet` — contact requests (simplified API), sync, accept
5. **PR-5** ✅: `PlatformAddressWallet` — DIP-17 sync, send, withdraw + review fixes
6. **PR-6** ✅: SPV lifecycle + TransactionStatus + EventHandler
7. **PR-7** ✅: Identity update + address fund flows + DPNS
8. **PR-8** ✅: Token operations — `TokenWallet`
9. **PR-9** ✅: Evo-tool integration Phase 1+2 — token + identity tasks migrated
10. **PR-10** ✅: ManagedIdentity — KeyStorage, IdentityStatus, DPNS names, 12-key discovery
11. **PR-11** ✅: Asset lock lifecycle + multi-mode funding
12. **PR-12** ✅: DashPay DIP-14/15 — 256-bit key derivation
13. **PR-13** ✅: Evo-tool integration Phase 3 — 20 tasks total migrated
14. **PR-14** ✅: Protocol completeness + evo-tool convergence — 27/42 tasks migrated
15. **PR-15** ✅: Shielded pool (feature-gated)
16. **PR-16** ✅: AssetLockFinalityEvent
17. **PR-17** ✅: Use dashcore asset lock builder
18. **PR-18** ✅: Replace evo-tool Wallet model with CoreWallet (~1,600 lines removed)
19. **PR-19** ✅: Migrate remaining Wallet fields (~2,700 lines removed)
22. **PR-22** ✅: ChangeSet-based persistence
**Uncommitted (on feat/platform-wallet):** Single-lock refactoring (PR-26 scope) — 7+ locks → single RwLock<PlatformWalletInfo>, state getters on PlatformWallet, CoreWallet cleanup

---

## PR-6: SPV lifecycle + TransactionStatus + EventHandler

### Status after v3.1-dev merge (2026-03-31)

**Already done** (by merging v3.1-dev with dashcore rev `5db46b4d` and fixing compilation):
- `TransactionContext::InBlock(BlockInfo)` — updated from named fields
- `check_core_transaction(&mut wallet, update_state, update_balance)` — extra params adapted
- `process_mempool_transaction(tx, is_instant_send) -> MempoolTransactionResult` — new signature
- `watched_outpoints()` — implemented via `get_spendable_utxos()`
- `TransactionContext::InstantSend` variant — used in mempool processing

**Cancelled**: `key-wallet-manager` crate merge into `key-wallet` — decision to keep them as separate crates. All imports remain `use key_wallet_manager::*`.

### What PR-6 now delivers

**1. TransactionStatus lifecycle tracking**

Add `TransactionStatus` enum to `events.rs` and per-transaction status tracking in `CoreWallet`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum TransactionStatus {
    Unconfirmed = 0,           // In mempool, no IS lock
    InstantSendLocked = 1,     // IS-locked, not yet mined
    Confirmed = 2,             // Mined in a block
    ChainLocked = 3,           // In a chain-locked block (highest finality)
}
```

- Track status per txid in `CoreWallet` (or via `SpvWalletAdapter`)
- Emit `PlatformWalletEvent::Wallet(WalletEvent::TransactionStatusChanged)` on transitions
- `process_instant_send_lock()` on `SpvWalletAdapter`: update status, call `mark_instant_send_utxos()` on WalletInfoInterface
- Pattern from evo-tool: `src/model/wallet/mod.rs` lines 520-577

**2. EventHandler implementation**

Implement `dash_spv::EventHandler` trait on a new `SpvEventForwarder` struct that forwards SPV events to `PlatformWalletEvent` broadcast channel:

```rust
pub(crate) struct SpvEventForwarder {
    event_tx: broadcast::Sender<PlatformWalletEvent>,
}

impl EventHandler for SpvEventForwarder {
    fn on_sync_event(&self, event: &SyncEvent)     { /* → PlatformWalletEvent::Spv(SpvEvent::Sync(event)) */ }
    fn on_network_event(&self, event: &NetworkEvent) { /* → PlatformWalletEvent::Spv(SpvEvent::Network(event)) */ }
    fn on_progress(&self, progress: &SyncProgress) { /* → PlatformWalletEvent::Spv(SpvEvent::Progress(progress)) */ }
    fn on_wallet_event(&self, event: &WalletEvent)  { /* → PlatformWalletEvent::Wallet(event) */ }
    fn on_error(&self, error: &str)                  { /* → tracing::error! */ }
}
```

`EventHandler` trait (from `dash-spv/src/client/event_handler.rs`):
- `on_sync_event(&self, event: &SyncEvent)` — sync lifecycle (headers stored, sync complete)
- `on_network_event(&self, event: &NetworkEvent)` — peer connection changes
- `on_progress(&self, progress: &SyncProgress)` — overall sync progress
- `on_wallet_event(&self, event: &WalletEvent)` — transaction received, balance updated
- `on_error(&self, error: &str)` — fatal errors
- All have default no-op implementations

**3. Wire SPV lifecycle via `SpvRuntime`**

SPV lifecycle is managed by `SpvRuntime` (extracted from `PlatformWalletManager`).
`PlatformWalletManager::spv().start(config)` / `spv().stop()` delegates to `SpvRuntime`:

```rust
// SpvRuntime creates the SpvWalletAdapter (multi-wallet) and SpvEventForwarder
// DashSpvClient<SpvWalletAdapter, PeerNetworkManager, DiskStorageManager, SpvEventForwarder>

impl SpvRuntime {
    pub async fn start(&self, config: ClientConfig) -> Result<(), PlatformWalletError> {
        let adapter = SpvWalletAdapter::new(self.wallets.clone(), self.event_tx.clone(), self.monitor_revision.clone());
        let handler = Arc::new(SpvEventForwarder::new(self.event_tx.clone()));
        // ...construct and start DashSpvClient
    }
}
```

Need to determine concrete types for `N: NetworkManager` and `S: StorageManager` — check what evo-tool uses (likely `PeerNetworkManager` and `DiskStorageManager` from dash-spv).

**~~4. AssetLockFinalityEvent tracking~~** — deferred to PR-11 (SPV migration)

Currently `CoreWallet` uses SDK's `wait_for_asset_lock_proof_for_transaction()` which polls DAPI.
The SPV-based approach (listen for IS/CL events via finality channel) requires SPV to be running,
which isn't guaranteed for standalone `PlatformWallet`. Will be implemented when evo-tool's
`SpvManager` is migrated to `SpvRuntime::start()` (via `PlatformWalletManager::spv()`) in PR-11.

### What was delivered (PR-6 + follow-up)

| File | Changes |
|------|---------|
| `src/events.rs` | `TransactionStatus` enum, `SpvEvent` (Sync/Network/Progress), `PlatformWalletEvent` (Wallet/Spv) |
| `src/spv/wallet_adapter.rs` | Full `WalletInterface` impl, multi-wallet block/mempool processing, per-tx status tracking |
| `src/spv/event_forwarder.rs` | `EventHandler` impl forwarding SPV sync/network/wallet events to `PlatformWalletEvent` |
| `src/spv/runtime.rs` | `SpvRuntime` — SPV lifecycle, finality waiters, `start(config)`/`stop()` |
| `src/manager.rs` | `PlatformWalletManager` — CRUD + `spv()` accessor |
| `src/wallet/core/wallet.rs` | `transaction_statuses` map, `transaction_status()`, `update_transaction_status()` (monotonic) |
| `src/error.rs` | `SpvAlreadyRunning`, `NoWalletsConfigured`, `SpvError` variants |
| `Cargo.toml` | `dash-spv` dependency under `manager` feature gate |

---

## PR-1 Status: Complete

### What was delivered

**Platform-wallet library** (`rs-platform-wallet`):
- `PlatformWallet` — standalone wallet with sub-wallets as stored fields, cheaply cloneable (all Arc fields)
- `CoreWallet` — balance, UTXOs, spendable UTXOs, address generation, monitored addresses, transaction history, immature transactions, synced/birth height, network
- `IdentityWallet`, `DashPayWallet`, `PlatformAddressWallet` — struct stubs sharing `wallet_info` and `wallet` Arcs
- `IdentitySigner` — stub for state transition signing
- `PlatformWalletManager` — multi-wallet coordinator with create/import/remove/list/get, event subscription
- `SpvWalletAdapter` — implements `WalletInterface` for SPV integration
- `IdentityManager` — refactored (no sdk field, added last_scanned_index)
- Events: `PlatformWalletEvent` (Wallet/Spv), `WalletEvent`, `SpvEvent`, `TransactionStatus`
- No `WalletHandle` — `PlatformWallet.clone()` is cheap (~35 atomic ops)
- `Wallet` stored as `Arc<RwLock<Wallet>>` (mutable — accounts added during contact establishment/sync)
- Clean `mod.rs` files (module defs + re-exports only)
- `Send + Sync` assertions in `tests/thread_safety.rs`

**Evo-tool integration** (`dash-evo-tool`):
- `PlatformWalletManager` added to `AppContext` with `DebugWrapper`
- `platform_wallets` bridge map (keyed by `WalletSeedHash`) + `WalletIdMapping` (bidirectional)
- Wallet creation/import/unlock registers with bridge via `register_with_platform_wallet_manager()`
- Lock/remove/clear cleans up bridge
- `get_platform_wallet()` / `require_platform_wallet()` helpers
- 7 backend tasks validate via bridge at entry point
- `generate_receive_address` has diagnostic logging comparing old vs new paths
- `transfer_to_addresses` tries `platform_wallets` first with fallback
- Migration guide documented in `platform_wallet_bridge.rs`

**Dashcore** (`rust-dashcore`):
- `&mut Wallet` → `&Wallet` in `WalletTransactionChecker::check_core_transaction`
- All test callers cleaned up

**Platform SDK** (separate PRs):
- PR #3375: dashcore rev update + `Network::Dash` → `Network::Mainnet` rename
- PR #3376: Extract fetch helpers to fix HRTB Send inference

---

## PR-2 Status: Complete

### What was delivered

**Platform-wallet library** (`rs-platform-wallet`):
- `CoreAddressInfo`, `CoreAccountSummary` types (`wallet/core/types.rs`)
- Per-address methods: `all_address_info()`, `address_info()`, `account_summaries()`, `utxos_by_address()`
- `Signer<PlatformAddress>` on `PlatformAddressWallet` — `blocking_read()` bridge with sequential lock acquisition (no dual-lock window)
- Asset lock tx building: `build_registration_asset_lock_transaction()`, `build_topup_asset_lock_transaction()`, `build_asset_lock_transaction()` — DIP-9 key derivation, greedy UTXO selection, two-pass fee calc, `AssetLockPayload`, P2PKH signing
- `broadcast_transaction()` via DAPI `BroadcastTransactionRequest`
- `send_transaction()` — full payment flow (UTXO select with correct output count, overflow-safe amount sum, build, sign, broadcast)
- `create_registration_asset_lock_proof()`, `create_topup_asset_lock_proof()` — build + broadcast + wait for proof via `Sdk::wait_for_asset_lock_proof_for_transaction()`
- `build_and_broadcast_*` convenience methods
- Error variants: `AssetLockTransaction`, `TransactionBroadcast`, `TransactionBuild`, `AssetLockProofWait`

**Evo-tool integration** (`dash-evo-tool`):
- 4 signing callsites migrated from old `Wallet` to `platform_wallet.platform()` as `Signer<PlatformAddress>` (transfer_platform_credits, withdraw_from_platform_address, fund_platform_address_from_asset_lock, top_up_identity_from_platform_addresses)
- Asset lock creation tasks use CoreWallet with fallback to legacy (`try_build_registration_via_platform_wallet`, `try_build_topup_via_platform_wallet`)
- Shared `broadcast_and_track_asset_lock` helper eliminates broadcast code duplication
- Address table UI: cached snapshot pattern via `WalletTask::LoadAddressInfo` → `BackendTaskSuccessResult::AddressInfo` → `cached_address_info` in `WalletsBalancesScreen`
- `CoreAddressInfo` re-exported in `platform_wallet_bridge.rs`

**Review fixes applied:**
- Fee estimation uses actual output count (not hardcoded 2)
- `total_output` sum uses `checked_add` to prevent overflow
- Signer drops `wallet_info` lock before acquiring `wallet` lock (no deadlock window)

### Next steps

See PR-3 (IdentityWallet) in the PR Sequence section below.
5. **Payment building**: `send_transaction()` requires coin selection, signing, broadcast via SPV or RPC.
6. **SPV lifecycle**: `start_spv()` / `stop_spv()` are stubs — need network config wiring.

---

## Problem Statement (historical — kept for context)

**`dash-evo-tool`** maintains its own self-written wallet and duplicates DashPay crypto inline:

- `src/model/wallet/` — custom wallet struct with `identities`, `utxos`, `platform_address_info` fields
- `backend_task/dashpay/dip14_derivation.rs` — DIP-14 256-bit key derivation
- `backend_task/dashpay/hd_derivation.rs` — DashPay contact xpub path wrapper
- `backend_task/dashpay/encryption.rs` — DIP-15 ECDH + AES-CBC (duplicates `rs-platform-encryption`)

**`rs-platform-wallet`** is the intended canonical library but is incomplete:

- No `PlatformWallet` struct — only `PlatformWalletInfo` (the old pattern, being deleted)
- No identity registration, top-up, withdrawal, or credit transfer
- No DIP-14 CKDpriv256/CKDpub256
- No DashPay payment address derivation or payment sending
- No DIP-17 `AddressProvider` implementation
- No signing facade for state transition submission
- No bincode serialization for `IdentityManager`, `ManagedIdentity`, `ContactRequest`, `EstablishedContact`

**What already exists and can be reused** (confirmed in codebase):

- `rs-platform-encryption` crate — `derive_shared_key_ecdh`, `encrypt_extended_public_key`, `decrypt_extended_public_key`, `encrypt_account_label` — already a dependency of `rs-platform-wallet`
- `ContactRequest` and `EstablishedContact` structs — fully implemented
- `ManagedIdentity` with contact request management — fully implemented
- `IdentityManager` — implemented (needs `Arc<RwLock<_>>` wrapping + `last_scanned_index` field + removal of `sdk` field)
- `platform_wallet_info/contact_requests.rs` — `send_contact_request`, `add_incoming_contact_request`, `add_sent_contact_request` — consolidate into `DashPayWallet`
- `platform_wallet_info/identity_discovery.rs` — `discover_identities` — consolidate into `IdentityWallet::sync()`

---

## Architecture (OUTDATED — see "Architecture (current)" section above)

> **NOTE**: This section describes the OLD multi-lock design. The current design uses a single
> `Arc<RwLock<PlatformWalletInfo>>` — see the "Architecture (current)" section at the top.

```
key-wallet (rust-dashcore) — reused types
├── Wallet                       ← mutable key store (mnemonic, xprv, accounts added during sync)
├── ManagedWalletInfo            ← mutable UTXO state, accounts, balance, address pools
├── ManagedAccountCollection     ← BIP44 + DashPay + PlatformPayment + Identity accounts
├── TransactionRouter            ← transaction classification + checking
├── WalletTransactionChecker     ← trait for tx matching (impl on ManagedWalletInfo)
├── TransactionContext           ← Mempool | InstantSend | InBlock(BlockInfo) | InChainLockedBlock(BlockInfo)
└── BlockInfo                    ← { height, block_hash, timestamp } (all required)

rs-platform-wallet
├── PlatformWallet               ← cheaply cloneable (~35 atomic ops), all Arc fields
│   ├── wallet_id: WalletId
│   ├── sdk:      Sdk                              ← ref-counted
│   ├── core:     CoreWallet                       ← balance, UTXOs, addresses, tx building, asset locks
│   │   ├── wallet:      Arc<RwLock<Wallet>>
│   │   ├── wallet_info: Arc<RwLock<ManagedWalletInfo>>
│   │   ├── transaction_statuses: Arc<RwLock<BTreeMap<Txid, TransactionStatus>>>
│   │   └── tracked_asset_locks: Arc<RwLock<Vec<TrackedAssetLock>>>
│   ├── identity: IdentityWallet                   ← register, discover, top-up, withdraw, transfer, update, DPNS
│   │   ├── wallet, wallet_info, identity_manager: Arc<RwLock<...>>
│   │   ├── signer_for(identity_id) → ManagedIdentitySigner (key_storage + IdentitySigner fallback)
│   │   ├── update_identity(add_keys, disable_keys) ← IdentityUpdateTransition
│   │   ├── top_up_from_addresses() / transfer_credits_to_addresses()
│   │   ├── register_name() / resolve_name() / search_names() ← DPNS
│   │   ├── register_identity(IdentityFundingMethod) ← multi-mode funding
│   │   └── top_up_identity(TopUpFundingMethod) ← multi-mode top-up
│   ├── dashpay:  DashPayWallet                    ← send/accept contact requests, sync contacts
│   │   ├── wallet, wallet_info, identity_manager: Arc<RwLock<...>>
│   │   ├── register_contact_payment_addresses() ← gap limit + SPV watch
│   │   ├── match_payment_to_contact() ← incoming payment attribution
│   │   └── DIP-14 256-bit derivation (ckd_priv_256/ckd_pub_256) ← moved to library
│   ├── platform: PlatformAddressWallet            ← DIP-17 sync, transfer, withdraw, fund_from_asset_lock
│   │   ├── wallet, wallet_info: Arc<RwLock<...>>
│   │   ├── balances: Arc<RwLock<BTreeMap<PlatformAddress, Credits>>>
│   │   └── implements Signer<PlatformAddress> (blocking_read bridge)
│   ├── tokens:   TokenWallet                      ← per-identity registry, sync, transfer, mint, burn, etc.
│   │   ├── wallet, identity_manager: Arc<RwLock<...>>
│   │   ├── watched: Arc<RwLock<Map<IdentityId, Set<TokenId>>>>
│   │   ├── balances: Arc<RwLock<Map<IdentityTokenKey, TokenAmount>>>
│   │   └── watch/unwatch/sync/transfer/mint/burn/freeze/purchase/claim/set_price
│   └── [shielded: Option<ShieldedWallet>]         ← feature-gated, Orchard ZK pool (PR-15)
│
├── PlatformWalletManager        ← multi-wallet + SPV coordinator (feature-gated: manager)
│   ├── sdk: Sdk
│   ├── wallets: Arc<RwLock<BTreeMap<WalletId, PlatformWallet>>>
│   ├── event_tx: broadcast::Sender<PlatformWalletEvent>
│   ├── spv: SpvRuntime                            ← extracted SPV lifecycle
│   └── sdk() / spv() / add_wallet() / remove_wallet() / get_wallet() / wallet_ids()
│
├── SpvRuntime (src/spv/runtime.rs)  ← SPV lifecycle, extracted from manager
│   ├── wallets: Arc<RwLock<BTreeMap<WalletId, PlatformWallet>>>
│   ├── event_tx: broadcast::Sender<PlatformWalletEvent>
│   ├── synced_height: AtomicU32
│   ├── monitor_revision: Arc<AtomicU64>           ← shared with SpvWalletAdapter
│   ├── finality_waiters: Mutex<BTreeMap<Txid, Option<AssetLockProof>>>
│   ├── client: RwLock<Option<SpvClient>>
│   └── start(config) / stop() / synced_height() / notify_wallets_changed()
│
├── SpvWalletAdapter (src/spv/wallet_adapter.rs)   ← multi-wallet WalletInterface
│   ├── wallets: Arc<RwLock<BTreeMap<WalletId, PlatformWallet>>>  ← ALL wallets
│   ├── process_block() iterates ALL wallets
│   ├── process_mempool_transaction() iterates ALL wallets
│   ├── watched_outpoints() unions ALL wallets (for bloom filter)
│   ├── process_instant_send_lock() → per-wallet status tracking
│   └── monitor_revision: Arc<AtomicU64> (shared with SpvRuntime)
│
├── SpvEventForwarder (src/spv/event_forwarder.rs) ← EventHandler impl
│   └── forwards SPV sync/network/wallet events → PlatformWalletEvent
│
├── Signing
│   ├── IdentitySigner           ← Signer<IdentityPublicKey> (ECDSA/BLS/EdDSA, DIP-9 paths)
│   ├── ManagedIdentitySigner    ← Signer<IdentityPublicKey> wrapping key_storage + IdentitySigner fallback
│   └── PlatformAddressWallet    ← Signer<PlatformAddress> (ECDSA P2PKH, DIP-17 paths)
│
├── Events
│   ├── PlatformWalletEvent      ← Wallet(WalletEvent) | Spv(SpvEvent)
│   ├── SpvEvent                 ← Sync(SyncEvent) | Network(NetworkEvent) | Progress(SyncProgress)
│   └── TransactionStatus        ← Unconfirmed | InstantSendLocked | Confirmed | ChainLocked (monotonic)
│
└── [ShieldedWallet]             ← PR-15: shield, unshield, transfer, withdraw (Orchard/Halo2)
    ├── keys.rs                  ← OrchardKeySet (SpendingKey → FullViewingKey → OrchardAddress)
    ├── store.rs                 ← ShieldedStore trait, InMemoryShieldedStore
    ├── prover.rs                ← CachedOrchardProver with cached ProvingKey
    ├── sync.rs                  ← note sync + nullifier sync
    ├── operations.rs            ← shield, unshield, transfer, withdraw, shield_from_asset_lock
    └── note_selection.rs        ← select_spendable_notes

rs-sdk (Dash Platform SDK) — operations used by platform-wallet
├── Identity: PutIdentity, TopUpIdentity, WithdrawFromIdentity, TransferToIdentity
├── Identity update: IdentityUpdateTransition (add/disable keys, nonce-based)
├── Identity from addresses: TopUpIdentityFromAddresses, TransferToAddresses
├── DashPay: create/send_contact_request, fetch sent/received/all requests
├── Platform addresses: TransferAddressFunds, WithdrawAddressFunds, TopUpAddress
├── DPNS: register_dpns_name, resolve_dpns_name_to_identity, search_dpns_names
├── Tokens: transfer, mint, burn, freeze, purchase, claim, balance queries
├── Shielded: ShieldFunds, UnshieldFunds, TransferShielded, WithdrawShielded, ShieldFromAssetLock
├── Documents: PutDocument, TransferDocument, PurchaseDocument (for DashPay internals)
├── Fetch/FetchMany: identity, documents, balances, keys, platform addresses
└── sync_address_balances() with AddressProvider trait
```

**Key design decisions:**
- **No WalletHandle — use PlatformWallet.clone()**: All fields are Arc-wrapped, clone is ~35 atomic
  ops (nanoseconds). A separate handle type added complexity without meaningful encapsulation.
- **Wallet is mutable** (`Arc<RwLock<Wallet>>`): Accounts are added during DashPay contact
  establishment and sync. The `check_core_transaction` trait takes `&mut Wallet` (write lock)
  for transaction checking, as it may update wallet state (gap limit maintenance).
- **Sub-wallets share state via Arc**: All hold `Arc<RwLock<ManagedWalletInfo>>` and
  `Arc<RwLock<Wallet>>`. SPV writes through the Arc — visible to all clones immediately.
- **Network from sdk.network**: Sub-wallets no longer store a `network` field — they use
  `self.sdk.network` to get the network. Eliminates redundant cached state.
- **Lock ordering**: Always acquire `wallet` before `wallet_info` to prevent deadlocks.
  Signers use sequential `blocking_read()` (drop first lock before acquiring second).
- **key-wallet-manager stays as separate crate**: Imports use `key_wallet_manager::*`.
  The `WalletInterface` trait, `WalletEvent`, `BlockProcessingResult`, `MempoolTransactionResult`
  are in `key_wallet_manager`.
- **SpvRuntime extracted from manager**: `SpvRuntime` is a standalone struct in `src/spv/runtime.rs`
  that owns the `DashSpvClient`, tracks sync height, and manages finality waiters. Can be used
  both with the multi-wallet manager and potentially standalone. Manager delegates via `spv()`.
- **Multi-wallet SPV adapter**: `SpvWalletAdapter` wraps `Arc<RwLock<BTreeMap<WalletId,
  PlatformWallet>>>` — processes blocks and mempool transactions against ALL managed wallets,
  not a single wallet. `watched_outpoints()` unions outpoints from all wallets for bloom filters.
- **Shared monitor_revision via Arc<AtomicU64>**: `SpvRuntime` and `SpvWalletAdapter` share a
  `monitor_revision` counter. `notify_wallets_changed()` bumps it on wallet add/remove, triggering
  bloom filter rebuild in SPV. No manual filter management needed.
- **Manager simplified to CRUD + spv()**: `PlatformWalletManager` has `sdk()`, `spv()`,
  `add_wallet()`, `remove_wallet()`, `get_wallet()`, `wallet_ids()`, `subscribe_events()`. No
  create/import convenience methods — callers construct `PlatformWallet` directly, then `add_wallet()`.
- **TransactionStatus lifecycle**: Unconfirmed → InstantSendLocked → Confirmed → ChainLocked.
  Tracked per transaction in CoreWallet. Events emitted on state changes.
- **PlatformWalletEvent**: Two variants only — `Wallet(WalletEvent)` and `Spv(SpvEvent)`.
  `SpvEvent` wraps `Sync(SyncEvent)`, `Network(NetworkEvent)`, `Progress(SyncProgress)` from
  dash-spv. `Spv` variant is feature-gated behind `manager`.
- **Feature-gated shielded**: Orchard/Halo2 deps are heavy (~30s ProvingKey). Behind `shielded`
  feature. ShieldedWallet is fundamentally different (client-side state, note trial decryption,
  commitment tree) so it's a separate sub-wallet, not an extension of PlatformAddressWallet.
- **Private key zeroization**: `Zeroizing<[u8; 32]>` for all derived key material. `blocking_read()`
  drops locks before acquiring the next. Signer closures validate key ID parameters.
- **Simplified DashPay API**: `send_contact_request(sender, recipient)` — 2 params. All key indices,
  ECDH, derivation resolved internally. `accept_contact_request(request)` — 1 param.
- **Lazy key derivation** (PR-10): `PrivateKeyData::AtWalletDerivationPath` avoids holding raw private
  keys in memory for wallet-backed identities. Keys are derived on-demand during signing.
- **Identity status tracking** (PR-10): `IdentityStatus` state machine tracks identity lifecycle
  from registration through confirmation. Enables UI to show pending/active/failed states.
- **Asset lock lifecycle** (PR-11): `TrackedAssetLock` tracks locks from broadcast to use. IS→CL
  fallback is automatic via `resolve_asset_lock_proof()`. No lost or double-spent locks.
- **Multi-mode funding** (PR-11): `IdentityFundingMethod`/`TopUpFundingMethod` enums let callers
  choose between wallet UTXOs, pre-existing proofs, specific UTXOs, or platform addresses.
- **DashPay protocol crypto in library** (PR-12): DIP-14 256-bit derivation, contact payment address
  registration with gap limit, account reference calculation — protocol specs, not app logic.
- **Owned vs watched identity split** (PR-14): `ManagedIdentity` (owned, has key_storage, can sign,
  identity_index required) vs `WatchedIdentity` (observed, read-only, no keys). Type system enforces
  the distinction — no runtime "can I sign?" checks. Loaded-by-DPNS-name identities go to watched.
- **ManagedIdentitySigner resolves from key_storage** (PR-14): Three-step key resolution: (1) clear
  bytes from storage, (2) derive from wallet at stored path, (3) fall back to standard IdentitySigner
  derivation. Created via `managed_identity.signer(wallet, network)` or
  `identity_wallet.signer_for(identity_id)`.

---

## Implementation Plan (OUTDATED struct definitions — see current code)

> **NOTE**: The struct definitions below show the OLD multi-lock design with separate
> `Arc<RwLock<Wallet>>`, `Arc<RwLock<ManagedWalletInfo>>`, etc. The CURRENT design uses
> a single `Arc<RwLock<PlatformWalletInfo>>` containing all mutable state. Sub-wallets
> now hold `state: Arc<RwLock<PlatformWalletInfo>>` instead of individual lock fields.
> See the source code for current struct definitions.

`PlatformWallet` is a standalone wallet type (usable without SPV/manager). Cheaply cloneable
(a few atomic increments). No separate `WalletHandle` — use `PlatformWallet.clone()` directly.
`PlatformWalletManager` is the multi-wallet + SPV coordinator (no `WalletManager<T>` dependency).

### Struct Definitions

```rust
// Standalone wallet — owns all state, sub-wallets as stored fields
// Usable directly for Platform-only operations (scripts, tests, no SPV needed)
// Same type is wrapped in per-wallet RwLock when managed by PlatformWalletManager
// NOTE: No `wallet` field on PlatformWallet — sub-wallets hold their own Arc refs
pub struct PlatformWallet {
    wallet_id: WalletId,
    sdk:      Sdk,          // cheaply cloneable (ref-counted)
    core:     CoreWallet,
    identity: IdentityWallet,
    dashpay:  DashPayWallet,
    platform: PlatformAddressWallet,
    tokens:   TokenWallet,
}

// Sub-wallets — stored fields, share wallet_info via Arc<RwLock<ManagedWalletInfo>>
// Network is accessed via sdk.network (no cached network field)
pub struct CoreWallet {
    sdk:         Sdk,
    wallet:      Arc<RwLock<Wallet>>,
    wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    transaction_statuses: Arc<RwLock<BTreeMap<Txid, TransactionStatus>>>,  // finality tracking
    tracked_asset_locks: Arc<RwLock<Vec<TrackedAssetLock>>>,  // asset lock lifecycle
}

pub struct IdentityWallet {
    sdk:              Sdk,
    wallet:           Arc<RwLock<Wallet>>,
    wallet_info:      Arc<RwLock<ManagedWalletInfo>>,
    identity_manager: Arc<RwLock<IdentityManager>>,
}

pub struct DashPayWallet {
    sdk:              Sdk,
    wallet:           Arc<RwLock<Wallet>>,
    wallet_info:      Arc<RwLock<ManagedWalletInfo>>,
    identity_manager: Arc<RwLock<IdentityManager>>,  // same instance as IdentityWallet
}

pub struct PlatformAddressWallet {
    sdk:         Sdk,
    wallet:      Arc<RwLock<Wallet>>,
    wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    balances:    Arc<RwLock<BTreeMap<PlatformAddress, Credits>>>,  // balance cache
}

pub struct TokenWallet {
    sdk:              Sdk,
    wallet:           Arc<RwLock<Wallet>>,
    identity_manager: Arc<RwLock<IdentityManager>>,
    watched:          Arc<RwLock<BTreeMap<Identifier, BTreeSet<Identifier>>>>,  // identity → tokens
    balances:         Arc<RwLock<BTreeMap<IdentityTokenKey, TokenAmount>>>,  // cache
}

// Multi-wallet + SPV coordinator (feature-gated: manager)
// Delegates SPV lifecycle to SpvRuntime; simplified CRUD API
pub struct PlatformWalletManager {
    sdk:      Sdk,
    wallets:  Arc<RwLock<BTreeMap<WalletId, PlatformWallet>>>,
    event_tx: broadcast::Sender<PlatformWalletEvent>,
    spv:      SpvRuntime,  // extracted SPV lifecycle
}

// SPV client runtime — owns the DashSpvClient, tracks sync height,
// and manages asset-lock finality proof waiting.
// Extracted from PlatformWalletManager so it can be used standalone.
pub struct SpvRuntime {
    wallets:           Arc<RwLock<BTreeMap<WalletId, PlatformWallet>>>,
    event_tx:          broadcast::Sender<PlatformWalletEvent>,
    synced_height:     AtomicU32,
    monitor_revision:  Arc<AtomicU64>,  // shared with SpvWalletAdapter
    finality_waiters:  Mutex<BTreeMap<Txid, Option<AssetLockProof>>>,
    client:            RwLock<Option<SpvClient>>,
}

// Multi-wallet SPV adapter — processes blocks against ALL wallets
pub(crate) struct SpvWalletAdapter {
    wallets:                Arc<RwLock<BTreeMap<WalletId, PlatformWallet>>>,
    event_tx:               broadcast::Sender<WalletEvent>,
    platform_event_tx:      broadcast::Sender<PlatformWalletEvent>,
    synced_height:          AtomicU32,
    filter_committed_height: AtomicU32,
    monitor_revision:       Arc<AtomicU64>,  // shared with SpvRuntime
}

// IdentityManager is shared between IdentityWallet and DashPayWallet.
// Implements Clone — all fields are cheap to clone (just Arc clones).
// IdentityWallet and DashPayWallet share the same IdentityManager
// instance because PlatformWallet constructs them from the same source at build time.
// Two collections: `managed` for owned identities (can sign), `watched` for observed (read-only).
pub struct IdentityManager {
    managed:             Arc<RwLock<IndexMap<Identifier, ManagedIdentity>>>,   // owned, has key_storage
    watched:             Arc<RwLock<IndexMap<Identifier, WatchedIdentity>>>,   // observed, read-only
    primary_identity_id: Arc<RwLock<Option<Identifier>>>,
    last_scanned_index:  Arc<RwLock<u32>>,  // persisted gap scan state
    // REMOVED: sdk: Option<Arc<Sdk>> — SDK flows through caller struct
}
// Clone is cheap — just Arc clones. IdentityWallet and DashPayWallet hold
// the same Arc pointers — mutations visible to both.

// ManagedIdentity — an owned identity with key material. Can sign transitions.
// Requires identity_index: u32 (always required, not Optional) — set during
// registration or discovery. Used for DIP-9 key derivation paths.
// (PR-10) Enhanced with KeyStorage, IdentityStatus, DPNS names, wallet association.
// (PR-14) identity_index is always required — type system enforces this.
pub struct ManagedIdentity {
    pub identity: Identity,
    pub identity_index: u32,                           // always required (not Optional)
    pub key_storage: BTreeMap<KeyID, (IdentityPublicKey, PrivateKeyData)>,  // (PR-10)
    pub status: IdentityStatus,                        // (PR-10) state machine
    pub dpns_names: Vec<DpnsNameInfo>,                 // (PR-10) associated DPNS names
    pub wallet_seed_hash: Option<[u8; 32]>,            // (PR-10) link to source wallet
    pub wallet_index: Option<u32>,                     // (PR-10) HD index in wallet
    pub sent_contact_requests: Vec<ContactRequest>,
    pub received_contact_requests: Vec<ContactRequest>,
    pub established_contacts: Vec<EstablishedContact>,
}

// WatchedIdentity — an observed identity without key material. Read-only, cannot sign.
// Loaded via load_identity_by_dpns_name() or other external lookups.
// No key_storage, no identity_index — just identity data + DPNS names + status.
pub struct WatchedIdentity {
    pub identity: Identity,
    pub dpns_names: Vec<DpnsNameInfo>,
    pub status: IdentityStatus,
}

// ManagedIdentitySigner — Signer<IdentityPublicKey> that resolves keys from a
// ManagedIdentity's key_storage with IdentitySigner fallback.
// Three-step key resolution:
//   1. Clear bytes from key_storage (PrivateKeyData::Clear)
//   2. Derive from wallet at stored path (PrivateKeyData::AtWalletDerivationPath)
//   3. Fall back to standard IdentitySigner derivation (DIP-9 path from identity_index)
// Created via managed_identity.signer(wallet, network) or identity_wallet.signer_for(identity_id).
pub struct ManagedIdentitySigner {
    key_storage: BTreeMap<KeyID, (IdentityPublicKey, PrivateKeyData)>,
    identity_signer: IdentitySigner,  // fallback for keys not in storage
}

// (PR-10) Private key data — either raw bytes or lazy wallet derivation.
pub enum PrivateKeyData {
    Clear(Zeroizing<[u8; 32]>),
    AtWalletDerivationPath {
        wallet_seed_hash: [u8; 32],
        derivation_path: DerivationPath,
    },
}

// (PR-10) Identity lifecycle state machine.
pub enum IdentityStatus {
    Unknown,            // Not yet checked against Platform
    PendingCreation,    // Registration submitted, awaiting confirmation
    Active,             // Confirmed on Platform
    FailedCreation,     // Registration failed (can retry)
    NotFound,           // Was active but no longer on Platform
}

// (PR-10) DPNS name associated with an identity.
pub struct DpnsNameInfo {
    pub label: String,
    pub acquired_at: Option<u64>,
}

// (PR-11) Asset lock lifecycle tracking.
pub struct TrackedAssetLock {
    pub transaction: Transaction,
    pub output_address: Address,
    pub amount_duffs: u64,
    pub proof: Option<AssetLockProof>,
    pub identity_id: Option<Identifier>,
    pub status: AssetLockStatus,
}

pub enum AssetLockStatus {
    Broadcast,           // TX sent, waiting for proof
    InstantLocked,       // IS proof received
    ChainLocked,         // CL proof received (higher finality)
    UsedForRegistration, // Linked to an identity
    UsedForTopUp,        // Linked to an identity top-up
}

// (PR-11) Multi-mode identity registration funding.
pub enum IdentityFundingMethod {
    UseAssetLock { proof: AssetLockProof, private_key: PrivateKey },
    FundWithWallet { amount_duffs: u64 },
    FundWithUtxo { outpoint: OutPoint, txout: TxOut, address: Address },
    FundFromAddresses { inputs: BTreeMap<PlatformAddress, Credits> },
}

// (PR-11) Multi-mode identity top-up funding.
pub enum TopUpFundingMethod {
    UseAssetLock { proof: AssetLockProof, private_key: PrivateKey },
    FundWithWallet { amount_duffs: u64 },
    FundWithUtxo { outpoint: OutPoint, txout: TxOut, address: Address },
}
```

**No dashcore changes required.** Only `key-wallet` crate types are used directly (`Wallet`,
`ManagedWalletInfo`, `ManagedAccountCollection`, `TransactionRouter`, `WalletTransactionChecker`).
`key-wallet-manager` remains a separate crate — imports use `key_wallet_manager::*`.

**Concurrency model**: Sub-wallets share `Arc<RwLock<ManagedWalletInfo>>` — this is the synchronization
point between SPV (writes UTXO state) and wallet operations (reads balance, builds transactions).
No outer per-wallet lock needed. The manager's `RwLock<BTreeMap>` is only for wallet add/remove.

**No WalletHandle**: `PlatformWallet.clone()` is cheap (~35 atomic ops, all Arc fields).
A separate handle type was removed — it added complexity without meaningful encapsulation.

**Sub-wallets are stored fields** on `PlatformWallet`:

```rust
impl PlatformWallet {
    pub fn core(&self)     -> &CoreWallet            { &self.core }
    pub fn core_mut(&mut self) -> &mut CoreWallet    { &mut self.core }
    pub fn identity(&self) -> &IdentityWallet        { &self.identity }
    pub fn dashpay(&self)  -> &DashPayWallet         { &self.dashpay }
    pub fn platform(&self) -> &PlatformAddressWallet { &self.platform }
    pub async fn sync(&self) -> Result<SyncResult, PlatformWalletError>
}

impl PlatformAddressWallet {
    pub fn new(
        sdk: Sdk,
        wallet: Arc<RwLock<Wallet>>,
        wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    ) -> Self {
        Self {
            sdk, wallet, wallet_info,
            balances: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}
```

`PlatformWalletManager` API — simplified CRUD + SPV access. Callers construct `PlatformWallet`
directly, then add it to the manager. No create/import convenience methods:

```rust
impl PlatformWalletManager {
    // Construction
    pub fn new(sdk: Sdk) -> Self;

    // Accessors
    pub fn sdk(&self) -> &Sdk;
    pub fn spv(&self) -> &SpvRuntime;

    // Wallet CRUD
    pub async fn add_wallet(&self, wallet: PlatformWallet) -> Result<PlatformWallet>;
    pub async fn remove_wallet(&self, wallet_id: &WalletId) -> Result<PlatformWallet>;
    pub async fn get_wallet(&self, wallet_id: &WalletId) -> Option<PlatformWallet>;
    pub async fn wallet_ids(&self) -> Vec<WalletId>;

    // Events — unified stream
    pub fn subscribe_events(&self) -> broadcast::Receiver<PlatformWalletEvent>;
}

impl SpvRuntime {
    pub fn new(wallets: Arc<RwLock<BTreeMap<WalletId, PlatformWallet>>>,
               event_tx: broadcast::Sender<PlatformWalletEvent>) -> Self;
    pub fn synced_height(&self) -> u32;
    pub fn notify_wallets_changed(&self);    // bumps monitor_revision
    pub async fn start(&self, config: ClientConfig) -> Result<()>;
    pub async fn stop(&self) -> Result<()>;
    pub async fn register_for_finality(&self, txid: Txid);
    pub async fn wait_for_finality(&self, txid: Txid, timeout: Duration) -> Result<AssetLockProof>;
}

// Unified event enum — two variants only
pub enum PlatformWalletEvent {
    Wallet(WalletEvent),            // from block processing (TransactionReceived, BalanceUpdated)
    #[cfg(feature = "manager")]
    Spv(SpvEvent),                  // from DashSpvClient
}

// SPV event — groups sync, network, and progress events from dash-spv
#[cfg(feature = "manager")]
pub enum SpvEvent {
    Sync(dash_spv::sync::SyncEvent),
    Network(dash_spv::network::NetworkEvent),
    Progress(dash_spv::sync::SyncProgress),
}
```

Call sites — standalone `PlatformWallet`:

```rust
let wallet = PlatformWallet::from_mnemonic(sdk, network, "word1 ...", "", 1_500_000, options)?;
wallet.identity().register_identity(amount, keys).await?;
wallet.dashpay().send_contact_request(&sender_id, &recipient_id).await?;
wallet.core().balance();
```

Call sites — managed via `PlatformWalletManager` (construct wallet, then add to manager):

```rust
let wallet = PlatformWallet::from_mnemonic(sdk, "word1 ...", "", 1_500_000, options)?;
let wallet = mgr.add_wallet(wallet).await?;  // returns clone
mgr.spv().start(config).await?;              // SPV syncs all managed wallets
wallet.identity().register_identity(amount, keys).await?;
wallet.dashpay().sync().await?;
wallet.core().balance();
```

`sync()` on `PlatformWallet` orchestrates Platform-side syncs (SPV runs independently in background):

```rust
pub async fn sync(&self) -> Result<SyncResult, PlatformWalletError> {
    self.identity().sync().await?;
    self.dashpay().sync().await?;
    self.platform().sync_platform_address_balances(None).await?;
    Ok(SyncResult::default())
}
```

---

### 1.1 Wallet Construction

> How a `PlatformWallet` is created from key material + Sdk.

`PlatformWallet` is SPV-free. It needs only key material and an `Sdk`. No SPV config here — SPV
lives in `PlatformWalletManager` (via `SpvRuntime`). There is no `wallet` field on `PlatformWallet`
itself — each sub-wallet holds its own `Arc<RwLock<Wallet>>` reference. Sub-wallets use
`sdk.network` for the network (no cached `network` field).

Creation methods mirror `key-wallet`'s `Wallet` constructors, plus `sdk` parameter:

```rust
impl PlatformWallet {
    // Mirrors key-wallet Wallet creation methods + sdk
    pub fn from_mnemonic(
        sdk: Sdk, network: Network, mnemonic: &str, passphrase: &str,
        birth_height: CoreBlockHeight, options: WalletAccountCreationOptions,
    ) -> Result<Self>;

    pub fn from_xprv(
        sdk: Sdk, network: Network, xprv: &str,
        options: WalletAccountCreationOptions,
    ) -> Result<Self>;

    pub fn from_seed(
        sdk: Sdk, network: Network, seed: Seed,
        options: WalletAccountCreationOptions,
    ) -> Result<Self>;

    pub fn from_seed_bytes(
        sdk: Sdk, network: Network, seed_bytes: &[u8; 64],
        options: WalletAccountCreationOptions,
    ) -> Result<Self>;

    pub fn from_xpub(
        sdk: Sdk, network: Network, xpub: &str, can_sign_externally: bool,
    ) -> Result<Self>;

    pub fn from_external_signable(
        sdk: Sdk, network: Network, xpub: &str,
    ) -> Result<Self>;

    pub fn random(
        sdk: Sdk, network: Network,
        options: WalletAccountCreationOptions,
    ) -> Result<(Self, Mnemonic)>;

    pub fn from_bytes(sdk: Sdk, wallet_bytes: &[u8]) -> Result<Self>;
}

// Standalone usage
let mut wallet = PlatformWallet::from_mnemonic(
    sdk, Network::Testnet, "word1 word2 ...", "",
    1_500_000, WalletAccountCreationOptions::Default,
)?;
wallet.identity().register_identity(amount, keys).await?;

// Multi-wallet with SPV — construct wallet, add to manager
let mgr = PlatformWalletManager::new(sdk.clone());
let wallet = PlatformWallet::from_mnemonic(
    sdk, "word1 word2 ...", "",
    1_500_000, WalletAccountCreationOptions::Default,
)?;
let wallet = mgr.add_wallet(wallet).await?;
mgr.spv().start(spv_config).await?;
```

**Internally**: each creation method calls `key-wallet`'s `Wallet::from_mnemonic()` (etc.) to create the
mutable key store (`Arc<RwLock<Wallet>>`), then `ManagedWalletInfo::from_wallet()` for UTXO state, then
wraps both with `IdentityManager::new()` into a `PlatformWallet`. `PlatformAddressWallet::new()` is
called with a fresh `balances` cache (`Arc<RwLock<BTreeMap<PlatformAddress, Credits>>>`).

**`WalletAccountCreationOptions`**: always required (matches dashcore). Callers pass
`WalletAccountCreationOptions::Default` for standard BIP-44 account 0 + identity + DIP-17 accounts.

**Birth height**: passed through to `ManagedWalletInfo::with_birth_height()` — used by SPV
to skip earlier blocks when loaded into `PlatformWalletManager`. Defaults to 0 (full sync).

**`ManagedIdentity` requires `identity_index: u32`** (not Optional) — set during registration or
gap-limit discovery. Used for DIP-9 key derivation paths. Operations that need the index
(e.g., `send_contact_request`) return `IdentityIndexNotSet` if missing.

#### Files

- `packages/rs-platform-wallet/src/wallet/platform_wallet.rs` (replaces `platform_wallet_info/mod.rs`)
- `packages/rs-platform-wallet/src/manager.rs` (feature-gated `manager`)

#### Migration

The old `platform_wallet_info/` module (currently staged as deleted in git) must be fully removed.
`lib.rs` currently still imports `pub mod platform_wallet_info` — update to `pub mod platform_wallet`.

---

### 1.2 Platform SDK Integration

> Sdk lives in `PlatformWallet` and each sub-wallet — never in `IdentityManager`.

**Current state**: SDK is stashed inside `IdentityManager.sdk: Option<Arc<Sdk>>` — accessed only by identity
discovery. Every async method that submits state transitions requires the caller to pass `&Sdk` separately.

**Goal**: `PlatformWallet` holds `sdk: Sdk` as a plain field (cheaply cloneable via internal ref-counting —
confirmed at `rs-sdk/src/sdk.rs:134`). Each sub-wallet receives a clone at construction. All async methods
on sub-structs call `self.sdk` internally.

#### SDK traits used by platform-wallet

**Identity operations** (trait methods on `Identity`):
- `PutIdentity` — `put_to_platform_and_wait_for_response(sdk, proof, key, signer, settings)`
- `TopUpIdentity` — `top_up_identity(sdk, proof, key, fee_increase, settings) -> u64`
- `WithdrawFromIdentity` — `withdraw(sdk, address, amount, fee, signing_key, signer, settings) -> u64`
  - Note: takes signer **by value**
- `TransferToIdentity` — `transfer_credits(sdk, to_id, amount, signing_key, signer, settings) -> (u64, u64)`
  - Note: takes signer **by value**

**Identity from addresses**:
- `TopUpIdentityFromAddresses` — fund identity from platform addresses
- `TransferToAddresses` — move identity credits to platform addresses

**Platform address operations**:
- `TransferAddressFunds` — transfer between platform addresses
- `WithdrawAddressFunds` — withdraw platform address credits to Core L1
- `TopUpAddress` — fund platform address from identity balance

**Shielded pool** (feature-gated):
- `ShieldFunds`, `UnshieldFunds`, `TransferShielded`, `WithdrawShielded`, `ShieldFromAssetLock`

**DPNS** (convenience wrappers):
- `register_dpns_name`, `resolve_dpns_name_to_identity`

**Token transitions**:
- Transfer, mint, burn, freeze, purchase, claim, balance queries

**Signing** (Signer trait implementations):
- `Signer<IdentityPublicKey>` — `IdentitySigner` (withdraw/transfer take signer **by value**)
- `Signer<PlatformAddress>` — `PlatformAddressWallet` directly

**Documents** (for DashPay internals):
- `PutDocument`, `TransferDocument`, `PurchaseDocument`

**Fetch/FetchMany**:
- Identity, documents, balances, keys, platform addresses
- `sync_address_balances()` with `AddressProvider` trait

#### Tasks

- **1.2.1** Add `sdk: Sdk` to `PlatformWallet` and each sub-wallet. Sub-wallets receive a clone at construction.
- **1.2.2** Remove `sdk: Option<Arc<Sdk>>` from `IdentityManager` — SDK access flows through the caller struct.

#### Files

- `packages/rs-platform-wallet/src/wallet/platform_wallet.rs`
- `packages/rs-platform-wallet/src/wallet/identity/manager.rs`

---

### 1.3 Core Wallet Capabilities

> Expose UTXO wallet: accounts, addresses, balances, send Dash, SPV sync, asset lock proofs.

`key-wallet` (`rust-dashcore/key-wallet`) already implements all the building blocks:
`Wallet` (immutable key store), `ManagedWalletInfo` (mutable runtime state),
`TransactionBuilder` (coin selection, fee calc, signing), `AddressPool` (gap limit),
`WalletInfoInterface` + `ManagedAccountOperations` traits.
`dash-spv` handles SPV header sync and BIP157/158 compact filter transaction delivery.

`CoreWallet` is a stored sub-struct that holds `Arc<RwLock<ManagedWalletInfo>>` and exposes
these capabilities without leaking key-wallet internals. (`WalletInterface` is implemented
by `SpvWalletAdapter`, not `CoreWallet` — see §1.3.5 and §1.7.)

**Note on `ManagedAccountCollection` field names** (confirmed from key-wallet source):
- Standard accounts: `standard_bip44_accounts: BTreeMap<u32, ManagedCoreAccount>` (NOT a single `core_accounts` field)
- DashPay receive: `dashpay_receival_accounts: BTreeMap<DashpayAccountKey, ManagedCoreAccount>`
- DashPay send: `dashpay_external_accounts: BTreeMap<DashpayAccountKey, ManagedCoreAccount>`
- Platform payments: `platform_payment_accounts: BTreeMap<PlatformPaymentAccountKey, ManagedPlatformAccount>`

#### 1.3.1 — Wallet Initialization

Accounts are created automatically at wallet construction — callers never call
`add_account` explicitly. `PlatformWallet::new()` passes
`WalletAccountCreationOptions::Default` to `key-wallet`, which derives standard BIP-44
accounts and populates the initial address pool. This matches how evo-tool initializes
wallets via `import_wallet_from_extended_priv_key`.

DashPay and DIP-17 platform payment accounts are added lazily on first use
(contact establishment / first platform address request).

#### 1.3.2 — Address Generation

```rust
pub fn next_receive_address(&mut self) -> Result<Address, CoreWalletError>

pub fn next_change_address(&mut self) -> Result<Address, CoreWalletError>

pub fn monitored_addresses(&self) -> Vec<Address>
// Returns ALL watched addresses: BIP44 core + DashPay receival + (optionally) DIP-17
// dash-spv uses this to match BIP157/158 compact block filters
```

Derives next unused BIP-44 external/change address respecting gap limit (20).
`monitored_addresses()` is the hook for SPV integration — `dash-spv` calls this via
`WalletInterface` to match BIP157/158 compact filters against wallet addresses.

**Critical**: `monitored_addresses()` must include addresses from **all** account types in
`ManagedAccountCollection`, not just `standard_bip44_accounts`. This is how DashPay receiving addresses
get watched for incoming payments — no separate registration step, no manual bloom filter
management. When `DashPayWallet::sync()` adds a new `DashpayReceivingFunds` account (on contact
accepted), those addresses automatically appear in the next `monitored_addresses()` call.

#### 1.3.3 — Balance & UTXO Access

```rust
// Methods on CoreWallet:
pub fn balance(&self) -> WalletCoreBalance
// confirmed, unconfirmed, total in duffs

pub fn utxos(&self) -> Vec<Utxo>
pub fn spendable_utxos(&self) -> Vec<Utxo>
// filtered: confirmed, non-dust, unlocked

pub fn transaction_history(&self) -> Vec<TransactionRecord>
pub fn immature_transactions(&self) -> Vec<TransactionRecord>
// coinbase outputs not yet mature (< 100 blocks)
```

All delegate to `WalletInfoInterface` on `wallet_info`.

**Per-address data** (research finding): `ManagedWalletInfo` already tracks richer per-address data
than the evo-tool model via `AddressPool::AddressInfo` (balance, total_received, total_sent, tx_count,
derivation_path, used status, label, metadata). CoreWallet needs methods to surface this:

```rust
pub async fn all_address_info(&self) -> Vec<CoreAddressInfo>
pub async fn address_info(&self, address: &Address) -> Option<CoreAddressInfo>
pub async fn account_summaries(&self) -> Vec<CoreAccountSummary>
pub async fn utxos_by_address(&self) -> BTreeMap<Address, Vec<Utxo>>
pub async fn derivation_path_for_address(&self, address: &Address) -> Option<(DerivationPath, AccountType)>
```

Platform credits/nonces are NOT in key-wallet — they come from Platform state queries and
stay in a separate cache (populated by `PlatformAddressWallet::sync()`).

**UI sync/async bridge**: Cached snapshot pattern — screen holds `Vec<CoreAddressInfo>`,
background task calls `core_wallet.all_address_info().await`, sends snapshot via `TaskResult`,
screen renders from cache. Matches existing evo-tool `AppAction::BackendTask` /
`display_task_result` pattern.

#### 1.3.4 — Transaction Send

key-wallet only **builds** transactions — it has no send method. Broadcasting is a
separate concern (RPC, SPV, or DAPI). `CoreWallet` exposes `TransactionBuilder` directly
rather than a custom request struct — callers compose exactly what they need:

```rust
// Methods on CoreWallet:
pub async fn send_transaction(
    &self,
    outputs: Vec<(Address, u64)>,
) -> Result<Txid, CoreWalletError>

// Power-user escape hatches for custom flows (DashPay, asset lock, etc.)
pub fn transaction_builder(&self) -> TransactionBuilder  // change_address pre-set
pub fn spendable_utxos_with_keys(&self) -> (Vec<Utxo>, impl Fn(&Utxo) -> Option<SecretKey>)
pub async fn broadcast_transaction(&self, tx: Transaction) -> Result<Txid, CoreWalletError>
```

Common case:

```rust
let txid = wallet.core.send_transaction(vec![(addr, amount_duffs)]).await?;
```

`send_transaction` handles coin selection (greedy UTXO selection with correct output count),
signing (P2PKH), and broadcast internally. Uses `checked_add` for overflow-safe amount sums.
Two-pass fee calculation: first pass estimates with placeholder, second pass with actual size.

**`broadcast_transaction`**: broadcasts a raw Core transaction via DAPI `BroadcastTransactionRequest`.
This is the primary broadcast path when SPV is not active.

**Broadcast paths**:
- **DAPI mode**: `broadcast_transaction()` via `BroadcastTransactionRequest` — always available
- **SPV mode**: `DashSpvClient::broadcast_transaction(tx)` → P2P to connected peers

**`TransactionStatus`** tracks the lifecycle of each transaction:
```rust
pub enum TransactionStatus {
    Unconfirmed,
    InstantSendLocked,
    Confirmed { height: u32 },
    ChainLocked { height: u32 },
}
```
Lifecycle: Unconfirmed → InstantSendLocked → Confirmed → ChainLocked.
Tracked per transaction in CoreWallet. Events emitted on state changes.

#### 1.3.5 — SPV Sync Integration

`dash-spv` (`DashSpvClient<W, N, S, H: EventHandler>`) is the P2P sync layer. It uses **BIP157/158 compact
block filters** (not Bloom filters). It accepts `Arc<RwLock<W: WalletInterface>>`.
`DashSpvClient` is now parameterized with `EventHandler` (generic `H`) for SPV event forwarding.

**`SpvWalletAdapter`** implements the full `WalletInterface` trait (from `key_wallet_manager`):
- `process_block()` — iterates wallets, locks each `wallet_info`, calls `check_core_transaction` per tx
- `process_mempool_transaction(tx, is_instant_send: bool)` → `MempoolTransactionResult`
- `watched_outpoints() -> Vec<OutPoint>` — for bloom filter construction
- `monitor_revision() -> u64` — bloom filter staleness detection; change triggers reconstruction
- `process_instant_send_lock()` — marks UTXOs as instant-send confirmed
- `monitored_addresses` — collects from all wallets' `ManagedWalletInfo`
- `synced_height` / `update_synced_height` — tracks via `AtomicU32`, updates each wallet

Note: `check_core_transaction()` has gained an `update_balance: bool` parameter.

SPV lives in `SpvRuntime` (accessed via `PlatformWalletManager::spv()`), not in `PlatformWallet`.
`PlatformWallet` is SPV-free.

**Wiring** (`SpvRuntime::start(config)`):

```rust
// SpvRuntime creates SpvWalletAdapter (multi-wallet) + SpvEventForwarder
let adapter = SpvWalletAdapter::new(wallets.clone(), event_tx.clone(), monitor_revision.clone());
let handler = Arc::new(SpvEventForwarder::new(event_tx.clone()));
let client = DashSpvClient::new(config, network, storage, adapter, handler).await?;
```

**Block processing call chain**:

```
DashSpvClient
  → SpvWalletAdapter::process_block()             // WalletInterface impl
  → wallets.read() → iterate wallets
  → for each wallet:
    → wallet.core.wallet_info.write()             // Arc<RwLock<MWI>> — inner lock
    → check_core_transaction(tx, update_balance)  // WalletTransactionChecker (key-wallet)
    → ManagedWalletInfo state mutated
    → PlatformWalletEvent::Wallet(...) emitted
```

**`PlatformWalletEvent`** (unified enum, two variants):
- `Wallet(WalletEvent)` — `TransactionReceived`, `BalanceUpdated` (from block/mempool processing)
- `Spv(SpvEvent)` — `Sync(SyncEvent)`, `Network(NetworkEvent)`, `Progress(SyncProgress)` (feature-gated: `manager`)

**`SpvEventForwarder`** impl (`EventHandler` trait) forwards SPV events to `PlatformWalletEvent`:
- `on_sync_event`, `on_network_event`, `on_progress`, `on_wallet_event`, `on_error`

**Event subscription**:
```rust
let rx: broadcast::Receiver<PlatformWalletEvent> = mgr.subscribe_events();
```

**Two event channels**: `WalletInterface::subscribe_events()` returns `WalletEvent` (for SPV).
`PlatformWalletManager::subscribe_events()` (public API) returns `PlatformWalletEvent` which
wraps `WalletEvent` + `SpvEvent`. Internally, the `SpvWalletAdapter` forwards `WalletEvent`s
into the `PlatformWalletEvent` channel.

**No reorg notification**: `WalletInterface` has no `process_reorg` method — reorgs are handled
only at the `ChainTipManager` level in dash-spv; the wallet is never notified.

`key-wallet-manager` remains a separate crate — imports use `key_wallet_manager::*`.
`WalletInterface`, `WalletEvent`, `BlockProcessingResult`, `MempoolTransactionResult` are in
`key_wallet_manager`.

Transaction broadcasting goes through `DashSpvClient::broadcast_transaction(tx)` — P2P
to connected peers (see §1.3.4). `dash-spv` also delivers InstantLock and ChainLock events
needed for asset lock proof creation (§1.3.6).

#### 1.3.6 — Asset Lock Proof Creation

Required for identity **registration** and **top-up** (§1.4).

```rust
pub async fn create_asset_lock_proof(
    &self,
    amount_duffs: u64,
) -> Result<(AssetLockProof, PrivateKey), CoreWalletError>
```

`CoreWallet` method — derives the next DIP-9 funding key internally, sources UTXOs
from `wallet_info`, builds an `AssetLock` special transaction via `TransactionBuilder`,
broadcasts it, waits for the InstantLock via SPV, returns `(AssetLockProof, funding_private_key)`.

**Two proof types** (both fully implemented in rs-dpp):
- `AssetLockProof::Instant` — wraps InstantLock + full transaction + output index. Primary path.
- `AssetLockProof::Chain` — wraps `core_chain_locked_height` + outpoint. Fallback if InstantLock
  is not received within timeout (suggest 60s, matching DashSync iOS behaviour).

**Important**: The fallback to `AssetLockProof::Chain` requires the referenced block height to be
ChainLocked from Platform's perspective. The wallet must poll block confirmation before using
a Chain proof.

DIP-9 funding key paths:
- Registration: `m/9'/coin'/5'/1'/identity_index` (non-hardened terminal index)
- Top-up (unbound): `m/9'/coin'/5'/2'/topup_index` (non-hardened terminal)
- Top-up (bound): `m/9'/coin'/5'/2'/registration_index'/topup_index`

**Note**: `ManagedAccountCollection` has dedicated fields for these:
`identity_registration: Option<ManagedCoreAccount>`,
`identity_topup: BTreeMap<u32, ManagedCoreAccount>`,
`identity_topup_not_bound: Option<ManagedCoreAccount>`.

**Implementation notes**:
- **DIP-9** (not DIP-13) is the funding key derivation standard. Paths use `m/9'/coin'/5'/...`.
- **Two-pass fee calculation**: first pass estimates with placeholder inputs, second pass with actual
  transaction size. Minimum fee: 3000 duffs. Size formula: `10 + inputs*148 + outputs*34 + 60` bytes.
- **Proof wait**: uses `Sdk::wait_for_asset_lock_proof_for_transaction()` (rs-sdk, 232 lines) which
  polls Platform for proof availability after broadcast.
- **Reuse**: key-wallet `TransactionBuilder` for UTXO selection (greedy strategy).
- **Port ~300-400 lines**: asset lock tx construction (version-3 `Transaction` with `AssetLockPayload`
  special payload, OP_RETURN burn output).
- **Port ~400 lines**: recovery scanning (scan DIP-9 funding paths for unconfirmed locks).
- DIP-9 key derivation reuses `Wallet::derive_extended_private_key()` + identity account paths.

Additional API for top-up:
```rust
pub async fn create_topup_asset_lock_proof(
    &self,
    amount_duffs: u64,
    identity_index: u32,
) -> Result<(AssetLockProof, PrivateKey), CoreWalletError>
```

#### 1.3.7 — Asset Lock Recovery

```rust
pub async fn recover_asset_locks(&self) -> Result<Vec<RecoveredAssetLock>, CoreWalletError>
```

Scans known funding key paths for broadcast-but-unconfirmed asset lock transactions
and attempts to recover or rebroadcast them. Mirrors evo-tool's
`CoreTask::RecoverAssetLocks`.

#### 1.3.8 — Asset Lock Tracking (PR-11)

`CoreWallet` tracks asset locks from broadcast through to usage. This replaces ad-hoc
tracking in evo-tool and ensures asset locks are not lost or double-spent.

```rust
// Methods on CoreWallet:
pub fn track_asset_lock(&self, lock: TrackedAssetLock)
pub fn unused_asset_locks(&self) -> Vec<&TrackedAssetLock>  // Broadcast or IS/CL-proved, not yet used
pub fn mark_asset_lock_used(&self, txid: &Txid, usage: AssetLockStatus)
```

`tracked_asset_locks: Arc<RwLock<Vec<TrackedAssetLock>>>` holds all asset locks created
by this wallet. Status transitions: `Broadcast → InstantLocked → UsedForRegistration` (or
`→ ChainLocked → UsedForTopUp`). The `resolve_asset_lock_proof()` method (see below)
updates the status as proofs arrive.

#### 1.3.9 — Asset Lock Proof Resolution with IS→CL Fallback (PR-11)

When an InstantSend proof is rejected by Platform (`AssetLockInstantLockProofInvalid`),
the wallet automatically falls back to a ChainLock proof:

```rust
pub async fn resolve_asset_lock_proof(
    &self,
    txid: &Txid,
) -> Result<AssetLockProof, CoreWalletError>
```

Steps:
1. Try InstantSend proof (primary path — fast, ~2s)
2. If Platform rejects IS proof → query DAPI for tx to check `is_chain_locked` and `height`
3. If chain-locked and Platform has verified that height → build `ChainAssetLockProof`
4. If not chain-locked → return `AssetLockNotChainLocked` error

This logic is shared by both identity registration and top-up flows.

#### 1.3.10 — UTXO Retry on Exhaustion (PR-11)

When building an asset lock TX fails due to insufficient UTXOs:
1. Release wallet lock
2. Refresh UTXOs (if SPV running, trigger rescan; otherwise return error)
3. Retry once

```rust
pub async fn build_asset_lock_with_retry(
    &self,
    amount_duffs: u64,
) -> Result<(Transaction, PrivateKey), CoreWalletError>
```

#### Files

- `packages/rs-platform-wallet/src/wallet/core/wallet.rs` (new)
- `packages/rs-platform-wallet/src/wallet/core/asset_lock.rs` (PR-11) — TrackedAssetLock, tracking methods
- Depends on: `key-wallet` (`ManagedWalletInfo`, `TransactionBuilder`, `WalletInfoInterface`,
  `ManagedAccountOperations`, `FeeRate`, `SelectionStrategy`)
- Depends on: `key-wallet-manager` — `WalletInterface`, `WalletEvent`,
  `BlockProcessingResult`, `MempoolTransactionResult`
- Depends on: `dash-spv` (`broadcast_transaction`, InstantLock/ChainLock events)

---

### 1.4 Identity Management

> Register, discover, refresh, top-up, withdraw, transfer, update identities. Register DPNS names.

All methods are on `IdentityWallet` which holds `sdk`, `wallet: Arc<RwLock<Wallet>>`, and `identity_manager`.
No `wallet: &Wallet` parameter anywhere — key derivation and signing use `self.wallet` directly.
`identity_index` is stored on `ManagedIdentity` as `u32` (always required, not Optional).

**Managed vs watched routing** (PR-14):
- `sync()` adds discovered identities to `managed` collection (owned, with key_storage)
- `load_identity_by_index()` adds to `managed` collection (owned, with key_storage)
- `load_identity_by_dpns_name()` adds to `watched` collection (observed, read-only, no keys)
- `signer_for(identity_id)` creates `ManagedIdentitySigner` from the managed identity's key_storage

**ManagedIdentity enrichments** (PR-10):
- `key_storage: BTreeMap<KeyID, (IdentityPublicKey, PrivateKeyData)>` — lazy wallet derivation via `AtWalletDerivationPath`; avoids storing raw private keys in memory for wallet-backed identities
- `status: IdentityStatus` — state machine tracking identity lifecycle (`Unknown → PendingCreation → Active`, with `FailedCreation` and `NotFound` branches)
- `dpns_names: Vec<DpnsNameInfo>` — DPNS names associated with this identity, populated during `sync()`
- `wallet_seed_hash: Option<[u8; 32]>` — links identity back to source wallet for key re-derivation on recovery
- `wallet_index: Option<u32>` — HD index in the wallet, paired with `wallet_seed_hash`

**SDK method surface** (confirmed from `rs-sdk` source — these are trait methods on `Identity`, not on `Sdk`):
- `Identity::put_to_platform_and_wait_for_response(sdk, asset_lock_proof, private_key, signer, settings)` — `PutIdentity` trait
- `identity.top_up_identity(sdk, asset_lock_proof, private_key, user_fee_increase, settings) -> Result<u64>` — `TopUpIdentity` trait
- `identity.withdraw(sdk, address, amount, core_fee_per_byte, signing_key, signer, settings) -> Result<u64>` — `WithdrawFromIdentity` trait
  - Note: takes signer **by value**
- `identity.transfer_credits(sdk, to_identity_id, amount, signing_key, signer, settings) -> Result<(u64, u64)>` — `TransferToIdentity` trait
  - Note: takes signer **by value**

**Additional SDK traits**:
- `TopUpIdentityFromAddresses` — fund identity from platform addresses
- `TransferToAddresses` — move identity credits to platform addresses
- Key update: no SDK trait — build `IdentityUpdateTransition` via DPP, broadcast with `BroadcastStateTransition`

#### 1.4.1 — Register New Identity

**Current** (PR-3):
```rust
pub async fn register_identity(
    &mut self,
    amount_duffs: u64,
    key_types: &[IdentityKeySpec],
) -> Result<Identity, PlatformWalletError>
```

**Enhanced** (PR-11) — multi-mode funding via `IdentityFundingMethod`:
```rust
pub async fn register_identity(
    &mut self,
    funding: IdentityFundingMethod,
    key_types: &[IdentityKeySpec],
) -> Result<Identity, PlatformWalletError>
```

The `IdentityFundingMethod` enum supports four funding paths:
- `FundWithWallet { amount_duffs }` — builds asset lock from wallet UTXOs (with UTXO retry on exhaustion), broadcasts, waits for proof with IS→CL fallback
- `UseAssetLock { proof, private_key }` — uses a pre-existing asset lock proof
- `FundWithUtxo { outpoint, txout, address }` — builds asset lock from a specific UTXO
- `FundFromAddresses { inputs }` — funds from platform addresses (no asset lock needed, uses `put_with_address_funding()`)

Steps (for `FundWithWallet`):

1. `core_wallet.build_asset_lock_with_retry(amount)` → `(Transaction, PrivateKey)` (PR-11: with UTXO retry)
2. `core_wallet.broadcast_transaction(tx)` + `core_wallet.track_asset_lock(...)` (PR-11: track from broadcast)
3. `core_wallet.resolve_asset_lock_proof(txid)` → `AssetLockProof` (PR-11: IS→CL fallback)
4. Set `ManagedIdentity.status = PendingCreation` (PR-10)
5. Derive auth keys at DIP-9 paths, build `IdentityPublicKey` entries
6. Store derivation paths in `key_storage` as `AtWalletDerivationPath` (PR-10)
7. Build `Identity` object with keys
8. `identity.put_to_platform_and_wait_for_response(&sdk, proof, &key, &signer, None)` → confirmed `Identity`
9. Set `ManagedIdentity.status = Active` (PR-10), store `wallet_seed_hash` and `wallet_index` (PR-10)
10. Add to `identity_manager`

SDK traits used:
- `PutIdentity::put_to_platform_and_wait_for_response` — takes `&Identity`, `AssetLockProof`, `&PrivateKey`, `&impl Signer<IdentityPublicKey>`, returns confirmed `Identity`
- `TopUpIdentity::top_up_identity` — takes `AssetLockProof`, `&PrivateKey`, returns `u64` (new balance). No signer needed.
- `WithdrawFromIdentity::withdraw` — takes `Option<Address>`, amount, signer **by value**, returns `u64`
- `TransferToIdentity::transfer_credits` — takes `Identifier`, amount, signer **by value**, returns `(u64, u64)`

**DIP-9 key path** (3-component path with `key_type`): The full path is
`m/9'/coin'/5'/0'/key_type'/identity_index'/key_index'`
where `key_type` is: `0'` = ECDSA, `1'` = BLS. The existing `key_derivation.rs` omits the
`key_type'` segment — this must be fixed. The `key_type'` level enables multi-algorithm keys
under the same identity index.

**`signer_for` factory** on `IdentityWallet`:
```rust
pub fn signer_for(
    &self,
    identity_id: &Identifier,
) -> Result<ManagedIdentitySigner, PlatformWalletError>
```
Looks up the `ManagedIdentity` from the `managed` collection (errors if identity is only watched),
clones its `key_storage`, and constructs a `ManagedIdentitySigner` with an `IdentitySigner` fallback.
Three-step key resolution: (1) clear bytes from storage, (2) derive from wallet at stored path,
(3) fall back to standard IdentitySigner derivation from `identity_index`.
Also available as `managed_identity.signer(wallet, network)` for direct construction.

#### 1.4.2 — Identity Discovery (DIP-9 gap-limit scan)

Implementation exists in the old `platform_wallet_info/identity_discovery.rs`.
Current behaviour (pre-PR-10):

- Derives ECDSA auth key at `key_index=0` only
- Queries Platform via `Identity::fetch(&sdk, PublicKeyHash(key_hash))` — unique key hash
- `start_index` and `gap_limit` passed by caller — state not persisted
- SDK pulled from `IdentityManager.sdk` (stale pattern)
- Errors during fetch silently treated as misses

**What was fixed (PR-3):**

- Moved to `IdentityWallet::sync()`, no parameters
- `last_scanned_index: u32` stored in `IdentityManager` — persisted and resumed
- Gap limit hardcoded to 5
- `PublicKeyHash` unique lookup — correct for authentication keys
- Fetch errors surfaced properly
- SDK sourced from `self.sdk` on `IdentityWallet`

**Enhanced discovery (PR-10):**

- Scan key indices 0..12 per identity index (12-key lookup window, matching evo-tool's `AUTH_KEY_LOOKUP_WINDOW`)
- Support ECDSA_HASH160 matching (not just full pubkey) — handles identities registered with hash-based key types
- Fetch DPNS names for each discovered identity via DPNS contract query (`records.identity == identity_id`)
- Store matched derivation paths in `KeyStorage` as `AtWalletDerivationPath` — enables lazy key derivation without holding raw private keys
- Set `IdentityStatus::Active` for discovered identities
- Store `wallet_seed_hash` and `wallet_index` on discovered identities for recovery

```rust
pub async fn sync(&self) -> Result<Vec<Identifier>, PlatformWalletError>
```

Discovered identities are added to the `managed` collection (owned, with key_storage).

#### 1.4.3 — Refresh Identity

```rust
pub async fn refresh_identity(
    &mut self,
    identity_id: &Identifier,
) -> Result<(), PlatformWalletError>
```

Fetches latest balance and keys from Platform, updates `ManagedIdentity`.

#### 1.4.4 — Top Up Identity Credits

**Current** (PR-3):
```rust
pub async fn top_up_identity(
    &mut self,
    identity_id: &Identifier,
    amount_duffs: u64,
) -> Result<u64, PlatformWalletError>  // returns new balance
```

**Enhanced** (PR-11) — multi-mode funding via `TopUpFundingMethod`:
```rust
pub async fn top_up_identity(
    &mut self,
    identity_id: &Identifier,
    funding: TopUpFundingMethod,
) -> Result<u64, PlatformWalletError>  // returns new balance
```

The `TopUpFundingMethod` enum supports three funding paths:
- `FundWithWallet { amount_duffs }` — builds asset lock from wallet UTXOs (with UTXO retry), broadcasts, waits for proof with IS→CL fallback
- `UseAssetLock { proof, private_key }` — uses a pre-existing asset lock proof
- `FundWithUtxo { outpoint, txout, address }` — builds asset lock from a specific UTXO

Note: `FundFromAddresses` for top-up uses `top_up_from_addresses()` (already implemented in PR-7).

Steps (for `FundWithWallet`):

1. `self.core.build_asset_lock_with_retry(amount_duffs)` → `(Transaction, PrivateKey)` (PR-11: UTXO retry)
2. `self.core.broadcast_transaction(tx)` + `self.core.track_asset_lock(...)` (PR-11: track lifecycle)
3. `self.core.resolve_asset_lock_proof(txid)` → `AssetLockProof` (PR-11: IS→CL fallback)
4. Call `identity.top_up_identity(&self.sdk, asset_lock_proof, private_key, None, None)` — `TopUpIdentity` trait
5. Update `ManagedIdentity` balance

**Note**: `top_up_identity` takes `private_key: [u8; 32]` — pass the raw bytes of the asset lock funding private key.

#### 1.4.5 — Withdraw Credits to Core

```rust
pub async fn withdraw_identity_credits(
    &mut self,
    identity_id: &Identifier,
    to_address: Option<Address>,  // None = next wallet receive address from self.core
    amount_credits: u64,
    core_fee_per_byte: Option<u32>,
) -> Result<u64, PlatformWalletError>  // returns remaining balance
```

Calls `identity.withdraw(&self.sdk, address, amount, core_fee_per_byte, signing_key, signer, settings)`.
Signs using `IdentitySigner` (see §1.10).

#### 1.4.6 — Transfer Credits Between Identities

```rust
pub async fn transfer_credits(
    &mut self,
    from_identity_id: &Identifier,
    to_identity_id: &Identifier,
    amount_credits: u64,
) -> Result<u64, PlatformWalletError>
```

Calls `identity.transfer_credits(&self.sdk, to_identity_id, amount, signing_key, signer, settings)`.
Returns `(from_balance, to_balance)` — expose the from-balance to caller.

#### 1.4.7 — Update Identity Keys

```rust
pub async fn add_key_to_identity(
    &mut self,
    identity_id: &Identifier,
    new_key_spec: IdentityKeySpec,
) -> Result<(), PlatformWalletError>

pub async fn disable_identity_key(
    &mut self,
    identity_id: &Identifier,
    key_id: u32,
) -> Result<(), PlatformWalletError>
```

`add_key_to_identity` builds an `IdentityUpdateTransition` via DPP (not a raw SDK trait) and
broadcasts it with `BroadcastStateTransition`. The new key is derived at the next available
key index under the identity's DIP-9 path.

#### 1.4.8 — Top Up from Platform Addresses

```rust
pub async fn top_up_from_addresses(
    &mut self,
    identity_id: &Identifier,
    from_addresses: BTreeMap<PlatformAddress, Credits>,
) -> Result<u64, PlatformWalletError>  // returns new balance
```

Uses `TopUpIdentityFromAddresses` SDK trait. Signs each address contribution with
its DIP-17 derived key via `Signer<PlatformAddress>`.

#### 1.4.9 — Transfer to Platform Addresses

```rust
pub async fn transfer_to_addresses(
    &mut self,
    identity_id: &Identifier,
    to_addresses: BTreeMap<PlatformAddress, Credits>,
) -> Result<u64, PlatformWalletError>  // returns remaining identity balance
```

Uses `TransferToAddresses` SDK trait.

#### 1.4.10 — DPNS Name Operations

Convenience wrappers around SDK DPNS methods:

```rust
pub async fn register_name(
    &mut self,
    identity_id: &Identifier,
    name: &str,
) -> Result<Identifier, PlatformWalletError>  // document id

pub async fn resolve_name(
    &self,
    name: &str,
) -> Result<Option<Identifier>, PlatformWalletError>  // identity id
```

`register_name` wraps `sdk.register_dpns_name()`. `resolve_name` wraps
`sdk.resolve_dpns_name_to_identity()`.

#### 1.4.11 — Load Identity by Index (PR-14)

Targeted lookup for a single wallet identity index (unlike `sync()` which does a gap scan).

```rust
/// Derives auth key at identity_index, queries Platform by key hash.
/// If found, adds to IdentityManager's `managed` collection with KeyStorage + DPNS names.
/// Returns None if no identity is registered at this index.
pub async fn load_identity_by_index(
    &self,
    identity_index: u32,
) -> Result<Option<Identity>, PlatformWalletError>
```

Used when the caller knows the specific index (e.g., wallet recovery, user-selected index).
Adds to `managed` collection (owned, with key_storage derived from wallet).

#### 1.4.12 — Refresh Identity (PR-14)

Fetch latest state for a known identity from Platform (balance, keys, revision).

```rust
/// Re-fetches the identity from Platform and updates the local ManagedIdentity.
/// Unlike sync() which discovers NEW identities, this updates an EXISTING one.
pub async fn refresh_identity(
    &self,
    identity_id: &Identifier,
) -> Result<Identity, PlatformWalletError>
```

Updates: `identity` field (balance, revision, keys), `status` → Active (if found),
`last_updated_balance_block_time`.

#### 1.4.13 — Batch DPNS Refresh (PR-14)

Refresh DPNS names for all managed identities.

```rust
/// Queries Platform for current DPNS names for each identity in the manager.
/// Updates ManagedIdentity.dpns_names for all identities.
pub async fn refresh_dpns_names(&self) -> Result<(), PlatformWalletError>
```

Used on app startup or periodic refresh to keep names current.

#### 1.4.14 — Load Identity by DPNS Name (PR-14)

Resolve a DPNS name and load the identity into the manager.

```rust
/// Resolves name → identity ID, fetches identity from Platform, adds to manager's
/// `watched` collection (read-only, no key material).
/// Returns None if name doesn't resolve.
pub async fn load_identity_by_dpns_name(
    &self,
    name: &str,
) -> Result<Option<Identity>, PlatformWalletError>
```

Combines `resolve_name()` + `Identity::fetch()` + adds to `watched` collection as `WatchedIdentity`
(observed, read-only, no keys). Cannot sign transitions for watched identities.

#### Files

- `packages/rs-platform-wallet/src/wallet/identity/wallet.rs` — IdentityWallet
- `packages/rs-platform-wallet/src/wallet/identity/manager.rs` — IdentityManager (managed + watched)
- `packages/rs-platform-wallet/src/wallet/identity/funding.rs` — IdentityFundingMethod, TopUpFundingMethod
- `packages/rs-platform-wallet/src/wallet/identity/managed_identity/mod.rs` — ManagedIdentity
- `packages/rs-platform-wallet/src/wallet/identity/managed_identity/key_storage.rs` — PrivateKeyData, IdentityStatus, DpnsNameInfo, WatchedIdentity
- `packages/rs-platform-wallet/src/wallet/identity/managed_identity/block_time.rs` — BlockTime
- `packages/rs-platform-wallet/src/wallet/identity/managed_identity/identity_ops.rs`
- `packages/rs-platform-wallet/src/wallet/identity/managed_identity/contact_requests.rs`
- `packages/rs-platform-wallet/src/wallet/identity/managed_identity/contacts.rs`
- `packages/rs-platform-wallet/src/wallet/identity/managed_identity/label.rs`
- `packages/rs-platform-wallet/src/wallet/identity/managed_identity/sync.rs`
- `packages/rs-platform-wallet/src/wallet/signer.rs` — IdentitySigner + ManagedIdentitySigner

---

### 1.5 DashPay — Contacts, Transactions, Sync

> Full DIP-14/15 implementation: contact requests, encrypted xpub exchange, payment address
> derivation, send/receive Dash between contacts.

**Existing** (PR-4): `send_contact_request`, `accept_contact_request`, `decrypt_incoming_contact_request`,
`derive_payment_address_for_contact`, `send_dashpay_payment`, `sync()`, profiles, auto-accept proofs.

**PR-12 adds**: DIP-14 256-bit derivation moved to library, contact payment address registration with
gap limit management, account reference calculation, incoming payment attribution via `match_payment_to_contact()`.

#### DIP-14 Background

DashPay uses 256-bit derivation (CKDpriv256/CKDpub256) for contact-specific address spaces:

```
m(userA)/9'/5'/15'/0'/(userA_id_256bit)/(userB_id_256bit)/index
```

The 256-bit identity ID indices prevent the 31-bit collision attack. `CKDpriv256` is fully
compatible with BIP32 for indices < 2^32; uses `ser_256(i)` (big-endian, 32 bytes) for larger indices.

**Current state**: Lives in `dash-evo-tool/src/backend_task/dashpay/dip14_derivation.rs`.
Moves to `packages/rs-platform-wallet/src/platform_wallet/dashpay/dip14.rs` (PR-12).
This is protocol-level crypto and belongs in the library, not in the application.

#### DIP-15 Background

A contact request document on Platform contains:

- `encryptedPublicKey` (exactly 96 bytes = IV 16 + ciphertext 80): AES-CBC-256 encrypted xpub
  - xpub is 78 bytes in BIP32 wire format → padded to 80 bytes via PKCS7 (2 padding bytes)
- `encryptedAccountLabel` (optional 48-80 bytes): encrypted account name
- `accountReference` (32-bit): `(version<<28) | (HMAC-SHA256(senderKey, xpub)_28bits XOR account_28bits)`
- `senderKeyIndex` / `recipientKeyIndex`: identity key indices used for ECDH
- `$createdAt`, `$createdAtCoreBlockHeight`: required system fields
- **Documents are immutable**: `documentsMutable: false, canBeDeleted: false` — no update/delete API

ECDH shared key: `SHA256( (y[31]&0x1 | 0x2) || x )` — confirmed correct per DIP-15.
Uses `libsecp256k1_ecdh` with compressed-point SHA256 hash (verify libsecp256k1 >= 0.3.0).

**The `rs-platform-encryption` crate already implements all DIP-15 crypto** (confirmed in codebase):
- `derive_shared_key_ecdh()`, `encrypt_extended_public_key()`, `decrypt_extended_public_key()`,
  `encrypt_account_label()`, `encrypt_aes_256_cbc()`, `decrypt_aes_256_cbc()`
- Already a dependency: `platform-encryption = { path = "../rs-platform-encryption" }`
- **Do NOT duplicate these functions** — reuse `rs-platform-encryption` directly.

**Recipient key purpose**: The recipient's key must have `Purpose::DECRYPTION` (confirmed from
SDK's `contact_request.rs:229` — the SDK validates `Purpose::DECRYPTION` on the recipient key, NOT `ENCRYPTION`).

#### 1.5.1 — DIP-14 Key Derivation (dashpay module) (PR-12: moved from evo-tool to library)

```rust
// packages/rs-platform-wallet/src/platform_wallet/dashpay/dip14.rs  (new file)
pub fn ckd_priv_256(
    parent: &ExtendedPrivKey,
    index: &[u8; 32],  // 32-byte big-endian index (must be big-endian — interop requirement)
    hardened: bool,
) -> Result<ExtendedPrivKey>

pub fn ckd_pub_256(
    parent: &ExtendedPubKey,
    index: &[u8; 32],  // non-hardened only
) -> Result<ExtendedPubKey>

pub fn derive_dashpay_contact_xpub(
    master: &ExtendedPrivKey,
    network: Network,
    account: u32,
    sender_id: &[u8; 32],
    recipient_id: &[u8; 32],
) -> Result<ExtendedPubKey>
// Path: m/9'/coin'/15'/0'/(sender_id_256bit)/(recipient_id_256bit)
// First 4 components hardened, last 2 (identity IDs) non-hardened
```

**DIP-14 test vectors** — must implement and pass before merging PR-3:
- Mnemonic: "birth kingdom trash renew flavor utility donkey gasp regular alert pave layer"
- Four vectors provided in DIP-14 Appendix A with full hex outputs

**Big-endian requirement**: `ser_256(i)` must use big-endian byte order (most-significant byte
first), matching BIP32's `ser_32`. Verify this in `ckd_priv_256` before relying on the output.

**Backward compatibility**: For indices < 2^32, `CKDpriv256` produces identical results to BIP32.

#### 1.5.2 — DIP-15 Encryption (reuse `rs-platform-encryption`)

```rust
// DO NOT re-implement — use existing rs-platform-encryption functions:
use platform_encryption::{
    derive_shared_key_ecdh,       // ECDH: SHA256((y[31]&0x1|0x2)||x)
    encrypt_extended_public_key,  // AES-CBC-256, IV(16) + ciphertext(80) = 96 bytes
    decrypt_extended_public_key,  // Returns ExtendedPubKey from 96-byte blob
    encrypt_account_label,        // Optional account label encryption
    compute_account_reference,    // (version<<28) | (HMAC-SHA256_28bits XOR account_28bits)
};
```

**Critical bug to fix**: The existing `add_incoming_contact_request` in `contact_requests.rs`
calls `ExtendedPubKey::decode(&encrypted_public_key)` on the raw encrypted bytes without first
decrypting them via AES-CBC-256. This must be fixed: decrypt first, then decode.

The correct flow:
```rust
let shared_key = derive_shared_key_ecdh(&our_privkey, &sender_pubkey);
let xpub = decrypt_extended_public_key(&contact_request.encrypted_public_key, &shared_key)?;
// Now xpub is the 78-byte BIP32 xpub — use it to create DashpayExternalAccount
```

#### 1.5.3 — Send Contact Request

Simplified 2-parameter API — all other parameters resolved internally by the wallet:

```rust
pub async fn send_contact_request(
    &self,
    sender_identity_id: &Identifier,
    recipient_identity_id: &Identifier,
) -> Result<(), PlatformWalletError>
```

Internally resolved:
- **identity_index**: looked up from `ManagedIdentity.identity_index` (u32, required)
- **sender_key_index**: first key with `Purpose::ENCRYPTION` on the sender identity
- **recipient_key_index**: first key with `Purpose::DECRYPTION` on the recipient identity (fetched from Platform)
  - ECDH key type validation: both keys must be ECDH-compatible (secp256k1)
- **account_index**: defaults to `0`
- **ECDH**: always performed using `EcdhProvider::SdkSide` (wallet has seed, can derive private key)

Steps:

1. Retrieve sender identity and its HD index from `IdentityManager`
2. Fetch recipient identity from Platform
3. Find sender ENCRYPTION key (first match) — validate ECDH key type
4. Find recipient DECRYPTION key (first match) — validate ECDH key type
5. Derive DashPay receiving-account xpub
6. Derive ECDH private key from wallet using `m/9'/coin'/5'/0'/0'/identity_index'/key_id'`
7. Submit via `sdk.send_contact_request()` with `EcdhProvider::SdkSide`
8. Store in `ManagedIdentity.sent_contact_requests`

**Note**: `contactRequest` documents are immutable — no retry/update API. If submission fails, it's a new request.

**Note**: `ManagedIdentity.identity_index` is `u32` (required). Operations return `IdentityIndexNotSet` if missing.

#### 1.5.3a — Accept Contact Request

Simplified 1-parameter API:

```rust
pub async fn accept_contact_request(
    &self,
    contact_request: &ContactRequest,
) -> Result<(), PlatformWalletError>
```

Internally:
1. Decrypt the incoming contact request (§1.5.4)
2. Create `DashpayReceivingFunds` account in `ManagedAccountCollection`
3. Store as `EstablishedContact`

All key indices, ECDH derivation, and account index resolution happen internally.

#### 1.5.4 — Decrypt Incoming Contact Request

Fix the existing implementation:

```rust
pub fn decrypt_incoming_contact_request(
    &self,
    our_identity_id: &Identifier,
    contact_request: &ContactRequest,
) -> Result<ExtendedPubKey, PlatformWalletError>
```

Steps:

1. Retrieve our DECRYPTION private key at `contact_request.recipient_key_index`
2. Retrieve sender's public key at `contact_request.sender_key_index`
3. Compute ECDH shared key: `derive_shared_key_ecdh(&our_privkey, &sender_pubkey)`
4. **Decrypt first**: `decrypt_extended_public_key(&contact_request.encrypted_public_key, &shared_key)?`
5. Store resulting xpub as `DashpayExternalAccount` in `ManagedAccountCollection`

#### 1.5.5 — Payment Address Derivation

```rust
pub fn derive_payment_address_for_contact(
    &self,
    our_identity_id: &Identifier,
    contact_id: &Identifier,
    payment_index: u32,
) -> Result<Address, PlatformWalletError>
```

Non-hardened BIP32 child of the stored `DashpayExternalAccount` xpub at `payment_index`.
Payment gap limit: **10** (per DIP-15: "a gap limit of 10 at this stage").
Document this as a deliberate choice (20 is more conservative but DIP-15 specifies 10).

**PR-12 enhancements:**

Contact payment address registration + gap limit management:
```rust
/// (PR-12) Register payment addresses for all established contacts.
/// Derives up to highest_receive_index + GAP_LIMIT addresses per contact.
/// Returns new addresses that should be added to SPV bloom filter.
pub async fn register_contact_payment_addresses(
    &self,
) -> Result<Vec<Address>, PlatformWalletError>

/// (PR-12) Process an incoming payment detected at a contact address.
/// Returns contact info if the address matches a known contact relationship.
pub fn match_payment_to_contact(
    &self,
    address: &Address,
) -> Option<(Identifier, Identifier, u32)>  // (owner_id, contact_id, address_index)
```

Gap limit = 20 per contact for receiving. When payment arrives at index N, extend
registration to N + 20. `register_contact_payment_addresses()` is called during
`sync()` and after each incoming payment to maintain the gap window.

Account reference calculation (PR-12):
```rust
/// (PR-12) Calculate account reference per DIP-15.
/// HMAC-SHA256(sender_secret, xpub_bytes) → take 28 MSBs → XOR with account bits.
pub fn calculate_account_reference(
    sender_secret_key: &[u8; 32],
    contact_xpub: &ExtendedPubKey,
    account_index: u32,
    version: u32,
) -> u32
```

#### 1.5.6 — Send Payment to Contact

```rust
pub async fn send_dashpay_payment(
    &self,
    our_identity_id: &Identifier,
    contact_id: &Identifier,
    amount_duffs: u64,
    fee_per_byte: u32,
) -> Result<Txid, PlatformWalletError>
```

Gets next unused payment index → derives address → coin-selects UTXOs →
builds, signs, broadcasts Core transaction → increments stored payment index.

#### 1.5.7 — DashPay Sync (`DashPayWallet::sync()`)

`DashPayWallet::sync()` is the Platform-side half of DashPay sync. It fetches new contact
request documents from DAPI and establishes the corresponding address accounts:

```rust
pub async fn sync(&self) -> Result<DashPaySyncResult, PlatformWalletError>
```

Uses `sdk.fetch_all_contact_requests_for_identity(identity, limit)` which returns
`(sent_requests, received_requests)` in one call.

For each known identity, in order:

1. Call `sdk.fetch_all_contact_requests_for_identity(&identity, None)` → `(sent, received)`
2. For each new incoming request: call `decrypt_incoming_contact_request()` to get the sender's xpub
3. Add a `DashpayReceivingFunds` account (`AccountType::DashpayReceivingFunds { index, user_identity_id, friend_identity_id }`) to `ManagedAccountCollection` — pre-derives gap_limit (20) addresses
4. For mutual contacts (both sent + received exist): ensure `DashpayReceivingFunds` account exists

**How incoming payments are detected (no manual registration needed):**

`CoreWallet::monitored_addresses()` returns addresses from ALL account types including
`dashpay_receival_accounts`. After `sync()` adds a new `DashpayReceivingFunds` account, the
next SPV compact filter pass automatically watches those addresses. No separate "register
dashpay addresses" task — the gap limit pool is maintained exactly like BIP44:

- When SPV delivers a tx matching a DashPay receiving address at index N:
  - `CoreWallet::process_transaction()` calls `wallet_info.process_transaction()`
  - key-wallet records the tx and marks that address used
  - If `N >= pool_size - gap_limit`, the pool is extended by deriving more addresses
  - Next `monitored_addresses()` call includes the new addresses — SPV picks them up

**Gap limits:**

- Receiving address pool per contact: 20 (same as BIP44 core, matches DIP-15: "watch highest_receive_index + 20 addresses per contact")
- Payment gap limit (sending): 10 (DIP-15 spec)

#### 1.5.8 — Profile Management

```rust
pub async fn create_dashpay_profile(
    &mut self,
    identity_id: &Identifier,
    display_name: Option<String>,
    bio: Option<String>,
    avatar_url: Option<String>,
) -> Result<Identifier, PlatformWalletError>

pub async fn update_dashpay_profile(
    &mut self,
    identity_id: &Identifier,
    display_name: Option<String>,
    bio: Option<String>,
    avatar_url: Option<String>,
) -> Result<(), PlatformWalletError>
```

#### 1.5.9 — Contact Info Document (Encrypted Private Metadata)

```rust
pub async fn update_contact_info(
    &mut self,
    identity_id: &Identifier,
    contact_id: &Identifier,
    nickname: Option<String>,
    accepted_account_reference: Option<u32>,
) -> Result<(), PlatformWalletError>
```

Submits DashPay `contactInfo` document — only visible to the identity owner.

#### 1.5.10 — DPNS Name Registration

DPNS usernames are the lookup mechanism for DashPay contact discovery.

```rust
pub async fn register_dpns_name(
    &mut self,
    identity_id: &Identifier,
    name: &str,
) -> Result<Identifier, PlatformWalletError>  // document id
```

#### 1.5.11 — Auto-Accept Proof

Auto-accept key derivation path: `m/9'/coin'/16'/timestamp'` (hardened timestamp).
Note: feature code `16'` (not `15'`) — distinct from the DashPay receiving fund path.
Proof format: 1-byte key type + 4-byte key index + 1-byte signature size + 32–96 bytes signature.

```rust
pub fn generate_auto_accept_proof(
    &self,
    sender_identity_id: &Identifier,
    recipient_identity_id: &Identifier,
) -> Result<Vec<u8>, PlatformWalletError>

pub fn verify_auto_accept_proof(
    &self,
    proof: &[u8],
    sender_identity: &Identity,
    recipient_identity: &Identity,
) -> bool
```

#### 1.5.12 — Reject Contact Request (PR-14)

```rust
/// Reject an incoming contact request by hiding it via contactInfo document.
/// Contact requests are immutable — rejection is done by creating/updating
/// a contactInfo document with display_hidden=true.
pub async fn reject_contact_request(
    &self,
    identity_id: &Identifier,
    contact_identity_id: &Identifier,
) -> Result<(), PlatformWalletError>
```

- Document type: `contactInfo` (DashPay contract)
- Sets `display_hidden: true`, other fields empty (nickname: None, note: None, accepted_accounts: [])

#### 1.5.13 — QR Auto-Accept Proof (PR-14)

```rust
/// Generate auto-accept proof for QR code sharing.
/// Derivation path: m/9'/coin'/16'/timestamp'
/// Signs: SHA256(sender_id || recipient_id || account_reference)
pub fn generate_auto_accept_proof(
    &self,
    sender_id: &Identifier,
    recipient_id: &Identifier,
    account_reference: u32,
    timestamp: u32,
) -> Result<Vec<u8>, PlatformWalletError>

/// Verify an auto-accept proof from a scanned QR code.
pub fn verify_auto_accept_proof(
    proof_bytes: &[u8],
    sender_id: &Identifier,
    recipient_id: &Identifier,
    account_reference: u32,
) -> Result<bool, PlatformWalletError>
```

- Proof format: key_type(1B) + timestamp(4B BE) + sig_size(1B) + signature(64B)
- Message: SHA256(sender_id(32B) || recipient_id(32B) || account_ref(4B LE))

#### 1.5.14 — Pre-Send Validation (PR-14)

```rust
/// Validate a contact request before sending.
/// Checks sender/recipient key types, purposes, security levels,
/// core height freshness, and account reference range.
pub fn validate_contact_request(
    sender_identity: &Identity,
    sender_key_index: u32,
    recipient_identity: &Identity,
    recipient_key_index: u32,
    account_reference: u32,
    core_height: u32,
) -> ContactRequestValidation

pub struct ContactRequestValidation {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}
```

Validation rules:
- Sender key must be ECDSA_SECP256K1, Purpose::ENCRYPTION, not disabled
- Recipient key must exist and be compatible with ECDH
- Core height within +-200 blocks of current
- Account reference within reasonable range

#### 1.5.15 — Account Label Encryption (PR-14)

```rust
/// Encrypt an account label for inclusion in contact request.
/// Uses ECDH shared key, CBC-AES-256 with PKCS7 padding.
/// Format: IV(16B) + ciphertext(32-64B). Max label: 62 bytes.
pub fn encrypt_account_label(label: &str, shared_key: &[u8; 32]) -> Result<Vec<u8>, PlatformWalletError>
pub fn decrypt_account_label(encrypted: &[u8], shared_key: &[u8; 32]) -> Result<String, PlatformWalletError>
```

#### 1.5.16 — Payment Address Registration (PR-14)

```rust
/// Register payment addresses for all established contacts.
/// Per contact: derives addresses up to highest_receive_index + GAP_LIMIT (20).
/// Returns new addresses for SPV bloom filter registration.
pub async fn register_contact_payment_addresses(
    &self,
) -> Result<ContactAddressRegistration, PlatformWalletError>

/// Match an incoming payment to a contact relationship.
pub fn match_payment_to_contact(
    &self,
    address: &Address,
) -> Option<ContactPaymentMatch>

pub struct ContactPaymentMatch {
    pub owner_id: Identifier,
    pub contact_id: Identifier,
    pub address_index: u32,
}

pub struct ContactAddressRegistration {
    pub new_addresses: Vec<Address>,
    pub contacts_processed: usize,
}
```

- Gap limit: 20 per contact
- Derivation path: m/9'/coin'/15'/0'/(our_id)/(contact_id)/index
- Track per-contact: highest_receive_index, registered_count
- When payment at index N arrives, extend to N + 20

#### 1.5.17 — Sent Contact Requests Query (PR-14)

```rust
/// Fetch sent (outgoing) contact requests from Platform.
pub async fn sent_contact_requests(
    &self,
    identity_id: &Identifier,
) -> Result<Vec<ContactRequest>, PlatformWalletError>
```

- Query: `$ownerId == identity_id`, order by `$createdAt`
- Currently only `sync_contact_requests()` fetches incoming; need both directions

#### Files

- `packages/rs-platform-wallet/src/wallet/dashpay/wallet.rs` — DashPayWallet struct + methods
- `packages/rs-platform-wallet/src/wallet/dashpay/dip14.rs` — DIP-14/15 crypto, ContactXpubData
- `packages/rs-platform-wallet/src/wallet/dashpay/auto_accept.rs` — QR auto-accept proof
- `packages/rs-platform-wallet/src/wallet/dashpay/validation.rs` — ContactRequestValidation
- `packages/rs-platform-wallet/src/wallet/dashpay/contact_request.rs` — contact request types
- `packages/rs-platform-wallet/src/wallet/dashpay/established_contact.rs` — established contact types
- `packages/rs-platform-wallet/src/wallet/dashpay/crypto.rs` — crypto helpers
- Reuses: `packages/rs-platform-encryption/` (DIP-15 crypto — do NOT duplicate)

---

### 1.6 Platform Addresses (DIP-17)

> Sync, send, transfer, and withdraw DIP-17 P2PKH credits through `PlatformWallet`.

**Key finding**: `ManagedAccountCollection` already has `platform_payment_accounts:
BTreeMap<PlatformPaymentAccountKey, ManagedPlatformAccount>`. `ManagedPlatformAccount` (key-wallet) tracks
per-address credit balances + gap-limit address pool. `PlatformWallet` must expose these
and implement the SDK's `AddressProvider` trait.

Derivation path (DIP-17): `m/9'/coin_type'/17'/account'/key_class'/index`
- `key_class' = 0'` for receive keys; `key_class' = 1'` reserved
- `index` is non-hardened
- Gap limit: 20 (`DIP17_GAP_LIMIT` constant in key-wallet `gap_limit.rs` — confirmed, 20 is the DIP-17 RECOMMENDED value)

#### 1.6.1 — AddressProvider Implementation

The rs-sdk's `sync_address_balances()` requires `&mut impl AddressProvider`.

**`PlatformPaymentAddressProvider`** implements the `AddressProvider` trait (confirmed from
`rs-sdk/src/platform/address_sync/provider.rs`):

```rust
pub trait AddressProvider: Send {
    fn gap_limit(&self) -> AddressIndex;
    fn pending_addresses(&self) -> Vec<(AddressIndex, AddressKey)>;  // AddressKey = [u8; 32]
    fn on_address_found(&mut self, index: AddressIndex, key: &[u8], funds: AddressFunds);
    fn on_address_absent(&mut self, index: AddressIndex, key: &[u8]);
    fn has_pending(&self) -> bool;
    fn highest_found_index(&self) -> Option<AddressIndex>;
    fn current_balances(&self) -> Vec<(AddressIndex, AddressKey, AddressFunds)>;
    fn last_sync_height(&self) -> u64;
}
```

**Note**: The trait uses a push-based callback API (`on_address_found`/`on_address_absent`), NOT
the `addresses()` / `apply_balance()` pattern described in earlier drafts. Implementors push
address indices into a `pending_addresses` set and handle SDK callbacks as balances arrive.

`PlatformAddressWallet` implements `AddressProvider` using `platform_payment_accounts` for
state storage. The `AddressKey` ([u8; 32]) is the DIP-17 derived P2PKH address key.

**Gap limit extension**: The gap limit extends for ANY found address (not just the highest index).
When `on_address_found` is called, the provider extends the pending set to maintain the gap limit
window beyond the newly found address.

**Balance cache**: `PlatformAddressWallet` maintains `balances: Arc<RwLock<BTreeMap<PlatformAddress, Credits>>>`
which is updated on each `on_address_found` callback. This cache is the source of truth for
`platform_credit_balance()` and `platform_address_info()` queries.

Function: `sync_address_balances(sdk: &Sdk, provider: &mut P, config, last_sync_timestamp)` at `rs-sdk`.

#### 1.6.2 — Platform Address Sync

```rust
pub async fn sync_platform_address_balances(
    &self,
    last_sync_timestamp: Option<u64>,
) -> Result<AddressSyncResult, PlatformWalletError>
```

Calls `sync_address_balances(&self.sdk, self, config, last_sync_timestamp)` where `self`
is the `AddressProvider` implementation.

#### 1.6.3 — Balance Accessors

```rust
pub fn platform_credit_balance(&self) -> u64
// Sum of platform_payment_accounts.values().credit_balance

pub fn platform_address_info(&self) -> BTreeMap<PlatformP2PKHAddress, (u64, u64)>
// (balance_credits, nonce) for each known funded address

pub fn next_platform_receive_address(
    &mut self,
    account: u32,
    key_class: u32,
) -> Result<PlatformP2PKHAddress, PlatformWalletError>
```

#### 1.6.4 — Send Credits to Platform Address (Top Up Address)

```rust
pub async fn top_up_platform_address(
    &self,
    identity_id: &Identifier,
    target_address: &PlatformP2PKHAddress,
    amount_credits: u64,
) -> Result<(), PlatformWalletError>
```

Calls `sdk::TopUpAddress` state transition, funded from the identity's balance.

#### 1.6.5 — Transfer Between Platform Addresses

```rust
pub async fn transfer_platform_address_funds(
    &self,
    from_addresses: BTreeMap<PlatformP2PKHAddress, u64>,  // address -> credits
    to_address: &PlatformP2PKHAddress,
    fee_strategy: AddressFundsFeeStrategy,
) -> Result<(), PlatformWalletError>
```

Calls `sdk::TransferAddressFunds`. Each `from_address` signed with its DIP-17 derived key.

#### 1.6.6 — Withdraw Platform Address Credits to Core

```rust
pub async fn withdraw_platform_address_funds(
    &self,
    from_addresses: BTreeMap<PlatformP2PKHAddress, u64>,
    to_core_address: Option<Address>,  // None = new wallet UTXO address
    fee_strategy: AddressFundsFeeStrategy,
    core_fee_per_byte: u32,
) -> Result<(), PlatformWalletError>
```

Calls `sdk::WithdrawAddressFunds::withdraw_address_funds()`.

#### 1.6.7 — Platform Address Signer

`Signer<PlatformAddress>` is implemented **directly on `PlatformAddressWallet`** (not a separate
struct). This gives a simpler API where `platform_wallet.platform()` can be passed as signer.

```rust
impl Signer<PlatformAddress> for PlatformAddressWallet {
    fn sign(&self, address: &PlatformAddress, data: &[u8]) -> Result<Vec<u8>> {
        // Sequential lock acquisition: acquire wallet read lock, derive key, drop lock
        // No dual-lock window — drops first lock before acquiring second
        let key = self.wallet.blocking_read()
            .derive_key_for_platform_address(address, self.network)?;
        // Sign with ECDSA P2PKH
        sign_ecdsa(key, data)
    }
}
```

**Implementation notes**:
- `Signer::sign()` is sync, wallet is behind `tokio::sync::RwLock`. Uses `blocking_read()` with
  sequential lock acquisition — drops `wallet` lock before acquiring any other lock (no deadlock window).
- Network is accessed via `sdk.network` (no cached field).
- 4 evo-tool callsites migrated: `transfer_platform_credits`, `withdraw_from_platform_address`,
  `fund_platform_address_from_asset_lock`, `top_up_identity_from_platform_addresses`.

#### 1.6.8 — Fund from Asset Lock

```rust
pub async fn fund_from_asset_lock(
    &self,
    target_address: &PlatformP2PKHAddress,
    amount_duffs: u64,
) -> Result<(), PlatformWalletError>
```

Builds an asset lock transaction targeting a platform address, broadcasts it, waits for proof,
then uses `TopUpAddress` SDK trait to credit the platform address.

#### Files

- `packages/rs-platform-wallet/src/wallet/platform_addresses/wallet.rs` — PlatformAddressWallet
- `packages/rs-platform-wallet/src/wallet/platform_addresses/provider.rs` — PlatformPaymentAddressProvider

---

### 1.7 Mempool Support

> Transaction lifecycle tracking, SPV mempool processing, bloom filter management.

**TransactionStatus** tracks the lifecycle of each Core transaction:

```rust
pub enum TransactionStatus {
    Unconfirmed,                    // broadcast but not yet confirmed
    InstantSendLocked,              // IS lock received from network
    Confirmed { height: u32 },      // included in a block
    ChainLocked { height: u32 },    // block is ChainLocked (final)
}
```

Lifecycle: `Unconfirmed → InstantSendLocked → Confirmed → ChainLocked`.
Tracked per transaction in CoreWallet. `PlatformWalletEvent::Wallet(WalletEvent)` emitted on transitions.

**SpvWalletAdapter** implements the full `WalletInterface` (from `key_wallet_manager`):

```rust
impl WalletInterface for SpvWalletAdapter {
    fn process_block(&mut self, block: &Block, height: u32) -> BlockProcessingResult;

    fn process_mempool_transaction(
        &mut self,
        tx: &Transaction,
        is_instant_send: bool,
    ) -> MempoolTransactionResult;

    fn watched_outpoints(&self) -> Vec<OutPoint>;
    // Returns outpoints the bloom filter should watch — for mempool tx matching

    fn monitor_revision(&self) -> u64;
    // Bloom filter staleness: when this changes, SPV reconstructs the bloom filter
    // Incremented when addresses or watched outpoints change

    fn process_instant_send_lock(&mut self, islock: &InstantSendLock);
    // Marks matching UTXOs as instant-send confirmed
}
```

**DashSpvClient** is parameterized with `EventHandler`:

```rust
pub struct DashSpvClient<W, N, S, H: EventHandler> { ... }

// Constructor: DashSpvClient::new(config, network, storage, wallet, Arc::new(handler))
```

**EventHandler** trait methods: `on_sync_event`, `on_network_event`, `on_progress`,
`on_wallet_event`, `on_error`. The platform-wallet impl forwards these to
`PlatformWalletEvent` variants.

**SpvRuntime** SPV lifecycle (accessed via `PlatformWalletManager::spv()`):

```rust
impl SpvRuntime {
    pub async fn start(&self, config: ClientConfig) -> Result<()>;
    // Creates DashSpvClient<SpvWalletAdapter, PeerNetworkManager, DiskStorageManager, SpvEventForwarder>

    pub async fn stop(&self) -> Result<()>;
    // Stops the client
}
```

**Bloom filter reconstruction**: Triggered when `monitor_revision()` changes. This happens
when new addresses are generated (gap limit extension, DashPay account creation) or when
watched outpoints change (new UTXOs received).

#### Files

- `packages/rs-platform-wallet/src/spv/wallet_adapter.rs` — SpvWalletAdapter (multi-wallet WalletInterface)
- `packages/rs-platform-wallet/src/spv/event_forwarder.rs` — SpvEventForwarder (EventHandler impl)
- `packages/rs-platform-wallet/src/spv/runtime.rs` — SpvRuntime (SPV lifecycle + finality)
- `packages/rs-platform-wallet/src/events.rs` — PlatformWalletEvent, SpvEvent, TransactionStatus

---

### 1.8 Token Operations

> `TokenWallet` sub-wallet with per-identity registry-based balance tracking.

#### Status: Complete (PR-8)

**Design**: Platform has no "list all tokens for an identity" query —
callers must specify which token IDs to track. `TokenWallet` uses a per-identity
registry: consumers call `watch(identity_id, token_id)` to register interest,
then `sync()` queries Platform for balances of all watched identity+token pairs.
This mirrors evo-tool's `identity_token_balances` DB table pattern.

```rust
pub struct TokenWallet {
    sdk:              Sdk,
    wallet:           Arc<RwLock<Wallet>>,
    identity_manager: Arc<RwLock<IdentityManager>>,
    watched:          Arc<RwLock<BTreeMap<Identifier, BTreeSet<Identifier>>>>,  // identity → tokens
    balances:         Arc<RwLock<BTreeMap<IdentityTokenKey, TokenAmount>>>,  // cache
}
```

**Registry** (per-identity):

```rust
wallet.tokens().watch(identity_id, token_id).await;
wallet.tokens().unwatch(&identity_id, &token_id).await;
wallet.tokens().unwatch_identity(&identity_id).await;
wallet.tokens().watched_for(&identity_id).await;  // → Vec<TokenId>
wallet.tokens().watched().await;                   // → Vec<(IdentityId, TokenId)>
```

**Sync** (queries Platform, updates cache):

```rust
wallet.tokens().sync().await?;  // fetches per identity × watched tokens
```

**Balance queries** (from cache):

```rust
wallet.tokens().balance(&identity_id, &token_id).await;       // → Option<TokenAmount>
wallet.tokens().balances_for_identity(&identity_id).await;     // → Map<TokenId, TokenAmount>
wallet.tokens().all_balances().await;                          // → Map<(IdentityId, TokenId), TokenAmount>
```

**User operations** (all take `Arc<DataContract>` + `TokenContractPosition` + identity):

```rust
wallet.tokens().transfer(contract, pos, &from_id, to_id, amount).await?;
wallet.tokens().purchase(contract, pos, &id, amount, total_price).await?;
wallet.tokens().claim(contract, pos, &id, distribution_type).await?;
```

**Admin operations**:

```rust
wallet.tokens().mint(contract, pos, &id, amount, recipient).await?;
wallet.tokens().burn(contract, pos, &id, amount).await?;
wallet.tokens().freeze(contract, pos, &id, target_id).await?;
wallet.tokens().unfreeze(contract, pos, &id, target_id).await?;
wallet.tokens().set_price(contract, pos, &id, price).await?;
```

All operations use SDK builders (`TokenTransferTransitionBuilder`, etc.) internally.
The `resolve_identity_and_signer()` helper resolves identity + HD index + signing key
from the identity manager for each operation.

**Evo-tool integration** (future PR): Replace direct SDK calls in
`backend_task/tokens/*.rs` with `platform_wallet.tokens().*` calls. The
per-identity watch registry replaces evo-tool's `identity_token_balances` DB table.

#### Files

- `packages/rs-platform-wallet/src/wallet/tokens/mod.rs`
- `packages/rs-platform-wallet/src/wallet/tokens/wallet.rs`

---

### 1.9 Shielded Pool

> Feature-gated (`shielded`) ZK-private transactions using Orchard/Halo2.
> `ShieldedWallet<S: ShieldedStore>` is generic over storage backend.

#### Design

ShieldedWallet is fundamentally different from other sub-wallets:
- Maintains **client-side state** (notes, nullifiers, commitment tree) that cannot be derived from Platform queries
- Requires a **storage backend** for persistence — abstracted via `ShieldedStore` trait
- Requires a **proving key** (~30s cold start, ~5MB memory) for ZK proof generation
- Uses **trial decryption** to discover incoming notes (scan all encrypted notes with viewing key)

Generic over storage: `ShieldedWallet<S: ShieldedStore>` — consumers provide in-memory (tests) or SQLite (production) storage.

#### ShieldedStore trait

```rust
/// Storage abstraction for shielded wallet state.
/// Consumers implement this for their persistence layer.
pub trait ShieldedStore: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    // --- Notes ---
    fn save_note(&mut self, note: &ShieldedNote) -> Result<(), Self::Error>;
    fn get_unspent_notes(&self) -> Result<Vec<ShieldedNote>, Self::Error>;
    fn get_all_notes(&self) -> Result<Vec<ShieldedNote>, Self::Error>;
    fn mark_spent(&mut self, nullifier: &[u8; 32]) -> Result<bool, Self::Error>;

    // --- Commitment tree ---
    fn append_commitment(&mut self, cmx: &[u8; 32], retention: Retention) -> Result<(), Self::Error>;
    fn checkpoint_tree(&mut self, checkpoint_id: u32) -> Result<(), Self::Error>;
    fn witness(&self, position: u64) -> Result<MerklePath, Self::Error>;
    fn tree_anchor(&self) -> Result<[u8; 32], Self::Error>;

    // --- Sync state ---
    fn last_synced_note_index(&self) -> Result<u64, Self::Error>;
    fn set_last_synced_note_index(&mut self, index: u64) -> Result<(), Self::Error>;
    fn nullifier_checkpoint(&self) -> Result<Option<NullifierSyncCheckpoint>, Self::Error>;
    fn set_nullifier_checkpoint(&mut self, checkpoint: NullifierSyncCheckpoint) -> Result<(), Self::Error>;
}
```

Built-in implementations:
- `InMemoryShieldedStore` — for tests and short-lived wallets (Vec + BTreeMap + in-memory tree)
- No SQLite in the library — evo-tool implements `ShieldedStore` using its existing `database/shielded.rs`

#### ShieldedNote

```rust
pub struct ShieldedNote {
    pub note: orchard::Note,          // Orchard note (value, rseed, rho)
    pub position: u64,                // Global position in commitment tree
    pub cmx: [u8; 32],               // Note commitment
    pub nullifier: [u8; 32],          // For detecting when spent
    pub block_height: u64,            // Where it appeared
    pub is_spent: bool,               // Nullifier was seen in global set
    pub value: u64,                   // Credits (convenience, same as note.value())
}
```

#### OrchardKeySet

```rust
/// ZIP-32 derived Orchard key hierarchy.
/// Derivation path: m/32'/coin_type'/account' (coin_type: 5=Mainnet, 1=Testnet)
pub struct OrchardKeySet {
    pub spending_key: SpendingKey,
    pub full_viewing_key: FullViewingKey,
    pub spend_auth_key: SpendAuthorizingKey,
    pub incoming_viewing_key: IncomingViewingKey,
    pub outgoing_viewing_key: OutgoingViewingKey,
    pub default_address: PaymentAddress,
}

impl OrchardKeySet {
    /// Derive from wallet seed bytes using ZIP-32.
    pub fn from_seed(seed: &[u8], network: Network, account: u32) -> Result<Self, Error>;

    /// Derive payment address at index.
    pub fn address_at(&self, index: u32) -> PaymentAddress;

    /// Prepare incoming viewing key for efficient trial decryption.
    pub fn prepared_ivk(&self) -> PreparedIncomingViewingKey;
}
```

#### ShieldedWallet<S>

```rust
pub struct ShieldedWallet<S: ShieldedStore> {
    sdk: Sdk,
    keys: OrchardKeySet,
    store: Arc<RwLock<S>>,
    network: Network,
}
```

**Construction:**
```rust
impl<S: ShieldedStore> ShieldedWallet<S> {
    pub fn new(sdk: Sdk, keys: OrchardKeySet, store: S, network: Network) -> Self;

    /// Derive keys from wallet seed and create shielded wallet.
    pub fn from_seed(sdk: Sdk, seed: &[u8], network: Network, account: u32, store: S) -> Result<Self, Error>;
}
```

**Sync operations:**
```rust
impl<S: ShieldedStore> ShieldedWallet<S> {
    /// Sync notes from Platform — trial decrypts all new encrypted notes.
    /// Appends all notes to commitment tree (for witness generation).
    /// Stores decrypted notes that belong to us.
    /// Returns count of new notes found.
    pub async fn sync_notes(&self) -> Result<SyncNotesResult, PlatformWalletError>;

    /// Check which owned notes have been spent (nullifier sync).
    /// Privacy-preserving: uses trunk/branch tree scan.
    /// Marks spent notes in store.
    /// Returns count of newly spent notes.
    pub async fn check_nullifiers(&self) -> Result<usize, PlatformWalletError>;

    /// Full sync: notes + nullifiers + balance update.
    pub async fn sync(&self) -> Result<ShieldedSyncSummary, PlatformWalletError>;
}

pub struct SyncNotesResult {
    pub new_notes: usize,
    pub total_scanned: u64,
}

pub struct ShieldedSyncSummary {
    pub notes_result: SyncNotesResult,
    pub newly_spent: usize,
    pub balance: u64,
}
```

**Balance queries:**
```rust
impl<S: ShieldedStore> ShieldedWallet<S> {
    /// Total unspent shielded balance.
    pub async fn balance(&self) -> Result<u64, PlatformWalletError>;

    /// Default payment address for receiving shielded funds.
    pub fn default_address(&self) -> &PaymentAddress;

    /// Derive address at specific index.
    pub fn address_at(&self, index: u32) -> PaymentAddress;
}
```

**Operations (5 transition types):**

Each operation:
1. Selects spendable notes (if spending)
2. Generates Merkle witness paths from commitment tree
3. Builds Orchard bundle via DPP `build_*_transition()` builders
4. Broadcasts via SDK traits (`ShieldFunds`, `UnshieldFunds`, `TransferShielded`, `WithdrawShielded`, `ShieldFromAssetLock`)
5. Marks spent notes in store

```rust
impl<S: ShieldedStore> ShieldedWallet<S> {
    /// Shield: platform addresses -> shielded pool.
    /// Uses Signer<PlatformAddress> for input authorization.
    pub async fn shield<Signer: dpp::identity::signer::Signer<PlatformAddress>>(
        &self,
        inputs: BTreeMap<PlatformAddress, Credits>,
        amount: u64,
        signer: &Signer,
    ) -> Result<(), PlatformWalletError>;

    /// Shield from asset lock: Core L1 -> shielded pool.
    pub async fn shield_from_asset_lock(
        &self,
        asset_lock_proof: AssetLockProof,
        private_key: &[u8],
        amount: u64,
    ) -> Result<(), PlatformWalletError>;

    /// Unshield: shielded pool -> platform address.
    pub async fn unshield(
        &self,
        to_address: &PlatformAddress,
        amount: u64,
    ) -> Result<(), PlatformWalletError>;

    /// Transfer: shielded pool -> shielded pool (private).
    pub async fn transfer(
        &self,
        to_address: &PaymentAddress,
        amount: u64,
    ) -> Result<(), PlatformWalletError>;

    /// Withdraw: shielded pool -> Core L1 address.
    pub async fn withdraw(
        &self,
        to_address: &Address,
        amount: u64,
        core_fee_per_byte: u32,
    ) -> Result<(), PlatformWalletError>;
}
```

**Proving key management:**
```rust
/// Cached proving key — built once (~30s), reused for all proofs.
/// Use `warm_up()` at app startup to avoid blocking first operation.
pub struct CachedOrchardProver {
    key: OnceLock<ProvingKey>,
}

impl CachedOrchardProver {
    pub fn new() -> Self;
    pub fn warm_up(&self);  // Build key in background
    pub fn is_ready(&self) -> bool;
}

impl OrchardProver for CachedOrchardProver {
    fn proving_key(&self) -> &ProvingKey { self.key.get_or_init(ProvingKey::build) }
}
```

The `CachedOrchardProver` is held as a static or on `PlatformWalletManager`. All `ShieldedWallet` instances share it.

**Note selection for spending:**
```rust
/// Select notes to cover the requested amount + fee.
/// Returns selected notes with Merkle witness paths from commitment tree.
fn select_spendable_notes(
    store: &S,
    amount: u64,
    fee: u64,
) -> Result<Vec<SpendableNote>, PlatformWalletError>;
```

Greedy selection: sort unspent notes by value descending, accumulate until >= amount + fee.

#### Integration with PlatformWallet

`ShieldedWallet` is a **standalone component** — not a field on `PlatformWallet`. This avoids
infecting `PlatformWallet` with the `S: ShieldedStore` type parameter. Consumers create
`ShieldedWallet` separately, providing their own `ShieldedStore` implementation:

```rust
// Consumer creates ShieldedWallet separately
let shielded = ShieldedWallet::from_seed(
    sdk, &seed_bytes, network, 0, InMemoryShieldedStore::new()
)?;
shielded.sync().await?;
shielded.shield(inputs, amount, &platform_signer).await?;
```

`ShieldedWallet` shares the `Sdk` with `PlatformWallet` but manages its own state through
the `ShieldedStore` backend.

#### Files

- `packages/rs-platform-wallet/src/wallet/shielded/mod.rs` — ShieldedWallet, re-exports
- `packages/rs-platform-wallet/src/wallet/shielded/keys.rs` — OrchardKeySet, ZIP-32 derivation
- `packages/rs-platform-wallet/src/wallet/shielded/store.rs` — ShieldedStore trait, ShieldedNote, InMemoryShieldedStore
- `packages/rs-platform-wallet/src/wallet/shielded/sync.rs` — sync_notes, check_nullifiers, sync
- `packages/rs-platform-wallet/src/wallet/shielded/operations.rs` — shield, unshield, transfer, withdraw, shield_from_asset_lock
- `packages/rs-platform-wallet/src/wallet/shielded/prover.rs` — CachedOrchardProver
- `packages/rs-platform-wallet/src/wallet/shielded/note_selection.rs` — select_spendable_notes

---

### 1.10 State Transition Signing Facade

> `PlatformWallet` provides `IdentitySigner` so callers never manage key material directly.

```rust
// platform_wallet/signer.rs
pub struct IdentitySigner {
    wallet:         Arc<RwLock<Wallet>>,
    identity_index: u32,  // required (u32, not Optional)
}

impl Signer<IdentityPublicKey> for IdentitySigner {
    fn sign(&self, key: &IdentityPublicKey, data: &[u8]) -> Result<Vec<u8>> {
        // Derive private key using 3-component DIP-9 path:
        // m/9'/coin'/5'/0'/key_type'/identity_index'/key_index'
        // where key_type: 0' = ECDSA, 1' = BLS
        let secret = Zeroizing::new(
            self.wallet.blocking_read()
                .derive_identity_key(self.identity_index, key.id(), key.key_type())?
        );
        match key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => sign_ecdsa(&secret, data),
            KeyType::BLS12_381 => sign_bls(&secret, data),
            KeyType::EDDSA_25519_HASH160 => sign_eddsa(&secret, data),
        }
    }
}
```

**Private key zeroization**: All derived key material uses `Zeroizing<[u8; 32]>`. Keys are
zeroed on drop — no plaintext key material persists in memory after signing.

Factory on `IdentityWallet` — no external `wallet` param, borrows from `self.wallet`:

```rust
pub fn signer_for_identity(
    &self,
    identity_id: &Identifier,
) -> Result<IdentitySigner, PlatformWalletError>
// Looks up identity_index from ManagedIdentity (u32, required)
```

**`PlatformAddressWallet` as `Signer<PlatformAddress>`**: See §1.6.7. Uses sequential lock
acquisition with `blocking_read()` — no dual-lock window.

**Important**: `WithdrawFromIdentity::withdraw` and `TransferToIdentity::transfer_credits` take
the signer **by value** (not by reference). Callers must construct a new `IdentitySigner` for
each call, or the signer must implement `Clone`.

**Implementation notes**:
- Derives keys at `m/9'/coin_type'/5'/0'/key_type'/identity_index'/key_index'` (3-component DIP-9 path)
- Signs based on key type: ECDSA (`secp256k1`), BLS (`bls-signatures`), EdDSA (`ed25519-dalek`)
- Sync/async bridge: `blocking_read()` — safe because SDK calls `sign()` from blocking context
- Replaces evo-tool's `QualifiedIdentity::sign()` long-term

#### Files

- `packages/rs-platform-wallet/src/wallet/signer.rs` (extend existing stub)

---

### 1.11 Serialization / Persistence

> `PlatformWallet` is the single persistence unit — callers (e.g. evo-tool's SQLite) store
> the blob and don't need to know about sub-struct layout.

```rust
// Top-level backup/restore — covers Wallet + ManagedWalletInfo + IdentityManager + DashPay state
pub fn backup(&self) -> Result<Vec<u8>, PlatformWalletError>
pub fn restore(data: &[u8]) -> Result<Self, PlatformWalletError>
```

`Sdk` is excluded from the blob (it's a live connection) — caller re-provides it via
`PlatformWallet::from_bytes(sdk, blob)`.

`ManagedWalletInfo` and `ManagedAccountCollection` already have `#[cfg(feature="bincode")]`
encode/decode. `ManagedPlatformAccount` and `PlatformP2PKHAddress` already have bincode.
Still missing serialization:

- `IdentityManager` — add bincode `Encode`/`Decode` (with `Arc<RwLock<_>>` wrapping, serialize inner values)
- `ManagedIdentity` (Identity + BlockTime + contact maps) — add bincode
- `ContactRequest` — add bincode
- `EstablishedContact` — add bincode

#### Files

- `packages/rs-platform-wallet/src/wallet/identity/serialization.rs` (new)
- `packages/rs-platform-wallet/src/wallet/identity/managed_identity/serialization.rs` (new)
- `packages/rs-platform-wallet/src/wallet/dashpay/contact_request.rs` (extend)
- `packages/rs-platform-wallet/src/wallet/dashpay/established_contact.rs` (extend)

---

### 1.12 Sync Architecture

There are **three distinct sync mechanisms** with different lifecycles:

#### Core chain sync — push-based, long-running

`dash-spv` runs as a permanent background task started once at app startup. It pushes
blocks and transactions to `CoreWallet` via `WalletInterface` callbacks — no polling needed:

```rust
// App startup — spawned once, runs until cancellation
tokio::spawn(async move {
    spv_client.run(cancellation_token).await
});
// dash-spv calls SpvWalletAdapter::process_block() reactively as blocks arrive
```

#### Mempool reconciliation — push-based, event-driven

SPV also delivers mempool transactions via `process_mempool_transaction(tx, is_instant_send)`.
The `TransactionStatus` lifecycle tracks each transaction:

```
Unconfirmed → InstantSendLocked → Confirmed { height } → ChainLocked { height }
```

- `process_mempool_transaction` is called when SPV receives an unconfirmed tx matching watched addresses
- `process_instant_send_lock` upgrades status from `Unconfirmed` to `InstantSendLocked`
- `process_block` upgrades to `Confirmed` when the tx appears in a block
- ChainLock events upgrade to `ChainLocked`

`PlatformWalletEvent::Wallet(WalletEvent)` is emitted on each status transition.

**Bloom filter staleness**: `monitor_revision()` is incremented when addresses or watched outpoints
change. SPV detects the change and reconstructs the bloom filter to include the new addresses.

#### Platform sync — poll-based, periodic

Platform state (identities, contacts, credit balances) is fetched via DAPI on a timer.
`PlatformWallet::sync()` is the single entry point:

```rust
pub async fn sync(&self) -> Result<SyncResult, PlatformWalletError>
```

Sync order:

1. `self.identity.sync()` — DIP-9 gap scan for new identities
2. `self.dashpay.sync()` — contact requests for all known identities
3. `self.platform.sync()` — DIP-17 address credit balances via DAPI
4. `self.shielded.sync()` (if feature enabled) — note sync + nullifier sync + tree updates

**Shielded note sync** (feature-gated): Trial decryption of Orchard output notes using the
`FullViewingKey`. Discovered notes stored in `NoteStore`. Nullifier sync detects spent notes.
Commitment tree updated with each batch of processed notes.

Designed to run on a timer in the app's background loop:

```rust
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        if let Err(e) = wallet.sync().await {
            tracing::warn!("Platform sync failed: {}", e);
        }
    }
});
```

Sub-struct `sync()` methods remain individually callable for fine-grained control.
`PlatformWallet` is `Send + Sync` — safe to share across threads via `Arc`.

---

## PR Sequence

Each PR implements features in `rs-platform-wallet` **and** immediately integrates into `evo-tool`.
Old evo-tool code is deleted in the same PR that introduces the replacement.

---

### PR-1: Project Scaffold + PlatformWallet + PlatformWalletManager + CoreWallet

**Library** (`rs-platform-wallet`):

- Clean up `lib.rs`: replace `pub mod platform_wallet_info` with `pub mod platform_wallet`
- `PlatformWallet` struct with stored sub-wallets sharing `Arc<RwLock<ManagedWalletInfo>>` (§Struct Definitions)
- `PlatformWallet` creation methods mirroring `key-wallet`'s `Wallet` constructors + `sdk` param (§1.1)
- `CoreWallet` with `Arc<RwLock<ManagedWalletInfo>>`, balance, UTXOs, address generation (§1.3)
- `PlatformWalletManager`: multi-wallet coordinator, `RwLock<BTreeMap>` for wallet add/remove
- `SpvWalletAdapter` implements `WalletInterface` using `key-wallet` types (`TransactionRouter`, `WalletTransactionChecker`) — no `WalletManager<T>` dependency (§1.3.5, §1.7)
- `PlatformWalletEvent` unified enum: `Wallet(WalletEvent)`, `Spv(SpvEvent)` (two variants only)
- `monitored_addresses()` returns ALL account types including `dashpay_receival_accounts`
- `send_transaction`, `broadcast_transaction`, asset lock proof creation (§1.3.4–1.3.6)
- Asset lock timeout/fallback: 60s InstantLock wait, then ChainLock polling
- `IdentitySigner` stub (§1.10) — needed for identity registration in PR-2
- `static_assertions::assert_impl_all!(PlatformWallet: Send, Sync)`
- `IdentityManager` refactor: add `last_scanned_index`, remove `sdk` field

**evo-tool integration**:

- Add `platform-wallet = { path = "../../platform/packages/rs-platform-wallet" }` to `Cargo.toml`
- Replace `AppContext.wallets` + `SpvManager` with `PlatformWalletManager`
- `wallet_lifecycle.rs`: construct via `PlatformWallet::from_mnemonic()` / `from_xprv()`, wire `sdk` from `AppContext.sdk`
- SPV: `SpvRuntime::start()` (via `PlatformWalletManager::spv()`) replaces manual `SpvManager` setup
- `PlatformWallet.clone()` replaces `WalletSeedHash` as wallet accessor (no WalletHandle)
- Delete `src/model/wallet/` (old custom wallet struct)

**Database migration** (in this PR):

- Add version byte to DB wallet record
- If old format: deserialize as old `Wallet`, convert to `PlatformWallet`, re-save
- On first run after migration: `IdentityManager` starts empty — identities re-discovered in PR-2

**Done when**: evo-tool builds with `PlatformWalletManager`; SPV sync works via `WalletInterface` impl; `send_transaction` works; PlatformWallet clone provides sync access to sub-wallets.

**PR-1 status**: ✅ Complete. Scaffold in place, bridge working, 7 backend tasks validating via bridge.

---

### PR-2: CoreWallet Deep Integration

**Library** (`rs-platform-wallet`):

- Per-address data methods on CoreWallet (§1.3.3): `all_address_info()`, `account_summaries()`, `utxos_by_address()`, `derivation_path_for_address()`
- `CoreAddressInfo` and `CoreAccountSummary` structs
- `Signer<PlatformAddress>` on `PlatformAddressWallet` (§1.6) with `blocking_read()` bridge
- Asset lock proof creation on CoreWallet (§1.3.6): `create_asset_lock_proof()`, `create_topup_asset_lock_proof()`
- Asset lock recovery (§1.3.7): `recover_asset_locks()`
- Transaction sending: `send_transaction()` on CoreWallet (§1.3.4)
- `PlatformAddressWallet` uses `sdk.network` for network access

**evo-tool integration**:

- Migrate 4 signing callsites from old `Wallet` to `platform_wallet.platform()` as `Signer<PlatformAddress>`
- Migrate `generate_receive_address` from diagnostic to primary path
- Add `WalletTask::LoadAddressTable` backend task using `CoreWallet::all_address_info()`
- Update address table UI to render from cached `CoreAddressInfo` snapshot
- Migrate `create_asset_lock` tasks to use `CoreWallet::create_asset_lock_proof()`

**Done when**: All backend tasks that touch balance/UTXOs/addresses use CoreWallet; signing uses PlatformAddressWallet; asset lock creation works through platform-wallet.

---

### PR-3: IdentityWallet

**Library** (`rs-platform-wallet`):

- `IdentityWallet` with `identity_manager`, sdk, wallet Arc (§1.4)
- `register_identity` (with corrected `m/9'/coin'/5'/0'/key_type'/identity_index'/key_index'` path), `sync()`, `refresh_identity` (§1.4.1–1.4.3)
- Identity discovery: gap limit 5, consider AUTH_KEY_LOOKUP_WINDOW = 12 for key index scanning
- `top_up_identity`, `withdraw_identity_credits`, `transfer_credits` (§1.4.4–1.4.6)
- `add_key_to_identity`, `disable_identity_key` (§1.4.7)
- `IdentitySigner` complete (§1.10)
- `IdentityManager` bincode serialization (§1.11 partial)
- DPNS name registration (§1.5.10, belongs to IdentityWallet for SDK access)

**evo-tool integration**:

| File | Action |
|------|--------|
| `backend_task/identity/discover_identities.rs` | → `wallet.identity.sync()` |
| `backend_task/identity/register_identity.rs` | → `wallet.identity.register_identity()` |
| `backend_task/identity/top_up_identity.rs` | → `wallet.identity.top_up_identity()` |
| `backend_task/identity/withdraw_from_identity.rs` | → `wallet.identity.withdraw_identity_credits()` |
| `backend_task/identity/transfer.rs` | → `wallet.identity.transfer_credits()` |
| `backend_task/identity/add_key_to_identity.rs` | → `wallet.identity.add_key_to_identity()` |

All signing replaced with `wallet.identity.signer_for_identity(identity_id)`.

**Done when**: Identity registration and discovery work in evo-tool via library; old identity task files deleted.

---

### PR-4: DashPayWallet (DIP-14 + DIP-15 + Sync)

**Library** (`rs-platform-wallet`):

- DIP-14: `ckd_priv_256`, `ckd_pub_256`, `derive_dashpay_contact_xpub` in `dashpay/dip14.rs` (§1.5.1)
  - Big-endian `ser_256(i)` — verify and test before relying on it
- DIP-15: Reuse `rs-platform-encryption` — do NOT duplicate functions (§1.5.2)
- Fix the AES decryption bug: `decrypt_extended_public_key` before `ExtendedPubKey::decode`
- Fix recipient key purpose: use `Purpose::DECRYPTION`, not `ENCRYPTION`
- `DashPayWallet` with `send_contact_request`, `decrypt_incoming_contact_request` (§1.5.3–1.5.4)
- `derive_payment_address_for_contact` (gap limit: 10), `send_dashpay_payment` (§1.5.5–1.5.6)
- `DashPayWallet::sync()` using `sdk.fetch_all_contact_requests_for_identity()` (§1.5.7)
- Profile, contact info, auto-accept proof (§1.5.8–1.5.11)
- `ManagedIdentity` contact maps + `ContactRequest` + `EstablishedContact` bincode (§1.11)

Test against DIP-14 Appendix A test vectors before merging.
Note: `contactRequest` documents are immutable — do not expose update/delete operations.

**evo-tool integration**:

| File | Action |
|------|--------|
| `backend_task/dashpay/dip14_derivation.rs` | Delete (replaced by `platform_wallet/dashpay/dip14.rs`) |
| `backend_task/dashpay/hd_derivation.rs` | Delete |
| `backend_task/dashpay/encryption.rs` | Delete (was duplicating `rs-platform-encryption`) |
| `backend_task/dashpay/contact_requests.rs` | → `wallet.dashpay.send_contact_request()` |
| `backend_task/dashpay/contacts.rs` | → `wallet.dashpay.sync()` |
| `backend_task/dashpay/payments.rs` | → `wallet.dashpay.send_dashpay_payment()` |
| `backend_task/dashpay/incoming_payments.rs` | → `wallet.dashpay.sync()` handles this |
| `backend_task/dashpay/profile.rs` | → `wallet.dashpay.create_dashpay_profile()` |
| `backend_task/dashpay/auto_accept_proof.rs` | → `wallet.dashpay.generate_auto_accept_proof()` |
| `backend_task/dashpay/contact_info.rs` | → `wallet.dashpay.update_contact_info()` |

**Done when**: DIP-14 vectors pass; contact requests sent/received and decrypted correctly (including AES decryption fix); incoming DashPay payments detected via SPV without manual address registration.

---

### PR-5: PlatformAddressWallet (DIP-17)

**Library** (`rs-platform-wallet`):

- `PlatformAddressWallet` with actual `AddressProvider` impl — push-based callbacks (`pending_addresses`, `on_address_found`, `on_address_absent`) (§1.6.1)
- `sync_platform_address_balances`, balance accessors (§1.6.2–1.6.3)
- `top_up_platform_address`, `transfer_platform_address_funds`, `withdraw_platform_address_funds` (§1.6.4–1.6.6)
- `Signer<PlatformAddress>` on `PlatformAddressWallet` directly (§1.6.7)

**evo-tool integration**:

- `backend_task/wallet/fetch_platform_address_balances.rs`: replace `WalletAddressProvider::new(&wallet, ...)` with `wallet.platform` as `AddressProvider`
- Replace `wallet.platform_address_info` field access with `wallet.platform.platform_address_info()`

**Done when**: DIP-17 address balance sync works; top-up, transfer, and withdrawal work in evo-tool.

---

### PR-7 Status: Complete

### What was delivered

- `IdentityWallet::update_identity(add_keys, disable_keys)` — `IdentityUpdateTransition` via DPP
  (nonce lookup, master key signing, broadcast_and_wait)
- `IdentityWallet::top_up_from_addresses()` — `TopUpIdentityFromAddresses` SDK trait
- `IdentityWallet::transfer_credits_to_addresses()` — `TransferToAddresses` SDK trait
- `IdentityWallet::register_name()` — DPNS username registration via `Sdk::register_dpns_name`
- `IdentityWallet::resolve_name()` — DPNS resolution via `Sdk::resolve_dpns_name`
- `IdentityWallet::search_names()` — DPNS prefix search via `Sdk::search_dpns_names`
- `PlatformAddressWallet::fund_from_asset_lock()` — `TopUpAddress` SDK trait

All identity fund flows now work: L1→identity, address→identity, identity→address.
Identity keys can be added/disabled. DPNS names can be registered, resolved, and searched.

---

### PR-8 Status: Complete

### What was delivered

**TokenWallet** — per-identity registry-based token balance tracking and operations:

- **Registry**: `watch(identity_id, token_id)` / `unwatch()` / `unwatch_identity()` / `watched_for()` / `watched()`
  — per-identity token watch list (mirrors evo-tool's `identity_token_balances` DB pattern)
- **Sync**: `sync()` queries Platform via `FetchMany<IdentityTokenBalancesQuery>` for each
  identity's watched tokens, updates local `BTreeMap<(IdentityId, TokenId), TokenAmount>` cache
- **Balance queries**: `balance()`, `balances_for_identity()`, `all_balances()` — read from cache
- **User operations**: `transfer()`, `purchase()`, `claim()` — SDK builders + broadcast
- **Admin operations**: `mint()`, `burn()`, `freeze()`, `unfreeze()`, `set_price()` — SDK builders + broadcast
- All operations take `Arc<DataContract>` + `TokenContractPosition` to identify the token
  (wallet doesn't store contract metadata, only balances)
- Shared `resolve_identity_and_signer()` helper for all token operations

**Evo-tool integration** (future PR): Replace direct SDK calls in `backend_task/tokens/*.rs`
with `platform_wallet.tokens().*`. The per-identity watch registry replaces the
`identity_token_balances` SQLite table.

---

### PR-9: Evo-tool integration

Replace ALL evo-tool backend tasks with platform-wallet calls across every domain:
tokens, identity, dashpay, core wallet, platform addresses. Evo-tool keeps its own
`SpvManager` — SPV migration is PR-11.

**Migration by domain** (in `dash-evo-tool/src/backend_task/`):

**Phase 1 — Tokens** (~17 tasks, all trivial SDK wrappers):
| Evo-tool task | Replaced by |
|---------------|-------------|
| `tokens/transfer_tokens.rs` | `wallet.tokens().transfer()` |
| `tokens/mint_tokens.rs` | `wallet.tokens().mint()` |
| `tokens/burn_tokens.rs` | `wallet.tokens().burn()` |
| `tokens/freeze_tokens.rs` | `wallet.tokens().freeze()` |
| `tokens/unfreeze_tokens.rs` | `wallet.tokens().unfreeze()` |
| `tokens/claim_tokens.rs` | `wallet.tokens().claim()` |
| `tokens/purchase_tokens.rs` | `wallet.tokens().purchase()` |
| `tokens/set_token_price.rs` | `wallet.tokens().set_price()` |
| `tokens/query_my_token_balances.rs` | `wallet.tokens().sync()` + `.balance()` |

**Phase 2 — Simple identity + DPNS** (trivial wrappers):
| Evo-tool task | Replaced by |
|---------------|-------------|
| `identity/withdraw_from_identity.rs` | `wallet.identity().withdraw_credits()` |
| `identity/transfer.rs` | `wallet.identity().transfer_credits()` |
| `identity/refresh_identity.rs` | `wallet.identity().sync()` |
| `identity/add_key_to_identity.rs` | `wallet.identity().update_identity()` |
| `identity/register_dpns_name.rs` | `wallet.identity().register_name()` |
| `identity/load_identity_by_dpns_name.rs` | `wallet.identity().resolve_name()` |

**Phase 3 — Identity registration + top-up + discovery** (asset lock handling):
| Evo-tool task | Replaced by |
|---------------|-------------|
| `identity/register_identity.rs` | `wallet.identity().register_identity()` (uses `wallet.core()` for asset locks) |
| `identity/top_up_identity.rs` | `wallet.identity().top_up_identity()` |
| `identity/discover_identities.rs` | `wallet.identity().sync()` |
| `identity/load_identity.rs` | Adapter: fetch via SDK + register in `identity_manager` |
| `identity/load_identity_from_wallet.rs` | Adapter: HD derivation + `wallet.identity().sync()` |

**Phase 4 — DashPay contacts** (encryption via `rs-platform-encryption`):
| Evo-tool task | Replaced by |
|---------------|-------------|
| `dashpay/contact_requests.rs` (send) | `wallet.dashpay().send_contact_request()` |
| `dashpay/contact_requests.rs` (accept) | `wallet.dashpay().accept_contact_request()` |
| `dashpay/contact_requests.rs` (load) | `wallet.dashpay().sync_contact_requests()` |
| `dashpay/contacts.rs` | `wallet.dashpay().established_contacts()` |

**Phase 5 — Core wallet + platform addresses**:
| Evo-tool task | Replaced by |
|---------------|-------------|
| `core/create_asset_lock.rs` | `wallet.core().build_registration_asset_lock_transaction()` + `.broadcast_transaction()` |
| `core/refresh_wallet_info.rs` | SPV feeds `ManagedWalletInfo` directly (no change needed) |
| Platform address transfer | `wallet.platform().transfer()` |
| Platform address withdraw | `wallet.platform().withdraw()` |
| Platform address fund | `wallet.platform().fund_from_asset_lock()` |
| Signing callsites (4+) | `wallet.platform()` as `Signer<PlatformAddress>` |

**Bridge architecture** (in `dash-evo-tool/src/context/`):
- `platform_wallet_bridge.rs` exists from PR-1 on `feat/platform-wallet` branch
- Extend bridge: `register_with_platform_wallet_manager()` for all wallet types
- Backend tasks call `require_platform_wallet()` → delegate to platform-wallet
- Evo-tool DB persistence remains — platform-wallet results are persisted by evo-tool after each operation

**What stays in evo-tool**:
- `SpvManager` — keeps its own `DashSpvClient`, `ConnectionStatus`, debounced reconciliation (→ PR-11)
- Database layer — SQLite persistence for wallet state, identities, tokens, contacts
- UI screens — presentation unchanged, backend calls change
- `QualifiedIdentity` model — adapter maps to/from platform-wallet's `IdentityManager`

**What gets deleted**:
- Direct SDK calls in backend tasks (replaced by `wallet.*()` calls)
- Duplicate crypto code (`dashpay/encryption.rs`, `dashpay/dip14_derivation.rs`) → use `rs-platform-encryption`
- Duplicate wallet model code in `src/model/wallet/` (partially — full deletion in PR-15)

**Done when**: All backend tasks delegate to platform-wallet. No direct SDK identity/token/address/dashpay
calls remain in evo-tool backend tasks. SPV and database stay.

**Done when**: All backend tasks delegate to platform-wallet. No direct SDK identity/token/address
calls remain in evo-tool (except SPV and database). Duplicate wallet code deleted.

---

### PR-10: Enrich ManagedIdentity

**Goal**: Make `ManagedIdentity` rich enough to replace evo-tool's `QualifiedIdentity` for
wallet-based identities. Any app using platform-wallet should get full identity management
without reimplementing key storage, status tracking, or discovery.

**1. KeyStorage with lazy wallet derivation**

Replace flat private key storage with a `PrivateKeyData` enum:

```rust
pub enum PrivateKeyData {
    /// Raw key bytes in memory.
    Clear(Zeroizing<[u8; 32]>),
    /// Derive on-demand from wallet at this path (key not held in memory).
    AtWalletDerivationPath {
        wallet_seed_hash: [u8; 32],
        derivation_path: DerivationPath,
    },
}
```

`ManagedIdentity` gets a `KeyStorage` map: `BTreeMap<KeyID, (IdentityPublicKey, PrivateKeyData)>`.

When signing, if the key is `AtWalletDerivationPath`, the signer resolves it by finding the
wallet by seed hash, acquiring a read lock, and deriving at the path. This avoids storing
private keys in memory for wallet-backed identities.

**2. IdentityStatus state machine**

```rust
pub enum IdentityStatus {
    Unknown,            // Not yet checked against Platform
    PendingCreation,    // Registration submitted, awaiting confirmation
    Active,             // Confirmed on Platform
    FailedCreation,     // Registration failed (can retry)
    NotFound,           // Was active but no longer on Platform
}
```

Status transitions: `Unknown → PendingCreation → Active` (happy path),
`PendingCreation → FailedCreation → Active` (retry), `Active → NotFound → Active` (reappears).

**3. DPNS name association**

```rust
pub struct DpnsNameInfo {
    pub label: String,          // e.g., "alice"
    pub acquired_at: Option<u64>,  // timestamp
}
```

Add `dpns_names: Vec<DpnsNameInfo>` to `ManagedIdentity`. Populated during `sync()` by querying
DPNS contract for documents with `records.identity == identity_id`.

**4. Enhanced identity discovery**

Current `sync()` only checks key_index 0 (primary auth key). Enhance to:
- Scan key indices 0..12 per identity index (12-key lookup window)
- Support ECDSA_HASH160 matching (not just full pubkey)
- Fetch DPNS names for discovered identities
- Store matched derivation paths in `KeyStorage` as `AtWalletDerivationPath`

**5. Wallet association**

Add `wallet_seed_hash: Option<[u8; 32]>` and `wallet_index: Option<u32>` to `ManagedIdentity`.
These link an identity back to the wallet it was registered from, enabling key re-derivation
on wallet recovery.

**Files to modify:**
- `src/wallet/identity/managed_identity/mod.rs` — KeyStorage, IdentityStatus, DpnsNameInfo, wallet fields
- `src/wallet/identity/wallet.rs` — enhanced `sync()` with multi-key window + DPNS
- `src/wallet/signer.rs` — support `AtWalletDerivationPath` resolution

**Done when**: `ManagedIdentity` has rich key storage, status tracking, DPNS names, and wallet
association. Discovery finds identities with any registered key, not just the primary.

---

### PR-11: Asset lock lifecycle + multi-mode funding

**Goal**: Handle the full asset lock lifecycle and support all identity funding modes.
Any app should be able to register/top-up identities without reimplementing IS→CL fallback
or UTXO management.

**1. Asset lock tracking**

```rust
pub struct TrackedAssetLock {
    pub transaction: Transaction,
    pub output_address: Address,
    pub amount_duffs: u64,
    pub proof: Option<AssetLockProof>,       // None until IS/CL arrives
    pub identity_id: Option<Identifier>,     // None until used for registration
    pub status: AssetLockStatus,
}

pub enum AssetLockStatus {
    Broadcast,           // TX sent, waiting for proof
    InstantLocked,       // IS proof received
    ChainLocked,         // CL proof received (higher finality)
    UsedForRegistration, // Linked to an identity
    UsedForTopUp,        // Linked to an identity top-up
}
```

Add `tracked_asset_locks: Arc<RwLock<Vec<TrackedAssetLock>>>` to `CoreWallet`.
Methods: `unused_asset_locks()`, `track_asset_lock()`, `mark_used()`.

**2. IS→CL fallback**

When Platform rejects an InstantSend proof (`AssetLockInstantLockProofInvalid`):
1. Query DAPI for the TX to check `is_chain_locked` and `height`
2. If chain-locked and Platform has verified that height → retry with `ChainAssetLockProof`
3. If not chain-locked → return `AssetLockExpired` error

This logic lives in a shared `resolve_asset_lock_proof()` method used by both
registration and top-up.

**3. Multi-mode identity registration**

```rust
pub enum IdentityFundingMethod {
    /// Use a pre-existing asset lock proof.
    UseAssetLock {
        proof: AssetLockProof,
        private_key: PrivateKey,
    },
    /// Build asset lock from wallet UTXOs.
    FundWithWallet {
        amount_duffs: u64,
    },
    /// Use a specific UTXO.
    FundWithUtxo {
        outpoint: OutPoint,
        txout: TxOut,
        address: Address,
    },
    /// Fund from platform addresses (no asset lock needed).
    FundFromAddresses {
        inputs: BTreeMap<PlatformAddress, Credits>,
    },
}
```

`IdentityWallet::register_identity()` updated to accept `IdentityFundingMethod`.
The `FundWithWallet` path builds the asset lock internally, broadcasts, waits for proof
(with IS→CL fallback). `FundFromAddresses` uses `put_with_address_funding()`.

**4. Multi-mode identity top-up**

Same pattern with `TopUpFundingMethod` (UseAssetLock, FundWithWallet, FundWithUtxo).
`FundFromAddresses` uses `top_up_from_addresses()` (already implemented in PR-7).

**5. UTXO retry on exhaustion**

When building an asset lock TX fails due to insufficient UTXOs:
1. Release wallet lock
2. Refresh UTXOs (if SPV running, trigger rescan; otherwise return error)
3. Retry once

**Files to create/modify:**
- `src/wallet/core/asset_lock.rs` — new: TrackedAssetLock, AssetLockStatus, tracking methods
- `src/wallet/core/wallet.rs` — add tracked_asset_locks field, resolve_asset_lock_proof()
- `src/wallet/identity/wallet.rs` — multi-mode register_identity(), top_up_identity()
- `src/wallet/identity/funding.rs` — new: IdentityFundingMethod, TopUpFundingMethod enums
- `src/error.rs` — AssetLockExpired, AssetLockNotChainLocked error variants

**Done when**: Identity registration/top-up works with all 4/3 funding modes.
IS→CL fallback is automatic. Asset locks are tracked from broadcast to use.

---

### PR-12: DashPay completeness

**Goal**: Move DashPay protocol-level crypto from evo-tool into platform-wallet.
DIP-14 256-bit derivation, contact payment addresses, and account reference calculation
are protocol specifications, not application logic.

**1. DIP-14 256-bit key derivation**

Move from evo-tool's `dip14_derivation.rs` into platform-wallet (or `rs-platform-encryption`):

```rust
/// Child key derivation with 256-bit index (DIP-14).
/// For contact-based derivation paths where identity IDs (32 bytes) are used as indices.
pub fn ckd_priv_256(
    parent: &ExtendedPrivKey,
    index: &[u8; 32],
    hardened: bool,
) -> Result<ExtendedPrivKey, Error>

pub fn ckd_pub_256(
    parent: &ExtendedPubKey,
    index: &[u8; 32],
) -> Result<ExtendedPubKey, Error>
```

**2. DashPay xpub derivation**

```rust
/// Derive the contact-specific extended public key.
/// Path: m/9'/coin'/15'/account'/(sender_id)/(recipient_id)
/// Uses DIP-14 256-bit derivation for the identity ID segments.
pub fn derive_contact_xpub(
    wallet: &Wallet,
    network: Network,
    account_index: u32,
    sender_id: &Identifier,
    recipient_id: &Identifier,
) -> Result<(ExtendedPubKey, [u8; 4], [u8; 32], [u8; 33]), Error>
// Returns: (xpub, parent_fingerprint, chain_code, compressed_pubkey)
```

**3. Account reference calculation (DIP-15)**

```rust
/// Calculate account reference per DIP-15.
/// HMAC-SHA256(sender_secret, xpub_bytes) → take 28 MSBs → XOR with account bits.
pub fn calculate_account_reference(
    sender_secret_key: &[u8; 32],
    contact_xpub: &ExtendedPubKey,
    account_index: u32,
    version: u32,
) -> u32
```

**4. Contact payment address derivation**

```rust
/// Derive payment receiving address for a contact at a given index.
/// Standard BIP32 from contact xpub: contact_xpub / index
pub fn derive_contact_payment_address(
    contact_xpub: &ExtendedPubKey,
    index: u32,
    network: Network,
) -> Address
```

**5. Contact payment address registration + gap limit**

Add to `DashPayWallet`:

```rust
/// Register payment addresses for all established contacts.
/// Derives up to highest_receive_index + GAP_LIMIT addresses per contact.
/// Returns new addresses that should be added to SPV bloom filter.
pub async fn register_contact_payment_addresses(
    &self,
) -> Result<Vec<Address>, PlatformWalletError>

/// Process an incoming payment detected at a contact address.
/// Returns contact info if the address matches a known contact relationship.
pub fn match_payment_to_contact(
    &self,
    address: &Address,
) -> Option<(Identifier, Identifier, u32)>  // (owner_id, contact_id, address_index)
```

Gap limit = 20 per contact. When payment arrives at index N, extend registration to N + 20.

**6. Account label encryption (optional)**

Move from evo-tool to `DashPayWallet`:
```rust
pub fn encrypt_account_label(label: &str, shared_key: &[u8; 32]) -> Vec<u8>
pub fn decrypt_account_label(encrypted: &[u8], shared_key: &[u8; 32]) -> Result<String, Error>
```

**Files to create/modify:**
- `src/wallet/dashpay/dip14.rs` — new: ckd_priv_256, ckd_pub_256
- `src/wallet/dashpay/contacts.rs` — new: derive_contact_xpub, account_reference, payment addresses
- `src/wallet/dashpay/wallet.rs` — add register_contact_payment_addresses(), match_payment_to_contact()
- `src/wallet/dashpay/payments.rs` — new: contact payment tracking, gap limit management

**Done when**: All DashPay crypto operations (DIP-14 derivation, ECDH, xpub encryption,
account reference, payment address derivation) are in platform-wallet. An app can build
full DashPay contact + payment flows without reimplementing protocol-level crypto.

---

### PR-13: Evo-tool integration Phase 3

### PR-13 Status: Complete

**What was delivered:**

Phase 3 identity migration (using enriched library from PR-10/11/12):
- `register_identity.rs` → `identity_wallet.register_identity_with_signer()` (with platform-wallet fallback)
- `top_up_identity.rs` → `identity_wallet.top_up_identity_with_signer()` (with platform-wallet fallback)
- `discover_identities.rs` → `identity_wallet.sync()` with QualifiedIdentity adapter (legacy fallback)

Remaining token tasks (4):
- `destroy_frozen_funds.rs` → `token_wallet.destroy_frozen_funds_with_signer()`
- `pause_tokens.rs` → `token_wallet.pause_with_signer()`
- `resume_tokens.rs` → `token_wallet.resume_with_signer()`
- `update_token_config.rs` → `token_wallet.update_config_with_signer()`

Platform-wallet additions:
- `register_identity_with_signer()` — register with external Identity + Signer
- `top_up_identity_with_signer()` — top up with external Identity + proof
- `identity_manager()` — read access for inspecting managed identities after sync
- 4 new TokenWallet methods + `_with_signer` variants (destroy, pause, resume, update_config)

**Migration tally (all phases):**

| Domain | Migrated | Total | Remaining | Details |
|--------|----------|-------|-----------|---------|
| **Tokens** | 13 | 13 | — | All complete |
| **Identity** | 11 | 13 | 2 | See details below |
| **DashPay** | 2 | 9 | 7 | See details below |
| **Core** | 1 | 7 | 6 | See details below |
| **Total** | **27** | **42** | **15** | |

**Tokens — 13/13 migrated:**
- ✅ `transfer_tokens.rs` → `token_wallet.transfer_with_signer()`
- ✅ `mint_tokens.rs` → `token_wallet.mint_with_signer()`
- ✅ `burn_tokens.rs` → `token_wallet.burn_with_signer()`
- ✅ `freeze_tokens.rs` → `token_wallet.freeze_with_signer()`
- ✅ `unfreeze_tokens.rs` → `token_wallet.unfreeze_with_signer()`
- ✅ `claim_tokens.rs` → `token_wallet.claim_with_signer()`
- ✅ `purchase_tokens.rs` → `token_wallet.purchase_with_signer()`
- ✅ `set_token_price.rs` → `token_wallet.set_price_with_signer()`
- ✅ `destroy_frozen_funds.rs` → `token_wallet.destroy_frozen_funds_with_signer()`
- ✅ `pause_tokens.rs` → `token_wallet.pause_with_signer()`
- ✅ `resume_tokens.rs` → `token_wallet.resume_with_signer()`
- ✅ `update_token_config.rs` → `token_wallet.update_config_with_signer()`
- ✅ `query_my_token_balances.rs` → `token_wallet.watch()` + `.sync()` + `.balance()`

**Identity — 11/13 migrated:**
- ✅ `withdraw_from_identity.rs` → `identity_wallet.withdraw_credits_with_signer()`
- ✅ `transfer.rs` → `identity_wallet.transfer_credits_with_signer()`
- ✅ `add_key_to_identity.rs` → `identity_wallet.update_identity_with_signer()`
- ✅ `register_dpns_name.rs` → `identity_wallet.register_name_with_signer()`
- ✅ `register_identity.rs` → `identity_wallet.register_identity_with_signer()` (with fallback)
- ✅ `top_up_identity.rs` → `identity_wallet.top_up_identity_with_signer()` (with fallback)
- ✅ `discover_identities.rs` → `identity_wallet.sync()` (with legacy fallback)
- ✅ `refresh_identity.rs` → `identity_wallet.refresh_identity_with_signer()` (with fallback)
- ✅ `load_identity_from_wallet.rs` → `identity_wallet.load_identity_by_index()` (with legacy fallback)
- ✅ `load_identity_by_dpns_name.rs` → `sdk.resolve_dpns_name()` + platform wallet watched identity
- ✅ `refresh_loaded_identities_dpns_names.rs` → `sdk.get_dpns_usernames_by_identity()`
- ❌ `load_identity.rs` — UI-driven manual import (user pastes ID, masternode types, manual key input). Genuinely app-level.
- ❌ Support files (`encryption.rs`, `dip14_derivation.rs`, `hd_derivation.rs`) — crypto utilities still used by non-migrated DashPay tasks

**DashPay — 2/9 migrated:**
- ✅ `contact_requests.rs` (send) → `platform_wallet.dashpay().send_contact_request()`
- ✅ `contact_requests.rs` (accept) → `platform_wallet.dashpay().send_contact_request()` (reciprocal)
- ❌ `contact_requests.rs` (load) — UI expects raw `Vec<(Identifier, Document)>`, platform-wallet returns `Vec<ContactRequest>` (different shape)
- ❌ `contact_requests.rs` (reject) — platform-wallet only does local removal, evo-tool persists rejection to Platform via contactInfo document
- ❌ `contacts.rs` — UI-specific contact list management
- ❌ `incoming_payments.rs` — SPV payment address registration, gap limit tracking
- ❌ `auto_accept_handler.rs` — evo-tool orchestration of auto-accept batching
- ❌ Support files (`encryption.rs`, `dip14_derivation.rs`, `hd_derivation.rs`, `validation.rs`) — still used by non-migrated tasks

**Core — 1/7 migrated:**
- ✅ `create_asset_lock.rs` — partial (uses `CoreWallet.build_asset_lock_transaction()` with fallback)
- ❌ `refresh_wallet_info.rs` — UTXO refresh from RPC/SPV, tightly coupled to evo-tool's SpvManager
- ❌ `refresh_single_key_wallet_info.rs` — single-key wallet refresh
- ❌ `send_single_key_wallet_payment.rs` — Core transaction from single-key wallet
- ❌ `recover_asset_locks.rs` — unused asset lock recovery from DB
- ❌ `start_dash_qt.rs` — subprocess launcher (not platform-related)
- ❌ `mod.rs` core task dispatch — orchestration logic

---

### PR-14: Protocol completeness — DashPay + Identity

**Goal**: Complete protocol-level support so any app can build full DashPay contact +
payment flows AND full identity management without reimplementing protocol logic.

**DashPayWallet additions:**
- `reject_contact_request()` — contactInfo document with display_hidden=true
- `generate_auto_accept_proof()` / `verify_auto_accept_proof()` — DIP-15 QR auto-accept
- `validate_contact_request()` — pre-send key/height/reference validation
- `encrypt_account_label()` / `decrypt_account_label()` — CBC-AES-256 with ECDH key
- `register_contact_payment_addresses()` — bulk address derivation + gap limit tracking
- `match_payment_to_contact()` — address → (owner, contact, index) lookup
- `sent_contact_requests()` — query outgoing requests from Platform
- `send_contact_request_with_signer()` / `accept_contact_request_with_signer()` — external signer variants

**IdentityWallet additions:**

```rust
/// Load a single identity by wallet index (not gap scan — targeted lookup).
/// Derives auth key at identity_index, queries Platform, adds to manager.
pub async fn load_identity_by_index(
    &self,
    identity_index: u32,
) -> Result<Option<Identity>, PlatformWalletError>
```

```rust
/// Refresh a known identity's state from Platform (balance, keys, revision).
/// Unlike sync() which discovers new identities, this updates an existing one.
pub async fn refresh_identity(
    &self,
    identity_id: &Identifier,
) -> Result<Identity, PlatformWalletError>
```

```rust
/// Refresh DPNS names for all managed identities.
/// Queries Platform for current names, updates ManagedIdentity.dpns_names.
pub async fn refresh_dpns_names(&self) -> Result<(), PlatformWalletError>
```

```rust
/// Load an identity by DPNS name resolution + fetch.
/// Combines resolve_name() + fetch identity + add to manager.
pub async fn load_identity_by_dpns_name(
    &self,
    name: &str,
) -> Result<Option<Identity>, PlatformWalletError>
```

**Files to create/modify:**
- `src/wallet/dashpay/auto_accept.rs` — new: proof generation + verification
- `src/wallet/dashpay/validation.rs` — new: pre-send validation
- `src/wallet/dashpay/payments.rs` — new: payment address registration + matching
- `src/wallet/dashpay/wallet.rs` — reject, sent_requests, label encryption, _with_signer methods
- `src/wallet/identity/wallet.rs` — load_identity_by_index, refresh_identity, refresh_dpns_names, load_identity_by_dpns_name

**Evo-tool migration** (same PR or follow-up):
- `load_identity_from_wallet.rs` → `wallet.identity().load_identity_by_index()`
- `refresh_identity.rs` → `wallet.identity().refresh_identity()`
- `refresh_loaded_identities_dpns_names.rs` → `wallet.identity().refresh_dpns_names()`
- `load_identity_by_dpns_name.rs` → `wallet.identity().load_identity_by_dpns_name()`
- DashPay tasks → `wallet.dashpay().*_with_signer()` methods

**Done when**: Full DashPay + identity protocol coverage. Only `load_identity.rs` (manual import
with masternode types) remains evo-tool-specific.

---

### PR-15: Shielded pool (feature-gated `shielded`)

**Goal**: Implement `ShieldedWallet<S: ShieldedStore>` — a standalone, storage-generic shielded
transaction component using Orchard/Halo2 ZK proofs. All code behind `#[cfg(feature = "shielded")]`.

**Key design decision**: Storage is abstracted via the `ShieldedStore` trait. The library provides
`InMemoryShieldedStore` for tests; consumers (evo-tool) bring their own persistence (SQLite).
This keeps the library dependency-light and testable without database infrastructure.

**Architectural note**: `ShieldedWallet` is **not** a field on `PlatformWallet`. It is a standalone
component that consumers create separately with their own `ShieldedStore` implementation. This
avoids infecting `PlatformWallet` with the `S: ShieldedStore` generic parameter. `ShieldedWallet`
shares the `Sdk` with `PlatformWallet` but manages its own state.

**Library** (`rs-platform-wallet`):

New `wallet/shielded/` module behind `#[cfg(feature = "shielded")]`:

- `mod.rs` — `ShieldedWallet<S>` struct (`Sdk`, `OrchardKeySet`, `Arc<RwLock<S>>`, `Network`),
  constructors (`new`, `from_seed`), re-exports
- `keys.rs` — `OrchardKeySet` (ZIP-32 key hierarchy: `SpendingKey`, `FullViewingKey`,
  `SpendAuthorizingKey`, `IncomingViewingKey`, `OutgoingViewingKey`, `PaymentAddress`),
  derivation from seed, address generation, `PreparedIncomingViewingKey` for trial decryption
- `store.rs` — `ShieldedStore` trait (note CRUD, commitment tree ops, sync state checkpoints),
  `ShieldedNote` struct, `InMemoryShieldedStore` (Vec + BTreeMap + in-memory tree)
- `sync.rs` — `sync_notes()` (trial decryption of encrypted notes, commitment tree append),
  `check_nullifiers()` (privacy-preserving trunk/branch scan), `sync()` (full orchestration),
  result types (`SyncNotesResult`, `ShieldedSyncSummary`)
- `operations.rs` — 5 transition types, each using DPP `build_*_transition()` builders and
  broadcasting via SDK traits (`ShieldFunds`, `UnshieldFunds`, `TransferShielded`,
  `WithdrawShielded`, `ShieldFromAssetLock`):
  - `shield()` — platform addresses to shielded pool (needs `Signer<PlatformAddress>`)
  - `shield_from_asset_lock()` — Core L1 to shielded pool via asset lock proof
  - `unshield()` — shielded pool to platform address
  - `transfer()` — shielded pool to shielded pool (private, to `PaymentAddress`)
  - `withdraw()` — shielded pool to Core L1 address
- `prover.rs` — `CachedOrchardProver` (`OnceLock<ProvingKey>`, `warm_up()` for background
  init, implements `OrchardProver` trait), shared across all `ShieldedWallet` instances
- `note_selection.rs` — `select_spendable_notes()` (greedy: sort by value descending,
  accumulate until >= amount + fee, returns notes with Merkle witness paths)

**Files**:
- `packages/rs-platform-wallet/src/wallet/shielded/mod.rs`
- `packages/rs-platform-wallet/src/wallet/shielded/keys.rs`
- `packages/rs-platform-wallet/src/wallet/shielded/store.rs`
- `packages/rs-platform-wallet/src/wallet/shielded/sync.rs`
- `packages/rs-platform-wallet/src/wallet/shielded/operations.rs`
- `packages/rs-platform-wallet/src/wallet/shielded/prover.rs`
- `packages/rs-platform-wallet/src/wallet/shielded/note_selection.rs`

**Done when**:
- `ShieldedStore` trait compiles with `InMemoryShieldedStore` passing unit tests
- `OrchardKeySet::from_seed()` derives correct keys (verified against reference vectors)
- `sync_notes()` trial-decrypts test notes and populates store
- `check_nullifiers()` detects spent notes and marks them
- All 5 operations build valid Orchard bundles via DPP builders and broadcast via SDK traits
- `CachedOrchardProver` initializes and generates valid proofs
- Note selection covers amount + fee or returns insufficient-funds error
- Full round-trip test: shield, sync, check balance, transfer, unshield

---

### PR-16: AssetLockFinalityEvent

**Scope change**: Originally planned to replace evo-tool's SpvManager with
PlatformWalletManager. After research, SpvManager has ~1,500 lines of app-specific
orchestration (ConnectionStatus push updates, 300ms debounced reconciliation, wallet-to-DB
sync, peer count tracking, quorum lookups, RPC/SPV mode switching) that is NOT protocol-level.

**Decision**: Keep evo-tool's SpvManager. It coexists with platform-wallet — both share
the same `ManagedWalletInfo` via `Arc<RwLock<>>`. Only add the protocol-level finality
tracking to platform-wallet.

**What to implement:**

```rust
impl SpvRuntime {
    /// Register a transaction to wait for finality (InstantLock or ChainLock).
    /// Call BEFORE broadcasting the transaction.
    pub async fn register_for_finality(&self, txid: Txid);

    /// Wait for a finality proof for a previously registered transaction.
    /// Returns the proof once an InstantLock or ChainLock is received.
    /// Timeout: configurable (default 5 minutes).
    pub async fn wait_for_finality(
        &self,
        txid: Txid,
        timeout: Duration,
    ) -> Result<AssetLockProof, PlatformWalletError>;
}
```

Internal state:
- `finality_waiters: Mutex<BTreeMap<Txid, Option<AssetLockProof>>>` on SpvRuntime
- `SpvEventForwarder` forwards `InstantLockReceived` / `ChainLockReceived` events
- Add a listener that updates `finality_waiters` when matching events arrive
- `wait_for_finality()` polls the map with sleep intervals (like evo-tool's pattern)

Critical invariant: call `register_for_finality()` BEFORE broadcasting to prevent
race where proof arrives before registration.

**Files to modify:**
- `src/spv/runtime.rs` — finality_waiters field + register/wait methods
- `src/spv/event_forwarder.rs` — forward finality events to waiter map
- `src/error.rs` — add FinalityTimeout variant

**Done when**: `wait_for_finality(txid)` returns an AssetLockProof when IS/CL event
arrives via SPV. CoreWallet's register_identity/top_up can optionally use this instead
of DAPI polling.

---

### PR-17: Comprehensive test suite

**Infrastructure**:
- `tests/common/mod.rs` — shared helpers: `create_test_wallet()`, `create_funded_wallet()`, `inject_utxos()`
- `dash-sdk` with `mocks` feature in `[dev-dependencies]`
- Known test mnemonic (`"abandon abandon..."`)
- E2E feature flag `#[cfg(feature = "e2e-tests")]`

**Unit tests** (~70 ported from evo-tool + new):
- Balance calculation (10 tests), UTXO selection (8), platform address info (4)
- Derivation paths (13), address derivation (6), seed lifecycle (2)
- Asset lock fee calc (9), wallet transactions (3)
- DIP-14 derivation (5), seed encryption (2)
- IdentityManager, ManagedIdentity, ContactRequest, EstablishedContact (existing 35 + new)

**Integration tests** (mock SDK):
- Wallet construction (10+ tests), manager CRUD (10+)
- IdentitySigner signing (8+), PlatformAddressSigner (5+)
- CoreWallet async queries (12+), asset lock building (8+)
- Identity registration/sync/topup/withdraw flow (mocked Platform)
- DashPay contact request flow (mocked)
- Platform address sync/transfer/withdraw (mocked)
- Token watch/sync/transfer/mint/burn (mocked)

**E2E tests** (live network, feature-gated):
- SPV sync + wallet balance (BackendTestContext pattern from evo-tool PR #778)
- Send/receive funds round-trip
- Identity registration + discovery
- Contact request send + accept between two wallets
- Platform address operations
- Token operations

---

### PR-18: Replace evo-tool Wallet model with CoreWallet (COMPLETED)

**Completed work:**

Platform-wallet:
- `Arc<WalletBalance>` — cloned PlatformWallet handles share balance atomics
- `blocking_wallet_info()` — sync read access for egui UI code
- CoreWallet convenience wrappers removed (done in earlier PRs)

Evo-tool:
- Embedded `Option<PlatformWallet>` inside evo-tool `Wallet` struct — set on unlock, cleared on lock
- All UI balance reads migrated to lock-free `WalletBalance` via `wallet.platform_wallet`
- All UI UTXO/address reads migrated to `blocking_wallet_info()` + `CoreAddressInfo`
- Removed `platform_wallets` bridge map from AppContext — all lookups go through `wallet.platform_wallet`
- Removed 6 duplicate fields from Wallet: `confirmed_balance`, `unconfirmed_balance`, `total_balance`, `spv_balance_known`, `address_balances`, `address_total_received`
- Balance methods (`confirmed_balance_duffs()`, `total_balance_duffs()`, etc.) delegate to PlatformWallet
- New `address_balance()` method reads per-address balance from CoreAddressInfo
- `funding_common` reads UTXOs from PlatformWallet's `get_spendable_utxos()`

Additional completed work (same PR):
- Migrated RPC send payment to `platform_wallet.core().send_transaction()`
- Migrated all asset lock building (create_asset_lock, register_identity, top_up_identity, fund_platform_address, shielded bundle) to `platform_wallet.core().build_asset_lock_transaction()`
- Removed all fallback paths (try PlatformWallet → fall back to old Wallet)
- Removed ~600 lines of dead asset lock building code from asset_lock_transaction.rs
- Removed build_standard_payment_transaction, build_multi_recipient_payment_transaction (~270 lines)
- Removed reload_utxos (~120 lines), utxos_by_address, max_balance
- Made broadcast_and_commit_asset_lock take Option<used_utxos> (None for PlatformWallet paths)
- Removed 22 obsolete tests (UTXO selection, balance fallbacks, utxos_by_address)
- Total: ~1,625 lines removed

**Remaining Wallet fields (PR-19 scope):**
- `utxos` — SPV reconciliation writes, _for_utxo asset lock paths, transaction_processing
- `known_addresses` — address derivation (receive_address, change_address), key lookup, bootstrap
- `watched_addresses` — address metadata, account summaries, UI display
- `transactions` — transaction history display

---

### PR-19: Migrate remaining Wallet fields

**Goal**: Remove `utxos`, `known_addresses`, `watched_addresses`, `transactions` from evo-tool's Wallet by migrating all remaining callers to PlatformWallet.

**Completed:**
- `register_contact_account()` on DashPayWallet — creates DashpayReceivingFunds managed accounts in ManagedWalletInfo when contacts are established
- Called automatically from `send_contact_request()`
- key-wallet already has: `ManagedAccountCollection::insert()` for DashpayReceivingFunds, `ManagedCoreAccount::from_account()` for creating managed wrappers with address pools

#### How DashPay interacts with the core wallet (DIP-14/15)

When a contact is established (mutual contact requests on Platform):

1. **Send request** (`DashPayWallet::send_contact_request()`):
   - Derives DashPay receiving-account xpub: `m/9'/coin'/15'/0'/(sender_id)/(recipient_id)` using DIP-14 256-bit derivation
   - Encrypts xpub with ECDH (recipient's decryption key)
   - Submits contactRequest document to Platform
   - **Now also**: creates `DashpayReceivingFunds` account in `ManagedWalletInfo` so SPV monitors incoming payment addresses

2. **Accept request** (`DashPayWallet::accept_contact_request()`):
   - Sends reciprocal request (calls `send_contact_request()`)
   - Auto-establish logic in ManagedIdentity detects both requests → creates `EstablishedContact`

3. **Address monitoring** (now automatic via ManagedWalletInfo):
   - `ManagedCoreAccount` for `DashpayReceivingFunds` has address pools with gap limit
   - SPV adapter iterates all accounts via `monitored_addresses()` → includes contact addresses
   - When incoming payment arrives, `check_core_transaction()` matches against address pools
   - Gap limit automatically derives more addresses as used addresses are consumed

4. **Previously (evo-tool manual flow, being removed)**:
   - `register_dashpay_addresses_for_identity()` manually derived addresses from seed
   - Inserted into `known_addresses` and `watched_addresses` BTreeMaps
   - Maintained `dashpay_contact_address_indices` DB table for gap limit tracking
   - Required explicit `RegisterDashPayAddresses` backend task trigger

#### Address types and their account mapping

| Address type | DIP | Derivation path | key-wallet account | Status |
|---|---|---|---|---|
| BIP44 receive/change | BIP44 | `m/44'/coin'/acct'/0or1/i` | `standard_bip44_accounts` | In ManagedWalletInfo ✓ |
| Identity registration | DIP-9 | `m/9'/coin'/5'/1'/i` | `identity_registration` | In ManagedWalletInfo ✓ |
| Identity top-up | DIP-9 | `m/9'/coin'/5'/2'/i` | `identity_topup` | In ManagedWalletInfo ✓ |
| DashPay receive | DIP-15 | `m/9'/coin'/15'/0'/(self)/(friend)/i` | `dashpay_receival_accounts` | **Now registered** ✓ |
| DashPay send (watch) | DIP-15 | contact xpub + index | `dashpay_external_accounts` | TODO |
| Platform payment | DIP-17 | `m/9'/coin'/17'/acct'/class'/i` | `platform_payment_accounts` | In ManagedWalletInfo ✓ |
| CoinJoin | - | `m/9'/coin'/cointype'/i` | `coinjoin_accounts` | In ManagedWalletInfo ✓ |
| Provider keys | - | various | `provider_*_keys` | In ManagedWalletInfo ✓ |

#### Remaining migration steps

**All phases COMPLETE.** 10/10 duplicate fields removed from evo-tool's Wallet struct.

Summary of completed work:
- [x] DashPay contact accounts registered in both key-wallet Wallet + ManagedWalletInfo
- [x] Address derivation delegated to PlatformWallet (blocking_next_receive/change_address)
- [x] Bootstrap skipped when PlatformWallet available (locked wallets show nothing — privacy)
- [x] All UI/backend reads migrated to CoreAddressInfo / WalletBalance / blocking_wallet_info
- [x] All asset lock building migrated to CoreWallet::build_asset_lock_transaction
- [x] RPC send payment migrated to CoreWallet::send_transaction
- [x] Removed fields: confirmed_balance, unconfirmed_balance, total_balance, spv_balance_known, address_balances, address_total_received, utxos, known_addresses, watched_addresses, transactions
- [x] Removed ~600 lines of asset lock building, ~400 lines of bootstrap, ~270 lines of tx building
- [x] Arc<Sdk> in PlatformWallet, Arc<PlatformWallet> in manager and evo-tool Wallet
- [x] WalletBalance reverted from Arc to plain (shared via Arc<PlatformWallet>)
- [x] Removed platform_wallets bridge map from AppContext

**Remaining in Wallet struct** (app-level metadata, NOT duplicates):
- `platform_wallet: Option<Arc<PlatformWallet>>` — canonical wallet
- `wallet_seed` — encrypted seed for persistence
- `uses_password`, `master_bip44_ecdsa_extended_public_key` — auth
- `unused_asset_locks` — asset lock tracking
- `alias`, `identities`, `is_main` — app metadata
- `platform_address_info` — platform credits (could migrate to PlatformAddressWallet)
- `core_wallet_name` — RPC config

**Remaining code that still references old patterns** (functional, not dead):
- `_for_utxo` asset lock paths (register_identity, top_up_identity) — need CoreWallet API
- `remove_selected_utxos` in utxos.rs — DB persistence for _for_utxo paths
- `update_address_balance`/`update_address_total_received` — DB persistence
- `platform_addresses`/`platform_receive_address` — reads watched_addresses but from platform_address_info
- DB tables (wallet_addresses, utxos, wallet_transactions) — kept for future serialization PR

**Total: ~2,700 lines removed from evo-tool.**

---

### PR-20: Complete Identity/Asset Lock Lifecycle

**Goal**: Platform-wallet provides one-call APIs for identity registration and
top-up. Apps never touch asset locks, finality tracking, or proof construction.

#### Current problem

Identity registration is split across repos:
1. **Evo-tool** builds asset lock, broadcasts, tracks finality via SPV, waits for proof
2. **Platform-wallet** has the identity state transition but expects pre-built proof
3. **Platform-wallet SPV runtime** has `register_for_finality()`/`wait_for_finality()` but they're NEVER CALLED
4. **Platform-wallet** has `broadcast_and_wait_for_asset_lock_proof()` that uses DAPI streaming instead of SPV

This means:
- Every app must reimplement asset lock orchestration (200+ lines)
- SPV finality infrastructure exists but is unused
- DAPI streaming approach is fragile (5min hardcoded timeout)
- `TrackedAssetLock.status` never updates beyond `Broadcast`

#### Layered design

**CoreWallet** — owns asset lock TX lifecycle (Core chain concerns):
```rust
/// Asset lock status on the Core chain.
/// Tracked until used, then removed from tracked set.
pub enum AssetLockStatus {
    Built,
    Broadcast,
    InstantSendLocked,
    ChainLocked,
}

/// A tracked asset lock — Core wallet knows about the TX, its status,
/// and how to re-derive the private key. Private keys stay in
/// key-wallet's Wallet, re-derived from funding_type + identity_index.
pub struct TrackedAssetLock {
    pub txid: Txid,
    pub funding_type: AssetLockFundingType,
    pub identity_index: u32,
    pub amount: u64,
    pub status: AssetLockStatus,
}

impl CoreWallet {
    /// Build asset lock TX (existing).
    pub async fn build_asset_lock_transaction(...) -> Result<...>

    /// Build + broadcast + wait for SPV proof. Returns when IS-lock or
    /// ChainLock is received. Tracks lifecycle internally.
    pub async fn create_funded_asset_lock_proof(
        &self,
        amount_duffs: u64,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        spv_runtime: Option<&SpvRuntime>,
    ) -> Result<(AssetLockProof, PrivateKey, Txid), PlatformWalletError>

    /// List unused (funded but not consumed) asset locks.
    pub fn unused_asset_locks(&self) -> Vec<&TrackedAssetLock>

    /// Scan Core chain for asset lock TXs not yet used.
    pub async fn recover_unused_asset_locks(&self) -> Vec<TrackedAssetLock>

    /// Remove a lock from tracking (called after successful use).
    pub fn remove_asset_lock(&self, txid: &Txid)
}
```

**IdentityWallet** — orchestrates identity operations using CoreWallet:
```rust
/// How to fund an identity operation.
pub enum IdentityFunding {
    /// Build asset lock from wallet UTXOs (most common).
    FromWalletBalance { amount_duffs: u64 },
    /// Use credits from a Platform address (DIP-17).
    FromPlatformAddress { address: PlatformAddress, amount_credits: Credits, nonce: u32 },
    /// Use an existing unused asset lock (recovery from previous attempt).
    FromExistingAssetLock { txid: Txid },
    /// Use a specific UTXO (QR-funded flow).
    FromUtxo { outpoint: OutPoint, tx_out: TxOut, address: Address },
}

impl IdentityWallet {
    /// Register identity — complete flow, one call.
    pub async fn register_identity(
        &self,
        funding: IdentityFunding,
        keys: IdentityKeys,
        identity_index: u32,
    ) -> Result<Identity, PlatformWalletError>

    /// Top up identity — complete flow, one call.
    pub async fn top_up_identity(
        &self,
        identity_id: Identifier,
        funding: IdentityFunding,
        identity_index: u32,
    ) -> Result<u64, PlatformWalletError>
}
```

Note: `FromExistingAssetLock` just takes `txid` — CoreWallet already
tracks the lock, has the proof, and can re-derive the private key.
No key material in IdentityFunding.

#### Key design decisions

**1. CoreWallet owns asset lock lifecycle**: Asset locks are Core chain
transactions used by multiple Platform features (identities, platform
addresses, shielded). CoreWallet tracks their status (Built → Broadcast
→ IS-locked → ChainLocked). When consumed by any Platform operation,
the lock is removed from tracking.

**2. Private keys stay in key-wallet**: `TrackedAssetLock` stores
`funding_type` + `identity_index` — enough to re-derive the private key
from the wallet seed when needed. No key material stored in tracking state.

**3. Transaction status from key-wallet**: Core TX confirmation status
(unconfirmed, IS-locked, confirmed, chainlocked) is already tracked by
key-wallet's `TransactionRecord.context`. `AssetLockStatus` mirrors this
for asset-lock-specific tracking until the lock is consumed.

**4. Remove when used, not track usage**: Once an asset lock is consumed
(identity registered, address funded, etc.), CoreWallet removes it from
the tracked set. No `UsedForRegistration` state — that's the consumer's
concern, not the Core wallet's.

**5. SPV finality (not DAPI streaming)**: Proof detection uses SPV's
`wait_for_finality()` which listens for InstantSend and ChainLock events
natively. No DAPI subscription streams.

**6. Recovery**: `recover_unused_asset_locks()` scans for funded-but-unused
locks on Core chain and adds them to tracking with appropriate status.

#### Implementation steps

**Steps 1, 4, 5 — DONE:**
- ✅ `TrackedAssetLock` + `AssetLockStatus` types (no private keys, remove when consumed)
- ✅ `AssetLockManager` extracted, shared across sub-wallets via `Arc`
- ✅ `IdentityWallet` uses `self.asset_locks` directly (no CoreWallet parameter)
- ✅ `funded_register_identity` / `funded_top_up_identity` call `remove_asset_lock` after use
- ✅ Evo-tool callers updated to `platform_wallet.asset_locks()`

**Step 2 — AssetLockManager subscribes to SPV events for finality:**
- Add `event_tx: broadcast::Sender<PlatformWalletEvent>` to `AssetLockManager`
- Pass it from `PlatformWallet::from_wallet_and_info()` (same channel SPV adapter uses)
- Replace `wait_for_proof_via_dapi()` with event-driven SPV waiting:
  ```rust
  async fn wait_for_proof(&self, txid: &Txid, timeout: Duration) -> Result<AssetLockProof> {
      let mut rx = self.event_tx.subscribe();
      let deadline = Instant::now() + timeout;
      loop {
          tokio::select! {
              event = rx.recv() => {
                  match event {
                      Ok(PlatformWalletEvent::Spv(SpvEvent::Sync(
                          SyncEvent::InstantLockReceived { instant_lock, .. }
                      ))) if instant_lock.txid == *txid => {
                          // Build InstantAssetLockProof from instant_lock
                          return Ok(proof);
                      }
                      Ok(PlatformWalletEvent::Spv(SpvEvent::Sync(
                          SyncEvent::ChainLockReceived { .. }
                      ))) => {
                          // Check if our tx is in a chain-locked block
                          // Build ChainAssetLockProof
                      }
                      _ => continue,
                  }
              }
              _ = tokio::time::sleep_until(deadline) => {
                  return Err(PlatformWalletError::FinalityTimeout(*txid));
              }
          }
      }
  }
  ```
- Update `create_funded_asset_lock_proof()` to use this instead of DAPI streaming
- Delete `wait_for_proof_via_dapi()` and `SpvRuntime::wait_for_finality()` (replaced)

**Step 3 — Asset lock recovery in AssetLockManager:**
- `recover_unused_asset_locks()` scans Core chain for funded-but-unused locks
- Move logic from evo-tool's `recover_asset_locks.rs`
- Recovered locks enter tracking at InstantSendLocked or ChainLocked status

**Step 6 — Simplify evo-tool:**
- Remove `transactions_waiting_for_finality` from AppContext
- Remove `spv_setup_finality_listener()` / `handle_spv_finality_event()` / `received_asset_lock_finality()`
- Remove `wait_for_asset_lock_proof()` polling
- Remove `broadcast_and_commit_asset_lock()`
- Remove `Wallet.unused_asset_locks` field (tracked by AssetLockManager)
- Remove `recover_asset_locks.rs` (moved to AssetLockManager)
- Simplify `create_asset_lock.rs` to call `asset_locks().create_funded_asset_lock_proof()`

---

### PR-21: Remove Remaining Duplication

**Goal**: Clean up remaining duplicated code identified in the duplication audit.

- Replace CoreWallet's `send_transaction()` manual UTXO selection with key-wallet's `TransactionBuilder`
- Remove dead `derive_account_xpub()` (already simplified to use AccountType)
- Remove blocking address derivation path construction duplication
- Clean up any remaining evo-tool code that duplicates platform-wallet

---

### PR-23: Merge Wallet + ManagedWalletInfo (dashcore)

Merge `Wallet` and `ManagedWalletInfo` in `key-wallet` — both are mutable and always used
together. Single `Arc<RwLock<Wallet>>` containing all state.

**Why**: The original split assumed `Wallet` was immutable (key store) while `ManagedWalletInfo`
was mutable (UTXO state). In practice, `Wallet` is also mutable — accounts are added during
DashPay contact establishment and sync. Having them behind separate `RwLock`s creates:
1. Lock ordering risk (must always acquire wallet before wallet_info)
2. Read starvation during block processing (SPV holds write locks on both for entire block)
3. Non-atomic updates when operations touch both structs (crash = inconsistent state)

**Investigation needed**: read starvation mitigation (per-tx lock release vs snapshot/MVCC vs
accept latency), atomic multi-struct update strategy (merge vs journaling vs eventual consistency).

---

### PR-22: ChangeSet-based Persistence (inspired by BDK)

**Goal**: Atomic state updates + persistence via a layered ChangeSet pattern.
Every mutation produces a delta that is applied atomically to in-memory state
and persisted atomically to storage. Two layers: key-wallet owns core wallet
deltas, platform-wallet composes them with platform-specific deltas.

#### Architecture: Two-Layer ChangeSets

```
key-wallet (dashcore)                    platform-wallet
┌─────────────────────┐                 ┌──────────────────────────────┐
│  WalletChangeSet    │                 │  PlatformWalletChangeSet     │
│  ├─ utxos           │  composed into  │  ├─ wallet: WalletChangeSet  │
│  ├─ transactions    │ ───────────────>│  ├─ identities               │
│  ├─ accounts        │                 │  ├─ contacts                 │
│  └─ balance         │                 │  ├─ platform_addresses       │
└─────────────────────┘                 │  ├─ shielded                 │
                                        │  └─ asset_locks              │
                                        └──────────────────────────────┘
```

**Flow for every operation:**
```
1. Operation executes (e.g., process_block, send_contact_request)
2. key-wallet mutation returns WalletChangeSet (UTXO/tx/account deltas)
3. platform-wallet wraps it + adds platform deltas → PlatformWalletChangeSet
4. apply() to in-memory state (single write lock, all or nothing)
5. stage() into accumulated changeset
6. persist() to storage (single DB transaction, all or nothing)
```

**Key insight**: Each layer owns its own deltas. key-wallet knows exactly what
UTXOs/transactions/addresses changed — it produces `WalletChangeSet` natively.
Platform-wallet composes it with identity/contact/platform state and persists
the whole `PlatformWalletChangeSet` atomically.

#### Layer 1: key-wallet `WalletChangeSet` (dashcore crate)

Lives in `rust-dashcore/key-wallet/src/changeset/`. Captures ALL core
wallet mutations from a single operation:

```rust
// key-wallet/src/changeset/changeset.rs

/// Delta of core wallet state from a single operation.
pub struct WalletChangeSet {
    /// Chain sync state (new block height + hash).
    pub chain: Option<ChainChangeSet>,
    /// UTXO changes (added from received outputs, spent from consumed inputs).
    pub utxos: Option<UtxoChangeSet>,
    /// Transaction changes (new transactions, confirmation/IS-lock status updates).
    pub transactions: Option<TransactionChangeSet>,
    /// Account changes (new accounts, address pool expansion, used address marking).
    pub accounts: Option<AccountChangeSet>,
    /// Aggregate balance change (recomputed from UTXO delta).
    pub balance: Option<BalanceChangeSet>,
}

pub struct ChainChangeSet {
    pub height: Option<u32>,
    pub block_hash: Option<BlockHash>,
}

pub struct UtxoChangeSet {
    /// UTXOs created by received transaction outputs.
    pub added: BTreeMap<OutPoint, Utxo>,
    /// UTXOs consumed by spent transaction inputs.
    pub spent: BTreeSet<OutPoint>,
    /// UTXOs whose InstantSend lock status changed.
    pub instant_locked: BTreeSet<OutPoint>,
}

pub struct TransactionChangeSet {
    /// New or updated transaction records.
    pub records: BTreeMap<Txid, TransactionRecord>,
}

pub struct AccountChangeSet {
    /// New accounts added (DashPay contacts, new identity accounts).
    pub new_accounts: Vec<AccountEntry>,
    /// Address pool indices advanced (account key → new last_revealed index).
    pub last_revealed: BTreeMap<AccountKey, u32>,
    /// Addresses marked as used.
    pub addresses_used: Vec<(AccountKey, Address)>,
}

pub struct BalanceChangeSet {
    pub spendable: i64,      // delta, not absolute
    pub unconfirmed: i64,
    pub immature: i64,
    pub locked: i64,
}
```

**Produced by**: `check_core_transaction()`, `record_transaction()`,
`confirm_transaction()`, `mark_utxos_instant_send()`, `maintain_gap_limit()`.
Each mutation method returns a `WalletChangeSet` instead of (or alongside)
mutating in place.

#### Layer 2: platform-wallet `PlatformWalletChangeSet`

Lives in `rs-platform-wallet/src/persistence/`. Composes key-wallet's
changeset with platform-specific deltas:

```rust
// platform-wallet/src/persistence/changeset.rs

/// Full delta of platform wallet state from a single operation.
pub struct PlatformWalletChangeSet {
    /// Core wallet changes (UTXOs, transactions, accounts, balance).
    /// Produced by key-wallet operations.
    pub wallet: Option<WalletChangeSet>,
    /// Identity changes (registered, updated, key changes, DPNS names).
    pub identities: Option<IdentityChangeSet>,
    /// DashPay contact changes (requests sent/received, contacts established).
    pub contacts: Option<ContactChangeSet>,
    /// Platform address changes (DIP-17 balance/nonce from Platform proofs).
    pub platform_addresses: Option<PlatformAddressChangeSet>,
    /// Shielded state changes (commitment tree, nullifiers).
    pub shielded: Option<ShieldedChangeSet>,
    /// Asset lock lifecycle changes (created, broadcast, confirmed, used).
    pub asset_locks: Option<AssetLockChangeSet>,
}
```
```

#### The Merge Trait

```rust
/// Combine two changesets. Used to batch multiple operations before persisting.
pub trait Merge: Default {
    fn merge(&mut self, other: Self);
    fn is_empty(&self) -> bool;
}
```

Merge semantics per sub-changeset:
- **UTXOs**: union of added, union of spent (idempotent — adding same UTXO twice is no-op)
- **Transactions**: insert or update (later status wins: chainlocked > confirmed > IS-locked > unconfirmed)
- **Identities**: monotonic revision (keep higher), append new keys
- **Contacts**: state machine ordering (pending < accepted < blocked)
- **Chain**: keep higher block height, insert new headers
- **Accounts**: append new addresses to pools, advance gap limit indices
- **Platform addresses**: keep higher nonce, update balance (last write wins from Platform proofs)

#### The Persistence Trait

```rust
/// Storage backend abstraction. Implementors choose their own storage
/// (SQLite, file, memory, remote). The trait guarantees atomic persistence.
pub trait WalletPersistence {
    type Error: std::error::Error;

    /// Load the aggregated state from storage.
    /// Returns a single ChangeSet representing the full stored state
    /// (equivalent to merging all previously persisted changesets).
    fn initialize(&mut self) -> Result<WalletChangeSet, Self::Error>;

    /// Persist a delta atomically. Either all sub-changesets are stored
    /// or none are. Implementations MUST guarantee atomicity (e.g.,
    /// SQLite transaction, atomic file write).
    fn persist(&mut self, changeset: &WalletChangeSet) -> Result<(), Self::Error>;
}
```

#### How Operations Produce ChangeSets

Every mutation on PlatformWallet returns a `WalletChangeSet`:

```rust
impl PlatformWallet {
    /// Process a new block from SPV.
    /// Computes changes (read-only), then applies atomically.
    pub fn process_block(&self, block: &Block, height: u32) -> WalletChangeSet {
        let mut changeset = WalletChangeSet::default();

        // 1. Update chain state
        changeset.chain = Some(ChainChangeSet { height, block_hash: block.header.hash() });

        // 2. Check each transaction against all accounts
        for tx in &block.txdata {
            let tx_changes = self.check_transaction(tx, height);
            changeset.merge(tx_changes);
        }

        // 3. Return delta — caller applies + persists
        changeset
    }

    /// Send a contact request (DashPay).
    /// Returns changes to identities + contacts + accounts.
    pub async fn send_contact_request(&self, ...) -> Result<WalletChangeSet, Error> {
        let mut changeset = WalletChangeSet::default();

        // 1. Create contact request document on Platform
        let request = self.dashpay().submit_request(...).await?;

        // 2. Record sent request
        changeset.contacts = Some(ContactChangeSet::request_sent(our_id, their_id, request));

        // 3. Register DashPay receiving account
        let account_changes = self.register_contact_account(our_id, their_id)?;
        changeset.accounts = Some(account_changes);

        Ok(changeset)
    }
}
```

#### The Staged ChangeSet Pattern

PlatformWallet accumulates changesets in a `stage` field until the caller
explicitly persists:

```rust
pub struct PlatformWallet {
    // ... existing fields ...

    /// Accumulated changesets not yet persisted.
    stage: RwLock<WalletChangeSet>,
}

impl PlatformWallet {
    /// Apply a changeset to in-memory state and stage for persistence.
    pub fn apply_and_stage(&self, changeset: WalletChangeSet) {
        // Apply to in-memory structs
        self.apply(changeset.clone());
        // Merge into staged changes
        self.stage.write().merge(changeset);
    }

    /// Persist all staged changes and clear the stage.
    pub fn persist(&self, persister: &mut impl WalletPersistence) -> Result<(), Error> {
        let staged = self.stage.write().take();
        if let Some(changeset) = staged {
            persister.persist(&changeset)?;
        }
        Ok(())
    }
}
```

#### In-Memory Atomicity

Two approaches (choose one):

**Option A — Single struct behind one RwLock (PR-21):**
Merge Wallet + ManagedWalletInfo + IdentityManager into one struct. The `apply()`
method takes `&mut self` — only one writer at a time, all changes atomic by Rust's
ownership rules. No lock ordering issues.

**Option B — Compute-then-apply (current multi-lock architecture):**
The changeset is computed without holding any write locks (read-only analysis).
Then `apply()` acquires all write locks in a fixed order, applies all changes,
releases all locks. If any lock acquisition fails, no changes are applied.

Option A is simpler and recommended. Option B works as a stepping stone.

#### Storage Atomicity

**SQLite implementation:**
```rust
impl WalletPersistence for SqlitePersister {
    fn persist(&mut self, changeset: &WalletChangeSet) -> Result<(), Error> {
        let tx = self.conn.transaction()?;  // BEGIN TRANSACTION

        if let Some(chain) = &changeset.chain {
            persist_chain(&tx, chain)?;
        }
        if let Some(utxos) = &changeset.utxos {
            persist_utxos(&tx, utxos)?;
        }
        if let Some(txs) = &changeset.transactions {
            persist_transactions(&tx, txs)?;
        }
        if let Some(ids) = &changeset.identities {
            persist_identities(&tx, ids)?;
        }
        if let Some(contacts) = &changeset.contacts {
            persist_contacts(&tx, contacts)?;
        }
        // ... all sub-changesets ...

        tx.commit()?;  // COMMIT — all or nothing
        Ok(())
    }
}
```

**File store implementation (for testing/dev):**
Append-only binary log. Each `persist()` appends one serialized changeset.
`initialize()` reads all entries, merges via `Merge` trait. Simple, no SQLite
dependency.

#### Recovery

If the app crashes:
- **After apply, before persist**: In-memory state is ahead of storage. On restart,
  `initialize()` loads last persisted state. SPV re-syncs from the stored chain height,
  re-producing the missing changesets. Platform state is re-fetched.
- **During persist (SQLite)**: Transaction rolls back. Storage is at the previous state.
  Same recovery as above.
- **After persist**: Both in sync. No recovery needed.

The gap between in-memory and storage is always bounded by the time since last `persist()`.
Calling `persist()` after every block or every user action keeps the gap small.

#### Layered Responsibilities

```
key-wallet (dashcore):
  - WalletChangeSet types + Merge trait
  - compute_*() methods — read-only, return changeset
  - apply() — mutate state from changeset
  - NO persister, NO stage, NO persistence awareness

platform-wallet:
  - PlatformWalletChangeSet (wraps key-wallet + platform deltas)
  - Optional persister field (configurable)
  - Calls key-wallet compute_*() → gets WalletChangeSet
  - Wraps in PlatformWalletChangeSet → queues on persister
  - Persister owns the pending buffer + flush strategy
  - apply() delegates to ManagedWalletInfo + IdentityManager
```

key-wallet stays pure — compute + apply. If used standalone
(without platform-wallet), no persistence overhead.

#### Persister Architecture

The persister lives on PlatformWallet. It owns the pending buffer
and decides when to flush. The wallet queues and forgets:

```rust
pub struct PlatformWallet {
    // ... wallet fields ...
    persister: Option<Arc<Mutex<dyn PlatformWalletPersistence>>>,
}

impl PlatformWallet {
    // Queue changeset — persister decides when to flush
    fn queue_persist(&self, changeset: PlatformWalletChangeSet) {
        if let Some(persister) = &self.persister {
            persister.lock().queue(changeset);
        }
        // No persister = no-op, no accumulation, no memory growth
    }

    fn set_persister(&mut self, persister: impl PlatformWalletPersistence) {
        self.persister = Some(Arc::new(Mutex::new(persister)));
    }
}
```

The persister owns flush strategy:
```rust
pub trait PlatformWalletPersistence {
    type Error: std::error::Error;

    /// Queue a changeset. Persister merges into pending buffer.
    /// May flush immediately or defer based on strategy.
    fn queue(&mut self, changeset: PlatformWalletChangeSet);

    /// Force flush all pending changes to storage.
    fn flush(&mut self) -> Result<(), Self::Error>;

    /// Load all persisted state as one changeset (for startup).
    fn initialize(&mut self) -> Result<PlatformWalletChangeSet, Self::Error>;
}

pub struct SqliteWalletPersister {
    db: Arc<Database>,
    seed_hash: [u8; 32],
    network: String,
    pending: PlatformWalletChangeSet,  // accumulates here
    strategy: FlushStrategy,
}

pub enum FlushStrategy {
    /// Flush after every queue() call
    Immediate,
    /// Flush every N queued changesets
    EveryN(usize),
    /// Never auto-flush — caller must call flush() explicitly
    Manual,
}

impl PlatformWalletPersistence for SqliteWalletPersister {
    fn queue(&mut self, changeset: PlatformWalletChangeSet) {
        self.pending.merge(changeset);
        match self.strategy {
            FlushStrategy::Immediate => { let _ = self.flush(); }
            FlushStrategy::EveryN(n) => { self.count += 1; if self.count >= n { let _ = self.flush(); } }
            FlushStrategy::Manual => {} // caller decides
        }
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        if let Some(changeset) = self.pending.take() {
            // Single SQLite transaction — all or nothing
            let tx = self.conn.transaction()?;
            self.persist_changeset(&tx, &changeset)?;
            tx.commit()?;
        }
        Ok(())
    }
}
```

**No persister = no memory growth.** The `queue_persist()` call is a
no-op when persister is None. No stage field accumulating forever.

#### Compute-Then-Apply Architecture

Every mutation follows the same pattern:

**Internal** (`compute_*`) — read-only, return changeset, don't mutate:
```rust
// key-wallet: pure computation
fn compute_record_transaction(&self, tx, context) -> WalletChangeSet { ... }
fn compute_mark_address_used(&self, address) -> AccountChangeSet { ... }
fn compute_maintain_gap_limit(&self, xpub) -> AccountChangeSet { ... }
fn compute_update_balance(&self) -> BalanceChangeSet { ... }
```

**Public** (existing names) — aggregate + apply, return result.
key-wallet returns changeset to caller for persistence:
```rust
// key-wallet public method
pub fn check_core_transaction(&mut self, tx, ctx) -> (TransactionCheckResult, WalletChangeSet) {
    // 1. Compute all changes (read-only)
    let changeset = self.compute_transaction_changeset(tx, ctx);

    // 2. Apply atomically (single &mut self)
    self.apply(&changeset);

    // 3. Return changeset to caller (for persistence)
    (result, changeset)
}

// platform-wallet SPV adapter wraps + queues
let (result, kw_changeset) = wallet_info.check_core_transaction(tx, ctx);
let platform_cs = PlatformWalletChangeSet { wallet: Some(kw_changeset), .. };
platform_wallet.queue_persist(platform_cs);  // persister handles the rest
```

**`apply()` method** on ManagedWalletInfo:
```rust
impl ManagedWalletInfo {
    pub fn apply(&mut self, changeset: &WalletChangeSet) {
        if let Some(utxos) = &changeset.utxos {
            for (outpoint, entry) in &utxos.added { self.add_utxo(outpoint, entry); }
            for outpoint in &utxos.spent { self.remove_utxo(outpoint); }
        }
        if let Some(txs) = &changeset.transactions {
            for (txid, record) in &txs.records { self.insert_transaction(txid, record); }
        }
        if let Some(accounts) = &changeset.accounts {
            for (idx, revealed) in &accounts.last_revealed { self.advance_pool(idx, revealed); }
            for (idx, addr) in &accounts.addresses_used { self.mark_used(idx, addr); }
        }
        if let Some(balance) = &changeset.balance {
            self.apply_balance_delta(balance);
        }
    }
}
```

Same for PlatformWallet — delegates to sub-stores:
```rust
impl PlatformWallet {
    pub fn apply(&self, changeset: &PlatformWalletChangeSet) {
        if let Some(wallet_cs) = &changeset.wallet {
            self.core().blocking_wallet_info_mut().apply(wallet_cs);
        }
        if let Some(contacts) = &changeset.contacts { /* IdentityManager */ }
        if let Some(identities) = &changeset.identities { /* IdentityManager */ }
        if let Some(platform_addrs) = &changeset.platform_addresses { /* metadata */ }
    }
}
```

`initialize()` uses `apply()` — same code path as runtime:
```rust
let changeset = persister.initialize()?;
platform_wallet.apply(&changeset);
```

**Consistency guarantees:**
- Compute phase fails → no state change, consistent
- Apply panics → Rust poisons the lock, no partial state visible
- Between apply and queue_persist → in-memory ahead of storage, re-sync fixes
- No persister → no accumulation, no memory growth

#### Implementation Steps (compute-then-apply refactor)

**Step 9 — Add `apply()` to ManagedWalletInfo (dashcore):**

Implement `apply(&mut self, changeset: &WalletChangeSet)` that applies
each sub-changeset to the wallet state. Used by both runtime mutations
and `initialize()` startup loading — same code path guarantees consistency.

**Step 10 — Split mutation methods into compute + apply (dashcore):**

For each mutation method in `ManagedCoreAccount` and `WalletTransactionChecker`,
extract read-only analysis into `compute_*` (returns changeset). The existing
public method becomes: compute + apply + return (result, changeset).

Methods to split:
- `record_transaction` → `compute_record_transaction` + apply
- `confirm_transaction` → `compute_confirm_transaction` + apply
- `mark_utxos_instant_send` → `compute_instant_send_lock` + apply
- `mark_address_used` → `compute_mark_address_used` + apply
- `maintain_gap_limit` → `compute_gap_limit_expansion` + apply
- `update_balance` → `compute_balance_update` + apply

`check_core_transaction` aggregates all compute_* results, applies once,
returns (result, changeset) to caller.

**Step 11 — Persister on PlatformWallet (platform-wallet):**

- Remove `stage: StdRwLock<PlatformWalletChangeSet>` field
- Add `persister: Option<Arc<Mutex<dyn PlatformWalletPersistence>>>`
- Add `queue_persist()` method — no-op without persister
- Add `set_persister()` method
- Update `PlatformWalletPersistence` trait: `queue()` + `flush()` + `initialize()`
- Add `FlushStrategy` enum (Immediate, EveryN, Manual)
- Add `apply()` on PlatformWallet delegating to ManagedWalletInfo + IdentityManager

**Step 12 — Update SqliteWalletPersister (evo-tool):**

- Implement new `PlatformWalletPersistence` trait (queue + flush + initialize)
- Add `pending: PlatformWalletChangeSet` buffer
- Add `strategy: FlushStrategy` field
- Move existing `persist()` logic into `flush()`
- Wire: user actions use `FlushStrategy::Immediate`,
  SPV sync uses `FlushStrategy::Manual` with periodic flush timer

**Step 13 — Wire initialize() through apply() (evo-tool):**

On startup:
```rust
let changeset = persister.initialize()?;
platform_wallet.apply(&changeset);
```

Replace scattered DB loading. Remove `persist_platform_wallet()` helper.
Persister is set on PlatformWallet at creation time.

#### Migration Strategy

The implementation touches 3 repos in order:

1. **dashcore** (key-wallet): Steps 1-2 done, Steps 9-10 next.
2. **platform** (platform-wallet): Steps 3-5 done, Step 11 next.
3. **evo-tool**: Steps 6-8 done, Steps 12-13 next.

Each step compiles independently. No intermediate fallback code.

#### What Stays in evo-tool's DB (app-level, NOT wallet state)

- Encrypted wallet seed (identity, not state)
- Wallet alias, is_main, uses_password (app preferences)
- DashPay contact UI metadata (display name, avatar, last seen)
- Settings, feature flags, proof logs
- Shielded commitment tree (via ShieldedStore trait — already persistent)

#### What Moves to WalletPersistence

- UTXOs, transactions, balances (currently in wallet_addresses, utxos, wallet_transactions tables)
- Identity state (registered, keys, DPNS names)
- Contact request state (sent, received, established)
- Platform address balances/nonces
- Asset lock lifecycle state
- Chain sync progress (height, block hashes)

#### Atomicity Guarantees

**In-memory**: Each mutation method in key-wallet mutates AND returns a delta.
The mutation is atomic (single `&mut self`). The delta is a faithful record.

**Cross-struct**: Platform operations (contacts, identities) produce a
`PlatformWalletChangeSet` that bundles ALL related deltas — e.g.,
`send_contact_request` produces `ContactChangeSet` + `AccountChangeSet`
(for the new DashPay account) in ONE changeset. Applied and persisted together.

**Storage**: `PlatformWalletPersistence::persist()` wraps all sub-changeset
writes in a single DB transaction. All or nothing.

**Recovery**: If crash after in-memory apply but before persist, restart
loads last persisted state via `initialize()`. SPV re-syncs from stored
chain height, reproducing the missing changesets.

**Done when**: 
- Every key-wallet mutation returns a `WalletChangeSet`
- Every platform-wallet operation returns a `PlatformWalletChangeSet`
- No direct DB writes outside the changeset path
- Recovery works correctly after crash at any point
- Audit confirms no atomicity gaps (all cross-struct changes bundled)
- SingleKeyWallet migrated to changeset path (currently uses direct DB writes — separate code path)

#### Implementation Plan

**Step 1 — key-wallet `WalletChangeSet` (dashcore repo):**

Create `rust-dashcore/key-wallet/src/changeset/` module:

```
key-wallet/src/changeset/
├── mod.rs
├── changeset.rs        // WalletChangeSet + sub-changesets
├── merge.rs            // Merge trait
└── traits.rs           // WalletPersistence trait (generic)
```

Define `Merge` trait, `WalletChangeSet`, all sub-changesets, and
`WalletPersistence` trait. key-wallet types use dashcore primitives
(`OutPoint`, `Txid`, `Transaction`, `BlockHash`, `Address`).

`check_core_transaction()` currently returns `TransactionCheckResult`
and mutates `ManagedWalletInfo` in place. Change it to ALSO return a
`WalletChangeSet` capturing what was mutated:
- `record_transaction()` → populate `transactions` + `utxos.added`
- `confirm_transaction()` → populate `transactions` status update
- `mark_utxos_instant_send()` → populate `utxos.instant_locked`
- `mark_address_used()` → populate `accounts.addresses_used`
- `maintain_gap_limit()` → populate `accounts.last_revealed`
- `update_balance()` → populate `balance`

Each of these methods currently returns void. Change each to return
a sub-changeset that the caller merges into the operation's
`WalletChangeSet`.

This is the **core refactor** — every mutation in key-wallet produces
a delta. The mutation still happens (in-memory state updated), but
the delta is also captured and returned to the caller.

**Step 2 — Refactor key-wallet mutations to return changesets (dashcore repo):**

For each mutation method in `ManagedCoreAccount` and `WalletTransactionChecker`:

```rust
// Before (mutates in place, returns nothing):
pub fn record_transaction(&mut self, tx: &Transaction, ...) -> TransactionRecord { ... }

// After (mutates in place AND returns delta):
pub fn record_transaction(&mut self, tx: &Transaction, ...) -> (TransactionRecord, WalletChangeSet) { ... }
```

Methods to change:
- `ManagedCoreAccount::record_transaction()` → return tx + UTXO deltas
- `ManagedCoreAccount::confirm_transaction()` → return status update delta
- `ManagedCoreAccount::mark_utxos_instant_send()` → return IS-lock delta
- `ManagedCoreAccount::mark_address_used()` → return address-used delta
- `AddressPool::maintain_gap_limit()` → return new-addresses delta
- `WalletTransactionChecker::check_core_transaction()` → aggregate all deltas from above
- `WalletTransactionChecker::update_balance()` → return balance delta

The `TransactionCheckResult` gains a `changeset: WalletChangeSet` field
that aggregates all sub-deltas from the operation.

**Step 3 — Rename platform-wallet changeset to PlatformWalletChangeSet:**

- Rename existing `WalletChangeSet` → `PlatformWalletChangeSet`
- Add `wallet: Option<key_wallet::WalletChangeSet>` field
- Update `Merge` impl to merge the `wallet` sub-changeset
- Update `stage_changeset()` / `persist()` to use `PlatformWalletChangeSet`
- Update SPV adapter to wrap key-wallet's changeset into platform changeset

**Step 4 — SPV adapter uses key-wallet changesets natively:**

Currently the SPV adapter reconstructs changesets from `TransactionCheckResult`.
After Step 2, it just takes the `result.changeset` field and wraps it:

```rust
let result = wi.check_core_transaction(tx, context, &mut w, true, true).await;
if result.state_modified {
    let platform_changeset = PlatformWalletChangeSet {
        wallet: Some(result.changeset),
        ..Default::default()
    };
    wallet.stage_changeset(platform_changeset);
}
```

No more manual TransactionEntry construction in the adapter.

**Step 5 — Contact/identity operations produce complete changesets:**

Each platform-wallet operation that mutates state returns a
`PlatformWalletChangeSet`:

```rust
// send_contact_request returns the complete delta:
pub async fn send_contact_request(&self, ...) -> Result<PlatformWalletChangeSet, Error> {
    let mut changeset = PlatformWalletChangeSet::default();

    // 1. Submit to Platform (external, no local state change)
    let result = self.sdk.send_contact_request(input, ...).await?;

    // 2. Record sent request → ContactChangeSet
    changeset.contacts = Some(ContactChangeSet { sent_requests: ... });

    // 3. Register account → AccountChangeSet (via key-wallet WalletChangeSet)
    let account_changeset = self.register_contact_account_changeset(...)?;
    changeset.wallet = Some(account_changeset);

    // 4. Store in IdentityManager → IdentityChangeSet
    changeset.identities = Some(IdentityChangeSet { ... });

    Ok(changeset)
}
```

Caller calls `apply_and_stage(changeset)` then `persist()`.

**Step 6 — Update SqlitePersister for PlatformWalletChangeSet:**

The existing `SqliteWalletPersister` is updated to:
- Persist `changeset.wallet` (key-wallet deltas: UTXOs, transactions, accounts)
- Persist `changeset.identities` (identity state)
- Persist `changeset.contacts` (contact requests, established)
- Persist `changeset.platform_addresses` (DIP-17 balances)
- Persist `changeset.asset_locks` (asset lock lifecycle)
- All in one SQLite transaction

**Step 7 — Remove old direct DB writes:**

- Remove `update_address_balance()`, `update_address_total_received()` direct calls
- Remove `replace_wallet_transactions()` direct calls
- Remove `insert_utxo()` / `drop_utxo()` direct calls
- Remove `reconcile_spv_wallets()` balance/UTXO writes (replaced by changeset flow)
- All persistence goes through `persist()` → `SqliteWalletPersister`

**Step 8 — Implement `initialize()` for startup:**

`SqliteWalletPersister::initialize()` loads all persisted state from DB
tables and returns a single `PlatformWalletChangeSet` representing the
full stored state. Platform-wallet applies it to rebuild in-memory state.

This replaces the current scattered DB loading in `get_wallets()`,
`load_wallet_transactions()`, etc.

#### File Structure

```
rust-dashcore/key-wallet/src/
├── persistence/
│   ├── mod.rs
│   ├── changeset.rs        // WalletChangeSet + UTXO/Tx/Account/Balance sub-changesets
│   ├── merge.rs            // Merge trait + impls for BTreeMap, BTreeSet, Option, Vec
│   └── traits.rs           // WalletPersistence trait (storage-agnostic)
└── ...

packages/rs-platform-wallet/src/
├── persistence/
│   ├── mod.rs
│   ├── changeset.rs        // PlatformWalletChangeSet (wraps key-wallet + platform deltas)
│   ├── merge.rs            // Merge trait (re-export from key-wallet + platform impls)
│   └── traits.rs           // PlatformWalletPersistence trait (extends key-wallet)
└── ...

dash-evo-tool/src/
├── persistence/
│   ├── mod.rs
│   └── sqlite.rs           // SqlitePersister implementing PlatformWalletPersistence
└── ...
```

---

## Address Type Coverage Summary

| Address type | DIP | Derivation path | key-wallet collection field | Plan section |
|---|---|---|---|---|
| Core UTXO receive | BIP44 | `m/44'/coin'/acct'/0/i` | `standard_bip44_accounts` | §1.3.2 |
| Core UTXO change | BIP44 | `m/44'/coin'/acct'/1/i` | `standard_bip44_accounts` | §1.3.2 |
| Identity reg. funding | DIP-9 | `m/9'/coin'/5'/1'/i` (non-hardened i) | `identity_registration` | §1.4.1 |
| Identity top-up funding | DIP-9 | `m/9'/coin'/5'/2'/i` (non-hardened i) | `identity_topup_not_bound` | §1.4.4 |
| Identity auth keys | DIP-9 | `m/9'/coin'/5'/0'/key_type'/id'/key'` | — | §1.4.1 |
| Auto-accept proof key | DIP-15 | `m/9'/coin'/16'/timestamp'` | — | §1.5.11 |
| DashPay receive from contact | DIP-15 | `m/9'/coin'/15'/0'/(self)/(friend)/i` | `dashpay_receival_accounts` | §1.5.3 |
| DashPay send to contact | DIP-15 | contact xpub + index | `dashpay_external_accounts` | §1.5.4 |
| Platform P2PKH (credits) | DIP-17 | `m/9'/coin'/17'/acct'/class'/i` | `platform_payment_accounts` | §1.6 |

---

## Risk Analysis

| Risk | Mitigation |
|---|---|
| `IdentityManager` fields not yet `Arc<RwLock<_>>`-wrapped | Refactor in PR-1; add `last_scanned_index` field; confirm tests pass |
| `AddressProvider` API mismatch — actual trait uses push-based callbacks, not `apply_balance()` | Use confirmed trait definition from `rs-sdk/src/platform/address_sync/provider.rs`; implement `pending_addresses`/`on_address_found`/`on_address_absent` |
| AES decryption bug in `add_incoming_contact_request` | Fix in PR-3 — `decrypt_extended_public_key` before `ExtendedPubKey::decode`; add unit test proving plaintext roundtrip |
| DIP-9 auth key path missing `key_type'` segment | Fix in PR-2 — use full path `m/9'/coin'/5'/0'/key_type'/identity_index'/key_index'`; note: existing deployed wallets may have used the old path (key_type' omitted = effectively key_type'=0') — document deviation |
| DIP-14 `ser_256(i)` endianness | Add unit test against DIP-14 Appendix A vectors before any contact request is submitted |
| BLS key derivation semantics | Use raw 32-byte seed from BIP32 derivation as BLS secret key (not scalar addition mod bls12381 group order) — matches DashSync iOS |
| DB migration corrupts existing wallets | Version byte in DB; fallback read → convert; test against real DB fixture |
| Asset lock proof: InstantLock timeout | Implement 60s timeout before falling back to ChainLock polling — confirm ChainLocked height is known to Platform before using Chain proof |
| `PlatformWallet` not `Send+Sync` | Add `static_assertions::assert_impl_all!(PlatformWallet: Send, Sync)` |
| `Arc<RwLock<ManagedWalletInfo>>` write starvation under concurrent SPV + Platform sync | SPV writes are short (tx update); Platform sync holds read lock briefly for balance reads — test under load |
| **Wallet + ManagedWalletInfo separation** — both are mutable (Wallet: accounts added during contact establishment; MWI: UTXOs/balances during sync). Original design assumed Wallet was immutable but it isn't. Two separate `RwLock`s create lock ordering risk and prevent atomic state updates. | Investigate merging in PR-6. Consider single struct behind one `RwLock`. |
| **Read starvation during block processing** — SPV `process_block()` holds write lock on both Wallet and ManagedWalletInfo for the entire block. During this time, CoreWallet read methods (`balance()`, `utxos()`, `all_address_info()`) are blocked. UI shows stale data until the block is fully processed. | Consider: (a) process transactions individually (release lock between txs), (b) use snapshot/MVCC pattern (clone state, process, swap), (c) accept the latency for now (blocks process in ms). |
| **Non-atomic state updates across structs** — Wallet, ManagedWalletInfo, and IdentityManager are separate structs behind separate locks. Operations that touch multiple (e.g., adding a DashPay account to Wallet + updating MWI addresses + updating IdentityManager contacts) cannot be atomic. A crash mid-operation leaves inconsistent state. | Investigate: (a) merge structs (PR-6), (b) WAL/journaling for multi-struct updates, (c) accept eventual consistency with recovery on restart. |
| `contactRequest` documents are immutable | Do not expose update/delete API; note in `send_contact_request` docs that retries create new documents |
| **`blocking_read()` deadlock risk in Signer::sign()** | DPP's `Signer` trait has sync `sign()` method but we use `tokio::sync::RwLock`. `blocking_read()` will deadlock if wallet write lock is held by same task. Document constraint: never call `sign()` while holding wallet write lock. Consider `std::sync::RwLock` for wallet in future. |
| **Signer code duplication** (IdentitySigner vs ManagedIdentitySigner) | Both have identical `sign()`/`sign_create_witness()`/`can_sign_with()` bodies. Extract shared `sign_with_key_bytes()` helper. Low priority — no correctness impact. |
| **ShieldedWallet spending ops incomplete** | `unshield()`, `transfer()`, `withdraw()` return runtime error — MerklePath witness deserialization not implemented. Output-only ops (`shield`, `shield_from_asset_lock`) work. Fix when integrating with evo-tool's SQLite `ClientPersistentCommitmentTree`. |
| **`rs-platform-wallet-ffi` broken type paths** | FFI crate references old type paths (`platform_wallet_info`, `identity_manager`, `managed_identity`) that were refactored. Fix in PR-19 by updating FFI imports to match new module structure. |
| **Auto-accept `account_reference` behavior change** | Platform-wallet uses `account_index` (0) as `account_reference`, not DIP-15 calculated value. Documented in evo-tool code. QR codes are session-scoped so old codes expire anyway. |

---

## Sources & References

### DIPs

- [DIP-0013: Identities in HD Wallets](https://github.com/dashpay/dips/blob/master/dip-0013.md) — auth, registration, top-up funding paths
- [DIP-0014: Extended Key Derivation (256-bit)](https://github.com/dashpay/dips/blob/master/dip-0014.md) — CKDpriv256/CKDpub256 spec and test vectors
- [DIP-0015: DashPay](https://github.com/dashpay/dips/blob/master/dip-0015.md) — contact request structure, ECDH, AES-CBC encryption, account reference, DashPay payment paths
- [DIP-0017: Dash Platform P2PKH Addresses](https://github.com/dashpay/dips/blob/master/dip-0017.md) — platform payment addresses at `m/9'/coin'/17'/account'/key_class'/index`

---

## TODO

- [x] **`manager` feature gates `PlatformWalletManager`** — DONE: manager module gated at lib.rs level.
- [ ] **Revisit events** — Remove fallback `WalletEvent` enum (only exists for `not(manager)` — is there a real use case without manager?). Remove duplicate `TransactionStatusChanged` from `PlatformWalletEvent` (already in `WalletEvent`). Review whether `TransactionStatus` enum is still needed or should use `TransactionContext` from dashcore.
- [ ] **Fix `rs-platform-wallet-ffi` broken type paths** — FFI crate references old module paths (`platform_wallet_info`, `identity_manager`, `managed_identity`) that were refactored. Update imports to match new module structure.
- [ ] **Signer code duplication** — `IdentitySigner` and `ManagedIdentitySigner` have identical `sign()`/`sign_create_witness()`/`can_sign_with()` bodies. Extract shared `sign_with_key_bytes()` helper.
- [ ] **ShieldedWallet spending ops** — `unshield()`, `transfer()`, `withdraw()` return runtime error. Need `MerklePath` witness resolution from `ShieldedStore`. Fix when integrating with evo-tool's SQLite `ClientPersistentCommitmentTree`.
- [ ] **Finality proof data** — `wait_for_finality()` returns `AssetLockProof::default()`. SPV `SyncEvent::InstantLockReceived` carries the actual `InstantLock` — use it to build proper proof.
- [ ] **Restore git rev dependency** — workspace Cargo.toml currently uses local path deps for dashcore. Restore `git = "..." rev = "..."` once cargo git cache issue is resolved.
- [ ] **`blocking_read()` deadlock risk** — `Signer::sign()` uses `blocking_read()` on tokio `RwLock`. Document constraint or consider `std::sync::RwLock` for wallet.
- [ ] **Expose wallet_info lock accessor** — CoreWallet getters each acquire the lock individually and clone data (e.g. `utxos()` clones entire BTreeSet). Add `pub async fn wallet_info() -> RwLockReadGuard<ManagedWalletInfo>` for callers who need multiple reads in one lock. Stop cloning in getters — return references via the guard. Not urgent: no current caller chains multiple getters.

---

### Key Repositories

| Repo | Disk Path | Notes |
| ---- | --------- | ----- |
| `rs-platform-wallet` | `packages/rs-platform-wallet/` | Target library (this plan) |
| `rs-platform-encryption` | `packages/rs-platform-encryption/` | DIP-15 crypto — already a dependency, do not duplicate |
| `rs-platform-wallet-ffi` | `packages/rs-platform-wallet-ffi/` | FFI layer — update exports in PR-5 |
| `key-wallet` | `../rust-dashcore/key-wallet/` | UTXO wallet, key derivation, TransactionBuilder, `WalletInterface` (manager feature) |
| `dash-spv` | `../rust-dashcore/dash-spv/` | SPV client, BIP157/158 sync, push-based |
| `rs-sdk` | `packages/rs-sdk/` | DAPI client (`Sdk`, `SdkBuilder`, `AddressProvider`) |
| `dash-evo-tool` | `../dash-evo-tool/` | Integration target |

### Platform Wallet (current — being replaced)

- [packages/rs-platform-wallet/src/platform_wallet_info/identity_discovery.rs](packages/rs-platform-wallet/src/platform_wallet_info/identity_discovery.rs) — consolidate into `IdentityWallet::sync()`
- [packages/rs-platform-wallet/src/platform_wallet_info/contact_requests.rs](packages/rs-platform-wallet/src/platform_wallet_info/contact_requests.rs) — consolidate into `DashPayWallet`; fix AES decryption bug
- [packages/rs-platform-wallet/src/platform_wallet_info/key_derivation.rs](packages/rs-platform-wallet/src/platform_wallet_info/key_derivation.rs) — fix `key_type'` path segment
- [packages/rs-platform-wallet/src/wallet/identity/managed_identity/mod.rs](packages/rs-platform-wallet/src/wallet/identity/managed_identity/mod.rs)
- [packages/rs-platform-wallet/src/wallet/dashpay/contact_request.rs](packages/rs-platform-wallet/src/wallet/dashpay/contact_request.rs)
- [packages/rs-platform-wallet/src/wallet/dashpay/established_contact.rs](packages/rs-platform-wallet/src/wallet/dashpay/established_contact.rs)

### SDK Transitions Used

**Identity**:
- `PutIdentity` trait — `packages/rs-sdk/src/platform/transition/put_identity.rs`
- `TopUpIdentity` trait — `packages/rs-sdk/src/platform/transition/top_up_identity.rs`
- `WithdrawFromIdentity` trait — `packages/rs-sdk/src/platform/transition/withdraw_from_identity.rs`
- `TransferToIdentity` trait — `packages/rs-sdk/src/platform/transition/transfer.rs`
- `TopUpIdentityFromAddresses` — fund identity from platform addresses
- `TransferToAddresses` — move identity credits to platform addresses

**Platform addresses**:
- `TransferAddressFunds` — transfer between platform addresses
- `WithdrawAddressFunds` — withdraw platform address credits to Core L1
- `TopUpAddress` — fund platform address from identity balance
- `AddressProvider` trait — `packages/rs-sdk/src/platform/address_sync/provider.rs`

**DashPay**:
- Contact requests — `packages/rs-sdk/src/platform/dashpay/contact_request.rs`

**DPNS**:
- `register_dpns_name`, `resolve_dpns_name_to_identity`

**Shielded** (feature-gated):
- `ShieldFunds`, `UnshieldFunds`, `TransferShielded`, `WithdrawShielded`, `ShieldFromAssetLock`

**Token transitions**:
- Transfer, mint, burn, freeze, purchase, claim, balance queries

**Signing**:
- `Signer<IdentityPublicKey>` — by value for withdraw/transfer
- `Signer<PlatformAddress>` — implemented on `PlatformAddressWallet`

### Evo Tool (to be replaced)

- `dash-evo-tool/src/model/wallet/mod.rs` — current `Wallet` struct (will be deleted in PR-1)
- `dash-evo-tool/src/app.rs` — `AppContext.wallets: RwLock<BTreeMap<WalletSeedHash, Arc<RwLock<Wallet>>>>`
- `dash-evo-tool/src/backend_task/dashpay/dip14_derivation.rs`
- `dash-evo-tool/src/backend_task/dashpay/hd_derivation.rs`
- `dash-evo-tool/src/backend_task/dashpay/encryption.rs`
- `dash-evo-tool/src/backend_task/identity/discover_identities.rs` — `AUTH_KEY_LOOKUP_WINDOW = 12`
- `dash-evo-tool/src/backend_task/wallet/fetch_platform_address_balances.rs`
