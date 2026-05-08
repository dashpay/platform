# Platform Wallet

The `rs-platform-wallet` crate (`packages/rs-platform-wallet`) is the Rust wallet
implementation for Dash Platform client applications. It bridges two distinct asset
layers: the Layer 1 UTXO chain and the Layer 2 Platform identity system.

## Overview

Traditional Dash wallets track coins — UTXOs, addresses, balances. Platform adds a
second layer on top: identities that hold credits, sign documents, and interact with
decentralized applications. Managing both layers together requires state that neither a
plain key-wallet nor the SDK alone provides.

`rs-platform-wallet` fills that gap. A single `PlatformWallet` handle gives you:

- **Layer 1 (Core):** HD accounts, UTXO tracking, address derivation, transaction
  broadcast via SPV, and a lock-free balance cache for UI rendering.
- **Layer 2 (Platform):** Multiple managed identities per wallet, asset lock lifecycle
  tracking (the on-ramp from UTXO to Platform credits), DPNS name records, DashPay
  contact management, and per-platform-address credit balances.

The design goal is a client that an application can embed without running a full node:
SPV provides chain data, the `dash-sdk` provides proof-verified Platform queries, and
the wallet ties the two together with a changeset-based persistence model that avoids
lock contention.

For the JavaScript/TypeScript equivalent — offline key derivation utilities used by the
Evo SDK — see [Wallet Utilities](../evo-sdk/wallet-utilities.md).

## Quick Start

The shortest path to a working manager is three calls — provide a `Sdk`, a
`PlatformWalletPersistence` implementation, and a `PlatformEventHandler`:

```rust
let sdk = Arc::new(Sdk::new_mock());
let persister = Arc::new(NoopPersister);
let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);

let manager = PlatformWalletManager::new(sdk, persister, event_handler);

let wallet = manager
    .create_wallet_from_seed_bytes(
        Network::Testnet,
        [0u8; 64],
        WalletAccountCreationOptions::Default,
    )
    .await?;
```

For the full runnable program — including no-op `PlatformWalletPersistence` and
`PlatformEventHandler` impls, balance reads, address derivation, and a `state()`
guard walk-through — see
[`packages/rs-platform-wallet/examples/basic_usage.rs`][basic_usage_quickstart].

[basic_usage_quickstart]: https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-platform-wallet/examples/basic_usage.rs

Production code replaces `Sdk::new_mock()` with a real `SdkBuilder` configuration
(see [Builder Pattern](builder-pattern.md)) and replaces the no-op persister with a
database-backed implementation of `PlatformWalletPersistence`.

## Core Types

### `PlatformWalletManager`

`PlatformWalletManager<P>` is the top-level coordinator. It is generic over a
persistence backend `P: PlatformWalletPersistence`. One manager instance owns:

- An `Arc<dash_sdk::Sdk>` for Platform queries.
- An `Arc<RwLock<WalletManager<PlatformWalletInfo>>>` that holds all wallets' key
  material and per-account UTXO state.
- An `Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>` for lightweight handle
  lookup without acquiring the SPV-contended manager lock.
- A `SpvRuntime` that drives block-filter sync across all registered wallets.
- A `PlatformAddressSyncManager` for periodic credit-balance refresh (BLAST sync).

The manager is not `Clone` — it is meant to be held in a single `Arc` and shared
across threads.

**Creating wallets:**

```rust
// From a BIP-39 mnemonic (language auto-detected across all supported wordlists)
let wallet = manager
    .create_wallet_from_mnemonic(
        "abandon abandon abandon ...",
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .await?;

// From raw 64-byte seed material
let wallet = manager
    .create_wallet_from_seed_bytes(Network::Testnet, seed, WalletAccountCreationOptions::Default)
    .await?;
```

### `PlatformWallet`

`PlatformWallet` is a lightweight, cheaply cloneable handle to a single wallet's
shared state. Clones are **shared references** — not independent copies. All clones
see the same UTXOs, balances, and identities through the `Arc<RwLock<...>>` inside
`PlatformWalletManager`.

`PlatformWallet` exposes four sub-wallet facets:

| Accessor | Type | Purpose |
|----------|------|---------|
| `wallet.core()` | `CoreWallet` | Addresses, UTXOs, transaction broadcast |
| `wallet.identity()` | `IdentityWallet` | Identity registration and top-ups |
| `wallet.platform()` | `PlatformAddressWallet` | Platform address credit balances |
| `wallet.tokens()` | `TokenWallet` | Platform token balances |

