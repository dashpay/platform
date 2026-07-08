//! Per-wallet portion of [`ClientStartState`](crate::changeset::ClientStartState).
//!
//! **Keyless by type.** This carries everything needed to *reconstruct*
//! a watch-only wallet — network, birth height, the account manifest,
//! the managed-state snapshot, identities, filtered asset locks — but
//! **no** [`Wallet`](key_wallet::Wallet) and no seed. The persister
//! can never mint a `Wallet`; the manager rebuilds a watch-only one via
//! [`Wallet::new_watch_only`](key_wallet::wallet::Wallet::new_watch_only)
//! from the manifest, applies this state, and defers signing-key
//! derivation to the on-demand sign path (`rs-platform-wallet-ffi`'s
//! `dash_sdk_sign_with_mnemonic_resolver_and_path` and its siblings).

use std::collections::BTreeMap;

use crate::changeset::identity_manager_start_state::IdentityManagerStartState;
use crate::changeset::AccountRegistrationEntry;
use crate::wallet::asset_lock::tracked::TrackedAssetLock;
use dashcore::OutPoint;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::Network;

/// Keyless per-wallet slice of the startup snapshot.
///
/// Used as the value type in
/// [`ClientStartState::wallets`](crate::changeset::ClientStartState::wallets).
/// The structural absence of a `Wallet`/seed field is the SECRETS.md
/// boundary, enforced by type rather than convention.
#[derive(Debug)]
pub struct ClientWalletStartState {
    /// Network the wallet is bound to.
    pub network: Network,
    /// Best estimate of the chain tip at creation time (`0` = scan
    /// from genesis / unknown).
    pub birth_height: u32,
    /// Keyless account manifest — the account-set oracle for building the
    /// watch-only wallet (one watch-only account per entry's xpub).
    pub account_manifest: Vec<AccountRegistrationEntry>,
    /// Full keyless managed-wallet snapshot: pools with exact derivation
    /// indices and `used` flags, per-account UTXO and tx-record
    /// attribution, IS-lock set, and sync metadata. [`ManagedWalletInfo`]
    /// carries **no key material** (see its docs: balances, account
    /// metadata, UTXO set), so the SECRETS.md boundary holds: still no
    /// `Wallet`, no seed.
    ///
    /// The manager consumes it directly after validating its
    /// `wallet_id`/`network` against the row and its account set against
    /// the manifest — preserving per-account attribution, the full SPV
    /// watch set, and pool used-state verbatim, without re-deriving
    /// anything. The FFI/iOS persister populates this.
    pub wallet_info: Box<ManagedWalletInfo>,
    /// Lean snapshot of this wallet's
    /// [`IdentityManager`](crate::wallet::identity::IdentityManager).
    pub identity_manager: IdentityManagerStartState,
    /// Asset locks not yet consumed by an identity registration /
    /// top-up, keyed by account index → outpoint. Terminal `Consumed`
    /// rows are already filtered out by the asset-lock reader.
    pub unused_asset_locks: BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>>,
}
