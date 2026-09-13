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

## Dependencies

- `key-wallet`: Core wallet functionality
- `key-wallet-manager`: Wallet management and SPV support
- `dpp`: Dash Platform Protocol types and identity definitions
- `dashcore`: Core blockchain types

## License

MIT