The `wallet.asset_locks()` accessor returns an `Arc<AssetLockManager>` for tracking
the lifecycle of asset lock transactions (the on-ramp from UTXO to Platform credits).

**Reading state:** Use `wallet.state().await` to acquire a read guard that derefs to
`PlatformWalletInfo`.

**Lock-free balance:** `wallet.balance()` returns `&Arc<WalletBalance>` without
acquiring any lock. It is updated by the SPV event handler after each block. Use it
for UI rendering where you cannot await.

### `PlatformWalletInfo`

`PlatformWalletInfo` is the mutable state struct that lives inside the shared
`WalletManager`. It is not accessed directly by application code — you reach it
through the `wallet.state()` guard. Its fields are:

```rust
pub struct PlatformWalletInfo {
    pub core_wallet: ManagedWalletInfo,   // accounts, UTXOs, transaction history
    pub balance: Arc<WalletBalance>,      // lock-free balance cache
    pub identity_manager: IdentityManager,
    pub tracked_asset_locks: BTreeMap<OutPoint, TrackedAssetLock>,
    pub token_watched: BTreeMap<Identifier, BTreeSet<Identifier>>,
    pub token_balances: BTreeMap<(Identifier, Identifier), TokenAmount>,
}
```

`ManagedWalletInfo` (from `key-wallet`) holds the HD accounts. `IdentityManager`
holds all `ManagedIdentity` records. The token maps index by `(identity_id,
contract_id)`.

## Wallet Lifecycle

### Creation

Both creation paths — mnemonic and seed bytes — end up calling `register_wallet`
internally. That function:

1. Creates a `ManagedWalletInfo` from the `Wallet` key material.
2. Inserts the new wallet into the `WalletManager` under a write lock.
3. Builds a `PlatformWallet` handle with all sub-wallet facets.
4. Loads persisted state from the `PlatformWalletPersistence` backend and applies
   any stored changeset.
5. Inserts the handle into the wallets map and returns an `Arc<PlatformWallet>`.

### Persistence Model

The crate uses a **BDK-style changeset approach**: every mutation method produces a
`PlatformWalletChangeSet` describing what changed. An external persister consumes
these changesets asynchronously, translating them to storage at its own pace.

```rust
pub struct PlatformWalletChangeSet {
    pub core: Option<key_wallet::changeset::WalletChangeSet>,
    pub identities: Option<IdentityChangeSet>,
    pub asset_locks: Option<AssetLockChangeSet>,
    pub platform_addresses: Option<PlatformAddressChangeSet>,
    pub token_balances: Option<TokenBalanceChangeSet>,
    // ... contact changesets
}
```

The changeset is **idempotent**: applying the same changeset twice produces the same
result as applying it once. This makes it safe to replay on startup to reconstruct
state. The `Merge` trait (also re-exported from this crate) lets you accumulate
multiple changesets into one before writing to storage.

**Recovery:** On startup, call `persister.load()` to retrieve a `ClientStartState`,
then pass it to `PlatformWalletManager`'s load path. The manager applies each stored
changeset to restore wallet and identity state without re-fetching from the network.

See `PERSISTENCE_REDESIGN.md` in `packages/rs-platform-wallet/` for the full design
rationale, including why the previous lock-contention approach was replaced.

## Private Key Storage

`rs-platform-wallet` is deliberately agnostic about where seed material and signing
keys live. The crate never persists the seed itself: `PlatformWalletPersistence`
captures only the changeset (UTXOs, identity records, balances, watch-only xpubs),
so an embedding application is responsible for choosing a storage strategy that
matches its threat model. Three patterns cover most deployments.

### In-process seed (development and dApps)

The simplest setup: load a BIP-39 mnemonic or 64-byte seed in memory and call
`create_wallet_from_mnemonic` / `create_wallet_from_seed_bytes`. The `Wallet`
retains the seed for HD derivation while the process is running. Drop the seed
material as soon as you no longer need it.

This is appropriate for tests, server-side bots, or dApps where the user explicitly
provides a seed per session. **Do not** ship this pattern to end-user mobile or
desktop apps without an OS-level secret store underneath it.

### OS-managed secret storage (mobile and desktop)

For shipped applications, route the seed through the platform's native secret
store. The wallet stays watch-only at rest and the seed is read on-demand for
signing:

- **iOS / macOS** — store the encoded mnemonic (or seed bytes) in the iOS
  Keychain via the Security framework, gated behind biometrics. The Swift SDK
  examples in `packages/swift-sdk/SwiftExampleApp` demonstrate the unlock-then-sign
  pattern.
