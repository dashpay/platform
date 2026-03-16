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

1. **PR-1**: Project scaffold + `CoreWallet` (UTXO, addresses, SPV, asset lock proof) → replace evo-tool's `src/model/wallet/`
2. **PR-2**: `IdentityWallet` (register, discover, top-up, withdraw, transfer) → replace identity backend tasks
3. **PR-3**: `DashPayWallet` (DIP-14, DIP-15, contact requests, payments, sync) → replace dashpay backend tasks
4. **PR-4**: `PlatformAddressWallet` (DIP-17 sync, send, withdraw) → replace platform address backend task
5. **PR-5**: Serialization / persistence + final cleanup

---

## Problem Statement

**`dash-evo-tool`** maintains its own self-written wallet and duplicates DashPay crypto inline:

- `src/model/wallet/` — custom wallet struct
- `backend_task/dashpay/dip14_derivation.rs` — DIP-14 256-bit key derivation
- `backend_task/dashpay/encryption.rs` — DIP-15 ECDH + AES-CBC

**`rs-platform-wallet`** is the intended canonical library but is incomplete:

- No identity registration, top-up, withdrawal, or credit transfer
- No DIP-14 CKDpriv256/CKDpub256 or DIP-15 encryption
- No DashPay payment address derivation or payment sending
- No DIP-17 `AddressProvider` implementation
- No signing facade for state transition submission

---

## Architecture

```
key-wallet (rust-dashcore)
├── Wallet — private key store, BIP32 derivation
└── ManagedWalletInfo
    └── accounts: ManagedAccountCollection
        ├── core_accounts              [BIP44 UTXOs, SECP/BLS/EdDSA]
        ├── dashpay_receival_accounts  [DIP-15 receive from contact, keyed by (account, selfId, friendId)]
        ├── dashpay_external_accounts  [DIP-15 send to contact]
        └── platform_payment_accounts  [DIP-17 P2PKH credits, keyed by (account, key_class)]

rs-platform-wallet (target)
└── PlatformWallet                             ← thin coordinator, owns Sdk + Arc<Wallet>
    ├── sdk:      Sdk                          ← cheaply cloneable (internally ref-counted)
    ├── wallet:   Arc<Wallet>                  ← immutable key store; no lock needed (read-only)
    ├── core:     CoreWallet                      ← Arc<ManagedWalletInfo> inside; impls WalletInterface
    ├── identity: IdentityWallet                  ← shares Arc<ManagedWalletInfo> + Arc<RwLock<IndexMap>>
    ├── dashpay:  DashPayWallet                   ← shares same Arcs; DIP-14/15 lives here
    └── platform: PlatformAddressWallet           ← shares Arc<ManagedWalletInfo>; impls AddressProvider

rs-sdk (Dash Platform SDK)
├── Identity::fetch() / topup / withdraw / transfer / register
├── Document CRUD (put/transfer/purchase)
├── sync_address_balances() → DIP-17 address sync
├── send_contact_request() → DashPay contact request submission
└── WithdrawAddressFunds / TransferAddressFunds / TopUpAddress
```

---

## Implementation Plan

`PlatformWallet` is the single public interface for all wallet operations.
It owns the `Sdk` reference, delegates UTXO mechanics to `ManagedWalletInfo`/`Wallet`,
and routes all Platform state transitions through `dash-sdk`.

### Struct Definitions

Sub-structs are stored as fields in `PlatformWallet`. All sub-structs share the same
`Arc<ManagedWalletInfo>` — mutations are visible across sub-structs without locking the
parent. `CoreWallet` is a concrete stored type that can implement `WalletInterface` for SPV
registration. `PlatformAddressWallet` can implement `AddressProvider` and be passed to the SDK
without a self-borrow conflict.

```rust
// All fields private — construction only via builder
pub struct PlatformWallet {
    sdk:      Sdk,          // cheaply cloneable (internally ref-counted)
    wallet:   Arc<Wallet>,  // immutable key store — no lock needed (read-only)
    core:     CoreWallet,
    identity: IdentityWallet,
    dashpay:  DashPayWallet,
    platform: PlatformAddressWallet,
}

// Sub-structs hold Arc clones — cheap to clone, no outer lock needed
pub struct CoreWallet {
    sdk:         Sdk,
    wallet:      Arc<Wallet>,
    wallet_info: Arc<RwLock<ManagedWalletInfo>>,   // shared with all sub-structs
}

pub struct IdentityWallet {
    sdk:              Sdk,
    wallet:           Arc<Wallet>,
    wallet_info:      Arc<RwLock<ManagedWalletInfo>>,  // shared — asset lock proof creation
    identity_manager: IdentityManager,
}

pub struct DashPayWallet {
    sdk:              Sdk,
    wallet:           Arc<Wallet>,
    wallet_info:      Arc<RwLock<ManagedWalletInfo>>,
    identity_manager: IdentityManager,  // Arc<RwLock<_>> inside — same instance as IdentityWallet
}

pub struct PlatformAddressWallet {
    sdk:         Sdk,
    wallet:      Arc<Wallet>,
    wallet_info: Arc<RwLock<ManagedWalletInfo>>,
}

// Arc<RwLock<_>> fields inside — Clone is a cheap Arc clone, no outer lock needed
pub struct IdentityManager {
    identities:          Arc<RwLock<IndexMap<Identifier, ManagedIdentity>>>,
    primary_identity_id: Arc<RwLock<Option<Identifier>>>,
    last_scanned_index:  Arc<RwLock<u32>>,
}
```

`PlatformWallet` exposes sub-structs via accessor methods (or direct field delegation):

