//! Changeset types for delta-based wallet persistence.
//!
//! Every wallet mutation produces a [`PlatformWalletChangeSet`] delta that is applied
//! to in-memory state and persisted atomically. No full-state snapshots —
//! only deltas.
//!
//! Sub-changesets are modelled after the real types used in `key-wallet` and
//! `platform-wallet` so they can be produced cheaply from live wallet state.

use std::collections::{BTreeMap, BTreeSet};

use dashcore::blockdata::transaction::{OutPoint, Transaction};
use dashcore::{BlockHash, Txid};

use dpp::prelude::AssetLockProof;

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::{CoreBlockHeight, Identifier};

use key_wallet::dip9::DerivationPathReference;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::wallet::asset_lock::tracked::AssetLockStatus;
use key_wallet::PlatformP2PKHAddress;

use crate::changeset::merge::Merge;
use crate::wallet::dashpay::ContactRequest;
use crate::wallet::identity::managed_identity::BlockTime;

// ---------------------------------------------------------------------------
// Chain
// ---------------------------------------------------------------------------

/// Changes to the core chain sync state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChainChangeSet {
    /// Latest synced core block height.
    pub height: Option<CoreBlockHeight>,
    /// Latest synced block hash.
    pub block_hash: Option<BlockHash>,
}

impl Merge for ChainChangeSet {
    fn merge(&mut self, other: Self) {
        // Keep the higher height (monotonic).
        if let Some(h) = other.height {
            self.height = Some(self.height.map_or(h, |cur| cur.max(h)));
        }
        if other.block_hash.is_some() {
            self.block_hash = other.block_hash;
        }
    }

    fn is_empty(&self) -> bool {
        self.height.is_none() && self.block_hash.is_none()
    }
}

// ---------------------------------------------------------------------------
// Transactions
// ---------------------------------------------------------------------------

/// A single transaction entry in the changeset.
///
/// Modelled after `key_wallet::managed_account::transaction_record::TransactionRecord`:
/// txid, full transaction, block context, net amount, fee, label.
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionEntry {
    /// The full transaction.
    pub transaction: Transaction,
    /// Block height the transaction was mined in, if confirmed.
    pub block_height: Option<CoreBlockHeight>,
    /// Block hash the transaction was mined in, if confirmed.
    pub block_hash: Option<BlockHash>,
    /// Timestamp (seconds since epoch) when the transaction was seen.
    pub timestamp: u64,
    /// Net amount for the wallet (positive = incoming, negative = outgoing).
    pub net_amount: i64,
    /// Fee paid, if we created the transaction.
    pub fee: Option<u64>,
    /// User-assigned label.
    pub label: Option<String>,
    /// Whether the transaction has an InstantSend lock.
    pub is_instant_locked: bool,
    /// Whether the transaction is in a ChainLocked block.
    pub is_chain_locked: bool,
}

/// Changes to the transaction store.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TransactionChangeSet {
    /// Inserted or updated transactions keyed by txid.
    /// Last-write-wins for updates (e.g. status promotion).
    pub transactions: BTreeMap<Txid, TransactionEntry>,
}

impl Merge for TransactionChangeSet {
    fn merge(&mut self, other: Self) {
        // Last write wins — later changesets carry higher finality status.
        self.transactions.extend(other.transactions);
    }

    fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }
}

// ---------------------------------------------------------------------------
// UTXOs
// ---------------------------------------------------------------------------

/// Changes to the UTXO set.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UtxoChangeSet {
    /// Newly created UTXOs (outpoint -> value in duffs).
    pub added: BTreeMap<OutPoint, u64>,
    /// Spent outpoints.
    pub spent: BTreeSet<OutPoint>,
}

impl Merge for UtxoChangeSet {
    fn merge(&mut self, other: Self) {
        self.added.extend(other.added);
        self.spent.extend(other.spent);
    }

