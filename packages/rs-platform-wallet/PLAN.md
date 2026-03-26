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
2. **PR-2** ✅: CoreWallet deep integration — `Signer<PlatformAddress>`, per-address data, asset locks, transaction sending
3. **PR-3** ✅: `IdentityWallet` — register, discover, top-up, withdraw, transfer, `IdentitySigner`
4. **PR-4** ✅: `DashPayWallet` — contact requests (simplified API), sync, accept
5. **PR-5** ✅: `PlatformAddressWallet` — DIP-17 sync, send, withdraw + review fixes
6. **PR-6**: Dashcore upstream sync + mempool support — crate merge, TransactionContext, SPV lifecycle, TransactionStatus, event wiring
7. **PR-7**: Missing identity/address operations + DPNS — add_key, top_up_from_addresses, transfer_to_addresses, fund_from_asset_lock, DPNS module
8. **PR-8**: Token operations — `TokenWallet` sub-wallet (transfer, balance, claim, purchase)
9. **PR-9**: Shielded pool (feature-gated `shielded`) — `ShieldedWallet` with Orchard key management, note/nullifier sync, 5 transition types
10. **PR-10**: Comprehensive test suite — port 72+ evo-tool tests, mock SDK integration tests, E2E framework
11. **PR-11**: Merge `Wallet` + `ManagedWalletInfo` in `key-wallet` (dashcore) — single `Arc<RwLock<Wallet>>`
12. **PR-12**: Serialization / persistence, remove old `wallets` map, delete `src/model/wallet/` + final cleanup

---

## PR-6: Dashcore upstream sync + mempool support

### Dashcore changes to incorporate (v0.42-dev since 42eb1d69)

**Must fix (will not compile):**

1. **`key-wallet-manager` crate merged into `key-wallet`** (5edf719f):
   - All `use key_wallet_manager::*` → `use key_wallet::manager::*`
   - Remove `key-wallet-manager` from Cargo.toml, use `key_wallet` with `manager` feature
   - Affects: SPV adapter, events.rs, PlatformWalletManager, Cargo.toml

2. **`TransactionContext` restructured** (213a9b4f, f2d2dfe8):
   - `InBlock { height, block_hash: Option, timestamp: Option }` → `InBlock(BlockInfo)` where `BlockInfo { height, block_hash, timestamp }` (all required)
   - New `TransactionContext::InstantSend` variant
   - `check_core_transaction()` gained `update_balance: bool` parameter
   - Affects: SPV adapter `process_block`, `process_mempool_transaction`

3. **`WalletInterface` trait expanded** (08ade6e8, e7c68d9d):
   - `process_mempool_transaction()`: added `is_instant_send: bool` param, returns `MempoolTransactionResult`
   - New required: `watched_outpoints() -> Vec<OutPoint>` (for bloom filter)
   - New with defaults: `monitor_revision()`, `process_instant_send_lock()`
   - Affects: SpvWalletAdapter must implement new methods

4. **`DashSpvClient` gained `EventHandler` generic** (c39db47d):
   - Constructor: `DashSpvClient::new(config, network, storage, wallet, Arc::new(handler))`
   - `DashSpvClient<W, N, S>` → `DashSpvClient<W, N, S, H: EventHandler>`
   - New `EventHandler` trait: `on_sync_event`, `on_network_event`, `on_progress`, `on_wallet_event`, `on_error`
   - Affects: `PlatformWalletManager::start_spv()` when wired up

**Should implement (defaults exist but functionality needs it):**
- `mark_instant_send_utxos()` on `WalletInfoInterface`
- `EventHandler` impl for SPV progress/wallet event forwarding to `PlatformWalletEvent`

### Evo-tool changes to backport (v1.0-dev since 7647ccf1)

1. **Mempool support** (0f01edd9): `TransactionStatus` enum, `MempoolStrategy::BloomFilter`, transaction deduplication
2. **Key-only address balances** (917b3471): RPC fallback for transaction history, provider account registration
3. **DAPI error classification** (65358ef4): Typed `TaskError` variants instead of raw gRPC errors
4. **DB migration** (8937c1c9): Consolidated migrations, `Network::Dash` → `Network::Mainnet` in DB
5. **E2E test harness** (fffc649e): `BackendTestContext` pattern for integration tests

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

---

## PR-2 Status: Complete

### What was delivered

