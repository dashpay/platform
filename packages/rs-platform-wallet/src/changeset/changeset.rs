//! Changeset types for delta-based wallet persistence.
//!
//! Every wallet mutation produces a [`PlatformWalletChangeSet`] delta that
//! is applied to in-memory state and persisted atomically. No full-state
//! snapshots — only deltas.
//!
//! # Shape
//!
//! `PlatformWalletChangeSet` embeds [`key_wallet::changeset::WalletChangeSet`]
//! verbatim in its `core` field — that sub-changeset carries every
//! core-wallet delta (chain, accounts, UTXOs, transactions, balance) in the
//! BDK-style per-account bucketing defined by key-wallet. Platform-specific
//! state that doesn't exist in key-wallet lives in dedicated sub-changesets:
//! identities, contacts, platform addresses, asset locks, and token balances.
//!
//! Earlier revisions of this file defined its own `ChainChangeSet`,
//! `TransactionChangeSet`, `UtxoChangeSet`, and `AccountChangeSet`. Those
//! were stand-ins from before key-wallet had its own changeset module and
//! used lossy flattened entries (e.g. `BTreeMap<OutPoint, u64>` for UTXOs,
//! losing address/script/is_coinbase/confirmation state). They are all
//! deleted; the `core` field replaces them with native `key-wallet` types.

use std::collections::{BTreeMap, BTreeSet};

use dashcore::blockdata::transaction::{OutPoint, Transaction};

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AssetLockProof;

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;

use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::wallet::asset_lock::tracked::AssetLockStatus;

use crate::changeset::merge::Merge;
use crate::wallet::dashpay::ContactRequest;
use crate::wallet::identity::managed_identity::BlockTime;

// ---------------------------------------------------------------------------
// Bridge: key_wallet::changeset::WalletChangeSet -> platform-wallet Merge
// ---------------------------------------------------------------------------
//
// platform-wallet has its own `Merge` trait that is semantically
// richer than key-wallet's (recursive merge on `BTreeMap<K, V: Merge>`),
// so we can't just import key-wallet's trait wholesale. This one-off
// impl delegates to the key-wallet `Merge` implementation that ships
// with `WalletChangeSet` so that
// `Option<key_wallet::changeset::WalletChangeSet>` satisfies
// `crate::changeset::merge::Merge` via the blanket impl.
impl Merge for key_wallet::changeset::WalletChangeSet {
    fn merge(&mut self, other: Self) {
        <Self as key_wallet::changeset::Merge>::merge(self, other)
    }

    fn is_empty(&self) -> bool {
        <Self as key_wallet::changeset::Merge>::is_empty(self)
    }
}

// ---------------------------------------------------------------------------
// Identities
// ---------------------------------------------------------------------------

/// A snapshot/delta entry for a single managed identity.
///
/// Modelled after [`crate::wallet::identity::managed_identity::ManagedIdentity`].
#[derive(Debug, Clone, PartialEq)]
pub struct IdentityEntry {
    /// The Platform identity.
    pub identity: Identity,
    /// HD identity index used during registration.
    pub identity_index: u32,
    /// User-defined label.
    pub label: Option<String>,
    /// Last block time when balance was updated.
    pub last_updated_balance_block_time: Option<BlockTime>,
    /// Last block time when keys were synced.
    pub last_synced_keys_block_time: Option<BlockTime>,
    /// DPNS usernames.
    pub dpns_names: Vec<String>,
    /// Top-up history: maps top-up index to amount (in duffs).
    pub top_ups: BTreeMap<u32, u64>,
}

/// Changes to the identity store.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IdentityChangeSet {
    /// Inserted or updated identities keyed by identifier.
    pub identities: BTreeMap<Identifier, IdentityEntry>,
}

impl Merge for IdentityChangeSet {
    fn merge(&mut self, other: Self) {
        for (id, entry) in other.identities {
            self.identities
                .entry(id)
                .and_modify(|existing| {
                    // Keep the identity with the higher revision.
                    if entry.identity.revision() >= existing.identity.revision() {
                        existing.identity = entry.identity.clone();
                    }
                    if entry.label.is_some() {
                        existing.label = entry.label.clone();
                    }
                    if entry.last_updated_balance_block_time.is_some() {
                        existing.last_updated_balance_block_time =
                            entry.last_updated_balance_block_time;
                    }
                    if entry.last_synced_keys_block_time.is_some() {
                        existing.last_synced_keys_block_time = entry.last_synced_keys_block_time;
                    }
                    // Append new DPNS names.
                    for name in &entry.dpns_names {
                        if !existing.dpns_names.contains(name) {
                            existing.dpns_names.push(name.clone());
                        }
                    }
                    // Merge top-ups (last write wins per index).
                    existing.top_ups.extend(entry.top_ups.iter());
                })
                .or_insert(entry);
        }
    }