    fn is_empty(&self) -> bool {
        self.added.is_empty() && self.spent.is_empty()
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
// Accounts
// ---------------------------------------------------------------------------

/// Changes to account address-derivation state.
///
/// Tracks the last revealed (used) address index per account / derivation-path
/// pair so that on reload the wallet knows how far to pre-generate.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AccountChangeSet {
    /// Last revealed address index per (account_index, derivation path reference).
    /// Updated when an address is observed on-chain.
    pub last_revealed: BTreeMap<(u32, DerivationPathReference), u32>,
}

impl Merge for AccountChangeSet {
    fn merge(&mut self, other: Self) {
        for (key, index) in other.last_revealed {
            self.last_revealed
                .entry(key)
                .and_modify(|existing| {
                    // Keep the higher index (monotonic).
                    *existing = (*existing).max(index);
                })
                .or_insert(index);
        }
    }

    fn is_empty(&self) -> bool {
        self.last_revealed.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Platform Addresses
// ---------------------------------------------------------------------------

/// Per-address balance/nonce snapshot used for Platform payment addresses.
#[derive(Debug, Clone, PartialEq)]
pub struct PlatformAddressEntry {
    /// Credit balance on this platform address.
    pub credit_balance: u64,
    /// Nonce (identity nonce) associated with this address, if known.
    pub nonce: Option<u64>,
}

/// Changes to the Platform address store.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlatformAddressChangeSet {
    /// Updated platform addresses keyed by `PlatformP2PKHAddress`.
    pub addresses: BTreeMap<PlatformP2PKHAddress, PlatformAddressEntry>,
}

impl Merge for PlatformAddressChangeSet {
    fn merge(&mut self, other: Self) {
        // Last write wins — the latest balance/nonce is the most current.
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
    /// Each credit output in an asset lock transaction is tracked independently
    /// because a single transaction can have up to 255 credit outputs (DIP-0027),
    /// each consumable separately.
    pub asset_locks: BTreeMap<OutPoint, AssetLockEntry>,
}

/// A single asset lock entry in the changeset.
///
/// Contains all fields needed to fully reconstruct a [`TrackedAssetLock`](crate::wallet::asset_lock::tracked::TrackedAssetLock).
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
// Top-Level PlatformWalletChangeSet
// ---------------------------------------------------------------------------

/// Delta of all wallet state changes from a single operation.
///
/// Composed of optional sub-changesets — `None` means no change in that area.
/// Use [`Merge::merge`] to combine multiple deltas before persisting.
///
/// Delta of all wallet state changes from a single operation.
///
/// Composed of optional sub-changesets — `None` means no change in that area.
/// Use [`Merge::merge`] to combine multiple deltas before persisting.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlatformWalletChangeSet {
    /// Core chain state (sync height, block hash).
    pub chain: Option<ChainChangeSet>,
    /// Account derivation state (last revealed indices).
    pub accounts: Option<AccountChangeSet>,
    /// Transaction changes (new transactions, status updates).
    pub transactions: Option<TransactionChangeSet>,
    /// UTXO changes (added, spent).
    pub utxos: Option<UtxoChangeSet>,
    /// Identity changes (registered, updated).
    pub identities: Option<IdentityChangeSet>,
    /// DashPay contact changes (requests sent/received, established).
    pub contacts: Option<ContactChangeSet>,
    /// Platform address balance/nonce changes.
    pub platform_addresses: Option<PlatformAddressChangeSet>,
    /// Asset lock lifecycle changes (created, locked, used).
    pub asset_locks: Option<AssetLockChangeSet>,
}

impl Merge for PlatformWalletChangeSet {
    fn merge(&mut self, other: Self) {
        self.chain.merge(other.chain);
        self.accounts.merge(other.accounts);
        self.transactions.merge(other.transactions);
        self.utxos.merge(other.utxos);
        self.identities.merge(other.identities);
        self.contacts.merge(other.contacts);
        self.platform_addresses.merge(other.platform_addresses);
        self.asset_locks.merge(other.asset_locks);
    }

