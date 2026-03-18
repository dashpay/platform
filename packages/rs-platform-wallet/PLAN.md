---
title: "feat: Platform Wallet — Complete Implementation & Evo Tool Integration"
type: feat
status: active
date: 2026-03-13
---

# feat: Platform Wallet — Complete Implementation & Evo Tool Integration

## Overview

**Goal**: Replace `dash-evo-tool`'s self-written wallet and duplicated DashPay crypto with `rs-platform-wallet`, building and integrating iteratively — one vertical slice at a time.

**Approach**: Each PR implements a feature in `rs-platform-wallet` **and** immediately wires it into `evo-tool`, replacing the corresponding old code. Both repos share a feature branch pair (`feat/platform-wallet` in each), linked via `path` dependency in Cargo.toml. No "build everything first, integrate later" — integration is part of every PR.

**Branch setup**:
- `platform` repo: `feat/platform-wallet` (feature branch, merges to `v3.1-dev` via PRs)
- `dash-evo-tool` repo: `feat/platform-wallet` (feature branch, merges to main via PRs)
- `Cargo.toml` in evo-tool: `platform-wallet = { path = "../../platform/packages/rs-platform-wallet" }`

**PR sequence** (each PR = library feature + evo-tool integration + old code deleted):

1. **PR-1** ✅: Project scaffold + `PlatformWallet` + `PlatformWalletManager` + `CoreWallet` + evo-tool bridge
2. **PR-2**: `CoreWallet` deep integration — signing, per-address data, asset locks, payment building + migrate evo-tool backend tasks fully
3. **PR-3**: `IdentityWallet` (register, discover, top-up, withdraw, transfer) → replace identity backend tasks
4. **PR-4**: `DashPayWallet` (DIP-14, DIP-15, contact requests, payments, sync) → replace dashpay backend tasks
5. **PR-5**: `PlatformAddressWallet` (DIP-17 sync, send, withdraw) → replace platform address backend task
6. **PR-6**: Merge `Wallet` and `ManagedWalletInfo` in `key-wallet` (dashcore) — both are mutable and always used together, having them as separate types behind separate locks adds unnecessary complexity. Single `Arc<RwLock<Wallet>>` containing all state.
7. **PR-7**: Serialization / persistence, remove old `wallets` map, delete `src/model/wallet/` + final cleanup

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
- Events: `PlatformWalletEvent`, `WalletEvent`, `SpvEvent`, `FinalityEvent`
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

### Blockers for deeper migration (PR-2 scope)

1. **Signing**: `Signer<PlatformAddress>` only implemented for the old `Wallet` model. PlatformWallet needs its own signing capability.
2. **Per-address data**: CoreWallet exposes aggregate balance and flat UTXO lists. Old model tracks per-address balances, derivation metadata, asset locks, platform address info.
3. **Sync/async mismatch**: UI runs synchronously (egui immediate mode), CoreWallet methods are async. Needs caching layer or backend-task-based data flow.
4. **Asset lock transactions**: `create_asset_lock_proof()` requires porting ~600 lines of transaction building from evo-tool.
5. **Payment building**: `send_transaction()` requires coin selection, signing, broadcast via SPV or RPC.
6. **SPV lifecycle**: `start_spv()` / `stop_spv()` are stubs — need network config wiring.

---

## Problem Statement

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

## Architecture

```
key-wallet (rust-dashcore) — reused types, NO WalletManager
├── Wallet                       ← immutable key store (mnemonic, xprv, accounts)
├── ManagedWalletInfo            ← mutable UTXO state, accounts, balance
├── ManagedAccountCollection     ← BIP44 + DashPay + PlatformPayment accounts
├── TransactionRouter            ← transaction classification + checking
└── WalletTransactionChecker     ← trait for tx matching (impl on ManagedWalletInfo)

rs-platform-wallet (target)
├── PlatformWallet               ← standalone wallet, sub-wallets as stored fields
│   ├── sdk:      Sdk
│   ├── wallet:   Arc<Wallet>                    ← immutable key store
│   ├── core:     CoreWallet                     ← Arc<RwLock<ManagedWalletInfo>> inside
│   ├── identity: IdentityWallet                 ← shares wallet_info Arc + IdentityManager
│   ├── dashpay:  DashPayWallet                  ← shares wallet_info Arc + IdentityManager
│   └── platform: PlatformAddressWallet          ← shares wallet_info Arc
│
├── PlatformWalletManager        ← multi-wallet + SPV coordinator
│   ├── sdk:        Sdk
│   ├── network:    Network
│   ├── wallets:    RwLock<BTreeMap<WalletId, PlatformWallet>>  ← lock only for add/remove wallet
│   ├── spv_client: Option<DashSpvClient<...>>
│   └── implements WalletInterface for SPV using key-wallet functions directly
│       .create_wallet_from_mnemonic() / .import_wallet_from_xprv() / ... → WalletHandle
│
└── WalletHandle                 ← cheap cloneable token, holds sub-wallet clones
    ├── wallet_id:  WalletId
    ├── core:       CoreWallet            ← cloned at creation (Arc fields — cheap)
    ├── identity:   IdentityWallet        ← cloned at creation
    ├── dashpay:    DashPayWallet         ← cloned at creation
    └── platform:   PlatformAddressWallet ← cloned at creation
        .identity() / .dashpay() / .platform() / .core()  ← sync access, no lock needed

rs-sdk (Dash Platform SDK)
├── Identity::fetch() / topup / withdraw / transfer / register
├── Sdk::send_contact_request() / fetch_all_contact_requests_for_identity()
├── sync_address_balances() → DIP-17 address sync
└── WithdrawAddressFunds / TransferAddressFunds / TopUpAddress
```

