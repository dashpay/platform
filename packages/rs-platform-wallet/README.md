# platform-wallet

A Dash Platform wallet implementation that extends traditional wallet functionality with Platform identity management.

## Overview

`platform-wallet` provides a `PlatformWalletInfo` struct that combines:
- Traditional wallet management from `key-wallet` (UTXOs, addresses, transactions)
- Dash Platform identity management (identities, credits, public keys)

This allows applications to manage both Layer 1 (blockchain) and Layer 2 (Platform) assets in a unified interface.

## Features

- **Wallet Management**: Full support for HD wallets, UTXO tracking, and transaction building
- **Identity Management**: Store and manage multiple Platform identities per wallet
- **SPV Support**: Compatible with SPVWalletManager for light client functionality
- **Identity Metadata**: Track per-identity metadata including credits, revision, and sync status

## Usage

```rust
use platform_wallet::PlatformWalletInfo;
use key_wallet_manager::wallet_manager::WalletManager;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use dpp::prelude::Identifier;

// Create a platform wallet
let wallet_id = [1u8; 32];
let mut wallet = PlatformWalletInfo::new(wallet_id, "My Wallet".to_string());

// Use with WalletManager
let mut manager = WalletManager::<PlatformWalletInfo>::new();

// Add identities (would come from Platform in real usage)
// let identity = load_identity_from_platform();
// wallet.add_identity(identity)?;

// Access wallet information
let balance = wallet.get_balance();
let addresses = wallet.monitored_addresses(Network::Mainnet);

// Access identity information
let identities = wallet.identities(); // Returns IndexMap<Identifier, Identity>
let primary = wallet.primary_identity();

// Access managed identities with metadata
let managed = wallet.managed_identities(); // Returns &IndexMap<Identifier, ManagedIdentity>
for (id, managed_identity) in managed {
    println!("Identity {}: label={:?}, active={}", 
             id, managed_identity.label, managed_identity.is_active);
}

// Manage identity metadata
if let Some(identity) = primary {
    let identity_id = identity.id();
    wallet.identity_manager.set_label(&identity_id, "Primary Identity".to_string())?;
    
    // Credit balance and revision are accessed directly from the identity
    let balance = identity.balance();
    let revision = identity.revision();
}
```

## Architecture

The package is structured as follows:

### Core Components

- **`PlatformWalletInfo`**: Main struct that wraps `ManagedWalletInfo` and adds identity support
  - Implements `WalletInfoInterface` for compatibility with wallet managers
  - Delegates wallet operations to the underlying `ManagedWalletInfo`
  - Manages identities through the `IdentityManager`

- **`IdentityManager`**: Handles storage and management of Platform identities
  - Uses `Identifier` type from DPP for all identity IDs
  - Maintains primary identity selection
  - Stores `ManagedIdentity` instances

- **`ManagedIdentity`**: Combines a Platform Identity with wallet-specific metadata
  - Contains the Platform `Identity` object
  - Last sync timestamp and height
  - User-defined labels
  - Active/inactive status
  - Note: Credit balance and revision are accessed from the Identity itself

## Persistence architecture

This section is normative: it records the agreed model for how wallet
state, the persister, and clients relate. Changes that violate these
invariants need an explicit architecture discussion first, not just a
code review.

```
              commands (send, register, sync, …)
   client ──────────────────────────────────────▶ platform-wallet
     │                                                   │
     │  reads (display)                    changesets    │  (single writer)
     ▼                                                   ▼
   ┌─────────────────────── persisted store ──────────────────────┐
   │      wallet-state tables: written ONLY by platform-wallet    │
   │      client-owned tables (UI prefs etc.): written by client  │
   └───────────────────────────────────────────────────────────────┘
                     ▲
                     │  load(persister) at launch — verbatim
                platform-wallet
```

### Roles

- **platform-wallet** is the authority for state *transitions*. Every
  mutation of wallet state happens here and is emitted as a changeset
  to the persister. Its in-memory state is volatile — a cache that is
  empty at process start.
