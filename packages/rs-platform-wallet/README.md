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

## Shielded balance API migration

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

## Final-inputs and broadcast-probe API migration

Spending only final coins and probing unresolved broadcasts bring these Rust
source and behaviour changes:

- `CoreWallet::finalize_transaction_with_options(builder, options, sources,
  source_index, signer)` takes a `FinalizeOptions` after the builder in place
  of the trailing `reservation_only: bool`: `reservation_only` and `inputs`
  (coins the build may spend, by outpoint — the wallet's current copy of each
  is seeded under the write guard). `finalize_transaction` keeps its
  signature.
- New `CoreWallet::finalize_transaction_from(make, options, sources,
  source_index, signer)` takes a factory (`Fn() ->
  Result<TransactionBuilder, PlatformWalletError> + Send + Sync`, by value,
  dropped before the signer runs) instead of a builder. It
  builds `make()`, and on a shortfall builds `make()` once more as a trial,
  without signing or keeping a reservation, with the coins that are not final
  yet treated as final: if key-wallet builds that, the shortfall is
  `CoreFundsAwaitingNetwork` (not for a `reservation_only` build, which spends
  only its final chosen inputs). A finalizer handed a plain builder cannot make
  it again, so its shortfall stays insufficient funds.
- `FinalizeOptions` is `#[non_exhaustive]`: build it with
  `FinalizeOptions::default()`, `with_inputs` and `with_reservation_only`.
- FFI: `core_wallet_tx_builder_set_fee_rate` refuses a rate whose fee
  arithmetic would overflow (`ErrorInvalidParameter`, above about 42.9 DASH
  per kB); `core_wallet_tx_builder_set_current_height` is accepted and ignored
  — the finalizers always built at the wallet's own height.
- `PlatformWalletError` gains `ChosenInputUnavailable { outpoint, problem:
  ChosenInputProblem }` (new public enum: `NotInFundingAccounts`,
  `NotSpendable`; FFI `ErrorInvalidParameter`): a coin chosen by outpoint the
  build cannot spend — and a coin seeded on the builder with `add_inputs`
  that selection picked but no funding account holds, which used to be a
  `TransactionBuild` string error. A short build that would only succeed by
  selecting such a seeded coin once waiting coins confirm now returns this
  refusal too, where it used to return `CoreInsufficientFunds` /
  `CorePooledInsufficientFunds`. An exhaustive `match` needs an arm for it.
- `PlatformWalletError` gains `CoreFundsAwaitingNetwork { available, waiting,
  required, outpoint }` (FFI code 59): the build's final coins fall short, but
  key-wallet would build it if the coins that are not yet confirmed or
  InstantSend-locked were final. An exhaustive `match` needs an arm for it.
- `WalletWorker` gains `BroadcastProbes`, reported in the shutdown report. An
  exhaustive `match` needs an arm for it.
- Behaviour: every payment build that funds from the wallet, and
  `pooled_spendable_balance` / `pooled_max_sendable`, use only confirmed or
  InstantSend-locked coins. A coin in `FinalizeOptions::inputs` that a
  funding account holds is judged as the wallet holds it when the build is
  finalized: a candidate if final by then (the only kind with
  `reservation_only`), refused with `CoreFundsAwaitingNetwork` naming its
  outpoint if not, with `InputMidBroadcast` if an in-flight broadcast pins it.
  A chosen coin the funding accounts don't hold (another account's, or spent
  since it was chosen) is refused by name in both funding modes: the caller
  picked it, so it is not dropped silently. The one exception: a coin another
  in-flight build holds reserved is left out by key-wallet's reservation
  filter (reservations are not readable from platform-wallet). A coin
  seeded on the builder itself (`add_inputs`) is the caller's snapshot; a
  selected one that has lost its final status is refused the same way.

## Dependencies

- `key-wallet`: Core wallet functionality
- `key-wallet-manager`: Wallet management and SPV support
- `dpp`: Dash Platform Protocol types and identity definitions
- `dashcore`: Core blockchain types

## License

MIT