**Key design decisions:**
- **No WalletManager<T>**: `PlatformWalletManager` implements `WalletInterface` directly using
  `key-wallet` types (`TransactionRouter`, `WalletTransactionChecker`, `ManagedWalletInfo`).
- **Sub-wallets share state via Arc**: All sub-wallets hold `Arc<RwLock<ManagedWalletInfo>>` and
  `Arc<Wallet>`. SPV writes to `ManagedWalletInfo` through the Arc — visible to `WalletHandle`'s
  cloned sub-wallets immediately. No outer per-wallet lock needed.
- **Single map lock**: `RwLock<BTreeMap<WalletId, PlatformWallet>>` is locked only for wallet
  add/remove. Sub-wallets handle their own concurrency via inner `Arc<RwLock<MWI>>`.
- **WalletHandle holds sub-wallet clones**: Cloned at creation (all Arc fields — cheap).
  Sync access, no await needed: `handle.identity().register_identity(...).await?`
- **Standalone + managed**: Same `PlatformWallet` type for both. Standalone uses `&self`/`&mut self`
  directly. Managed clones sub-wallets into `WalletHandle`.
- **No dashcore changes**: Only `key-wallet` crate types are used directly. `key-wallet-manager`
  (`WalletManager<T>`) is not a dependency.

---

## Implementation Plan

`PlatformWallet` is a standalone wallet type (usable without SPV/manager).
`PlatformWalletManager` is the multi-wallet + SPV coordinator (no `WalletManager<T>` dependency).
`WalletHandle` is a cheap per-wallet token returned by the manager.

### Struct Definitions

```rust
// Standalone wallet — owns all state, sub-wallets as stored fields
// Usable directly for Platform-only operations (scripts, tests, no SPV needed)
// Same type is wrapped in per-wallet RwLock when managed by PlatformWalletManager
pub struct PlatformWallet {
    sdk:      Sdk,          // cheaply cloneable (ref-counted)
    wallet:   Arc<Wallet>,  // immutable key store
    core:     CoreWallet,
    identity: IdentityWallet,
    dashpay:  DashPayWallet,
    platform: PlatformAddressWallet,
}

// Sub-wallets — stored fields, share wallet_info via Arc<RwLock<ManagedWalletInfo>>
pub struct CoreWallet {
    sdk:         Sdk,
    wallet:      Arc<Wallet>,
    wallet_info: Arc<RwLock<ManagedWalletInfo>>,
}

pub struct IdentityWallet {
    sdk:              Sdk,
    wallet:           Arc<Wallet>,
    wallet_info:      Arc<RwLock<ManagedWalletInfo>>,
    identity_manager: IdentityManager,
}

pub struct DashPayWallet {
    sdk:              Sdk,
    wallet:           Arc<Wallet>,
    wallet_info:      Arc<RwLock<ManagedWalletInfo>>,
    identity_manager: IdentityManager,  // same instance as IdentityWallet (Arc clone)
}

pub struct PlatformAddressWallet {
    sdk:         Sdk,
    wallet:      Arc<Wallet>,
    wallet_info: Arc<RwLock<ManagedWalletInfo>>,
}

// Multi-wallet + SPV coordinator — no WalletManager<T> dependency
// Implements WalletInterface for SPV using key-wallet functions directly
pub struct PlatformWalletManager {
    sdk:        Sdk,
    network:    Network,
    wallets:    RwLock<BTreeMap<WalletId, PlatformWallet>>,  // lock only for add/remove
    spv_client: Option<DashSpvClient<Self, N, S>>,  // None until start_spv(); N=NetworkManager, S=Storage — concrete types TBD
    event_tx:   broadcast::Sender<PlatformWalletEvent>,
    synced_height: AtomicU32,
}

// Cheap cloneable token per loaded wallet — holds sub-wallet clones (all Arc fields)
// Created by PlatformWalletManager, lives independently — no lock needed for access
pub struct WalletHandle {
    wallet_id: WalletId,
    core:      CoreWallet,
    identity:  IdentityWallet,
    dashpay:   DashPayWallet,
    platform:  PlatformAddressWallet,
}

// IdentityManager is shared between IdentityWallet and DashPayWallet.
// Implements Clone — all fields are cheap to clone (IndexMap is cloned by value,
// but since both sub-wallets hold their own copy via Arc<IdentityManager> or by
// wrapping the mutable fields, sharing is handled at the sub-wallet level).
// For concurrent access: IdentityWallet and DashPayWallet share the same IdentityManager
// instance because PlatformWallet constructs them from the same source at build time.
// WalletHandle clones sub-wallets which clone the IdentityManager (same Arc references inside).
pub struct IdentityManager {
    identities:          Arc<RwLock<IndexMap<Identifier, ManagedIdentity>>>,
    primary_identity_id: Arc<RwLock<Option<Identifier>>>,
    last_scanned_index:  Arc<RwLock<u32>>,  // NEW — not yet present; persisted gap scan state
    // REMOVED: sdk: Option<Arc<Sdk>> — SDK moves to PlatformWallet
}
// Clone is cheap — just Arc clones. IdentityWallet and DashPayWallet hold
// the same Arc pointers — mutations visible to both.
```