```rust
impl PlatformWallet {
    pub fn core(&self)     -> &CoreWallet            { &self.core }
    pub fn core_mut(&mut self) -> &mut CoreWallet    { &mut self.core }
    pub fn identity(&self) -> &IdentityWallet        { &self.identity }
    pub fn dashpay(&self)  -> &DashPayWallet         { &self.dashpay }
    pub fn platform(&self) -> &PlatformAddressWallet { &self.platform }
}
```

Call sites:

```rust
wallet.core().send_transaction(outputs).await?
wallet.identity().register_identity(amount, keys).await?
wallet.dashpay().send_contact_request(sender, recipient).await?
wallet.platform().sync_balances().await?
```

`sync()` on `PlatformWallet` orchestrates sub-struct syncs:

```rust
pub async fn sync(&self) -> Result<SyncResult, PlatformWalletError> {
    self.identity.sync().await?;
    self.dashpay.sync().await?;
    self.platform.sync_platform_address_balances(None).await?;
    Ok(SyncResult::default())
}
```

---

### 1.1 Wallet Construction

> How a `PlatformWallet` is created from a seed, mnemonic, xprv, xpub, or randomly.

`PlatformWallet` wraps `key-wallet`'s `Wallet` + `ManagedWalletInfo` and adds `Sdk`.
There are two independent axes of configuration:

1. **Key material** — mnemonic, seed, xprv, xpub, or random
2. **Network connection** — `NetworkOptions` (builds `Sdk` internally) or pre-built `Sdk`

A **builder pattern** avoids a combinatorial explosion of constructors. Two axes are
each **mutually exclusive**:

1. **Key material** — `with_mnemonic`, `with_xprv`, `with_xpub`, `with_seed` — only one
   allowed; `WalletType` is an enum in key-wallet, enforced at `build()`.
2. **SDK source** — `with_sdk` (pre-built) vs `with_network_options` (builder creates it
   internally) — using both is a `build()` error; `with_sdk` also fixes the network.

```rust
// Most common — developer provides mnemonic and network config
let wallet = PlatformWallet::builder()
    .with_mnemonic("word1 word2 ...", None)  // passphrase optional
    .with_network_options(opts)              // builds Sdk internally
    .with_name("My Wallet")
    .with_birth_height(1_500_000)            // skip blocks before wallet was created
    .build()?;

// Import from xprv
let wallet = PlatformWallet::builder()
    .with_xprv("xprv...")
    .with_network_options(opts)
    .build()?;

// Watch-only / hardware wallet
let wallet = PlatformWallet::builder()
    .with_xpub("xpub...", ExternalSigning::Supported)
    .with_network_options(opts)
    .build()?;

// For callers that already own an Sdk (e.g. evo-tool with ArcSwap<Sdk>)
let wallet = PlatformWallet::builder()
    .with_mnemonic("word1 word2 ...", None)
    .with_sdk(existing_sdk)                  // network derived from sdk.network
    .build()?;

// Generate a new random wallet (returns mnemonic for user to write down)
let (wallet, mnemonic) = PlatformWallet::generate(opts)?;
```

**Key material variants** — all mutually exclusive, delegate to `key-wallet`'s `Wallet`:

| Builder method                        | key-wallet equivalent                                     | Notes                               |
| ------------------------------------- | --------------------------------------------------------- | ----------------------------------- |
| `.with_mnemonic(phrase, passphrase?)` | `Wallet::from_mnemonic` / `from_mnemonic_with_passphrase` | passphrase NOT stored               |
| `.with_seed(bytes: [u8; 64])`         | `Wallet::from_seed_bytes`                                 | raw BIP39 seed                      |
| `.with_xprv(base58)`                  | `Wallet::from_extended_key`                               | full signing capability             |
| `.with_xpub(base58, signing)`         | `Wallet::from_xpub`                                       | watch-only or hardware wallet       |
| `generate()` fn                       | `Wallet::new_random`                                      | returns `(PlatformWallet, Mnemonic)`|

**`WalletAccountCreationOptions`**: builder uses `Default` (standard BIP-44 account 0 +
identity + DIP-17 accounts). Advanced callers can override via `.account_options(...)`.

**Birth height**: passed through to `ManagedWalletInfo::with_birth_height()` — SPV sync
starts from this block, skipping earlier history. Defaults to 0 (full sync).

#### Files

- `packages/rs-platform-wallet/src/wallet/builder.rs` (new)
- `packages/rs-platform-wallet/src/wallet/mod.rs`

---

### 1.2 Platform SDK Integration

> Make `PlatformWallet` the SDK access point for all callers.

**Current state**: SDK is stashed inside `IdentityManager.sdk` — accessed only by identity
discovery. Every async method that submits state transitions requires the caller to pass `&Sdk`
separately.

**Goal**: Each stored sub-struct (`CoreWallet`, `IdentityWallet`, `DashPayWallet`, `PlatformAddressWallet`)
holds `sdk: Sdk` as a field. All methods call `self.sdk` without requiring callers to manage
SDK lifecycle separately. `Sdk` is cheaply cloneable (internally ref-counted); no `Arc` wrapper.

#### Tasks

- **1.2.1** ✅ Add `sdk: Sdk` to each sub-struct. Clone from `PlatformWallet`'s sdk at construction.
- **1.2.2** Remove `sdk` from `IdentityManager`; all SDK access flows through the sub-struct `sdk` fields.

#### Files

- `packages/rs-platform-wallet/src/wallet/mod.rs`
- `packages/rs-platform-wallet/src/identity_manager/mod.rs`

---

### 1.2 Core Wallet Capabilities

> Expose UTXO wallet: accounts, addresses, balances, send Dash, SPV sync, asset lock proofs.