    fn is_empty(&self) -> bool {
        self.identities.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Contacts
// ---------------------------------------------------------------------------

/// A single contact request entry in the changeset.
///
/// Modelled after [`crate::wallet::dashpay::ContactRequest`].
#[derive(Debug, Clone, PartialEq)]
pub struct ContactRequestEntry {
    /// The contact request.
    pub request: ContactRequest,
}

/// Changes to the DashPay contact store.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContactChangeSet {
    /// Sent contact requests keyed by (our identity, recipient identity).
    pub sent_requests: BTreeMap<(Identifier, Identifier), ContactRequestEntry>,
    /// Incoming contact requests keyed by (sender identity, our identity).
    pub incoming_requests: BTreeMap<(Identifier, Identifier), ContactRequestEntry>,
    /// Newly established contacts (bidirectional): set of
    /// (our identity, contact identity) pairs.
    pub established: BTreeSet<(Identifier, Identifier)>,
}

impl Merge for ContactChangeSet {
    fn merge(&mut self, other: Self) {
        self.sent_requests.extend(other.sent_requests);
        self.incoming_requests.extend(other.incoming_requests);
        self.established.extend(other.established);
    }

    fn is_empty(&self) -> bool {
        self.sent_requests.is_empty()
            && self.incoming_requests.is_empty()
            && self.established.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Platform Addresses
// ---------------------------------------------------------------------------

/// Changes to the Platform address store.
///
/// Mirrors [`PlatformWalletInfo.platform_address_balances`] exactly:
/// a map from [`PlatformAddress`] (P2PKH or P2SH) to [`Credits`]
/// (the balance in duffs). Last-write-wins on merge.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlatformAddressChangeSet {
    /// Updated platform addresses keyed by `PlatformAddress`.
    pub addresses: BTreeMap<PlatformAddress, Credits>,
}

impl Merge for PlatformAddressChangeSet {
    fn merge(&mut self, other: Self) {
        // Last write wins — the latest balance is the most current.
        self.addresses.extend(other.addresses);
    }

    fn is_empty(&self) -> bool {
        self.addresses.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Asset Locks
// ---------------------------------------------------------------------------

/// Changes to the asset lock store.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AssetLockChangeSet {
    /// Asset lock entries keyed by outpoint (txid + output index).
    ///
    /// Each credit output in an asset lock transaction is tracked
    /// independently because a single transaction can have up to 255
    /// credit outputs (DIP-0027), each consumable separately.
    pub asset_locks: BTreeMap<OutPoint, AssetLockEntry>,
}

/// A single asset lock entry in the changeset.
///
/// Contains all fields needed to fully reconstruct a
/// [`TrackedAssetLock`](crate::wallet::asset_lock::tracked::TrackedAssetLock).
#[derive(Debug, Clone, PartialEq)]
pub struct AssetLockEntry {
    /// The outpoint identifying this credit output (txid + vout).
    pub out_point: OutPoint,
    /// The full asset lock transaction.
    pub transaction: Transaction,
    /// BIP44 account index that funded this asset lock (UTXO source).
    pub account_index: u32,
    /// Which funding account to derive the one-time key from.
    pub funding_type: AssetLockFundingType,
    /// Identity index used during creation.
    pub identity_index: u32,
    /// The amount locked (in duffs).
    pub amount_duffs: u64,
    /// Current status on Core chain.
    pub status: AssetLockStatus,
    /// The asset lock proof, available once IS-locked or ChainLocked.
    pub proof: Option<AssetLockProof>,
}

impl Merge for AssetLockChangeSet {
    fn merge(&mut self, other: Self) {
        // Last write wins — later status is higher finality.
        self.asset_locks.extend(other.asset_locks);
    }

    fn is_empty(&self) -> bool {
        self.asset_locks.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Token Balances
// ---------------------------------------------------------------------------

/// Changes to watched Platform token balances.
///
/// Mirrors `PlatformWalletInfo.token_balances`
/// (`BTreeMap<(Identifier, Identifier), TokenAmount>`) and
/// `PlatformWalletInfo.token_watched`
/// (`BTreeMap<Identifier, BTreeSet<Identifier>>`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TokenBalanceChangeSet {
    /// Updated token balances keyed by `(identity_id, token_id)`.
    /// Last write wins on merge.
    pub balances: BTreeMap<(Identifier, Identifier), u64>,

    /// Tokens newly watched per identity.
    /// Merged via set union on the inner `BTreeSet`.
    pub watched: BTreeMap<Identifier, BTreeSet<Identifier>>,
}

impl Merge for TokenBalanceChangeSet {
    fn merge(&mut self, other: Self) {
        self.balances.extend(other.balances);
        for (identity_id, tokens) in other.watched {
            self.watched.entry(identity_id).or_default().extend(tokens);
        }
    }