**No dashcore changes required.** Only `key-wallet` crate types are used directly (`Wallet`,
`ManagedWalletInfo`, `ManagedAccountCollection`, `TransactionRouter`, `WalletTransactionChecker`).
The `key-wallet-manager` crate (`WalletManager<T>`) is not a dependency.

**Concurrency model**: Sub-wallets share `Arc<RwLock<ManagedWalletInfo>>` — this is the synchronization
point between SPV (writes UTXO state) and wallet operations (reads balance, builds transactions).
No outer per-wallet lock needed. The manager's `RwLock<BTreeMap>` is only for wallet add/remove.

**`WalletHandle` lifecycle**: Holds cloned sub-wallets (Arc fields). After creation, it's independent
of the manager. Removing a wallet from the manager doesn't invalidate outstanding handles — they
continue to work (same Arcs). SPV updates to `ManagedWalletInfo` are visible through the shared Arc.

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
```

`PlatformWalletManager` API — mirrors dashcore wallet creation methods, uses `key-wallet` types directly:

```rust
impl PlatformWalletManager {
    // Construction
    pub fn new(sdk: Sdk, spv_config: ClientConfig, network: Network) -> Self;

    // Wallet creation — uses key-wallet's Wallet + ManagedWalletInfo directly
    pub async fn create_wallet_from_mnemonic(
        &self, mnemonic: &str, passphrase: &str,
        birth_height: CoreBlockHeight,
        account_options: WalletAccountCreationOptions,
    ) -> Result<WalletHandle>;

    pub async fn create_wallet_with_random_mnemonic(
        &self,
        account_options: WalletAccountCreationOptions,
    ) -> Result<(WalletHandle, Mnemonic)>;

    pub async fn import_wallet_from_xprv(
        &self, xprv: &str,
        account_options: WalletAccountCreationOptions,
    ) -> Result<WalletHandle>;

    pub async fn import_wallet_from_xpub(
        &self, xpub: &str, can_sign_externally: bool,
    ) -> Result<WalletHandle>;

    // Wallet restoration
    pub async fn import_wallet_from_bytes(
        &self, wallet_bytes: &[u8],
    ) -> Result<WalletHandle>;

    // Wallet lifecycle
    pub async fn remove_wallet(&self, wallet_id: &WalletId) -> Result<PlatformWallet>;

    // Wallet access
    pub async fn get_wallet_handle(&self, wallet_id: &WalletId) -> Option<WalletHandle>;
    pub async fn list_wallets(&self) -> Vec<WalletId>;

    // SPV lifecycle
    pub async fn start_spv(&mut self) -> Result<()>;
    pub async fn stop_spv(&mut self) -> Result<()>;

    // Events — unified stream, grouped by source channel
    pub fn subscribe_events(&self) -> broadcast::Receiver<PlatformWalletEvent>;
}

// Unified event enum — variants per source channel
pub enum PlatformWalletEvent {
    Wallet(WalletEvent),       // from block processing (TransactionReceived, BalanceUpdated)
    Spv(SpvEvent),             // from DashSpvClient (SyncProgress, PeerConnected, PeerDisconnected)
    Finality(FinalityEvent),   // InstantLock / ChainLock
}
```

`WalletHandle` holds sub-wallet clones — sync access, no locks:

```rust
impl WalletHandle {
    pub fn core(&self)     -> &CoreWallet            { &self.core }
    pub fn identity(&self) -> &IdentityWallet        { &self.identity }
    pub fn dashpay(&self)  -> &DashPayWallet         { &self.dashpay }
    pub fn platform(&self) -> &PlatformAddressWallet { &self.platform }
}
```

Call sites — standalone `PlatformWallet`:

```rust
let wallet = PlatformWallet::from_mnemonic(sdk, network, "word1 ...", "", 1_500_000, options)?;
wallet.identity().register_identity(amount, keys).await?;
wallet.dashpay().send_contact_request(sender, recipient).await?;
wallet.core().balance();
```

Call sites — managed via `WalletHandle` (same API, no awaits on accessors):

```rust
let handle = mgr.create_wallet_from_mnemonic("...", "", height, options).await?;
handle.identity().register_identity(amount, keys).await?;
handle.dashpay().sync().await?;
handle.core().balance();
```

`sync()` on `WalletHandle` orchestrates Platform-side syncs (SPV runs independently in background):

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
lives in `PlatformWalletManager`.

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

// Multi-wallet with SPV — use PlatformWalletManager (same creation signatures)
let mgr = PlatformWalletManager::new(sdk, spv_config, network);
let handle = mgr.create_wallet_from_mnemonic(
    "word1 word2 ...", "", 1_500_000,
    WalletAccountCreationOptions::Default,
).await?;
mgr.start_spv().await?;
```

**Internally**: each creation method calls `key-wallet`'s `Wallet::from_mnemonic()` (etc.) to create the
immutable key store, then `ManagedWalletInfo::from_wallet()` for UTXO state, then wraps both with
`IdentityManager::new()` into a `PlatformWallet`.

**`WalletAccountCreationOptions`**: always required (matches dashcore). Callers pass
`WalletAccountCreationOptions::Default` for standard BIP-44 account 0 + identity + DIP-17 accounts.

**Birth height**: passed through to `ManagedWalletInfo::with_birth_height()` — used by SPV
to skip earlier blocks when loaded into `PlatformWalletManager`. Defaults to 0 (full sync).

#### Files

- `packages/rs-platform-wallet/src/wallet/platform_wallet.rs` (new — replaces `platform_wallet_info/mod.rs`)
- `packages/rs-platform-wallet/src/platform_wallet_manager/mod.rs` (new)
- `packages/rs-platform-wallet/src/wallet_handle/mod.rs` (new)