`key-wallet` (`rust-dashcore/key-wallet`) already implements all the building blocks:
`Wallet` (immutable key store), `ManagedWalletInfo` (mutable runtime state),
`TransactionBuilder` (coin selection, fee calc, signing), `AddressPool` (gap limit),
`WalletInfoInterface` + `ManagedAccountOperations` traits.
`dash-spv` handles SPV header sync and BIP157/158 compact filter transaction delivery.

`CoreWallet` is a stored sub-struct that holds `Arc<RwLock<ManagedWalletInfo>>` and exposes
these capabilities without leaking key-wallet internals. It implements `WalletInterface`
as a concrete stored type, so SPV registration is straightforward.

#### 1.2.1 — Wallet Initialization

Accounts are created automatically at wallet construction — callers never call
`add_account` explicitly. `PlatformWallet::new()` passes
`WalletAccountCreationOptions::Default` to `key-wallet`, which derives standard BIP-44
accounts and populates the initial address pool. This matches how evo-tool initializes
wallets via `import_wallet_from_extended_priv_key`.

DashPay and DIP-17 platform payment accounts are added lazily on first use
(contact establishment / first platform address request).

#### 1.2.2 — Address Generation

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
`ManagedAccountCollection`, not just `core_accounts`. This is how DashPay receiving addresses
get watched for incoming payments — no separate registration step, no manual bloom filter
management. When `DashPayWallet::sync()` adds a new `DashpayReceivingFunds` account (on contact
accepted), those addresses automatically appear in the next `monitored_addresses()` call.

#### 1.2.3 — Balance & UTXO Access

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

#### 1.2.4 — Transaction Send

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

Custom flow (e.g. specific fee rate, coin selection strategy):

```rust
let (utxos, key_fn) = wallet.core.spendable_utxos_with_keys();
let tx = wallet.core
    .transaction_builder()
    .add_output(&addr, amount_duffs)?
    .set_fee_rate(FeeRate::from_sat_per_byte(5))
    .select_inputs(&utxos, SelectionStrategy::LargestFirst, current_height, key_fn)?
    .build()?;
let txid = wallet.core.broadcast_transaction(tx).await?;
```

`subtract_fee_from_amount` and fee-override-on-retry are UI-level concerns — callers
handle them before calling `.add_output()`. No `WalletPaymentRequest` wrapper needed.
Dash P2PKH transactions have no memo field in the protocol; `memo` in evo-tool's
existing `WalletPaymentRequest` is dead code and is not carried forward.

`send_transaction` handles coin selection, signing, and broadcast internally — two broadcast paths:

- **SPV mode**: `DashSpvClient::broadcast_transaction(tx)` → P2P to connected peers
  (`dash-spv/src/client/transactions.rs`)
- **RPC mode**: `core_client.send_raw_transaction(tx)` → Dash Core JSON-RPC

`rs-sdk` (DAPI/Platform SDK) has no Core transaction broadcast — it's Platform-only.
The SPV client (`DashSpvClient`) is the P2P layer for Core transactions.

#### 1.2.5 — SPV Sync Integration

`dash-spv` (`DashSpvClient<W, N, S>`) is the P2P sync layer. It uses **BIP157/158 compact
block filters** (not Bloom filters). It takes a `WalletInterface` generic parameter — the
wallet registers itself so `dash-spv` can deliver relevant transactions.

`CoreWallet` implements `WalletInterface` from `key-wallet-manager` — it is the natural
boundary, wrapping `Arc<RwLock<ManagedWalletInfo>>`. `PlatformWallet` passes `wallet.core.clone()`
to `DashSpvClient` at startup; the client holds it and calls back into `CoreWallet` as blocks arrive.
Because `CoreWallet` holds an `Arc` clone, SPV and `PlatformWallet` share the same `ManagedWalletInfo`
without any additional locking at the `PlatformWallet` level.

```rust
impl WalletInterface for CoreWallet {
    fn monitored_addresses(&self) -> Vec<Address>
    // dash-spv uses these to match compact filters

    fn process_transaction(&mut self, tx: &Transaction, height: u32, block_time: u64) -> bool
    // called by dash-spv when a matching tx is found — delegates to wallet_info

    fn synced_height(&self) -> u32
    fn set_synced_height(&mut self, height: u32)
}
```

Transaction broadcasting goes through `DashSpvClient::broadcast_transaction(tx)` — P2P
to connected peers (see §1.2.4). `dash-spv` also delivers InstantLock and ChainLock events
needed for asset lock proof creation (§1.2.6).

#### 1.2.6 — Asset Lock Proof Creation

Required for identity **registration** and **top-up** (§1.3).

```rust
pub async fn create_asset_lock_proof(
    &self,
    amount_duffs: u64,
) -> Result<(AssetLockProof, PrivateKey), CoreWalletError>
```

`CoreWallet` method — derives the next DIP-13 funding key internally, sources UTXOs
from `wallet_info`, builds an `AssetLock` special transaction via `TransactionBuilder`,
broadcasts it, waits for the InstantLock via SPV, returns `(AssetLockProof, funding_private_key)`.

DIP-13 funding key paths:

- Registration: `m/9'/coin'/5'/1'/identity_index` (one-time, non-reusable)
- Top-up (unbound): `m/9'/coin'/5'/2'/topup_index`
- Top-up (bound): `m/9'/coin'/5'/2'/registration_index'/topup_index`

#### 1.2.7 — Asset Lock Recovery

```rust
pub async fn recover_asset_locks(&self) -> Result<Vec<RecoveredAssetLock>, CoreWalletError>
```

Scans known funding key paths for broadcast-but-unconfirmed asset lock transactions
and attempts to recover or rebroadcast them. Mirrors evo-tool's
`CoreTask::RecoverAssetLocks`.

#### Files

- `packages/rs-platform-wallet/src/wallet/core_wallet.rs` (new)
- Depends on: `key-wallet` (`ManagedWalletInfo`, `TransactionBuilder`, `WalletInfoInterface`,
  `ManagedAccountOperations`, `FeeRate`, `SelectionStrategy`)