- **Android** — use the Android Keystore plus `EncryptedSharedPreferences`.
- **Linux / Windows** — Secret Service (libsecret) or DPAPI / Windows Credential
  Manager respectively, fronted by a small Rust crate such as `keyring`.

The integration shape is the same in all three cases:
1. Persist only public material via `PlatformWalletPersistence` (xpubs, address
   pools, identity records).
2. Keep the wallet watch-only in the manager for routine queries and balance
   display — no seed material in memory.
3. On a signing path, fetch the seed from the OS secret store, register a fully
   keyed wallet for the operation, perform the signing, then drop the in-memory
   key material.

### External signers (hardware wallets, HSMs, custodial)

Hardware wallets (Trezor, Ledger), remote signers, or custodial services do not
expose private keys at all. Integrate them by implementing the
`dpp::identity::signer::Signer` trait and routing identity operations through the
external-signer variants on `IdentityWallet` (`register_identity_with_funding_external_signer`,
`top_up_identity_with_signer`, `withdraw_credits_with_external_signer`).
For UTXO-side spends, build transactions using the wallet's PSBT helpers and sign
externally before broadcasting through `SpvBroadcaster`.

This is the **recommended path for production**: the platform wallet handles state,
balances, and changeset persistence, while the signer module isolates key custody.
The internal `IdentitySigner` flow is being phased out for the same reason — it
forces the seed into the wallet process and is incompatible with watch-only setups.

## Identity Flow

The lifecycle from seed phrase to a funded Platform identity runs through three
phases — **register**, **top up**, and **withdraw** — each exposed as a method on
`IdentityWallet` (accessed via `platform_wallet.identity()`):

- **Register** converts a UTXO into an asset lock transaction that funds a new
  identity on Platform. The asset lock is created, signed, and broadcast on the Core
  chain; the Platform state transition is sent to DAPI once the asset lock is
  confirmed. See `register_identity_with_funding_external_signer` (recommended);
  the internal-signer variants `register_identity` and `register_identity_with_funding`
  are being phased out.
- **Top up** locks another UTXO and adds credits to an existing identity. The
  internal-signer variants `top_up_identity` and `top_up_identity_with_funding` are
  being phased out in favour of the external-signer flow.
- **Withdraw** creates a Platform state transition that returns credits to the Core
  chain as Dash. See `withdraw_credits_with_external_signer` (recommended); the
  internal-signer variants `withdraw_credits` and `withdraw_credits_with_signer` are
  being phased out.

Each operation has a convenience variant that funds from wallet UTXOs and signs with
the wallet's internal signer, plus an `_with_funding` / `_external_signer` variant
for explicit control over the funding source and signer. **For new code, prefer the
`_external_signer` variants** — the internal `IdentitySigner` is being deprecated due
to incompatibility with watch-only wallets and potential Tokio worker deadlocks.