#### Migration

The old `platform_wallet_info/` module (currently staged as deleted in git) must be fully removed.
`lib.rs` currently still imports `pub mod platform_wallet_info` — update to `pub mod platform_wallet`.

---

### 1.2 Platform SDK Integration

> Sdk lives in `PlatformWallet` and `WalletHandle` — never in `IdentityManager`.

**Current state**: SDK is stashed inside `IdentityManager.sdk: Option<Arc<Sdk>>` — accessed only by identity
discovery. Every async method that submits state transitions requires the caller to pass `&Sdk` separately.

**Goal**: `PlatformWallet` holds `sdk: Sdk` as a plain field (cheaply cloneable via internal ref-counting —
confirmed at `rs-sdk/src/sdk.rs:134`). `WalletHandle` clones it at load time. All async methods on
sub-structs call `self.sdk` internally.

#### Tasks

- **1.2.1** Add `sdk: Sdk` to `PlatformWallet`. All sub-structs (built on-the-fly from `WalletHandle`) receive it via the handle's `sdk` field.
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
by `PlatformWalletManager`, not `CoreWallet` — see §1.3.5.)

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

#### 1.3.4 — Transaction Send

key-wallet only **builds** transactions — it has no send method. Broadcasting is a
separate concern (RPC or SPV). `CoreWallet` exposes `TransactionBuilder` directly
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

`send_transaction` handles coin selection, signing, and broadcast internally — two broadcast paths:

- **SPV mode**: `DashSpvClient::broadcast_transaction(tx)` → P2P to connected peers
  (`dash-spv/src/client/transactions.rs`)
- **RPC mode**: `core_client.send_raw_transaction(tx)` → Dash Core JSON-RPC

`rs-sdk` (DAPI/Platform SDK) has no Core transaction broadcast — it's Platform-only.
The SPV client (`DashSpvClient`) is the P2P layer for Core transactions.

#### 1.3.5 — SPV Sync Integration

`dash-spv` (`DashSpvClient<W, N, S>`) is the P2P sync layer. It uses **BIP157/158 compact
block filters** (not Bloom filters). It accepts `Arc<RwLock<W: WalletInterface>>`.

**`WalletInterface` is implemented by `PlatformWalletManager` directly** — no `WalletManager<T>`
dependency. `PlatformWalletManager` uses `key-wallet` types (`TransactionRouter`,
`WalletTransactionChecker` trait on `ManagedWalletInfo`) to process blocks.

SPV lives in `PlatformWalletManager`, not in `PlatformWallet`. `PlatformWallet` is SPV-free.

**Wiring** (`PlatformWalletManager::start_spv()`):

```rust
// PlatformWalletManager implements WalletInterface — pass Arc<RwLock<Self>> to SPV client
let spv = DashSpvClient::new(spv_config, net_manager, storage, self_arc).await?;
```

**Block processing call chain**:

```
DashSpvClient
  → PlatformWalletManager::process_block()       // WalletInterface impl
  → wallets.read() → iterate wallets
  → for each wallet:
    → wallet.core.wallet_info.write()             // Arc<RwLock<MWI>> — inner lock
    → check_core_transaction(tx, ...)             // WalletTransactionChecker (key-wallet)
    → ManagedWalletInfo state mutated
    → PlatformWalletEvent::Wallet(...) emitted
```

**`PlatformWalletEvent`** (unified enum):
- `Wallet(WalletEvent)` — `TransactionReceived`, `BalanceUpdated`
- `Spv(SpvEvent)` — sync progress, peer connections
- `Finality(FinalityEvent)` — InstantLock, ChainLock

**Event subscription**:
```rust
let rx: broadcast::Receiver<PlatformWalletEvent> = mgr.subscribe_events();
```

**`WalletInterface` methods** (implemented on `PlatformWalletManager`):
- `process_block` — iterates wallets, locks each `wallet_info`, calls `check_core_transaction` per tx
- `monitored_addresses` — collects from all wallets' `ManagedWalletInfo`
- `synced_height` / `update_synced_height` — tracks via `AtomicU32`, updates each wallet
- `subscribe_events` — returns `broadcast::Receiver<WalletEvent>` (trait requirement for SPV)

**Two event channels**: `WalletInterface::subscribe_events()` returns `WalletEvent` (for SPV).
`PlatformWalletManager::subscribe_events()` (public API) returns `PlatformWalletEvent` which
wraps `WalletEvent` + `SpvEvent` + `FinalityEvent`. Internally, the manager forwards `WalletEvent`s
into the `PlatformWalletEvent` channel as `PlatformWalletEvent::Wallet(event)`.

**No reorg notification**: `WalletInterface` has no `process_reorg` method — reorgs are handled
only at the `ChainTipManager` level in dash-spv; the wallet is never notified.

Note: `key-wallet-manager` will be merged into `key-wallet` — this is a packaging change only,
no API impact. Feature gate `feature = "manager"` in `Cargo.toml` may change accordingly.

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

`CoreWallet` method — derives the next DIP-13 funding key internally, sources UTXOs
from `wallet_info`, builds an `AssetLock` special transaction via `TransactionBuilder`,
broadcasts it, waits for the InstantLock via SPV, returns `(AssetLockProof, funding_private_key)`.

