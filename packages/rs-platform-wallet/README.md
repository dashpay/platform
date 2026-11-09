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

## Identity-funded shield recovery

`shielded_shield_from_identity` constructs a wallet-visible activity entry and persists the exact signed transition before broadcasting. If the activity cannot be constructed or the durable retry guard cannot be written, the call fails before broadcast. Once submitted, relay errors and nonce snapshots never make a replacement payment safe: an unused nonce may execute later, and a consumed nonce may belong to the original payment.

Sync retries the original signed bytes until the nonce can no longer execute, then stops broadcasting while keeping the payment pending until its outputs are observed. Another shield from the same identity returns `ShieldedIdentityDebitPending` before building or broadcasting. Account registration also refuses to remove or replace the owner of an unresolved debit; rebind the original account to continue scanning. These guards survive ordinary restarts. Explicitly clearing shielded state or removing the wallet deletes its recovery records and should not be used to retry an unresolved payment.

On restart, registration checks that the viewing keys recover the pending payment's original outputs. Sync resolves the guard from those outputs even if the host lost the live activity record or a scan batch groups several payments together.

If a payment remains unresolved, hosts can inspect `NetworkShieldedCoordinator::identity_debit_recovery_records` and offer explicit recovery through `PlatformWallet::abandon_shielded_identity_debit`. The caller must acknowledge that the original payment may already have executed or could still execute. This stops automatic retries and releases only the selected wallet/account/activity guard; it does **not** cancel the signed transaction or automatically create another payment. A separately authorized new payment may debit the identity again.

Recovery preserves the original signed record with an `Unknown` outcome across restarts and account rebinds. A stale host `Pending` activity is overlaid with `Unknown`; later observation of the original outputs can still confirm it. Neither nonce expiry nor a number of empty scans proves failure. Clearing shielded state is not a recovery action because it deletes these records.

Startup errors distinguish damaged recovery metadata or undecodable signed bytes (`ShieldedRecoveryCorrupted`) from missing/replaced viewing keys (`ShieldedRecoveryKeysRequired`). Failure to recover an output set can also mean damaged ciphertext or output metadata; restore the original keys or a known-good backup before choosing explicit recovery. Invalid SQLite identifiers, nullifiers, and state flags fail startup without deleting their rows. When signed bytes are damaged but the recovery key remains readable, listing retains its account/activity identifiers and leaves the undecodable identity, nonce, and amount absent.

## Private contact request payments after a restart

Before a private contact request's payment can reach the network, the shielded store saves a record of it in its private payment journal: the sender and recipient identities, the request's account reference, the commitment of the entry it appends and the nullifiers its spend consumes. The file store writes the journal, and the payment's redrive record, in one transaction on a connection at `synchronous=FULL`, so the record survives a crash or a power loss; a record that cannot be saved refuses the payment before anything is broadcast. The journal is kept apart from the spend's notes and redrive record: no note release, redrive removal, undecodable transition, account rebind or purge removes it. A record is removed only when its entry is read back from DashPay's private store and recorded durably, or when the transition provably never left the wallet (refused before anything was sent: no address to send it to, or the SDK's own refusal). An endpoint's answer, a consensus verdict included, proves nothing: no proof or quorum signature backs it, and an endpoint that kept or forwarded the signed transition can answer with any verdict. A payment that never lands otherwise keeps its pair held; that fails closed.

Until a payment's reservation is restored from the journal, nothing shows that the pair is taken: the request's entry is not in DashPay's store until the payment executes. So a reloaded wallet sends and accepts no contact request, public or private, while such a payment may be unrestored, and returns `PrivateContactPaymentsNotRestored` (FFI code 72) with nothing read, signed or broadcast. The in-kind accept is held the same way. Two things hold it:

- In a build with the `shielded` feature, `load_from_persistor` holds a wallet that bound a shielded account before (its persisted viewing key shows it). A wallet with no persisted viewing key never bound one, since a bind persists the key before installing anything and refuses when it cannot.
- In every build, an identity whose private scan state marks a private payment that may be unresolved (`PrivateContactRequestsPosition::private_payments_unresolved`) is held. The wallet stores the marker durably before such a payment may reach the network, flushing a backend that does not commit a store inline, and stores it cleared once the identity holds no private request it is paying for or has not read back. A loaded hold is kept even when the backend no longer attests the scan state.

A private contact request is therefore paid for only over a backend attesting `PRIVATE_CONTACT_REQUEST_SCAN_STATE` together with `ATOMIC_CHANGESETS` (`PersistenceCapabilities::PRIVATE_CONTACT_SCAN_STATE_RESTART`), as the scan state is written in changesets that must commit whole: any other refuses the private send and the private accept with `PrivateContactRecoveryUnsupported` (FFI code 73) before anything is sealed or paid, and a marker that cannot be stored or flushed refuses the payment too. A payment that may have reached the network keeps its pair held and its marker set whatever its spend's outcome, until its record is removed as above. A pair held so refuses another private payment (`PrivateContactRequestPending`, FFI code 71) and holds public sends and accepts.