**Platform-wallet library** (`rs-platform-wallet`):
- `CoreAddressInfo`, `CoreAccountSummary` types (`wallet/core/types.rs`)
- Per-address methods: `all_address_info()`, `address_info()`, `account_summaries()`, `utxos_by_address()`
- `Signer<PlatformAddress>` on `PlatformAddressWallet` — `blocking_read()` bridge with sequential lock acquisition (no dual-lock window), cached `network` field
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
key-wallet (rust-dashcore) — reused types
├── Wallet                       ← mutable key store (mnemonic, xprv, accounts added during sync)
├── ManagedWalletInfo            ← mutable UTXO state, accounts, balance, address pools
├── ManagedAccountCollection     ← BIP44 + DashPay + PlatformPayment + Identity accounts
├── TransactionRouter            ← transaction classification + checking
├── WalletTransactionChecker     ← trait for tx matching (impl on ManagedWalletInfo)
├── key_wallet::manager          ← WalletInterface, WalletEvent, BlockProcessingResult,
│                                   MempoolTransactionResult (merged from key-wallet-manager)
├── TransactionContext           ← Mempool | InstantSend | InBlock(BlockInfo) | InChainLockedBlock(BlockInfo)
└── BlockInfo                    ← { height, block_hash, timestamp } (all required)

rs-platform-wallet
├── PlatformWallet               ← cheaply cloneable (~35 atomic ops), all Arc fields
│   ├── sdk:      Sdk                              ← ref-counted
│   ├── core:     CoreWallet                       ← balance, UTXOs, addresses, tx building, asset locks
│   │   ├── wallet:      Arc<RwLock<Wallet>>
│   │   ├── wallet_info: Arc<RwLock<ManagedWalletInfo>>
│   │   └── network:     Network (cached)
│   ├── identity: IdentityWallet                   ← register, discover, top-up, withdraw, transfer, DPNS
│   │   ├── wallet, wallet_info, identity_manager: Arc<RwLock<...>>
│   │   ├── network: Network (cached)
│   │   └── signer_for_identity() → IdentitySigner
│   ├── dashpay:  DashPayWallet                    ← send/accept contact requests, sync contacts
│   │   ├── wallet, wallet_info, identity_manager: Arc<RwLock<...>>
│   │   └── network: Network (cached)
│   ├── platform: PlatformAddressWallet            ← DIP-17 sync, transfer, withdraw, fund
│   │   ├── wallet, wallet_info: Arc<RwLock<...>>
│   │   ├── balances: Arc<RwLock<BTreeMap<PlatformAddress, Credits>>>
│   │   ├── network: Network (cached)
│   │   └── implements Signer<PlatformAddress> (blocking_read bridge)
│   └── [shielded: Option<ShieldedWallet>]         ← feature-gated, Orchard ZK pool (PR-9)
│
├── PlatformWalletManager        ← multi-wallet + SPV coordinator
│   ├── sdk, network, wallets: RwLock<BTreeMap<WalletId, PlatformWallet>>
│   ├── SpvWalletAdapter         ← implements WalletInterface for SPV
│   │   ├── process_block() / process_mempool_transaction()
│   │   ├── watched_outpoints() (for bloom filter)
│   │   ├── process_instant_send_lock()
│   │   └── monitor_revision() (bloom filter staleness)
│   ├── EventHandler impl        ← forwards SPV events to PlatformWalletEvent
│   └── start_spv() / stop_spv() ← DashSpvClient<W, N, S, H> lifecycle
│
├── Signing
│   ├── IdentitySigner           ← Signer<IdentityPublicKey> (ECDSA/BLS/EdDSA, DIP-9 paths)
│   └── PlatformAddressWallet    ← Signer<PlatformAddress> (ECDSA P2PKH, DIP-17 paths)
│
├── Events
│   ├── PlatformWalletEvent      ← Wallet(WalletEvent) | Spv(SpvEvent) | Finality(FinalityEvent) | MempoolTransaction
│   └── TransactionStatus        ← Unconfirmed | InstantSendLocked | Confirmed{h} | ChainLocked{h}
│
├── [TokenWallet]                ← PR-8: transfer, balance, claim, purchase
│
└── [ShieldedWallet]             ← PR-9: shield, unshield, transfer, withdraw (Orchard/Halo2)
    ├── keys.rs                  ← SpendingKey → FullViewingKey → OrchardAddress
    ├── note_store.rs            ← DecryptedNote persistence, SpendableNote selection
    ├── nullifier_store.rs       ← NullifierProvider impl
    ├── commitment_tree.rs       ← local Sinsemilla tree (SQLite-backed)
    ├── prover.rs                ← OrchardProver with cached ProvingKey
    └── sync.rs                  ← note sync + nullifier sync + tree updates