    fn is_empty(&self) -> bool {
        self.balances.is_empty() && self.watched.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Top-Level PlatformWalletChangeSet
// ---------------------------------------------------------------------------

/// Delta of all wallet state changes from a single operation.
///
/// `core` carries the full `key_wallet::changeset::WalletChangeSet` — chain,
/// balance, account_keys, and per-account buckets (UTXOs, transactions,
/// addresses used, highest-used index). Platform-specific deltas (identities,
/// contacts, platform addresses, asset locks, token balances) live in
/// dedicated sub-changesets.
///
/// Composed of optional sub-changesets — `None` means no change in that
/// area. Use [`Merge::merge`] to combine multiple deltas before persisting.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlatformWalletChangeSet {
    /// Core wallet state from key-wallet: chain, balance, account keys,
    /// and per-account buckets (UTXOs, transactions, addresses used,
    /// highest-used index).
    pub core: Option<key_wallet::changeset::WalletChangeSet>,
    /// Identity changes (registered, updated).
    pub identities: Option<IdentityChangeSet>,
    /// DashPay contact changes (requests sent/received, established).
    pub contacts: Option<ContactChangeSet>,
    /// Platform address balance/nonce changes.
    pub platform_addresses: Option<PlatformAddressChangeSet>,
    /// Asset lock lifecycle changes (created, locked, used).
    pub asset_locks: Option<AssetLockChangeSet>,
    /// Platform token balance / watch changes.
    pub token_balances: Option<TokenBalanceChangeSet>,
}

impl Merge for PlatformWalletChangeSet {
    fn merge(&mut self, other: Self) {
        // `key_wallet::changeset::WalletChangeSet` implements `Merge`
        // itself; delegate via the `Option<T>: Merge` blanket impl from
        // this crate's merge module.
        self.core.merge(other.core);
        self.identities.merge(other.identities);
        self.contacts.merge(other.contacts);
        self.platform_addresses.merge(other.platform_addresses);
        self.asset_locks.merge(other.asset_locks);
        self.token_balances.merge(other.token_balances);
    }

    fn is_empty(&self) -> bool {
        self.core.is_empty()
            && self.identities.is_empty()
            && self.contacts.is_empty()
            && self.platform_addresses.is_empty()
            && self.asset_locks.is_empty()
            && self.token_balances.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_changeset() {
        let cs = PlatformWalletChangeSet::default();
        assert!(cs.is_empty());
    }

    #[test]
    fn test_platform_address_changeset_merge() {
        let addr1 = PlatformAddress::P2pkh([1u8; 20]);
        let addr2 = PlatformAddress::P2pkh([2u8; 20]);

        let mut a = PlatformAddressChangeSet::default();
        a.addresses.insert(addr1.clone(), 100);

        let mut b = PlatformAddressChangeSet::default();
        b.addresses.insert(addr1.clone(), 200); // last write wins
        b.addresses.insert(addr2.clone(), 50);

        a.merge(b);
        assert_eq!(a.addresses.get(&addr1), Some(&200));
        assert_eq!(a.addresses.get(&addr2), Some(&50));
    }

    #[test]
    fn test_token_balance_changeset_merge() {
        let identity_a = Identifier::from([1u8; 32]);
        let identity_b = Identifier::from([2u8; 32]);
        let token_x = Identifier::from([10u8; 32]);
        let token_y = Identifier::from([11u8; 32]);

        let mut a = TokenBalanceChangeSet::default();
        a.balances.insert((identity_a, token_x), 100);
        a.watched.entry(identity_a).or_default().insert(token_x);

        let mut b = TokenBalanceChangeSet::default();
        // Same identity/token — last-write-wins.
        b.balances.insert((identity_a, token_x), 200);
        // New token on same identity — merged into the watched set.
        b.watched.entry(identity_a).or_default().insert(token_y);
        // New identity.
        b.balances.insert((identity_b, token_x), 50);

        a.merge(b);

        assert_eq!(a.balances.get(&(identity_a, token_x)), Some(&200));
        assert_eq!(a.balances.get(&(identity_b, token_x)), Some(&50));
        let watched_a = a.watched.get(&identity_a).unwrap();
        assert!(watched_a.contains(&token_x));
        assert!(watched_a.contains(&token_y));
    }

    #[test]
    fn test_take_empty_changeset() {
        let mut cs = PlatformWalletChangeSet::default();
        assert!(cs.take().is_none());
    }

    #[test]
    fn test_take_non_empty_changeset() {
        let mut cs = PlatformWalletChangeSet {
            identities: Some(IdentityChangeSet::default()),
            ..Default::default()
        };
        // The identities changeset is an empty placeholder — so cs is
        // still `is_empty()`. Push a real entry.
        let identity_id = Identifier::from([1u8; 32]);
        // We can't easily construct a full Identity here, so use asset_locks
        // for the non-empty path.
        let mut cs = PlatformWalletChangeSet {
            asset_locks: Some(AssetLockChangeSet {
                asset_locks: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        OutPoint::default(),
                        AssetLockEntry {
                            out_point: OutPoint::default(),
                            transaction: Transaction {
                                version: 3,
                                lock_time: 0,
                                input: vec![],
                                output: vec![],
                                special_transaction_payload: None,
                            },
                            account_index: 0,
                            funding_type: AssetLockFundingType::IdentityRegistration,
                            identity_index: 0,
                            amount_duffs: 1000,
                            status: AssetLockStatus::Built,
                            proof: None,
                        },
                    );
                    m
                },
            }),
            ..Default::default()
        };
        let _ = identity_id;
        let taken = cs.take();
        assert!(taken.is_some());
        assert!(cs.is_empty());
    }
}