**Two proof types** (both fully implemented in rs-dpp):
- `AssetLockProof::Instant` — wraps InstantLock + full transaction + output index. Primary path.
- `AssetLockProof::Chain` — wraps `core_chain_locked_height` + outpoint. Fallback if InstantLock
  is not received within timeout (suggest 60s, matching DashSync iOS behaviour).

**Important**: The fallback to `AssetLockProof::Chain` requires the referenced block height to be
ChainLocked from Platform's perspective. The wallet must poll block confirmation before using
a Chain proof.

DIP-13 funding key paths:
- Registration: `m/9'/coin'/5'/1'/identity_index` (non-hardened terminal index)
- Top-up (unbound): `m/9'/coin'/5'/2'/topup_index` (non-hardened terminal)
- Top-up (bound): `m/9'/coin'/5'/2'/registration_index'/topup_index`

**Note**: `ManagedAccountCollection` has dedicated fields for these:
`identity_registration: Option<ManagedCoreAccount>`,
`identity_topup: BTreeMap<u32, ManagedCoreAccount>`,
`identity_topup_not_bound: Option<ManagedCoreAccount>`.

#### 1.3.7 — Asset Lock Recovery

```rust
pub async fn recover_asset_locks(&self) -> Result<Vec<RecoveredAssetLock>, CoreWalletError>
```

Scans known funding key paths for broadcast-but-unconfirmed asset lock transactions
and attempts to recover or rebroadcast them. Mirrors evo-tool's
`CoreTask::RecoverAssetLocks`.

#### Files

- `packages/rs-platform-wallet/src/wallet/core/wallet.rs` (new)
- Depends on: `key-wallet` (`ManagedWalletInfo`, `TransactionBuilder`, `WalletInfoInterface`,
  `ManagedAccountOperations`, `FeeRate`, `SelectionStrategy`)
- Depends on: `key-wallet-manager` (feature = "manager") — `WalletInterface` trait
- Depends on: `dash-spv` (`broadcast_transaction`, InstantLock/ChainLock events)

---

### 1.4 Identity Management

> Register, discover, refresh, top-up, withdraw, transfer, update identities. Register DPNS names.

All methods are on `IdentityWallet` which holds `sdk`, `wallet: Arc<Wallet>`, and `identity_manager`.
No `wallet: &Wallet` parameter anywhere — key derivation and signing use `self.wallet` directly.

**SDK method surface** (confirmed from `rs-sdk` source — these are trait methods on `Identity`, not on `Sdk`):
- `Identity::put_to_platform_and_wait_for_response(sdk, asset_lock_proof, private_key, signer, settings)` — `PutIdentity` trait
- `identity.top_up_identity(sdk, asset_lock_proof, private_key, user_fee_increase, settings) -> Result<u64>` — `TopUpIdentity` trait
- `identity.withdraw(sdk, address, amount, core_fee_per_byte, signing_key, signer, settings) -> Result<u64>` — `WithdrawFromIdentity` trait
- `identity.transfer_credits(sdk, to_identity_id, amount, signing_key, signer, settings) -> Result<(u64, u64)>` — `TransferToIdentity` trait

#### 1.4.1 — Register New Identity

```rust
pub async fn register_identity(
    &mut self,
    amount_duffs: u64,
    key_types: &[IdentityKeySpec],
) -> Result<Identity, PlatformWalletError>
```

Steps:

1. `self.core.create_asset_lock_proof(amount_duffs)` → `(AssetLockProof, funding_private_key)`
   (next identity index tracked internally, derives `m/9'/coin'/5'/1'/identity_index`)
2. Derive auth keys from `m/9'/coin'/5'/0'/key_type'/identity_index'/key_index'` via `self.wallet`
3. Build and sign `IdentityCreateTransition` via `PutIdentity::put_to_platform_and_wait_for_response()`
4. Broadcast, wait for proof, add to `identity_manager`

**DIP-13 key path note**: The full path is `m/9'/coin'/5'/0'/key_type'/identity_index'/key_index'`
where `key_type` is: `0'` = ECDSA, `1'` = BLS. The existing `key_derivation.rs` omits the
`key_type'` segment — this must be fixed. The `key_type'` level enables multi-algorithm keys
under the same identity index.

#### 1.4.2 — Identity Discovery (DIP-13 gap-limit scan)

Implementation exists in the old `platform_wallet_info/identity_discovery.rs`.
Current behaviour:

- Derives ECDSA auth key at `key_index=0` only
- Queries Platform via `Identity::fetch(&sdk, PublicKeyHash(key_hash))` — unique key hash
- `start_index` and `gap_limit` passed by caller — state not persisted
- SDK pulled from `IdentityManager.sdk` (stale pattern)
- Errors during fetch silently treated as misses

**What needs fixing:**

- Move to `IdentityWallet::sync()`, no parameters
- Store `last_scanned_index: u32` in `IdentityManager` — persist and resume from it
- Gap limit hardcoded to 5 (implementation convention — DIP-13 does not specify a gap limit value; 5 matches the registration-funding bloom filter batch size and is a safe conservative choice)
- Consider scanning multiple key indices per identity index: evo-tool's `discover_identities.rs` uses `AUTH_KEY_LOOKUP_WINDOW = 12` — scanning 12 consecutive key indices per identity index provides more robust discovery for wallets with non-sequential key usage
- Use `PublicKeyHash` (unique lookup) — correct for authentication keys, one identity per key hash
- Surface fetch errors properly
- SDK sourced from `self.sdk` on `IdentityWallet`

```rust
pub async fn sync(&self) -> Result<Vec<Identifier>, PlatformWalletError>
```