- Depends on: `dash-spv` (`WalletInterface` impl, `broadcast_transaction`, InstantLock/ChainLock events)

---

### 1.3 Identity Management

> Register, discover, refresh, top-up, withdraw, transfer, update identities. Register DPNS names.

All methods are on `IdentityWallet` which holds `sdk`, `wallet: Arc<Wallet>`, and `identity_manager`.
No `wallet: &Wallet` parameter anywhere — key derivation and signing use `self.wallet` directly.

#### 1.3.1 — Register New Identity

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
2. Derive auth keys from `m/9'/coin'/5'/0'/key_type'/identity_index'/key_index'` via `self.core`
3. Build and sign `IdentityCreateTransition` via `self.sdk`
4. Broadcast, wait for proof, add to `identity_manager`

#### 1.3.2 — Identity Discovery (DIP-13 gap-limit scan)

Implementation exists in the old `identity_discovery.rs` (now deleted with the rename).
Current behaviour:

- Derives ECDSA auth key at `key_index=0` only
- Queries Platform via `Identity::fetch(&sdk, PublicKeyHash(key_hash))`
- `start_index` and `gap_limit` passed by caller — state not persisted
- SDK pulled from `IdentityManager.sdk` (stale pattern — sdk moves to `IdentityWallet`)
- Errors during fetch silently treated as misses (just prints to stderr)

**What needs fixing:**

- Move from `PlatformWalletInfo::discover_identities` → `IdentityWallet::sync()`, no parameters
- Store `last_scanned_index: u32` in `IdentityManager` — persist and resume from it
- Gap limit hardcoded to 5 (DIP-13 spec), remove caller-controlled parameter
- Derive auth keys for all standard key types (ECDSA, BLS, EdDSA), not just ECDSA index 0
- Surface fetch errors properly instead of swallowing them to stderr
- SDK sourced from `self.sdk` on `IdentityWallet`, not from `IdentityManager.sdk`

```rust
pub async fn sync(&mut self) -> Result<Vec<Identifier>, PlatformWalletError>
```

#### 1.3.3 — Refresh Identity

```rust
pub async fn refresh_identity(
    &mut self,
    identity_id: &Identifier,
) -> Result<(), PlatformWalletError>
```

Fetches latest balance and keys from Platform, updates `ManagedIdentity`.

#### 1.3.4 — Top Up Identity Credits

```rust
pub async fn top_up_identity(
    &mut self,
    identity_id: &Identifier,
    amount_duffs: u64,
) -> Result<u64, PlatformWalletError>  // returns new balance
```

Steps:

1. `self.core.create_asset_lock_proof(amount_duffs)` — derives next top-up key internally
2. Submit `IdentityTopUpTransition` via `self.sdk`
3. Update `ManagedIdentity` balance

#### 1.3.5 — Withdraw Credits to Core

```rust
pub async fn withdraw_identity_credits(
    &mut self,
    identity_id: &Identifier,
    to_address: Option<Address>,  // None = next wallet receive address from self.core
    amount_credits: u64,
    core_fee_per_byte: Option<u32>,
) -> Result<u64, PlatformWalletError>  // returns remaining balance
```

Calls `sdk::WithdrawFromIdentity::withdraw()` with the identity's withdrawal key.
Signs using `IdentitySigner` (see §1.6).

#### 1.3.6 — Transfer Credits Between Identities

```rust
pub async fn transfer_credits(
    &mut self,
    from_identity_id: &Identifier,
    to_identity_id: &Identifier,
    amount_credits: u64,
) -> Result<u64, PlatformWalletError>
```

#### 1.3.7 — Update Identity Keys

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

- `packages/rs-platform-wallet/src/wallet/identity_wallet.rs` (new)
- `packages/rs-platform-wallet/src/wallet/identity_discovery.rs` (extend)

---

### 1.4 DashPay — Contacts, Transactions, Sync

> Full DIP-14/15 implementation: contact requests, encrypted xpub exchange, payment address
> derivation, send/receive Dash between contacts.

#### DIP-14 Background

DashPay uses 256-bit derivation (CKDpriv256/CKDpub256) for contact-specific address spaces:

```
m(userA)/9'/5'/15'/0'/(userA_id_256bit)/(userB_id_256bit)/index
```

The 256-bit identity ID indices prevent the 31-bit collision attack. `CKDpriv256` is fully
compatible with BIP32 for indices < 2^32; uses `ser_256(i)` for larger indices.

**Current state**: Lives in `dash-evo-tool/src/backend_task/dashpay/dip14_derivation.rs`.
Moves to `packages/rs-platform-wallet/src/wallet/dashpay/dip14.rs` — DashPay-specific derivation lives alongside the DashPay operations that use it.

#### DIP-15 Background

A contact request document on Platform contains:

- `encryptedPublicKey` (96 bytes): AES-CBC-256 encrypted xpub (IV 16 + ciphertext 80)
- `encryptedAccountLabel` (optional 48-80 bytes): encrypted account name
- `accountReference` (32-bit): `(version<<28) | (HMAC-SHA256(senderKey, xpub)_28bits XOR account_28bits)`
- `senderKeyIndex` / `recipientKeyIndex`: identity keys used for ECDH

ECDH shared key: `SHA256( (y[31]&0x1 | 0x2) || x )` via `libsecp256k1_ecdh`.

**Current state**: Lives in `dash-evo-tool/src/backend_task/dashpay/encryption.rs`.
Moves to `packages/rs-platform-wallet/src/wallet/dashpay/encryption.rs` — encryption module lives inside `rs-platform-wallet`, no separate crate needed.