rs-sdk (Dash Platform SDK) — operations used by platform-wallet
├── Identity: PutIdentity, TopUpIdentity, WithdrawFromIdentity, TransferToIdentity
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
  establishment and sync. The `check_core_transaction` trait takes `&Wallet` (read lock) for
  transaction checking, but other operations need write access.
- **Sub-wallets share state via Arc**: All hold `Arc<RwLock<ManagedWalletInfo>>` and
  `Arc<RwLock<Wallet>>`. SPV writes through the Arc — visible to all clones immediately.
- **Lock ordering**: Always acquire `wallet` before `wallet_info` to prevent deadlocks.
  Signers use sequential `blocking_read()` (drop first lock before acquiring second).
- **key-wallet-manager merged into key-wallet**: All imports use `key_wallet::manager::*`.
  The `WalletInterface` trait, `WalletEvent`, `BlockProcessingResult`, `MempoolTransactionResult`
  are in `key_wallet::manager`.
- **Mempool support**: `SpvWalletAdapter` implements the full `WalletInterface` including
  `process_mempool_transaction(tx, is_instant_send)`, `watched_outpoints()`, `monitor_revision()`.
  `DashSpvClient` is parameterized with `EventHandler` for SPV event forwarding.
- **TransactionStatus lifecycle**: Unconfirmed → InstantSendLocked → Confirmed → ChainLocked.
  Tracked per transaction in CoreWallet. Events emitted on state changes.
- **Feature-gated shielded**: Orchard/Halo2 deps are heavy (~30s ProvingKey). Behind `shielded`
  feature. ShieldedWallet is fundamentally different (client-side state, note trial decryption,
  commitment tree) so it's a separate sub-wallet, not an extension of PlatformAddressWallet.
- **Private key zeroization**: `Zeroizing<[u8; 32]>` for all derived key material. `blocking_read()`
  drops locks before acquiring the next. Signer closures validate key ID parameters.
- **Simplified DashPay API**: `send_contact_request(sender, recipient)` — 2 params. All key indices,
  ECDH, derivation resolved internally. `accept_contact_request(request)` — 1 param.

---

## Implementation Plan

`PlatformWallet` is a standalone wallet type (usable without SPV/manager). Cheaply cloneable (~35
atomic ops — all Arc fields). No separate `WalletHandle` — use `PlatformWallet.clone()` directly.
`PlatformWalletManager` is the multi-wallet + SPV coordinator (no `WalletManager<T>` dependency).

### Struct Definitions