- **The persisted store** is the authority for state *history*: it is
  the only copy of the wallet that survives a restart, and it doubles
  as the client's **read model** — UIs *may* render persisted rows directly
  and reactively. Display therefore never blocks on platform-wallet
  being unlocked or synced; the local seedless restore is still a startup gate.
- **Clients** (dash-evo-tool, the iOS SDK app, …) issue commands to
  platform-wallet and read the store freely. They never write
  wallet-state rows.

### Invariants

1. **Single writer** (enforced by review, not the storage layer). Only platform-wallet's changesets mutate
   wallet-state tables. Clients may keep their own tables (UI
   preferences, view state) in the same database; ownership is per
   table family, never shared.
2. **The store schema is a versioned public contract.** Two parties
   depend on it — the persister's writes and every client's reads — so
   schema changes are breaking changes for clients, not private
   refactors.
3. **Reads never feed back into writes** except through platform-wallet
   commands. A client that computes something from persisted rows and
   wants it stored must go through a platform-wallet API.
4. **`load()` is verbatim.** At launch, platform-wallet reconstructs
   itself from the store through
   [`PlatformWalletPersistence::load`]; the store contains exactly what
   platform-wallet wrote, so the load path must consume it as-is.
   Re-deriving, re-inferring, or "repairing" state during load is
   forbidden — a lossy round-trip here silently diverges the wallet
   from its own history (per-account attribution, address-pool
   `used` flags, and SPV watch-set coverage are the historical
   casualties). Anything genuinely missing from the store re-warms on
   the next sync, never inside `load()`.
5. **Persist errors are hard errors.** The store is the only durable
   copy, and part of it — the account manifest, address used-flags,
   birth heights, identity/contact associations — is *local-only*: no
   chain rescan can ever reconstruct it. A swallowed persister write
   error is silent, permanent data loss discovered at the next launch.
6. **Load is seedless.** The store never carries a seed or a
   `Wallet`; restore produces watch-only wallets
   (`Wallet::new_watch_only`) and signing keys are derived on demand
   via the resolver-backed sign paths. See the trust-boundary notes on
   [`PlatformWalletPersistence::load`] for what is (and is not)
   authenticated on this path.

### What restore is for

Because the store is the read model, restoring platform-wallet at
launch is **not** about showing balances or history — the client
already renders those from the store. It exists to refill the
operational state that only lives in platform-wallet's memory:

- **Detection** — the SPV watch set is the address-pool contents;
  without it, incoming payments to existing addresses are not seen.
- **Spending** — coin/input selection runs against the in-memory UTXO
  set.
- **Resume** — sync watermarks, tracked asset locks mid-registration,
  and fresh-receive-address (`used`) state.

Persisters that can reconstruct the full keyless snapshot hand it back
as `ClientWalletStartState::wallet_info` (consumed verbatim, per
invariant 4). The flattened projection fields
(`core_state`/`used_core_addresses`) are a transitional fallback for
persisters that cannot build a snapshot yet, and are slated for
removal once every in-tree persister produces snapshots.

## Key Features

### Wallet Operations (via ManagedWalletInfo)
- HD wallet support (BIP32/BIP44)
- UTXO tracking and management
- Transaction building and fee estimation
- Address generation with gap limit
- Multiple account types (standard, coinjoin, identity)

### Identity Operations
- Add/remove identities
- Primary identity selection
- Access identity balance and revision (from Identity object)
- Custom labeling for identities
- Active/inactive status tracking
- Last sync timestamp/height tracking

### Compatibility
- Works with `WalletManager<PlatformWalletInfo>` for standard wallet management
- Works with `SPVWalletManager<PlatformWalletInfo>` for SPV/light client functionality
- Fully compatible with existing `key-wallet-manager` infrastructure

## Dependencies

- `key-wallet`: Core wallet functionality
- `key-wallet-manager`: Wallet management and SPV support
- `dpp`: Dash Platform Protocol types and identity definitions
- `dashcore`: Core blockchain types

## License

MIT