#### 1.4.1 — DIP-14 Key Derivation (dashpay module)

```rust
// packages/rs-platform-wallet/src/wallet/dashpay/dip14.rs  (new file)
pub fn ckd_priv_256(
    parent: &ExtendedPrivKey,
    index: &[u8; 32],
    hardened: bool,
) -> Result<ExtendedPrivKey>

pub fn ckd_pub_256(
    parent: &ExtendedPubKey,
    index: &[u8; 32],
) -> Result<ExtendedPubKey>

pub fn derive_dashpay_contact_xpub(
    master: &ExtendedPrivKey,
    network: Network,
    account: u32,
    sender_id: &[u8; 32],
    recipient_id: &[u8; 32],
) -> Result<ExtendedPubKey>
```

Test against DIP-14 Appendix A vectors (seed: "birth kingdom trash renew flavor utility donkey gasp regular alert pave layer").

#### 1.4.2 — DIP-15 ECDH + Encryption (dashpay encryption module)

```rust
// packages/rs-platform-wallet/src/wallet/dashpay/encryption.rs
pub fn ecdh_shared_key(
    private_key: &SecretKey,
    public_key: &PublicKey,
) -> [u8; 32]
// Formula: SHA256( (y[31]&0x1 | 0x2) || x )

pub fn aes_cbc_256_encrypt(key: &[u8; 32], plaintext: &[u8]) -> (Vec<u8>, [u8; 16])
pub fn aes_cbc_256_decrypt(key: &[u8; 32], iv: &[u8; 16], ciphertext: &[u8]) -> Result<Vec<u8>>

pub fn encrypt_extended_public_key(xpub: &ExtendedPubKey, shared_key: &[u8; 32]) -> Vec<u8>
// IV(16) + ciphertext(80) = 96 bytes total
pub fn decrypt_extended_public_key(data: &[u8; 96], shared_key: &[u8; 32]) -> Result<ExtendedPubKey>

pub fn compute_account_reference(
    account: u32,
    sender_secret_key_bytes: &[u8],
    xpub_bytes: &[u8],
    version: u8,
) -> u32
// ASK = HMAC-SHA256(senderSecretKey, xpub)
// result = (version << 28) | (ASK_28msb XOR (account & 0x0FFFFFFF))
```

#### 1.4.3 — Send Contact Request

Already partially implemented in `contact_requests.rs`. Complete and consolidate:

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
2. Find recipient first ENCRYPTION key
3. Derive contact xpub via DIP-14: `derive_dashpay_contact_xpub(..., sender_id, recipient_id)`
4. ECDH shared key from sender private key + recipient public key
5. Encrypt xpub → `encryptedPublicKey`
6. Compute `accountReference`
7. Submit `DocumentsBatchTransition` via rs-sdk `send_contact_request()`
8. Store in `ManagedIdentity.sent_contact_requests`
9. Add `DashpayReceivingFunds` account to `ManagedAccountCollection`

#### 1.4.4 — Decrypt Incoming Contact Request

```rust
pub fn decrypt_incoming_contact_request(
    &self,
    our_identity_id: &Identifier,
    contact_request: &ContactRequest,
) -> Result<ExtendedPubKey, PlatformWalletError>
```

Steps:

1. Retrieve our ENCRYPTION private key at `contact_request.recipient_key_index`
2. Retrieve sender public key at `contact_request.sender_key_index`
3. Compute ECDH shared key
4. Decrypt `contact_request.encrypted_public_key` → `ExtendedPubKey`
5. Store xpub as `DashpayExternalAccount` in `ManagedAccountCollection`

#### 1.4.5 — Payment Address Derivation

```rust
pub fn derive_payment_address_for_contact(
    &self,
    our_identity_id: &Identifier,
    contact_id: &Identifier,
    payment_index: u32,
) -> Result<Address, PlatformWalletError>
```

Non-hardened BIP32 child of the stored `DashpayExternalAccount` xpub at `payment_index`.
Payment gap limit: 10 (per DIP-15 §Created At Timestamp sync notes).

#### 1.4.6 — Send Payment to Contact

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

#### 1.4.7 — DashPay Sync (`DashPayWallet::sync()`)

`DashPayWallet::sync()` is the Platform-side half of DashPay sync. It fetches new contact
request documents from DAPI and establishes the corresponding address accounts:

```rust
pub async fn sync(&mut self) -> Result<DashPaySyncResult, PlatformWalletError>
```

For each known identity, in order:

1. Fetch incoming contact requests from Platform since last sync timestamp
2. For each new request: call `decrypt_incoming_contact_request()` to get the sender's xpub
3. Add a `DashpayReceivingFunds` account to `ManagedAccountCollection` keyed by
   `(our_identity_id, sender_identity_id)` — pre-derives `gap_limit` (20) addresses
4. Also fetch outgoing contact requests that now have a matching incoming (mutual) — those
   are established contacts; ensure the `DashpayReceivingFunds` account exists for them too

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

- Receiving address pool per contact: 20 (same as BIP44 core)
- DIP-15 specifies wallet should watch `highest_receive_index + 20` addresses per contact

#### 1.4.8 — Profile Management

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

#### 1.4.9 — Contact Info Document (Encrypted Private Metadata)

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

#### 1.4.10 — DPNS Name Registration

DPNS usernames are the lookup mechanism for DashPay contact discovery — registering a
name makes the identity findable by other users.

```rust
pub async fn register_dpns_name(
    &mut self,
    identity_id: &Identifier,
    name: &str,
) -> Result<Identifier, PlatformWalletError>  // document id
```

#### 1.4.11 — Auto-Accept Proof

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