#### 1.4.3 — Refresh Identity

```rust
pub async fn refresh_identity(
    &mut self,
    identity_id: &Identifier,
) -> Result<(), PlatformWalletError>
```

Fetches latest balance and keys from Platform, updates `ManagedIdentity`.

#### 1.4.4 — Top Up Identity Credits

```rust
pub async fn top_up_identity(
    &mut self,
    identity_id: &Identifier,
    amount_duffs: u64,
) -> Result<u64, PlatformWalletError>  // returns new balance
```

Steps:

1. `self.core.create_asset_lock_proof(amount_duffs)` — derives next top-up key internally
2. Call `identity.top_up_identity(&self.sdk, asset_lock_proof, private_key, None, None)` — `TopUpIdentity` trait
3. Update `ManagedIdentity` balance

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
Signs using `IdentitySigner` (see §1.7).

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

#### Files

- `packages/rs-platform-wallet/src/wallet/identity/wallet.rs` (new)
- Consolidates: `platform_wallet_info/identity_discovery.rs`, `platform_wallet_info/key_derivation.rs`

---

### 1.5 DashPay — Contacts, Transactions, Sync

> Full DIP-14/15 implementation: contact requests, encrypted xpub exchange, payment address
> derivation, send/receive Dash between contacts.

#### DIP-14 Background

DashPay uses 256-bit derivation (CKDpriv256/CKDpub256) for contact-specific address spaces:

```
m(userA)/9'/5'/15'/0'/(userA_id_256bit)/(userB_id_256bit)/index
```

The 256-bit identity ID indices prevent the 31-bit collision attack. `CKDpriv256` is fully
compatible with BIP32 for indices < 2^32; uses `ser_256(i)` (big-endian, 32 bytes) for larger indices.

**Current state**: Lives in `dash-evo-tool/src/backend_task/dashpay/dip14_derivation.rs`.
Moves to `packages/rs-platform-wallet/src/platform_wallet/dashpay/dip14.rs`.

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

#### 1.5.1 — DIP-14 Key Derivation (dashpay module)

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

Consolidate from `platform_wallet_info/contact_requests.rs::send_contact_request()`:

```rust
pub async fn send_contact_request(
    &mut self,
    sender_identity_id: &Identifier,
    recipient_identity: &Identity,
    account_index: u32,
    auto_accept_proof: Option<Vec<u8>>,
    signing_key_index: u32,
) -> Result<Identifier, PlatformWalletError>  // document id
```

Steps:

1. Find sender ENCRYPTION key at `signing_key_index`
2. Find recipient first DECRYPTION key (purpose = `DECRYPTION`, not `ENCRYPTION`)
3. Derive contact xpub via DIP-14: `derive_dashpay_contact_xpub(..., sender_id, recipient_id)`
4. ECDH shared key: `derive_shared_key_ecdh(sender_privkey, recipient_pubkey)`
5. Encrypt xpub: `encrypt_extended_public_key(&xpub, &shared_key)` → 96 bytes
6. Compute `accountReference` via `compute_account_reference(account, sender_key_bytes, xpub_bytes, version=0)`
7. Submit via `sdk.send_contact_request()` (SDK method with `EcdhProvider` closure)
8. Store in `ManagedIdentity.sent_contact_requests`
9. Add `DashpayReceivingFunds` account to `ManagedAccountCollection`

**Note**: `contactRequest` documents are immutable — no retry/update API. If submission fails, it's a new request.

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

#### Files

- `packages/rs-platform-wallet/src/platform_wallet/dashpay/dip14.rs` (new — DIP-14 CKDpriv256/CKDpub256)
- `packages/rs-platform-wallet/src/platform_wallet/dashpay/mod.rs` (new — consolidates `platform_wallet_info/contact_requests.rs`)
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

**Actual `AddressProvider` trait** (confirmed from `rs-sdk/src/platform/address_sync/provider.rs`):

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

`PlatformAddress` signing requires the private key at its DIP-17 derivation index:

```rust
pub struct PlatformAddressSigner {
    wallet:          Arc<Wallet>,
    address_key_map: BTreeMap<PlatformP2PKHAddress, DerivationPath>,
}

impl Signer<PlatformAddress> for PlatformAddressSigner { ... }
```

Factory on `PlatformAddressWallet` — borrows `self.wallet`:

```rust
pub fn platform_address_signer(
    &self,
    addresses: &[PlatformP2PKHAddress],
) -> Result<PlatformAddressSigner, PlatformWalletError>
```

#### Files

- `packages/rs-platform-wallet/src/platform_wallet/platform_addresses.rs` (new)
- `packages/rs-platform-wallet/src/platform_wallet/platform_address_signer.rs` (new)

---

### 1.7 State Transition Signing Facade

> `PlatformWallet` provides `IdentitySigner` so callers never manage key material directly.

```rust
// platform_wallet/signer.rs
pub struct IdentitySigner {
    wallet:         Arc<Wallet>,
    identity_index: u32,
}

impl Signer<IdentityPublicKey> for IdentitySigner {
    fn sign(&self, key: &IdentityPublicKey, data: &[u8]) -> Result<Vec<u8>>
    // Derives private key from wallet Arc using key.id() + key.key_type()
}
```

Factory on `IdentityWallet` — no external `wallet` param, borrows from `self.wallet`:

```rust
pub fn signer_for_identity(
    &self,
    identity_id: &Identifier,
) -> Result<IdentitySigner, PlatformWalletError>
```

#### Files

- `packages/rs-platform-wallet/src/platform_wallet/signer.rs` (new)