```rust
// Standalone wallet — owns all state, sub-wallets as stored fields
// Usable directly for Platform-only operations (scripts, tests, no SPV needed)
// Same type is wrapped in per-wallet RwLock when managed by PlatformWalletManager
// NOTE: No `wallet` field on PlatformWallet — sub-wallets hold their own Arc refs
pub struct PlatformWallet {
    sdk:      Sdk,          // cheaply cloneable (ref-counted)
    core:     CoreWallet,
    identity: IdentityWallet,
    dashpay:  DashPayWallet,
    platform: PlatformAddressWallet,
}

// Sub-wallets — stored fields, share wallet_info via Arc<RwLock<ManagedWalletInfo>>
// Each sub-wallet caches `network: Network` to avoid lock acquisition for network queries
pub struct CoreWallet {
    sdk:         Sdk,
    wallet:      Arc<RwLock<Wallet>>,
    wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    network:     Network,  // cached at construction
}

pub struct IdentityWallet {
    sdk:              Sdk,
    wallet:           Arc<RwLock<Wallet>>,
    wallet_info:      Arc<RwLock<ManagedWalletInfo>>,
    identity_manager: IdentityManager,
    network:          Network,  // cached at construction
}

pub struct DashPayWallet {
    sdk:              Sdk,
    wallet:           Arc<RwLock<Wallet>>,
    wallet_info:      Arc<RwLock<ManagedWalletInfo>>,
    identity_manager: IdentityManager,  // same instance as IdentityWallet (Arc clone)
    network:          Network,  // cached at construction
}

pub struct PlatformAddressWallet {
    sdk:         Sdk,
    wallet:      Arc<RwLock<Wallet>>,
    wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    balances:    Arc<RwLock<BTreeMap<PlatformAddress, Credits>>>,  // balance cache
    network:     Network,  // cached at construction
}

// Multi-wallet + SPV coordinator — no WalletManager<T> dependency
// Implements WalletInterface for SPV using key-wallet functions directly
pub struct PlatformWalletManager {
    sdk:        Sdk,
    network:    Network,
    wallets:    RwLock<BTreeMap<WalletId, PlatformWallet>>,  // lock only for add/remove
    spv_client: Option<DashSpvClient<Self, N, S, H>>,  // None until start_spv(); H: EventHandler
    event_tx:   broadcast::Sender<PlatformWalletEvent>,
    synced_height: AtomicU32,
}

// IdentityManager is shared between IdentityWallet and DashPayWallet.
// Implements Clone — all fields are cheap to clone (just Arc clones).
// IdentityWallet and DashPayWallet share the same IdentityManager
// instance because PlatformWallet constructs them from the same source at build time.
pub struct IdentityManager {
    identities:          Arc<RwLock<IndexMap<Identifier, ManagedIdentity>>>,
    primary_identity_id: Arc<RwLock<Option<Identifier>>>,
    last_scanned_index:  Arc<RwLock<u32>>,  // persisted gap scan state
    // REMOVED: sdk: Option<Arc<Sdk>> — SDK flows through caller struct
}
// Clone is cheap — just Arc clones. IdentityWallet and DashPayWallet hold
// the same Arc pointers — mutations visible to both.

// ManagedIdentity requires identity_index: u32 (not Optional) — set during
// registration or discovery. Used for DIP-9 key derivation paths.
```