- `packages/rs-platform-wallet/src/wallet/dashpay/dip14.rs` (new — DIP-14 CKDpriv256/CKDpub256)
- `packages/rs-platform-wallet/src/wallet/dashpay/encryption.rs` (new — DIP-15 ECDH + AES)
- `packages/rs-platform-wallet/src/wallet/dashpay/mod.rs` (new — consolidates contact_requests.rs)

---

### 1.5 Platform Addresses (DIP-17)

> Sync, send, transfer, and withdraw DIP-17 P2PKH credits through `PlatformWallet`.

**Key finding**: `ManagedAccountCollection` already has `platform_payment_accounts:
BTreeMap<PlatformPaymentAccountKey, Account>`. `ManagedPlatformAccount` (key-wallet) tracks
per-address credit balances + gap-limit address pool. `PlatformWallet` must expose these
and implement the SDK's `AddressProvider` trait.

Derivation path (DIP-17): `m/9'/coin_type'/17'/account'/key_class'/index`
Gap limit: 20 (`DIP17_GAP_LIMIT` constant already in key-wallet `gap_limit.rs`).

#### 1.5.1 — AddressProvider Implementation

The rs-sdk's `sync_address_balances()` requires `&mut impl AddressProvider`:

```rust
// platform_wallet/platform_address_provider.rs
impl AddressProvider for PlatformAddressWallet {
    fn addresses(&self, account: u32, key_class: u32) -> Vec<PlatformP2PKHAddress>
    fn next_unused_address(&mut self, account: u32, key_class: u32) -> PlatformP2PKHAddress
    fn apply_balance(&mut self, address: &PlatformP2PKHAddress, balance: u64, nonce: u64)
    fn found_balances(&self) -> Vec<(Address, AddressFunds)>
    fn found_balances_with_indices(&self) -> Vec<(u32, (&Address, &AddressFunds))>
    // no apply_results_to_wallet — PlatformAddressWallet already holds the state
}
```

Reads/writes from `wallet_info.accounts.platform_payment_accounts`.

#### 1.5.2 — Platform Address Sync

```rust
pub async fn sync_platform_address_balances(
    &mut self,
    last_sync_timestamp: Option<u64>,
) -> Result<AddressSyncResult, PlatformWalletError>
```

Calls `self.sdk.sync_address_balances(self_as_provider, config, last_sync_timestamp)`.
Updates `platform_payment_accounts` via `apply_balance()`.

#### 1.5.3 — Balance Accessors

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

#### 1.5.4 — Send Credits to Platform Address (Top Up Address)

```rust
pub async fn top_up_platform_address(
    &self,
    identity_id: &Identifier,
    target_address: &PlatformP2PKHAddress,
    amount_credits: u64,
) -> Result<(), PlatformWalletError>
```

Calls `sdk::TopUpAddress` state transition, funded from the identity's balance.

#### 1.5.5 — Transfer Between Platform Addresses

```rust
pub async fn transfer_platform_address_funds(
    &self,
    from_addresses: BTreeMap<PlatformP2PKHAddress, u64>,  // address -> credits
    to_address: &PlatformP2PKHAddress,
    fee_strategy: AddressFundsFeeStrategy,
) -> Result<(), PlatformWalletError>
```

Calls `sdk::TransferAddressFunds`. Each `from_address` signed with its DIP-17 derived key.

#### 1.5.6 — Withdraw Platform Address Credits to Core

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

#### 1.5.7 — Platform Address Signer

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

- `packages/rs-platform-wallet/src/wallet/platform_addresses.rs` (new)
- `packages/rs-platform-wallet/src/wallet/platform_address_signer.rs` (new)

---

### 1.6 State Transition Signing Facade

> `PlatformWallet` provides `IdentitySigner` so callers never manage key material directly.

```rust
// wallet/signer.rs
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

- `packages/rs-platform-wallet/src/wallet/signer.rs` (new)

---

### 1.7 Serialization / Persistence

> `PlatformWallet` is the single persistence unit — callers (e.g. evo-tool's SQLite) store
> the blob and don't need to know about sub-struct layout.

```rust
// Top-level backup/restore — covers Wallet + ManagedWalletInfo + IdentityManager + DashPay state
pub fn backup(&self) -> Result<Vec<u8>, PlatformWalletError>
pub fn restore(data: &[u8]) -> Result<Self, PlatformWalletError>
```

`Sdk` is excluded from the blob (it's a live connection) — caller re-provides it via
`PlatformWalletBuilder::with_sdk(sdk).restore(blob)` or `with_network_options(opts).restore(blob)`.

`ManagedWalletInfo` and `ManagedAccountCollection` already have `#[cfg(feature="bincode")]`
encode/decode. `ManagedPlatformAccount` and `PlatformP2PKHAddress` already have bincode.
Still missing serialization:

- `IdentityManager` — add bincode `Encode`/`Decode`
- `ManagedIdentity` (Identity + BlockTime + contact maps) — add bincode
- `ContactRequest` — add bincode
- `EstablishedContact` — add bincode

#### Files

- `packages/rs-platform-wallet/src/identity_manager/serialization.rs` (new)
- `packages/rs-platform-wallet/src/managed_identity/serialization.rs` (new)
- `packages/rs-platform-wallet/src/contact_request.rs`
- `packages/rs-platform-wallet/src/established_contact.rs`

---

### 1.8 Sync Architecture

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
pub async fn sync(&mut self) -> Result<SyncResult, PlatformWalletError>
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

---

## PR Sequence

Each PR implements features in `rs-platform-wallet` **and** immediately integrates into `evo-tool`.
Old evo-tool code is deleted in the same PR that introduces the replacement.

---

### PR-1: Project Scaffold + CoreWallet

**Library** (`rs-platform-wallet`):