---

### 1.8 Serialization / Persistence

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

### 1.9 Sync Architecture

There are **two distinct sync mechanisms** with different lifecycles:

#### Core chain sync — push-based, long-running

`dash-spv` runs as a permanent background task started once at app startup. It pushes
blocks and transactions to `CoreWallet` via `WalletInterface` callbacks — no polling needed:

```rust
// App startup — spawned once, runs until cancellation
tokio::spawn(async move {
    spv_client.run(cancellation_token).await
});
// dash-spv calls CoreWallet::process_block() reactively as blocks arrive
```

#### Platform sync — poll-based, periodic

Platform state (identities, contacts, credit balances) is fetched via DAPI on a timer.
`PlatformWallet::sync()` is the single entry point:

```rust
pub async fn sync(&self) -> Result<SyncResult, PlatformWalletError>
```

Sync order:

1. `self.identity.sync()` — DIP-13 gap scan for new identities
2. `self.dashpay.sync()` — contact requests for all known identities
3. `self.platform.sync()` — DIP-17 address credit balances via DAPI

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
- `PlatformWalletManager` implements `WalletInterface` directly using `key-wallet` types (`TransactionRouter`, `WalletTransactionChecker`) — no `WalletManager<T>` dependency (§1.3.5)
- `WalletHandle`: holds cloned sub-wallets (all Arc fields), sync access, no locks needed
- `PlatformWalletEvent` unified enum: `Wallet(WalletEvent)`, `Spv(SpvEvent)`, `Finality(FinalityEvent)`
- `monitored_addresses()` returns ALL account types including `dashpay_receival_accounts`
- `send_transaction`, `broadcast_transaction`, asset lock proof creation (§1.3.4–1.3.6)
- Asset lock timeout/fallback: 60s InstantLock wait, then ChainLock polling
- `IdentitySigner` stub (§1.7) — needed for identity registration in PR-2
- `static_assertions::assert_impl_all!(PlatformWallet: Send, Sync)`
- `IdentityManager` refactor: add `last_scanned_index`, remove `sdk` field

**evo-tool integration**:

- Add `platform-wallet = { path = "../../platform/packages/rs-platform-wallet" }` to `Cargo.toml`
- Replace `AppContext.wallets` + `SpvManager` with `PlatformWalletManager`
- `wallet_lifecycle.rs`: construct via `PlatformWallet::from_mnemonic()` / `from_xprv()`, wire `sdk` from `AppContext.sdk`
- SPV: `PlatformWalletManager::start_spv()` replaces manual `SpvManager` setup
- `WalletHandle` replaces `WalletSeedHash` as wallet accessor
- Delete `src/model/wallet/` (old custom wallet struct)

**Database migration** (in this PR):

- Add version byte to DB wallet record
- If old format: deserialize as old `Wallet`, convert to `PlatformWallet`, re-save
- On first run after migration: `IdentityManager` starts empty — identities re-discovered in PR-2

**Done when**: evo-tool builds with `PlatformWalletManager`; SPV sync works via `WalletInterface` impl; `send_transaction` works; `WalletHandle` provides sync access to sub-wallets.

---

### PR-2: IdentityWallet

**Library** (`rs-platform-wallet`):

- `IdentityWallet` with `identity_manager`, sdk, wallet Arc (§1.4)
- `register_identity` (with corrected `m/9'/coin'/5'/0'/key_type'/identity_index'/key_index'` path), `sync()`, `refresh_identity` (§1.4.1–1.4.3)
- Identity discovery: gap limit 5, consider AUTH_KEY_LOOKUP_WINDOW = 12 for key index scanning
- `top_up_identity`, `withdraw_identity_credits`, `transfer_credits` (§1.4.4–1.4.6)
- `add_key_to_identity`, `disable_identity_key` (§1.4.7)
- `IdentitySigner` complete (§1.7)
- `IdentityManager` bincode serialization (§1.8 partial)
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

### PR-3: DashPayWallet (DIP-14 + DIP-15 + Sync)

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
- `ManagedIdentity` contact maps + `ContactRequest` + `EstablishedContact` bincode (§1.8)

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

### PR-4: PlatformAddressWallet (DIP-17)

**Library** (`rs-platform-wallet`):

- `PlatformAddressWallet` with actual `AddressProvider` impl — push-based callbacks (`pending_addresses`, `on_address_found`, `on_address_absent`) (§1.6.1)
- `sync_platform_address_balances`, balance accessors (§1.6.2–1.6.3)
- `top_up_platform_address`, `transfer_platform_address_funds`, `withdraw_platform_address_funds` (§1.6.4–1.6.6)
- `PlatformAddressSigner` (§1.6.7)

**evo-tool integration**:

- `backend_task/wallet/fetch_platform_address_balances.rs`: replace `WalletAddressProvider::new(&wallet, ...)` with `wallet.platform` as `AddressProvider`
- Replace `wallet.platform_address_info` field access with `wallet.platform.platform_address_info()`

**Done when**: DIP-17 address balance sync works; top-up, transfer, and withdrawal work in evo-tool.

---

### PR-5: Serialization + Final Cleanup

**Library** (`rs-platform-wallet`):

- `PlatformWallet::backup()` / `restore()` — full bincode blob excluding `Sdk` (§1.8)
- Any remaining missing `Encode`/`Decode` impls
- Ensure `rs-platform-wallet-ffi` re-exports any new functions (FFI layer exists at `packages/rs-platform-wallet-ffi/`)