`PlatformWalletManager::configure_shielded` restores the payments of every held wallet from the journal, with no keys, and a load after it restores those it loads. Binding a wallet's shielded account restores its own; a bind never waits for a store that a scan holds, and defers the restoration to a task instead, the wallet held until it succeeds. A private sync does not restore. Hosts that use shielded support call `configure_shielded` right after the load. A host that opens no shielded store this session, and any build without the `shielded` feature, keeps those wallets' and identities' contact requests refused: the private payment may still execute, and a public request would publish the pair the private one kept off chain. `DashPayView::private_contact_readiness` (FFI `platform_wallet_get_private_contact_readiness`) reads whether the payments are restored, whether the private store was read to its end since load, and whether the marker is set, so a host shows the wallet's state rather than inferring it from refusals.

A restoration fails closed, every hold staying, when the coordinator is for another network, when a journal record does not read back, when a saved spend's transition does not decode and no journal record accounts for it, when a private contact request payment saved before the journal kept it has no request saved with it, and when an identity is marked as holding an unresolved payment while the wallet's journal is new (the shielded store was deleted or replaced, so it cannot show that payment resolved). The first three clear once the damaged spend lands or its anchor is pruned, which removes its redrive record, or once a known-good store file is restored; the last one only once a known-good store is restored, or by removing the wallet and adding it again when the payment is known to have resolved.

### Shielded spends whose broadcast may have been delivered

Every shielded spend that consumes notes (shielded transfers, unshields, withdrawals, identity top-ups and creations from the pool, and private document creations) broadcasts in single dispatches and keeps what each one proved: the SDK's own retry loop returns only its last error, which cannot show that an earlier attempt was admitted with its response lost. Any dispatch that reached an endpoint, whatever it answered (a consensus verdict included, which is unauthenticated), or failed in a way that allows delivery, any transport error whatever its gRPC code, makes the whole broadcast possibly delivered. Such a spend waits for its result, and reports `ShieldedSpendUnconfirmed` (FFI code 18) when that wait fails or answers with a verdict, its notes and redrive record kept so the redrive pass resolves it: a host must not pay again with other notes. Only a spend refused before anything was sent reports `ShieldedBroadcastFailed` and releases its notes at once. The dispatches stay within the SDK's configured retry budget: one dispatch and its retries. A re-broadcast refused by a node does not prove that no other node holds the original, so the redrive pass keeps such a spend's notes reserved until it lands or its anchor is pruned. Shields into the pool (from platform addresses or an identity) keep their own handling.

## Shielded balance API migration

Custom `ShieldedStore` implementations must also implement the private
contact payment methods, which have no default: `arm_private_contact_payment`,
`private_contact_redrives_for_wallet` and the private payment journal
(`open_private_payment_journal`, `private_payment_journal`,
`journal_private_payments`, `discharge_private_payment`). A store that cannot
keep a payment's records durably returns an error, which refuses the payment;
it must never report success while dropping them.

The local shielded balance API introduces three Rust source compatibility changes:

- Custom `ShieldedStore` implementations must implement
  `spendable_balance(&self, id: SubwalletId) -> Result<u64, Self::Error>`.
  The unchecked default was removed. Sum only the reservation-aware notes
  returned by `get_unspent_notes(id)`, use `checked_add` in a `try_fold`, and
  map overflow into the implementation's storage error. Do not wrap, saturate,
  or replace overflow with zero. The checked implementations in
  [InMemoryShieldedStore](src/wallet/shielded/store.rs) and
  [FileBackedShieldedStore](src/wallet/shielded/file_store.rs) are the reference
  method bodies; no additional error-conversion trait bound is required.
- `ShieldedSyncSummary::balance_total()` now returns
  `Result<u64, PlatformWalletError>`. Callers must propagate or handle
  `ShieldedStoreError` when individually valid account balances have an
  unrepresentable wallet-wide sum. A caller that already returns
  `Result<_, PlatformWalletError>` can use `let total = summary.balance_total()?;`.
  A failed total is unavailable, not a successful zero balance. The C callback
  layout is unchanged; the bridge reports an unsuccessful wallet result.
- `ShieldedSubwalletStartState` struct literals require `has_sync_state`.
  Set it from the presence of a persisted scan-state row, including a row whose
  index is zero. An absent row is different from a recorded empty scan; do not
  infer presence merely from `last_synced_index > 0`.

## Shielded store lifetime migration

`ShieldedStore` now requires `Send + Sync + 'static`: a spend dropped before it hands its reserved notes over releases them on a spawned task when the store is locked, and that task owns the store's `Arc`. This is a Rust source compatibility change, not a consensus one.

A custom `ShieldedStore` that borrows from its embedding application (a `&'a` connection, configuration or buffer held in the store type) no longer satisfies the trait. Move the borrowed resource into the store, by value or behind an `Arc` (an `Arc<Mutex<_>>` or `Arc<RwLock<_>>` for one shared with the application), so the store type has no lifetime parameter. The in-repo [InMemoryShieldedStore](src/wallet/shielded/store.rs) and [FileBackedShieldedStore](src/wallet/shielded/file_store.rs) already own what they use and need no change.

## Dependencies

- `key-wallet`: Core wallet functionality
- `key-wallet-manager`: Wallet management and SPV support
- `dpp`: Dash Platform Protocol types and identity definitions
- `dashcore`: Core blockchain types

## License

MIT