- `PlatformWallet` struct skeleton with builder (§1.1, §Struct Definitions)
- `CoreWallet` with `ManagedWalletInfo` Arc, `WalletInterface` impl (§1.2)
- `monitored_addresses()` returns all account types including dashpay receival
- `send_transaction`, `broadcast_transaction`, asset lock proof creation (§1.2.4–1.2.6)
- `IdentitySigner` stub (§1.6) — needed for identity registration in PR-2
- `static_assertions::assert_impl_all!(PlatformWallet: Send, Sync)`

**evo-tool integration**:

- Add `platform-wallet = { path = "../../platform/packages/rs-platform-wallet" }` to `Cargo.toml`
- Replace `AppContext.wallets` type: `Arc<RwLock<Wallet>>` → `Arc<RwLock<PlatformWallet>>`
- `wallet_lifecycle.rs`: construct via builder on import/creation, wire `sdk` from `AppContext.sdk`
- `backend_task/core/refresh_wallet_info.rs`: feed through `CoreWallet::process_transaction()`
- Delete `src/model/wallet/` (old custom wallet struct)

**Database migration** (in this PR):

- Add version byte to DB wallet record
- If old format: deserialize as old `Wallet`, convert to `PlatformWallet`, re-save
- On first run after migration: `IdentityManager` starts empty — identities re-discovered in PR-2

**Done when**: evo-tool builds with `PlatformWallet` as wallet type; SPV sync works; `send_transaction` works.

---

### PR-2: IdentityWallet

**Library** (`rs-platform-wallet`):

- `IdentityWallet` with `identity_manager`, sdk, wallet Arc (§1.3)
- `register_identity`, `discover_identities` / `sync()`, `refresh_identity` (§1.3.1–1.3.3)
- `top_up_identity`, `withdraw_identity_credits`, `transfer_credits` (§1.3.4–1.3.6)
- `add_key_to_identity`, `disable_identity_key` (§1.3.7)
- `IdentitySigner` complete (§1.6)
- `IdentityManager` bincode serialization (§1.7 partial)

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

- DIP-14: `ckd_priv_256`, `ckd_pub_256`, `derive_dashpay_contact_xpub` in `dashpay/dip14.rs` (§1.4.1)
- DIP-15: `ecdh_shared_key`, AES-CBC encrypt/decrypt, `encrypt_extended_public_key`, `compute_account_reference` in `dashpay/encryption.rs` (§1.4.2)
- `DashPayWallet` with `send_contact_request`, `decrypt_incoming_contact_request` (§1.4.3–1.4.4)
- `derive_payment_address_for_contact`, `send_dashpay_payment` (§1.4.5–1.4.6)
- `DashPayWallet::sync()` — fetches contact requests, adds `DashpayReceivingFunds` accounts, gap-limit pool management (§1.4.7)
- Profile, contact info, DPNS name, auto-accept proof (§1.4.8–1.4.11)
- `ManagedIdentity` contact maps + `ContactRequest` + `EstablishedContact` bincode (§1.7)

Test against DIP-14 Appendix A vectors before merging.

**evo-tool integration**:

| File | Action |
|------|--------|
| `backend_task/dashpay/dip14_derivation.rs` | Delete |
| `backend_task/dashpay/encryption.rs` | Delete |
| `backend_task/dashpay/hd_derivation.rs` | Delete |
| `backend_task/dashpay/contact_requests.rs` | → `wallet.dashpay.send_contact_request()` |
| `backend_task/dashpay/contacts.rs` | → `wallet.dashpay.sync()` |
| `backend_task/dashpay/payments.rs` | → `wallet.dashpay.send_dashpay_payment()` |
| `backend_task/dashpay/incoming_payments.rs` | → `wallet.dashpay.sync()` handles this |
| `backend_task/dashpay/profile.rs` | → `wallet.dashpay.create_dashpay_profile()` |
| `backend_task/dashpay/auto_accept_proof.rs` | → `wallet.dashpay.generate_auto_accept_proof()` |
| `backend_task/dashpay/contact_info.rs` | → `wallet.dashpay.update_contact_info()` |

**Done when**: DIP-14 vectors pass; contact requests sent/received and decrypted; incoming DashPay payments detected via SPV without manual address registration.

---

### PR-4: PlatformAddressWallet (DIP-17)

**Library** (`rs-platform-wallet`):

- `PlatformAddressWallet` with `AddressProvider` impl (§1.5.1)
- `sync_platform_address_balances`, balance accessors (§1.5.2–1.5.3)
- `top_up_platform_address`, `transfer_platform_address_funds`, `withdraw_platform_address_funds` (§1.5.4–1.5.6)
- `PlatformAddressSigner` (§1.5.7)

**evo-tool integration**:

- `backend_task/wallet/fetch_platform_address_balances.rs`: replace `WalletAddressProvider::new(&wallet, ...)` with `wallet.platform` as `AddressProvider`
- Replace `wallet.platform_address_info` field access with `wallet.platform.platform_address_info()`

**Done when**: DIP-17 address balance sync works; top-up, transfer, and withdrawal work in evo-tool.

---

### PR-5: Serialization + Final Cleanup

**Library** (`rs-platform-wallet`):

- `PlatformWallet::backup()` / `restore()` — full bincode blob excluding `Sdk` (§1.7)
- Any remaining missing `Encode`/`Decode` impls

**evo-tool integration**:

- Replace SQLite wallet blob serialization with `PlatformWallet::backup()`/`restore()`
- Wire `PlatformWalletBuilder::with_sdk(sdk).restore(blob)` on wallet load
- Remove any remaining evo-tool wallet shim code

**Done when**: Wallet persists and restores correctly across restarts; no old wallet code remains in evo-tool.

---

## Address Type Coverage Summary

