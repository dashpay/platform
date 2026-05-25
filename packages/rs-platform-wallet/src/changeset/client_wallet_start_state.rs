//! Per-wallet portion of [`ClientStartState`](crate::changeset::ClientStartState).
//!
//! **Keyless by type.** This carries everything needed to *reconstruct*
//! a watch-only wallet — network, birth height, the account manifest,
//! the rebuilt core-state projection, identities, filtered asset locks —
//! but **no** [`Wallet`](key_wallet::Wallet) and no seed. The persister
//! can never mint a `Wallet`; the manager rebuilds a watch-only one via
//! [`Wallet::new_watch_only`](key_wallet::wallet::Wallet::new_watch_only)
//! from the manifest, applies this state, and defers signing-key
//! derivation to the on-demand sign path
//! ([`sign_with_mnemonic_resolver`] and its siblings), which fail-closed
//! gate the resolver-supplied seed against the loaded `wallet_id`.
//!
//! [`sign_with_mnemonic_resolver`]: https://docs.rs/rs-platform-wallet-ffi/

use std::collections::BTreeMap;

use crate::changeset::identity_manager_start_state::IdentityManagerStartState;
use crate::changeset::{AccountRegistrationEntry, CoreChangeSet};
use crate::wallet::asset_lock::tracked::TrackedAssetLock;
use dashcore::OutPoint;
use key_wallet::Network;

/// Keyless per-wallet slice of the startup snapshot.
///
/// Used as the value type in
/// [`ClientStartState::wallets`](crate::changeset::ClientStartState::wallets).
/// The structural absence of a `Wallet`/seed field is the SECRETS.md
/// boundary, enforced by type rather than convention.
#[derive(Debug)]
pub struct ClientWalletStartState {
    /// Network the wallet is bound to (from `wallet_metadata`).
    pub network: Network,
    /// Best estimate of the chain tip at creation time (`0` = scan
    /// from genesis / unknown).
    pub birth_height: u32,
    /// Keyless account manifest — the account-set oracle and the
    /// per-account xpub cross-check source for the wrong-seed gate.
    pub account_manifest: Vec<AccountRegistrationEntry>,
    /// Keyless projection of the persisted core rows (UTXOs, tx
    /// records, IS-locks, sync watermarks, `last_applied_chain_lock`).
    /// The manager applies this onto a fresh
    /// `ManagedWalletInfo::from_wallet` skeleton **after** the
    /// seed-derived wallet passes the wrong-seed gate. Rebuilt by the
    /// `core_state::load_state` reader (item B).
    pub core_state: CoreChangeSet,
    /// Lean snapshot of this wallet's
    /// [`IdentityManager`](crate::wallet::identity::IdentityManager).
    pub identity_manager: IdentityManagerStartState,
    /// Asset locks not yet consumed by an identity registration /
    /// top-up, keyed by account index → outpoint. Terminal `Consumed`
    /// rows are already filtered out by the asset-lock reader.
    pub unused_asset_locks: BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>>,
}
