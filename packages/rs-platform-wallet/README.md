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

Before a private contact request's payment can reach the network, the shielded store saves a record of it in its private payment journal: the sender and recipient identities, the request's account reference, the commitment of the entry it appends, the SHA-256 of the whole entry, the position the sender's read of DashPay's private store rested at when it paid, and the nullifiers its spend consumes. The file store writes the journal, and the payment's redrive record, in one transaction on a connection at `synchronous=FULL` with `fullfsync` (on Apple platforms a plain sync does not flush the drive's own cache), so the record survives a crash or a power loss; a record that cannot be saved refuses the payment before anything is broadcast. The journal is kept apart from the spend's notes and redrive record: no note release, redrive removal, undecodable transition, account rebind or purge removes it. A record is removed only when its entry is read back from DashPay's private store and recorded durably, or when the transition provably never left the wallet (refused before anything was sent: no address to send it to, or the SDK's own refusal). An endpoint's answer, a consensus verdict included, proves nothing: no proof or quorum signature backs it, and an endpoint that kept or forwarded the signed transition can answer with any verdict. A record is also discharged once its payment is proven expired, judged against Platform's recorded anchors fetched before the shielded sync pass and proved at a height H: the spend's anchor, which the record keeps with the proved height of the set it was read from, was read at or below H and is absent from the set at H, the pass's note scan covers H, and its entry is absent from DashPay's private store read from the lowest position the records being judged kept through its end, with every page proved at or above H. An entry counts as the record's only when its whole bytes hash to the hash the record keeps: one that only starts with the commitment, which anyone who saw the commitment can append, does not hold the record. A record journaled before the hash and the position were kept matches by its commitment and is read for from the store's start. Whether its inputs are spent does not matter: the transition appends its entry in the block that spends them. A payment that never lands therefore frees its pair once its anchor ages out (Platform keeps an anchor for `shielded_anchor_retention_blocks`, 1000 blocks, and prunes every `shielded_anchor_pruning_interval`, 100 blocks). A restoration that finds a record whose request the sender's tracked request already records or supersedes, as a private sync read it back in a process that ended before discharging it, discharges it, unless a flush of the wallet's records failed for good in this process (the tracked request may then be in memory only). Nothing else, a relay's refusal or a missing redrive record included, discharges a record.

Until a payment's reservation is restored from the journal, nothing shows that the pair is taken: the request's entry is not in DashPay's store until the payment executes. So a wallet sends and accepts no contact request, public or private, while such a payment may be unrestored, and returns `PrivateContactPaymentsNotRestored` (FFI code 72) with nothing read, signed or broadcast. The in-kind accept is held the same way. These hold it:

- In a build with the `shielded` feature, a wallet published, registered or loaded, while no shielded store is configured is held when its persister attests the private scan state: only such a persister lets a private payment be paid, so only its wallet can have journal records that no marker shows, as after the wallet was removed and registered again from the same seed, in this process or a new one. A wallet whose persister does not attest is never held for want of a store. With a store configured, the journal is read as the wallet is published.
- In a build with the `shielded` feature, `load_from_persistor` also holds a wallet that bound a shielded account before (its persisted viewing key shows it). A wallet with no persisted viewing key never bound one, since a bind persists the key before installing anything and refuses when it cannot.
- In a build with the `shielded` feature, a wallet stays held while its last restoration failed, or waits for a busy store.
- In every build, an identity whose private scan state marks a private payment that may be unresolved (`PrivateContactRequestsPosition::private_payments_unresolved`) is held. The wallet stores the marker durably before such a payment may reach the network, flushing a backend that does not commit a store inline, and stores it cleared once the identity holds no private request it is paying for or has not read back. A loaded hold is kept even when the backend no longer attests the scan state.

A private contact request is therefore paid for only over a backend attesting `PRIVATE_CONTACT_REQUEST_SCAN_STATE` (bit 15) together with `ATOMIC_CHANGESETS` (bit 0) (`PersistenceCapabilities::PRIVATE_CONTACT_SCAN_STATE_RESTART`), as the scan state is written in changesets that must commit whole: any other refuses the private send and the private accept with `PrivateContactRecoveryUnsupported` (FFI code 73) before anything is sealed or paid, and a marker that cannot be stored or flushed refuses the payment too. A payment that may have reached the network keeps its pair held and its marker set whatever its spend's outcome, until its record is removed as above. A pair held so refuses another private payment (`PrivateContactRequestPending`, FFI code 71) and holds public sends and accepts. The wallet holds it, beyond the sender identity, for as long as the journal keeps the payment's record: removing the identity and loading it again does not release it.

The first `PlatformWalletManager::configure_shielded` restores from the journal, with no keys, the payments of every held wallet and of every wallet the journal may keep payments of; a wallet registered or loaded after it is restored as it is published. A repeat call, as a host makes on every bind, never waits for a store that a scan holds: it restores only the wallets still held, each deferred to a task while the store is busy. Binding a wallet's shielded account restores its own, deferring the same way, the wallet held until it succeeds. A contact request refused as not restored first runs one more restoration, which does not wait on the store. A private sync does not restore. A host whose persister attests the private scan state therefore configures the shielded store when it activates the manager, before loading or creating wallets: until then every such wallet's contact requests are refused with 72, and a configure that fails keeps them refused until one succeeds. A host that opens no shielded store this session keeps those wallets' and identities' contact requests refused, and a build without the `shielded` feature keeps the marked identities' refused: the private payment may still execute, and a public request would publish the pair the private one kept off chain. `DashPayView::private_contact_readiness` (FFI `platform_wallet_get_private_contact_readiness`) reads whether the payments are restored, why the last restoration failed if it did, whether the private store was read to its end, and whether a payment may be unresolved (the marker, or a journal record the wallet holds the pair for), so a host shows the wallet's state rather than inferring it from refusals. `DashPayView::private_pair_held` reads one pair's hold as a send decides it.

Every contact request and acceptance, public or private, first reads DashPay's private contact request store for the identity from its cursor to the store's end, with the keys Platform proves the identity holds, so a private request that landed since the last read (from another device of the identity, or a contact's request to it) is seen before the request is decided; the claim of a public pair checks again, in its lock, that the read still reaches the end. A pass reaches the end only at the highest store size any of its pages proved, so a later page from a node behind does not end it early, and after a key change rewinds the read, it counts only when the page that reaches the end was proved at or above the height the earlier read had reached. While the read cannot finish, the request is refused with `PrivateContactStoreNotRead` (FFI code 74), nothing reserved, sealed, paid or broadcast; calling again continues the read. On a network proven below the protocol version that serves private documents, nothing is read. An SDK pinned below that version refuses the read (FFI code 75), whatever the network runs.

A restoration fails closed, every hold staying, when the coordinator is for another network, when a journal record does not read back, when a saved spend's transition does not decode and no journal record accounts for it, when a private contact request payment saved before the journal kept it has no request saved with it, and when an identity is marked as holding an unresolved payment while the wallet's journal is new (the shielded store was deleted or replaced, so it cannot show that payment resolved). The first clears with a store for the wallet's network. A journal record that does not read back is never removed, so it clears only once a known-good store file is restored; one whose wallet id does not read holds every wallet. The undecodable transition and the payment with no request clear once the damaged spend lands or its anchor is pruned, which removes its redrive record, or once a known-good store file is restored; the last one only once a known-good store is restored, or by removing the wallet and adding it again when the payment is known to have resolved. The readiness gives the failure's reason while the wallet is held.

### Shielded spends whose broadcast may have been delivered

Every shielded spend that consumes notes (shielded transfers, unshields, withdrawals, identity top-ups and creations from the pool, and private document creations) broadcasts in single dispatches and keeps what each one proved: the SDK's own retry loop returns only its last error, which cannot show that an earlier attempt was admitted with its response lost. Any dispatch that reached an endpoint, whatever it answered (a consensus verdict included, which is unauthenticated), or failed in a way that allows delivery, any transport error whatever its gRPC code, makes the whole broadcast possibly delivered. Such a spend waits for its result, and reports `ShieldedSpendUnconfirmed` (FFI code 18) when that wait fails or answers with a verdict, its notes and redrive record kept so the redrive pass resolves it: a host must not pay again with other notes. An identity creation reports `ShieldedBroadcastUnconfirmed` (FFI code 17) instead, carrying the identity's derived id. Earlier versions reported these verdicts and transport failures as `ShieldedBroadcastFailed` (FFI code 16). Only a spend refused before anything was sent reports `ShieldedBroadcastFailed` and releases its notes at once. The dispatches stay within the SDK's configured retry budget: one dispatch and its retries. A re-broadcast refused by a node does not prove that no other node holds the original, so the redrive pass keeps such a spend's notes reserved until it lands or its anchor is pruned. Shields into the pool (from platform addresses or an identity) keep their own handling.

## Shielded balance API migration

Custom `ShieldedStore` implementations must also implement the private
contact payment methods, which have no default: `arm_private_contact_payment`,
`private_contact_redrives_for_wallet`, `retire_undelivered_spend` and the
private payment journal (`private_payment_journal_open`,
`open_private_payment_journal`, `private_payment_journal`,
`journal_private_payments`, `discharge_private_payment`). A store that cannot
keep a payment's records durably returns an error, which refuses the payment;
it must never report success while dropping them. `retire_undelivered_spend`
removes a spend's saved record and its journal records in one durable write,
and refuses, changing nothing, once a re-broadcast of the record was claimed
(`PendingRedrive::attempts` above 0).

`arm_private_contact_payment` takes the entries a payment appends as
`&[PrivatePaymentEntry]` (each entry's commitment and the SHA-256 of its whole
bytes) and the sender's private store read position when it paid
(`store_position: Option<u64>`), and `PrivatePaymentRecord` struct literals
require `entry_hash` and `store_position`. A store keeps both with each journal
record. `FileBackedShieldedStore` writes journal records in format version 4
and still reads version 3 records, which carry neither: such a record matches
an entry by its commitment and is read for from the store's start.

Note reservations are owned by the spend attempt that took them: stores must
implement `mark_pending_by(id, nullifier, reservation)`, which reserves a note
for the attempt `reservation`, and `clear_pending_reserved_by(id, nullifier,
reservation)`, which releases it only while that same attempt still holds it,
returning whether it did. Keep the owner with each reservation and compare and
clear it under the store's own lock. Forwarding `clear_pending_reserved_by` to
the unconditional `clear_pending` defeats the protection it gives: a cleanup
that runs after a purge, a rebind or a wallet's replacement would release a
newer attempt's notes or delete its records.

Anchored note reservations and saved records now keep the proved height of
the anchor set their anchor was read from: `set_pending_spend` takes an
`anchor_height: Option<u64>`, `StalePendingSpend` carries it as a fourth
element, `PendingRedrive` struct literals require `anchor_height`, and stores
must implement `note_anchor_recorded`, which gives that height to the
reservations and records of an anchor that carry none, durably for a record.
A store keeps a record's height with it, so a restart restores it. A note
spend's record, or a journal record, saved without a height (one saved before
heights were kept) keeps its height unknown. Its anchor is a root of a tree no
larger than the largest the store has held, so a store can implement
`legacy_anchor_tree_bound` to return that size; the default `None` keeps such
records held until a set is seen holding their anchor. `FileBackedShieldedStore`
keeps the size durably, raised at every open and before every
`reset_commitment_tree`, and reports the size held when it was opened. With a
bound, the sync fetches the pool's proved leaf count before the anchor set,
and such a record is released, or discharged, on a set lacking its anchor only
when that count, proved at or below the set's height, reaches the bound and
one past the position of every note spent on that anchor that the store
holds, the scan covers the set, and, for a journal record, the store read
through its end lacks its entry at that set's height (one hashing to the hash
the record keeps, or, for a record that keeps none, one starting with its
commitment). Otherwise it stays held.

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