    fn is_empty(&self) -> bool {
        self.chain.is_empty()
            && self.accounts.is_empty()
            && self.transactions.is_empty()
            && self.utxos.is_empty()
            && self.identities.is_empty()
            && self.contacts.is_empty()
            && self.platform_addresses.is_empty()
            && self.asset_locks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::hashes::Hash;

    #[test]
    fn test_empty_changeset() {
        let cs = PlatformWalletChangeSet::default();
        assert!(cs.is_empty());
    }

    #[test]
    fn test_chain_changeset_merge_keeps_higher_height() {
        let mut a = ChainChangeSet {
            height: Some(100),
            block_hash: None,
        };
        let b = ChainChangeSet {
            height: Some(200),
            block_hash: Some(BlockHash::all_zeros()),
        };
        a.merge(b);
        assert_eq!(a.height, Some(200));
        assert_eq!(a.block_hash, Some(BlockHash::all_zeros()));
    }

    #[test]
    fn test_chain_changeset_merge_does_not_regress_height() {
        let mut a = ChainChangeSet {
            height: Some(200),
            block_hash: None,
        };
        let b = ChainChangeSet {
            height: Some(100),
            block_hash: None,
        };
        a.merge(b);
        assert_eq!(a.height, Some(200));
    }

    #[test]
    fn test_utxo_changeset_merge() {
        let op1 = OutPoint::default();
        let mut a = UtxoChangeSet::default();
        a.added.insert(op1, 5000);

        let mut b = UtxoChangeSet::default();
        b.spent.insert(op1);

        a.merge(b);
        assert!(a.added.contains_key(&op1));
        assert!(a.spent.contains(&op1));
    }

    #[test]
    fn test_wallet_changeset_merge() {
        let mut a = PlatformWalletChangeSet {
            chain: Some(ChainChangeSet {
                height: Some(100),
                block_hash: None,
            }),
            ..Default::default()
        };
        let b = PlatformWalletChangeSet {
            chain: Some(ChainChangeSet {
                height: Some(200),
                block_hash: Some(BlockHash::all_zeros()),
            }),
            utxos: Some(UtxoChangeSet {
                added: {
                    let mut m = BTreeMap::new();
                    m.insert(OutPoint::default(), 1000);
                    m
                },
                spent: BTreeSet::new(),
            }),
            ..Default::default()
        };

        assert!(!a.is_empty());
        a.merge(b);
        assert_eq!(a.chain.as_ref().unwrap().height, Some(200));
        assert!(a.utxos.is_some());
    }

    #[test]
    fn test_account_changeset_merge_keeps_higher_index() {
        let mut a = AccountChangeSet::default();
        a.last_revealed
            .insert((0, DerivationPathReference::BIP44), 10);

        let mut b = AccountChangeSet::default();
        b.last_revealed
            .insert((0, DerivationPathReference::BIP44), 5);
        b.last_revealed
            .insert((1, DerivationPathReference::BIP44), 3);

        a.merge(b);
        // Should keep the higher index for account 0.
        assert_eq!(
            a.last_revealed.get(&(0, DerivationPathReference::BIP44)),
            Some(&10)
        );
        // Should have the new entry for account 1.
        assert_eq!(
            a.last_revealed.get(&(1, DerivationPathReference::BIP44)),
            Some(&3)
        );
    }

    #[test]
    fn test_platform_address_changeset_merge() {
        let addr1 = PlatformP2PKHAddress::new([1u8; 20]);
        let addr2 = PlatformP2PKHAddress::new([2u8; 20]);

        let mut a = PlatformAddressChangeSet::default();
        a.addresses.insert(
            addr1.clone(),
            PlatformAddressEntry {
                credit_balance: 100,
                nonce: Some(1),
            },
        );

        let mut b = PlatformAddressChangeSet::default();
        b.addresses.insert(
            addr1.clone(),
            PlatformAddressEntry {
                credit_balance: 200,
                nonce: Some(2),
            },
        );
        b.addresses.insert(
            addr2.clone(),
            PlatformAddressEntry {
                credit_balance: 50,
                nonce: None,
            },
        );

        a.merge(b);
        // addr1 should have the updated (last-write-wins) values.
        let entry1 = a.addresses.get(&addr1).unwrap();
        assert_eq!(entry1.credit_balance, 200);
        assert_eq!(entry1.nonce, Some(2));
        // addr2 should exist.
        assert!(a.addresses.contains_key(&addr2));
    }

    #[test]
    fn test_take_empty_changeset() {
        let mut cs = PlatformWalletChangeSet::default();
        assert!(cs.take().is_none());
    }

    #[test]
    fn test_take_non_empty_changeset() {
        let mut cs = PlatformWalletChangeSet {
            chain: Some(ChainChangeSet {
                height: Some(100),
                block_hash: None,
            }),
            ..Default::default()
        };
        let taken = cs.take();
        assert!(taken.is_some());
        assert!(cs.is_empty());
    }
}