**evo-tool integration**:

- Replace SQLite wallet blob serialization with `PlatformWallet::backup()`/`restore()`
- Wire `PlatformWallet::from_bytes(sdk, blob)` on wallet load
- Remove any remaining evo-tool wallet shim code

**Done when**: Wallet persists and restores correctly across restarts; no old wallet code remains in evo-tool.

---

## Address Type Coverage Summary

| Address type | DIP | Derivation path | key-wallet collection field | Plan section |
|---|---|---|---|---|
| Core UTXO receive | BIP44 | `m/44'/coin'/acct'/0/i` | `standard_bip44_accounts` | §1.3.2 |
| Core UTXO change | BIP44 | `m/44'/coin'/acct'/1/i` | `standard_bip44_accounts` | §1.3.2 |
| Identity reg. funding | DIP-13 | `m/9'/coin'/5'/1'/i` (non-hardened i) | `identity_registration` | §1.4.1 |
| Identity top-up funding | DIP-13 | `m/9'/coin'/5'/2'/i` (non-hardened i) | `identity_topup_not_bound` | §1.4.4 |
| Identity auth keys | DIP-13 | `m/9'/coin'/5'/0'/key_type'/id'/key'` | — | §1.4.1 |
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
| DIP-13 auth key path missing `key_type'` segment | Fix in PR-2 — use full path `m/9'/coin'/5'/0'/key_type'/identity_index'/key_index'`; note: existing deployed wallets may have used the old path (key_type' omitted = effectively key_type'=0') — document deviation |
| DIP-14 `ser_256(i)` endianness | Add unit test against DIP-14 Appendix A vectors before any contact request is submitted |
| BLS key derivation semantics | Use raw 32-byte seed from BIP32 derivation as BLS secret key (not scalar addition mod bls12381 group order) — matches DashSync iOS |
| DB migration corrupts existing wallets | Version byte in DB; fallback read → convert; test against real DB fixture |
| Asset lock proof: InstantLock timeout | Implement 60s timeout before falling back to ChainLock polling — confirm ChainLocked height is known to Platform before using Chain proof |
| `PlatformWallet` not `Send+Sync` | Add `static_assertions::assert_impl_all!(PlatformWallet: Send, Sync)` |
| `Arc<RwLock<ManagedWalletInfo>>` write starvation under concurrent SPV + Platform sync | SPV writes are short (tx update); Platform sync holds read lock briefly for balance reads — test under load |
| `contactRequest` documents are immutable | Do not expose update/delete API; note in `send_contact_request` docs that retries create new documents |

---

## Sources & References

### DIPs

- [DIP-0013: Identities in HD Wallets](https://github.com/dashpay/dips/blob/master/dip-0013.md) — auth, registration, top-up funding paths
- [DIP-0014: Extended Key Derivation (256-bit)](https://github.com/dashpay/dips/blob/master/dip-0014.md) — CKDpriv256/CKDpub256 spec and test vectors
- [DIP-0015: DashPay](https://github.com/dashpay/dips/blob/master/dip-0015.md) — contact request structure, ECDH, AES-CBC encryption, account reference, DashPay payment paths
- [DIP-0017: Dash Platform P2PKH Addresses](https://github.com/dashpay/dips/blob/master/dip-0017.md) — platform payment addresses at `m/9'/coin'/17'/account'/key_class'/index`

### Key Repositories

| Repo | Disk Path | Notes |
| ---- | --------- | ----- |
| `rs-platform-wallet` | `packages/rs-platform-wallet/` | Target library (this plan) |
| `rs-platform-encryption` | `packages/rs-platform-encryption/` | DIP-15 crypto — already a dependency, do not duplicate |
| `rs-platform-wallet-ffi` | `packages/rs-platform-wallet-ffi/` | FFI layer — update exports in PR-5 |
| `key-wallet` | `../rust-dashcore/key-wallet/` | UTXO wallet, key derivation, TransactionBuilder |
| `key-wallet-manager` | `../rust-dashcore/key-wallet-manager/` | `WalletInterface` trait (feature = "manager") |
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

- `PutIdentity` trait — `packages/rs-sdk/src/platform/transition/put_identity.rs`
- `TopUpIdentity` trait — `packages/rs-sdk/src/platform/transition/top_up_identity.rs`
- `WithdrawFromIdentity` trait — `packages/rs-sdk/src/platform/transition/withdraw_from_identity.rs`
- `TransferToIdentity` trait — `packages/rs-sdk/src/platform/transition/transfer.rs`
- `AddressProvider` trait — `packages/rs-sdk/src/platform/address_sync/provider.rs`
- Contact requests — `packages/rs-sdk/src/platform/dashpay/contact_request.rs`

### Evo Tool (to be replaced)

- `dash-evo-tool/src/model/wallet/mod.rs` — current `Wallet` struct (will be deleted in PR-1)
- `dash-evo-tool/src/app.rs` — `AppContext.wallets: RwLock<BTreeMap<WalletSeedHash, Arc<RwLock<Wallet>>>>`
- `dash-evo-tool/src/backend_task/dashpay/dip14_derivation.rs`
- `dash-evo-tool/src/backend_task/dashpay/hd_derivation.rs`
- `dash-evo-tool/src/backend_task/dashpay/encryption.rs`
- `dash-evo-tool/src/backend_task/identity/discover_identities.rs` — `AUTH_KEY_LOOKUP_WINDOW = 12`
- `dash-evo-tool/src/backend_task/wallet/fetch_platform_address_balances.rs`