End-to-end code samples for each flow are not yet checked in alongside this chapter;
see [`packages/rs-platform-wallet/examples/basic_usage.rs`][basic_usage] for the
manager bring-up path. Worked examples for register / top-up / withdraw are tracked
as a follow-up against the integration-test framework PR
([dashpay/platform#3549](https://github.com/dashpay/platform/pull/3549)).

[basic_usage]: https://github.com/dashpay/platform/blob/v3.1-dev/packages/rs-platform-wallet/examples/basic_usage.rs

### Asset Lock Lifecycle

Asset locks are tracked via `AssetLockManager`. Each lock moves through a state
machine:

```text
Created → Broadcast → Seen in mempool → InstantLocked / ChainLocked → Spent (on Platform)
```

`TrackedAssetLock` holds the current `AssetLockStatus` and the `OutPoint` of the
funding UTXO. When an InstantLock event arrives via SPV, the manager notifies any
`AssetLockManager` waiters that are blocking on confirmation.

```rust
let asset_locks = wallet.asset_locks();
let locked = asset_locks.list_tracked_locks_blocking();
for lock in locked {
    println!("outpoint={} status={:?}", lock.out_point, lock.status);
}
```

## Balance and UTXO Tracking

### SPV Sync

`SpvRuntime` drives the block-filter sync loop across all registered wallets. After
registering wallets, explicitly start the runtime with `SpvRuntime::start` or
`SpvRuntime::spawn_in_background` to begin syncing. The sync loop:

1. Downloads compact block filters from the peer network.
2. Matches filters against each wallet's monitored addresses.
3. Fetches full blocks for matches, extracts relevant transactions.
4. Updates UTXOs, transaction history, and the lock-free `WalletBalance` atomic.
5. Emits a balance-changed event so the UI can re-render without holding a lock.

### Address Derivation

Addresses follow BIP-44 derivation. The default account is `0` for standard
payments. A separate account index is used internally for identity funding UTXOs.

```rust
// Derive the next unused receive address for account 0
let addr = wallet.core().next_receive_address_for_account(0).await?;

// Inspect UTXOs under the read guard
let state = wallet.state().await;
let synced_height = state.core_wallet.synced_height();
if let Some(account) = state.core_wallet.accounts.standard_bip44_accounts.get(&0) {
    let utxos = account.spendable_utxos(synced_height);
    println!("{} spendable UTXOs", utxos.len());
}
```

For the Platform address system (Bech32m P2PKH/P2SH/Orchard), see
[Platform Addresses](../addresses/platform-addresses.md). Credit balances for
platform addresses are tracked in the platform-address provider state
(`PlatformPaymentAddressProvider`, per-account `found` map) and refreshed by
`PlatformAddressSyncManager`.

### Transaction Broadcast

`CoreWallet` holds a `SpvBroadcaster` that routes signed transactions to connected
SPV peers. Asset lock transactions are broadcast through the same path, but tracked
separately by `AssetLockManager` so their confirmation status drives the identity
registration flow.

## Event Handling

`PlatformWalletManager` dispatches all SPV and Platform events through a
`PlatformEventManager`, which fans out to every registered `PlatformEventHandler`.
The application-supplied handler passed to `PlatformWalletManager::new` is one
subscriber; two internal handlers are always registered:

- `LockNotifyHandler` — wakes `AssetLockManager` async waiters when an
  InstantLock or ChainLock is seen.
- `BalanceUpdateHandler` — updates the lock-free `WalletBalance` atomics after
  each block, using a separate `wallets` map so it never contends with the
  SPV `wallet_manager` lock.

Implement `PlatformEventHandler` to react to balance changes, lock events, identity
sync completions, and contact updates:

```rust
use platform_wallet::events::{EventHandler, PlatformEventHandler};

struct MyHandler;
impl EventHandler for MyHandler {
    // override event methods as needed
}
impl PlatformEventHandler for MyHandler {}
```

## FFI Integration

`packages/rs-platform-wallet-ffi` exposes the wallet and identity API through a
C-compatible FFI layer, enabling integration with Swift, Kotlin, C++, and any
language that can call C functions.

The FFI surface uses an opaque **handle-based** model: all Rust objects are kept
alive in a thread-safe handle store; callers receive integer handles and pass them
back to subsequent API calls. Memory is freed by calling the matching `_destroy()`
function.

**Key entry points:**

| Function | Purpose |
|----------|---------|
| `platform_wallet_info_create_from_mnemonic` | Create wallet from BIP-39 phrase |
| `platform_wallet_info_create_from_seed` | Create wallet from 64-byte seed |
| `identity_manager_create` | Create a new identity manager |
| `managed_identity_create_from_identity_bytes` | Wrap a DPP-serialized identity |
| `managed_identity_get_balance` | Read identity credit balance |
| `platform_wallet_string_free` | Free C string returned by the library |

All functions return a `PlatformWalletFFIResult` status code. Check for
`PLATFORM_WALLET_FFI_SUCCESS` before reading output parameters.

The iOS Swift binding is built on top of this FFI layer and lives at
`packages/swift-sdk/`. For build instructions, see the iOS build guide:
[`packages/swift-sdk/BUILD_GUIDE_FOR_AI.md`](../../packages/swift-sdk/BUILD_GUIDE_FOR_AI.md).
For the full FFI API surface and C header details, see the
[`rs-platform-wallet-ffi` README](../../packages/rs-platform-wallet-ffi/README.md).

## Further Reading

- [Platform Addresses](../addresses/platform-addresses.md) — Bech32m address types
  used for Platform credit tracking.
- [Wallet Utilities (Evo SDK)](../evo-sdk/wallet-utilities.md) — JavaScript/TypeScript
  offline key derivation utilities for the same key hierarchy.
- [Architecture Overview](../architecture/overview.md) — Where `rs-platform-wallet`
  sits in the full crate map.
- [Builder Pattern](builder-pattern.md) — How to construct the `Sdk` instance that
  `PlatformWalletManager` requires.
- [BLAST Sync](blast-sync.md) — Details of the platform-address credit-balance sync
  that `PlatformAddressSyncManager` drives.