| Address type | DIP | Derivation path | key-wallet collection field | Platform 1 section |
|---|---|---|---|---|
| Core UTXO receive | BIP44 | `m/44'/coin'/acct'/0/i` | `core_accounts` | ✓ via `WalletInfoInterface` |
| Core UTXO change | BIP44 | `m/44'/coin'/acct'/1/i` | `core_accounts` | ✓ via `WalletInfoInterface` |
| Identity reg. funding | DIP-13 | `m/9'/coin'/5'/1'/i` | — | §1.3.1 |
| Identity top-up funding | DIP-13 | `m/9'/coin'/5'/2'/i` | — | §1.3.4 |
| Identity auth keys | DIP-13 | `m/9'/coin'/5'/0'/ktype'/id'/key'` | — | §1.3.1 |
| DashPay receive from contact | DIP-15 | `m/9'/coin'/15'/acct'/(self)/(friend)/i` | `dashpay_receival_accounts` | §1.4.3 |
| DashPay send to contact | DIP-15 | contact xpub + index | `dashpay_external_accounts` | §1.4.4 |
| Platform P2PKH (credits) | DIP-17 | `m/9'/coin'/17'/acct'/class'/i` | `platform_payment_accounts` | §1.5 |

---

## Risk Analysis

| Risk | Mitigation |
|---|---|
| `IdentityManager`/`ManagedIdentity` not serializable | Add bincode impls as §1.7; test round-trip before Phase 2 |
| DB migration corrupts existing wallets | Version byte in DB; fallback read → convert; test against real DB fixture |
| DIP-14 `index_to_child_number` interop with DashSync iOS | Verify against DashSync test vectors; add cross-client vector test |
| Gap limit confusion (DIP-13: 5 auth / 30 topup; DIP-15: 10 payment) | Named constants per use case; never share a limit variable |
| `PlatformWallet` not `Send+Sync` | Add `static_assertions::assert_impl_all!(PlatformWallet: Send, Sync)` |
| `Arc<RwLock<ManagedWalletInfo>>` write starvation under concurrent SPV + Platform sync | SPV writes are short (tx update); Platform sync holds read lock briefly for balance reads — test under load |

---

## Sources & References

### DIPs

- [DIP-0013: Identities in HD Wallets](https://github.com/dashpay/dips/blob/master/dip-0013.md) — auth, registration, top-up, invitation funding paths; gap limits
- [DIP-0014: Extended Key Derivation (256-bit)](https://github.com/dashpay/dips/blob/master/dip-0014.md) — CKDpriv256/CKDpub256 spec and test vectors
- [DIP-0015: DashPay](https://github.com/dashpay/dips/blob/master/dip-0015.md) — contact request structure, ECDH, AES-CBC encryption, account reference, DashPay payment paths
- [DIP-0017: Dash Platform P2PKH Addresses](https://github.com/dashpay/dips/blob/master/dip-0017.md) — platform payment addresses at `m/9'/coin'/17'/account'/key_class'/index`

### Key Repositories

| Repo | Disk Path | Notes |
| ---- | --------- | ----- |
| `rs-platform-wallet` | `packages/rs-platform-wallet/` | Target library (this plan) |
| `key-wallet` | `../rust-dashcore/key-wallet/` | UTXO wallet, key derivation, TransactionBuilder |
| `key-wallet-manager` | `../rust-dashcore/key-wallet-manager/` | `WalletInterface` trait |
| `dash-spv` | `../rust-dashcore/dash-spv/` | SPV client, BIP157/158 sync, push-based |
| `rs-sdk` | `packages/rs-sdk/` | DAPI client (`Sdk`, `SdkBuilder`) |
| `dash-evo-tool` | `../dash-evo-tool/` | Phase 2 integration target |

### Platform Wallet (current)

- [packages/rs-platform-wallet/src/wallet/mod.rs](packages/rs-platform-wallet/src/wallet/mod.rs)
- [packages/rs-platform-wallet/src/wallet/identity_discovery.rs](packages/rs-platform-wallet/src/wallet/identity_discovery.rs)
- [packages/rs-platform-wallet/src/wallet/contact_requests.rs](packages/rs-platform-wallet/src/wallet/contact_requests.rs)
- [packages/rs-platform-wallet/src/managed_identity/mod.rs](packages/rs-platform-wallet/src/managed_identity/mod.rs)

### Key Wallet

- DIP-17 account: `rust-dashcore/key-wallet/src/managed_account/managed_platform_account.rs`
- Account collection: `rust-dashcore/key-wallet/src/account/account_collection.rs` — `platform_payment_accounts`
- Gap limits: `rust-dashcore/key-wallet/src/gap_limit.rs` — `DIP17_GAP_LIMIT = 20`

### SDK Transitions Used

- [packages/rs-sdk/src/platform/transition/withdraw_from_identity.rs](packages/rs-sdk/src/platform/transition/withdraw_from_identity.rs)
- [packages/rs-sdk/src/platform/transition/top_up_identity.rs](packages/rs-sdk/src/platform/transition/top_up_identity.rs)
- [packages/rs-sdk/src/platform/transition/address_credit_withdrawal.rs](packages/rs-sdk/src/platform/transition/address_credit_withdrawal.rs)
- [packages/rs-sdk/src/platform/transition/transfer_address_funds.rs](packages/rs-sdk/src/platform/transition/transfer_address_funds.rs)
- [packages/rs-sdk/src/platform/transition/top_up_address.rs](packages/rs-sdk/src/platform/transition/top_up_address.rs)

### Evo Tool (to be replaced)

- `dash-evo-tool/src/backend_task/dashpay/dip14_derivation.rs`
- `dash-evo-tool/src/backend_task/dashpay/encryption.rs`
- `dash-evo-tool/src/backend_task/wallet/fetch_platform_address_balances.rs`
- `dash-evo-tool/src/model/wallet/`