**No dashcore changes required.** Only `key-wallet` crate types are used directly (`Wallet`,
`ManagedWalletInfo`, `ManagedAccountCollection`, `TransactionRouter`, `WalletTransactionChecker`).
`key-wallet-manager` is merged into `key-wallet` — all imports use `key_wallet::manager::*`.

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
        network: Network,
    ) -> Self {
        Self {
            sdk, wallet, wallet_info, network,
            balances: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}
```

`PlatformWalletManager` API — mirrors dashcore wallet creation methods, uses `key-wallet` types directly:

```rust
impl PlatformWalletManager {
    // Construction
    pub fn new(sdk: Sdk, spv_config: ClientConfig, network: Network) -> Self;

    // Wallet creation — uses key-wallet's Wallet + ManagedWalletInfo directly
    // Returns PlatformWallet (cheaply cloneable — all Arc fields)
    pub async fn create_wallet_from_mnemonic(
        &self, mnemonic: &str, passphrase: &str,
        birth_height: CoreBlockHeight,
        account_options: WalletAccountCreationOptions,
    ) -> Result<PlatformWallet>;

    pub async fn create_wallet_with_random_mnemonic(
        &self,
        account_options: WalletAccountCreationOptions,
    ) -> Result<(PlatformWallet, Mnemonic)>;

    pub async fn import_wallet_from_xprv(
        &self, xprv: &str,
        account_options: WalletAccountCreationOptions,
    ) -> Result<PlatformWallet>;

    pub async fn import_wallet_from_xpub(
        &self, xpub: &str, can_sign_externally: bool,
    ) -> Result<PlatformWallet>;

    // Wallet restoration
    pub async fn import_wallet_from_bytes(
        &self, wallet_bytes: &[u8],
    ) -> Result<PlatformWallet>;

    // Wallet lifecycle
    pub async fn remove_wallet(&self, wallet_id: &WalletId) -> Result<PlatformWallet>;

    // Wallet access
    pub async fn get_wallet(&self, wallet_id: &WalletId) -> Option<PlatformWallet>;
    pub async fn list_wallets(&self) -> Vec<WalletId>;

    // SPV lifecycle — DashSpvClient<W, N, S, H: EventHandler>
    pub async fn start_spv(&mut self) -> Result<()>;
    pub async fn stop_spv(&mut self) -> Result<()>;

    // Events — unified stream, grouped by source channel
    pub fn subscribe_events(&self) -> broadcast::Receiver<PlatformWalletEvent>;
}

// Unified event enum — variants per source channel
pub enum PlatformWalletEvent {
    Wallet(WalletEvent),            // from block processing (TransactionReceived, BalanceUpdated)
    Spv(SpvEvent),                  // from DashSpvClient (SyncProgress, PeerConnected, PeerDisconnected)
    Finality(FinalityEvent),        // InstantLock / ChainLock
    MempoolTransaction,             // from mempool processing
}
```

Call sites — standalone `PlatformWallet`:

```rust
let wallet = PlatformWallet::from_mnemonic(sdk, network, "word1 ...", "", 1_500_000, options)?;
wallet.identity().register_identity(amount, keys).await?;
wallet.dashpay().send_contact_request(&sender_id, &recipient_id).await?;
wallet.core().balance();
```

Call sites — managed via `PlatformWalletManager` (same API — PlatformWallet is cheaply cloneable):

```rust
let wallet = mgr.create_wallet_from_mnemonic("...", "", height, options).await?;
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
lives in `PlatformWalletManager`. There is no `wallet` field on `PlatformWallet` itself — each
sub-wallet holds its own `Arc<RwLock<Wallet>>` reference. Sub-wallets also cache `network: Network`
at construction to avoid lock acquisition for network queries.

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
let wallet = mgr.create_wallet_from_mnemonic(
    "word1 word2 ...", "", 1_500_000,
    WalletAccountCreationOptions::Default,
).await?;
mgr.start_spv().await?;
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

- `packages/rs-platform-wallet/src/wallet/platform_wallet.rs` (new — replaces `platform_wallet_info/mod.rs`)
- `packages/rs-platform-wallet/src/platform_wallet_manager/mod.rs` (new)

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

**`SpvWalletAdapter`** implements the full `WalletInterface` trait (from `key_wallet::manager`):
- `process_block()` — iterates wallets, locks each `wallet_info`, calls `check_core_transaction` per tx
- `process_mempool_transaction(tx, is_instant_send: bool)` → `MempoolTransactionResult`
- `watched_outpoints() -> Vec<OutPoint>` — for bloom filter construction
- `monitor_revision() -> u64` — bloom filter staleness detection; change triggers reconstruction
- `process_instant_send_lock()` — marks UTXOs as instant-send confirmed
- `monitored_addresses` — collects from all wallets' `ManagedWalletInfo`
- `synced_height` / `update_synced_height` — tracks via `AtomicU32`, updates each wallet

Note: `check_core_transaction()` has gained an `update_balance: bool` parameter.

SPV lives in `PlatformWalletManager`, not in `PlatformWallet`. `PlatformWallet` is SPV-free.

**Wiring** (`PlatformWalletManager::start_spv()`):

```rust
// DashSpvClient::new(config, network, storage, wallet, Arc::new(handler))
let handler = Arc::new(SpvEventHandler::new(event_tx.clone()));
let spv = DashSpvClient::new(spv_config, network, storage, self_arc, handler).await?;
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

**`PlatformWalletEvent`** (unified enum):
- `Wallet(WalletEvent)` — `TransactionReceived`, `BalanceUpdated`
- `Spv(SpvEvent)` — sync progress, peer connections
- `Finality(FinalityEvent)` — InstantLock, ChainLock
- `MempoolTransaction` — from mempool processing

**EventHandler** impl forwards SPV events to `PlatformWalletEvent`:
- `on_sync_event`, `on_network_event`, `on_progress`, `on_wallet_event`, `on_error`

**Event subscription**:
```rust
let rx: broadcast::Receiver<PlatformWalletEvent> = mgr.subscribe_events();
```

**Two event channels**: `WalletInterface::subscribe_events()` returns `WalletEvent` (for SPV).
`PlatformWalletManager::subscribe_events()` (public API) returns `PlatformWalletEvent` which
wraps `WalletEvent` + `SpvEvent` + `FinalityEvent` + `MempoolTransaction`. Internally, the
manager forwards `WalletEvent`s into the `PlatformWalletEvent` channel.

**No reorg notification**: `WalletInterface` has no `process_reorg` method — reorgs are handled
only at the `ChainTipManager` level in dash-spv; the wallet is never notified.

`key-wallet-manager` is merged into `key-wallet` — all imports use `key_wallet::manager::*`.
`WalletInterface`, `WalletEvent`, `BlockProcessingResult`, `MempoolTransactionResult` are in
`key_wallet::manager`.

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

#### Files

- `packages/rs-platform-wallet/src/wallet/core/wallet.rs` (new)
- Depends on: `key-wallet` (`ManagedWalletInfo`, `TransactionBuilder`, `WalletInfoInterface`,
  `ManagedAccountOperations`, `FeeRate`, `SelectionStrategy`)
- Depends on: `key-wallet` with `manager` feature — `WalletInterface`, `WalletEvent`,
  `BlockProcessingResult`, `MempoolTransactionResult` (merged from key-wallet-manager)
- Depends on: `dash-spv` (`broadcast_transaction`, InstantLock/ChainLock events)

---

### 1.4 Identity Management

> Register, discover, refresh, top-up, withdraw, transfer, update identities. Register DPNS names.

All methods are on `IdentityWallet` which holds `sdk`, `wallet: Arc<RwLock<Wallet>>`, and `identity_manager`.
No `wallet: &Wallet` parameter anywhere — key derivation and signing use `self.wallet` directly.
`identity_index` is stored on `ManagedIdentity` as `u32` (required, not Optional).

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

```rust
pub async fn register_identity(
    &mut self,
    amount_duffs: u64,
    key_types: &[IdentityKeySpec],
) -> Result<Identity, PlatformWalletError>
```

Steps:

1. `core_wallet.create_registration_asset_lock_proof(amount, index)` → `(AssetLockProof, PrivateKey)`
2. Derive auth keys at DIP-9 paths, build `IdentityPublicKey` entries
3. Build `Identity` object with keys
4. `identity.put_to_platform_and_wait_for_response(&sdk, proof, &key, &signer, None)` → confirmed `Identity`
5. Add to `identity_manager`

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

**`signer_for_identity` factory** on `IdentityWallet`:
```rust
pub fn signer_for_identity(
    &self,
    identity_id: &Identifier,
) -> Result<IdentitySigner, PlatformWalletError>
```
Looks up the `identity_index: u32` from the `ManagedIdentity` (required field), constructs an
`IdentitySigner` with the wallet Arc and index. Returns `IdentityIndexNotSet` if the identity
was added without an index.

#### 1.4.2 — Identity Discovery (DIP-9 gap-limit scan)

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
- Gap limit hardcoded to 5 (implementation convention — DIP-9 does not specify a gap limit value; 5 matches the registration-funding bloom filter batch size and is a safe conservative choice)
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
- `network: Network` is cached on `PlatformAddressWallet` at construction.
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

- `packages/rs-platform-wallet/src/wallet/platform_address_wallet.rs` (extend)

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
Tracked per transaction in CoreWallet. `PlatformWalletEvent::MempoolTransaction` emitted on transitions.

**SpvWalletAdapter** implements the full `WalletInterface` (from `key_wallet::manager`):

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

**PlatformWalletManager** SPV lifecycle:

```rust
impl PlatformWalletManager {
    pub async fn start_spv(&mut self) -> Result<()>;
    // Creates DashSpvClient<SpvWalletAdapter, N, S, SpvEventHandler>
    // Spawns background task with cancellation token

    pub async fn stop_spv(&mut self) -> Result<()>;
    // Cancels the background task, drops the client
}
```

**Bloom filter reconstruction**: Triggered when `monitor_revision()` changes. This happens
when new addresses are generated (gap limit extension, DashPay account creation) or when
watched outpoints change (new UTXOs received).

#### Files

- `packages/rs-platform-wallet/src/spv/adapter.rs`
- `packages/rs-platform-wallet/src/spv/event_handler.rs`
- `packages/rs-platform-wallet/src/events.rs`

---

### 1.8 Token Operations

> `TokenWallet` sub-wallet for platform token management.

**TokenWallet** is a new sub-wallet on `PlatformWallet`:

```rust
pub struct TokenWallet {
    sdk:              Sdk,
    wallet:           Arc<RwLock<Wallet>>,
    wallet_info:      Arc<RwLock<ManagedWalletInfo>>,
    identity_manager: IdentityManager,
    network:          Network,
}
```

**Core operations**:

```rust
pub async fn transfer(
    &self, identity_id: &Identifier, token_id: &Identifier,
    to_identity_id: &Identifier, amount: u64,
) -> Result<(), PlatformWalletError>

pub async fn balance(
    &self, identity_id: &Identifier, token_id: &Identifier,
) -> Result<u64, PlatformWalletError>

pub async fn claim_rewards(
    &self, identity_id: &Identifier, token_id: &Identifier,
) -> Result<u64, PlatformWalletError>
```

**Market operations**:

```rust
pub async fn purchase(
    &self, identity_id: &Identifier, token_id: &Identifier, amount: u64,
) -> Result<(), PlatformWalletError>

pub async fn set_price(
    &self, identity_id: &Identifier, token_id: &Identifier, price: u64,
) -> Result<(), PlatformWalletError>
```

**Admin operations** (optional — only for token contract owners):

```rust
pub async fn mint(&self, identity_id: &Identifier, token_id: &Identifier, amount: u64, to: &Identifier) -> Result<(), PlatformWalletError>
pub async fn burn(&self, identity_id: &Identifier, token_id: &Identifier, amount: u64) -> Result<(), PlatformWalletError>
pub async fn freeze(&self, identity_id: &Identifier, token_id: &Identifier, target: &Identifier) -> Result<(), PlatformWalletError>
pub async fn pause(&self, identity_id: &Identifier, token_id: &Identifier) -> Result<(), PlatformWalletError>
```

All operations use the corresponding SDK token transition traits. Balance queries support
per-identity and per-address lookups.

#### Files

- `packages/rs-platform-wallet/src/wallet/tokens/mod.rs` (new)
- `packages/rs-platform-wallet/src/wallet/tokens/wallet.rs` (new)

---

### 1.9 Shielded Pool

> Feature-gated shielded transactions using Orchard/Halo2. Behind `feature = "shielded"`.

**ShieldedWallet** is fundamentally different from other sub-wallets — it maintains client-side
state (note store, nullifier set, commitment tree) that cannot be derived from Platform queries alone.

```rust
#[cfg(feature = "shielded")]
pub struct ShieldedWallet {
    spending_key:       SpendingKey,
    full_viewing_key:   FullViewingKey,
    orchard_address:    OrchardAddress,
    note_store:         NoteStore,          // DecryptedNote persistence, SpendableNote selection
    nullifier_store:    NullifierStore,     // NullifierProvider impl for spent-note detection
    commitment_tree:    CommitmentTree,     // local Sinsemilla tree (SQLite-backed)
    prover:             CachedOrchardProver,// OrchardProver with cached ProvingKey (~30s init)
    sdk:                Sdk,
    network:            Network,
}
```

**Orchard key hierarchy**: `SpendingKey → FullViewingKey → OrchardAddress`.
The spending key is derived from the wallet's master seed.

**Note sync**: Trial decryption of all Orchard output notes using the `FullViewingKey`.
Notes that decrypt successfully belong to this wallet and are stored in the `NoteStore`.

**Nullifier sync**: Monitors the global nullifier set to detect when owned notes have been
spent. Updates the `NoteStore` to mark spent notes.

**5 transition types**:

```rust
// Platform addresses → shielded pool (needs Signer<PlatformAddress>)
pub async fn shield(&self, from_addresses: BTreeMap<PlatformAddress, Credits>) -> Result<()>

// Core L1 → shielded pool (via asset lock)
pub async fn shield_from_asset_lock(&self, amount_duffs: u64) -> Result<()>

// Shielded pool → platform address
pub async fn unshield(&self, to_address: &PlatformAddress, amount: Credits) -> Result<()>

// Shielded pool → shielded pool (private transfer)
pub async fn transfer(&self, to_address: &OrchardAddress, amount: Credits) -> Result<()>

// Shielded pool → Core L1
pub async fn withdraw(&self, to_address: &Address, amount: Credits) -> Result<()>
```

**Implementation notes**:
- Uses DPP `build_*_transition()` builders (not raw SDK traits) for the Orchard pipeline
- Local Sinsemilla commitment tree is SQLite-backed (wraps `grovedb-commitment-tree`)
- `CachedOrchardProver`: caches the `ProvingKey` after first initialization (~30s cold start)
- SDK traits: `ShieldFunds`, `UnshieldFunds`, `TransferShielded`, `WithdrawShielded`, `ShieldFromAssetLock`

**Sync integration**: `ShieldedWallet::sync()` orchestrates note sync + nullifier sync + tree updates.
Called as part of `PlatformWallet::sync()` when the shielded feature is enabled.

#### Files

- `packages/rs-platform-wallet/src/wallet/shielded/mod.rs` (new)
- `packages/rs-platform-wallet/src/wallet/shielded/keys.rs` (new)
- `packages/rs-platform-wallet/src/wallet/shielded/note_store.rs` (new)
- `packages/rs-platform-wallet/src/wallet/shielded/nullifier_store.rs` (new)
- `packages/rs-platform-wallet/src/wallet/shielded/commitment_tree.rs` (new)
- `packages/rs-platform-wallet/src/wallet/shielded/prover.rs` (new)
- `packages/rs-platform-wallet/src/wallet/shielded/sync.rs` (new)
- `packages/rs-platform-wallet/src/wallet/shielded/operations.rs` (new)

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

`PlatformWalletEvent::MempoolTransaction` is emitted on each status transition.

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
- `PlatformWalletEvent` unified enum: `Wallet(WalletEvent)`, `Spv(SpvEvent)`, `Finality(FinalityEvent)`, `MempoolTransaction`
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
- SPV: `PlatformWalletManager::start_spv()` replaces manual `SpvManager` setup
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
- Add `network: Network` to `PlatformAddressWallet`

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

### PR-7: Missing identity/address operations + DPNS

**Library** (`rs-platform-wallet`):

- `IdentityWallet`: `add_key_to_identity()` — build `IdentityUpdateTransition` via DPP, broadcast
- `IdentityWallet`: `top_up_from_addresses()` — `TopUpIdentityFromAddresses` SDK trait
- `IdentityWallet`: `transfer_to_addresses()` — `TransferToAddresses` SDK trait
- `IdentityWallet`: `register_name()`, `resolve_name()` — convenience wrappers around SDK DPNS methods
- `PlatformAddressWallet`: `fund_from_asset_lock()` — `TopUpAddress` SDK trait

**Done when**: All identity fund flows work (L1→identity, address→identity, identity→address).
DPNS names can be registered and resolved via IdentityWallet convenience methods.

---

### PR-8: Token operations

**Library** (`rs-platform-wallet`):

- New `wallet/tokens/` module with `TokenWallet` sub-wallet
- Core operations: `transfer()`, `balance()`, `claim_rewards()`
- Market operations: `purchase()`, `set_price()`
- Admin operations (optional): `mint()`, `burn()`, `freeze()`, `pause()`
- Token balance queries: per-identity, per-address
- Feature-gated if deps are heavy

**Done when**: Token transfers and balance queries work through platform-wallet.

---

### PR-9: Shielded pool (feature-gated `shielded`)

**Library** (`rs-platform-wallet`):

- New `wallet/shielded/` module behind `#[cfg(feature = "shielded")]`:
  - `ShieldedWallet` struct: SpendingKey, FullViewingKey, SpendAuthorizingKey, note store
  - `keys.rs` — Orchard key derivation and management
  - `note_store.rs` — DecryptedNote persistence, SpendableNote selection
  - `nullifier_store.rs` — NullifierProvider impl for privacy-preserving spent-note detection
  - `commitment_tree.rs` — local Sinsemilla tree (wraps grovedb-commitment-tree SQLite)
  - `prover.rs` — OrchardProver impl with cached ProvingKey
  - `sync.rs` — orchestrates note sync + nullifier sync + tree updates
  - `operations.rs` — shield, unshield, transfer, withdraw, shield_from_asset_lock
- Uses DPP `build_*_transition()` builders (not raw SDK traits) for Orchard pipeline
- `PlatformWallet`: `shielded: Option<ShieldedWallet>` (None if not set up)

5 transition types:
- Shield: platform addresses → shielded pool (needs `Signer<PlatformAddress>`)
- ShieldFromAssetLock: Core L1 → shielded pool
- Unshield: shielded pool → platform address
- ShieldedTransfer: shielded pool → shielded pool (private)
- ShieldedWithdrawal: shielded pool → Core L1

**Done when**: Full shielded lifecycle works. Note sync discovers incoming funds.

---

### PR-10: Comprehensive test suite

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

**E2E tests** (live network, feature-gated):
- SPV sync + wallet balance (BackendTestContext pattern from evo-tool PR #778)
- Send/receive funds round-trip
- Identity registration + discovery
- Contact request send + accept between two wallets
- Platform address operations

---

### PR-11: Merge Wallet + ManagedWalletInfo (dashcore)

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

### PR-12: Serialization + Final Cleanup

**Library** (`rs-platform-wallet`):

- `PlatformWallet::backup()` / `restore()` — full bincode blob excluding `Sdk` (§1.11)
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
