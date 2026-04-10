//! Apply a [`PlatformWalletChangeSet`] onto a [`PlatformWalletInfo`]
//! during restore.
//!
//! Inverse of the mutation methods that emit changesets in Phase 9a-2:
//! given a persisted [`PlatformWalletChangeSet`], it replays each
//! sub-changeset onto the in-memory state so the wallet converges to
//! the same state the original mutations produced.
//!
//! # Invariants
//!
//! - **Idempotent.** Applying the same changeset N times produces the
//!   same state as applying it once. Callers may re-run apply on
//!   startup or after a partial write without additional bookkeeping.
//! - **No re-emission.** `apply_changeset` returns `Result<(), _>`, not
//!   a new changeset. Mutation bookkeeping that internally produces
//!   changesets (e.g. `update_balance`) discards them.
//! - **Best-effort routing.** Contact entries whose owner identity
//!   isn't present in the wallet are logged with `tracing::warn!` and
//!   skipped — orphans usually mean a stale persisted entry from
//!   before the owner identity was removed.
//! - **Loud on core failures.** If `key_wallet`'s core apply fails (HD
//!   account derivation cascade), the platform apply fails too. Core
//!   state must land before any platform-specific bucket runs.
//!
//! # Ordering
//!
//! 1. `cs.core` — runs first via `ManagedWalletInfo::apply_changeset`.
//!    Wallet account state must exist before balance recompute.
//! 2. `cs.identities` — insert/update entries, then `removed`, then
//!    primary identity fixup.
//! 3. `cs.contacts` — sent/incoming inserts, tombstone removes,
//!    established promotions (each routed to the owning `ManagedIdentity`).
//! 4. `cs.platform_addresses` — direct map insert / remove on the
//!    cached balance map.
//! 5. `cs.asset_locks` — insert / remove tracked locks (with the
//!    `AssetLockEntry` → `TrackedAssetLock` field rename).
//! 6. `cs.token_balances` — balance updates + watch / unwatch deltas.
//! 7. `update_balance()` — recompute the cached `WalletBalance` from
//!    the now-restored UTXO set; the returned changeset is discarded.

use key_wallet::wallet::Wallet;

use crate::changeset::PlatformWalletChangeSet;
use crate::wallet::asset_lock::tracked::TrackedAssetLock;
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// Errors returned by [`PlatformWalletInfo::apply_changeset`].
///
/// Only restore failures that *cascade* (i.e. would cause downstream
/// entries to be silently dropped) are surfaced as errors. Orphan
/// entries that fail to route are logged and skipped.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ApplyError {
    /// Re-applying the core (`key_wallet`) sub-changeset failed. Usually
    /// means the target wallet lacks the key material to re-derive an
    /// HD account, or the changeset is for a different network.
    ///
    /// Stored as `String` to keep the platform-wallet public API
    /// decoupled from `key_wallet`'s error enum.
    #[error("core wallet apply failed: {0}")]
    CoreApply(String),
}

impl PlatformWalletInfo {
    /// Apply a [`PlatformWalletChangeSet`] onto this `PlatformWalletInfo`,
    /// using `wallet` as the source of HD key material for any account
    /// types that need to be re-derived.
    ///
    /// See the module docs for invariants and ordering. Typical caller
    /// is the persister loader on startup, holding the
    /// `WalletManager` write lock to obtain the split borrow of
    /// `(&mut Wallet, &mut PlatformWalletInfo)`.
    pub fn apply_changeset(
        &mut self,
        wallet: &mut Wallet,
        cs: &PlatformWalletChangeSet,
    ) -> Result<(), ApplyError> {
        // 1. Core wallet state — chain, accounts, UTXOs, transactions,
        //    addresses used, highest_used. Must run first so the per-
        //    account buckets exist before anything platform-side
        //    references them.
        if let Some(core) = &cs.core {
            self.core_wallet
                .apply_changeset(wallet, core)
                .map_err(|e| ApplyError::CoreApply(e.to_string()))?;
        }

        // 2. Identities.
        if let Some(id_cs) = &cs.identities {
            for entry in id_cs.identities.values() {
                self.identity_manager.apply_identity_entry(entry);
            }
            for removed_id in &id_cs.removed {
                self.identity_manager.apply_remove(removed_id);
            }
            // Primary-identity fixup: prefer an explicit selection from
            // the changeset; otherwise, if the current primary was just
            // removed, re-derive by picking the first remaining
            // identity (matches `IdentityManager::remove_identity`).
            if let Some(new_primary) = id_cs.primary_identity {
                self.identity_manager.primary_identity_id = Some(new_primary);
            } else if let Some(current) = self.identity_manager.primary_identity_id {
                if id_cs.removed.contains(&current) {
                    self.identity_manager.primary_identity_id =
                        self.identity_manager.identities.keys().next().copied();
                }
            }
            if let Some(idx) = id_cs.last_scanned_index {
                self.identity_manager.last_scanned_index = idx;
            }
        }

        // 3. Contacts. Each entry routes to its owning ManagedIdentity by
        //    `(owner, contact)` key; orphans (owner not in the wallet)
        //    are logged and skipped.
        if let Some(contact_cs) = &cs.contacts {
            // Sent inserts.
            for ((owner, _contact), entry) in &contact_cs.sent_requests {
                match self.identity_manager.managed_identity_mut(owner) {
                    Some(managed) => {
                        managed.apply_sent_contact_request(entry.request.clone());
                    }
                    None => tracing::warn!(
                        owner = %owner,
                        "skipping sent contact request during apply: owner identity not in wallet"
                    ),
                }
            }
            // Incoming inserts.
            for ((owner, _contact), entry) in &contact_cs.incoming_requests {
                match self.identity_manager.managed_identity_mut(owner) {
                    Some(managed) => {
                        managed.apply_incoming_contact_request(entry.request.clone());
                    }
                    None => tracing::warn!(
                        owner = %owner,
                        "skipping incoming contact request during apply: owner identity not in wallet"
                    ),
                }
            }
            // Tombstone removes.
            for (owner, contact) in &contact_cs.removed_sent {
                if let Some(managed) = self.identity_manager.managed_identity_mut(owner) {
                    managed.apply_removed_sent(contact);
                }
            }
            for (owner, contact) in &contact_cs.removed_incoming {
                if let Some(managed) = self.identity_manager.managed_identity_mut(owner) {
                    managed.apply_removed_incoming(contact);
                }
            }
            // Established promotions — drop any matching pending
            // entries on both sides per the auto-establishment contract.
            for ((owner, _contact), established) in &contact_cs.established {
                match self.identity_manager.managed_identity_mut(owner) {
                    Some(managed) => {
                        managed.apply_established_contact(established.clone());
                    }
                    None => tracing::warn!(
                        owner = %owner,
                        "skipping established contact during apply: owner identity not in wallet"
                    ),
                }
            }
        }

        // 4. Platform address balances. Last-write-wins on the cached
        //    map; tombstones drop entries unconditionally.
        if let Some(addr_cs) = &cs.platform_addresses {
            for (addr, credits) in &addr_cs.addresses {
                self.platform_address_balances.insert(*addr, *credits);
            }
            for addr in &addr_cs.removed {
                self.platform_address_balances.remove(addr);
            }
        }

        // 5. Asset locks. Inline the AssetLockEntry → TrackedAssetLock
        //    mapping (the only field difference is `amount_duffs` vs
        //    `amount`).
        if let Some(al_cs) = &cs.asset_locks {
            for (out_point, entry) in &al_cs.asset_locks {
                self.tracked_asset_locks.insert(
                    *out_point,
                    TrackedAssetLock {
                        out_point: entry.out_point,
                        transaction: entry.transaction.clone(),
                        account_index: entry.account_index,
                        funding_type: entry.funding_type,
                        identity_index: entry.identity_index,
                        amount: entry.amount_duffs,
                        status: entry.status.clone(),
                        proof: entry.proof.clone(),
                    },
                );
            }
            for out_point in &al_cs.removed {
                self.tracked_asset_locks.remove(out_point);
            }
        }

        // 6. Token balances + watch registry deltas.
        if let Some(tok_cs) = &cs.token_balances {
            for (key, balance) in &tok_cs.balances {
                self.token_balances.insert(*key, *balance);
            }
            for key in &tok_cs.removed_balances {
                self.token_balances.remove(key);
            }
            for (identity_id, tokens) in &tok_cs.watched {
                self.token_watched
                    .entry(*identity_id)
                    .or_default()
                    .extend(tokens.iter().copied());
            }
            for (identity_id, tokens) in &tok_cs.unwatched {
                if let Some(set) = self.token_watched.get_mut(identity_id) {
                    for token in tokens {
                        set.remove(token);
                    }
                    if set.is_empty() {
                        self.token_watched.remove(identity_id);
                    }
                }
            }
        }

        // 7. Recompute cached UI balance from the now-restored UTXO set.
        //    `update_balance` returns its own changeset internally; we
        //    discard it (apply does not re-emit).
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
        let _ = self.core_wallet.update_balance();
        // Mirror the recomputed balance into the lock-free Arc that the
        // UI reads.
        let core_balance = &self.core_wallet.balance;
        self.balance.set(
            core_balance.spendable(),
            core_balance.unconfirmed(),
            core_balance.immature(),
            core_balance.locked(),
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    use dashcore::OutPoint;
    use dpp::address_funds::PlatformAddress;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::Identity;
    use dpp::prelude::Identifier;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;

    use crate::changeset::{
        AssetLockChangeSet, AssetLockEntry, ContactChangeSet, ContactRequestEntry,
        IdentityChangeSet, IdentityEntry, PlatformAddressChangeSet, PlatformWalletChangeSet,
        TokenBalanceChangeSet,
    };
    use crate::wallet::asset_lock::tracked::AssetLockStatus;
    use crate::wallet::core::WalletBalance;
    use crate::wallet::dashpay::{ContactRequest, EstablishedContact};
    use crate::wallet::identity::managed_identity::ManagedIdentity;
    use crate::wallet::identity::IdentityManager;
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
    use key_wallet::wallet::Wallet;
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

    fn build_test_wallet() -> Wallet {
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::None)
            .expect("test wallet")
    }

    fn empty_info(wallet: &Wallet) -> PlatformWalletInfo {
        PlatformWalletInfo {
            core_wallet: ManagedWalletInfo::from_wallet(wallet),
            balance: std::sync::Arc::new(WalletBalance::new()),
            identity_manager: IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
            platform_address_balances: BTreeMap::new(),
            token_watched: BTreeMap::new(),
            token_balances: BTreeMap::new(),
        }
    }

    fn make_test_identity(id_byte: u8, revision: u64) -> Identity {
        Identity::V0(IdentityV0 {
            id: Identifier::from([id_byte; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision,
        })
    }

    fn make_test_contact_request(sender: u8, recipient: u8) -> ContactRequest {
        ContactRequest::new(
            Identifier::from([sender; 32]),
            Identifier::from([recipient; 32]),
            0,
            0,
            0,
            vec![0u8; 96],
            100_000,
            0,
        )
    }

    #[test]
    fn apply_empty_changeset_is_noop() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);
        let cs = PlatformWalletChangeSet::default();
        info.apply_changeset(&mut wallet, &cs).expect("apply");
        assert!(info.identity_manager.is_empty());
        assert!(info.tracked_asset_locks.is_empty());
        assert!(info.platform_address_balances.is_empty());
        assert!(info.token_balances.is_empty());
    }

    #[test]
    fn apply_identity_insert_then_remove_clears_primary() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        let id_a = Identifier::from([1u8; 32]);
        let id_b = Identifier::from([2u8; 32]);

        // Insert two identities; the first becomes primary.
        let mut cs = PlatformWalletChangeSet::default();
        let mut id_cs = IdentityChangeSet::default();
        let managed_a = ManagedIdentity::new(make_test_identity(1, 0), 0);
        let managed_b = ManagedIdentity::new(make_test_identity(2, 0), 1);
        id_cs.identities.insert(id_a, IdentityEntry::from_managed(&managed_a));
        id_cs.identities.insert(id_b, IdentityEntry::from_managed(&managed_b));
        id_cs.primary_identity = Some(id_a);
        cs.identities = Some(id_cs);

        info.apply_changeset(&mut wallet, &cs).expect("apply insert");
        assert_eq!(info.identity_manager.primary_identity_id, Some(id_a));
        assert_eq!(info.identity_manager.identity_count(), 2);

        // Remove the primary; apply should re-select the next available.
        let mut cs = PlatformWalletChangeSet::default();
        let mut id_cs = IdentityChangeSet::default();
        id_cs.removed.insert(id_a);
        cs.identities = Some(id_cs);

        info.apply_changeset(&mut wallet, &cs).expect("apply remove");
        assert_eq!(info.identity_manager.identity_count(), 1);
        assert_eq!(info.identity_manager.primary_identity_id, Some(id_b));
    }

    #[test]
    fn apply_identity_double_apply_is_idempotent() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        let id = Identifier::from([7u8; 32]);
        let mut managed = ManagedIdentity::new(make_test_identity(7, 1), 3);
        managed.label = Some("alice".into());
        managed.top_ups.insert(0, 100_000);

        let mut cs = PlatformWalletChangeSet::default();
        let mut id_cs = IdentityChangeSet::default();
        id_cs.identities.insert(id, IdentityEntry::from_managed(&managed));
        cs.identities = Some(id_cs);

        info.apply_changeset(&mut wallet, &cs).expect("first apply");
        info.apply_changeset(&mut wallet, &cs).expect("second apply");

        let restored = info.identity_manager.managed_identity(&id).expect("present");
        assert_eq!(restored.label.as_deref(), Some("alice"));
        assert_eq!(restored.identity_index, 3);
        assert_eq!(restored.top_ups.get(&0), Some(&100_000));
    }

    #[test]
    fn apply_contacts_with_orphan_owner_skips_silently() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        // No identities in the manager — every contact entry is orphaned.
        let owner = Identifier::from([99u8; 32]);
        let other = Identifier::from([1u8; 32]);
        let mut cs = PlatformWalletChangeSet::default();
        let mut contact_cs = ContactChangeSet::default();
        contact_cs.sent_requests.insert(
            (owner, other),
            ContactRequestEntry {
                request: make_test_contact_request(99, 1),
            },
        );
        cs.contacts = Some(contact_cs);

        // Apply must not error.
        info.apply_changeset(&mut wallet, &cs).expect("apply");
    }

    #[test]
    fn apply_established_drops_pending_entries() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        // Set up an owning identity with pre-existing pending requests.
        let owner_id = Identifier::from([1u8; 32]);
        let other_id = Identifier::from([2u8; 32]);

        let mut id_cs = IdentityChangeSet::default();
        let mut managed = ManagedIdentity::new(make_test_identity(1, 0), 0);
        managed
            .sent_contact_requests
            .insert(other_id, make_test_contact_request(1, 2));
        managed
            .incoming_contact_requests
            .insert(other_id, make_test_contact_request(2, 1));
        id_cs.identities.insert(owner_id, IdentityEntry::from_managed(&managed));
        let mut cs = PlatformWalletChangeSet::default();
        cs.identities = Some(id_cs);
        info.apply_changeset(&mut wallet, &cs).expect("apply identity");

        // Now apply an established contact for the same pair.
        let established = EstablishedContact::new(
            other_id,
            make_test_contact_request(1, 2),
            make_test_contact_request(2, 1),
        );
        let mut contact_cs = ContactChangeSet::default();
        contact_cs.established.insert((owner_id, other_id), established);
        let mut cs = PlatformWalletChangeSet::default();
        cs.contacts = Some(contact_cs);
        info.apply_changeset(&mut wallet, &cs).expect("apply established");

        let managed = info
            .identity_manager
            .managed_identity(&owner_id)
            .expect("owner present");
        assert!(managed.established_contacts.contains_key(&other_id));
        assert!(!managed.sent_contact_requests.contains_key(&other_id));
        assert!(!managed.incoming_contact_requests.contains_key(&other_id));
    }

    #[test]
    fn apply_platform_addresses_insert_and_remove() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        let addr1 = PlatformAddress::P2pkh([10u8; 20]);
        let addr2 = PlatformAddress::P2pkh([20u8; 20]);

        // Insert two.
        let mut addr_cs = PlatformAddressChangeSet::default();
        addr_cs.addresses.insert(addr1, 100);
        addr_cs.addresses.insert(addr2, 200);
        let mut cs = PlatformWalletChangeSet::default();
        cs.platform_addresses = Some(addr_cs);
        info.apply_changeset(&mut wallet, &cs).expect("apply insert");
        assert_eq!(info.platform_address_balances.get(&addr1), Some(&100));
        assert_eq!(info.platform_address_balances.get(&addr2), Some(&200));

        // Remove one, update the other.
        let mut addr_cs = PlatformAddressChangeSet::default();
        addr_cs.addresses.insert(addr1, 150);
        addr_cs.removed.insert(addr2);
        let mut cs = PlatformWalletChangeSet::default();
        cs.platform_addresses = Some(addr_cs);
        info.apply_changeset(&mut wallet, &cs).expect("apply remove");
        assert_eq!(info.platform_address_balances.get(&addr1), Some(&150));
        assert!(!info.platform_address_balances.contains_key(&addr2));
    }

    #[test]
    fn apply_asset_locks_field_rename() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        let out_point = OutPoint::default();
        let mut al_cs = AssetLockChangeSet::default();
        al_cs.asset_locks.insert(
            out_point,
            AssetLockEntry {
                out_point,
                transaction: dashcore::Transaction {
                    version: 3,
                    lock_time: 0,
                    input: vec![],
                    output: vec![],
                    special_transaction_payload: None,
                },
                account_index: 0,
                funding_type: AssetLockFundingType::IdentityRegistration,
                identity_index: 0,
                amount_duffs: 5_000,
                status: AssetLockStatus::Built,
                proof: None,
            },
        );
        let mut cs = PlatformWalletChangeSet::default();
        cs.asset_locks = Some(al_cs);
        info.apply_changeset(&mut wallet, &cs).expect("apply");
        let lock = info
            .tracked_asset_locks
            .get(&out_point)
            .expect("lock present");
        assert_eq!(lock.amount, 5_000);

        // Tombstone removes it.
        let mut al_cs = AssetLockChangeSet::default();
        al_cs.removed.insert(out_point);
        let mut cs = PlatformWalletChangeSet::default();
        cs.asset_locks = Some(al_cs);
        info.apply_changeset(&mut wallet, &cs).expect("apply remove");
        assert!(!info.tracked_asset_locks.contains_key(&out_point));
    }

    #[test]
    fn apply_token_unwatch_clears_set_and_removes_empty_identity() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        let identity = Identifier::from([5u8; 32]);
        let token = Identifier::from([10u8; 32]);

        let mut tok_cs = TokenBalanceChangeSet::default();
        tok_cs.balances.insert((identity, token), 999);
        let mut watched = BTreeSet::new();
        watched.insert(token);
        tok_cs.watched.insert(identity, watched);
        let mut cs = PlatformWalletChangeSet::default();
        cs.token_balances = Some(tok_cs);
        info.apply_changeset(&mut wallet, &cs).expect("apply watch");
        assert_eq!(info.token_balances.get(&(identity, token)), Some(&999));
        assert!(info.token_watched.get(&identity).unwrap().contains(&token));

        // Unwatch the token — set becomes empty, identity entry should
        // be removed entirely.
        let mut tok_cs = TokenBalanceChangeSet::default();
        let mut unwatched = BTreeSet::new();
        unwatched.insert(token);
        tok_cs.unwatched.insert(identity, unwatched);
        tok_cs.removed_balances.insert((identity, token));
        let mut cs = PlatformWalletChangeSet::default();
        cs.token_balances = Some(tok_cs);
        info.apply_changeset(&mut wallet, &cs).expect("apply unwatch");
        assert!(!info.token_balances.contains_key(&(identity, token)));
        assert!(!info.token_watched.contains_key(&identity));
    }

    #[test]
    fn apply_double_apply_full_changeset_is_idempotent() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        let identity = Identifier::from([3u8; 32]);
        let mut managed = ManagedIdentity::new(make_test_identity(3, 0), 0);
        managed.label = Some("bob".into());

        let mut id_cs = IdentityChangeSet::default();
        id_cs.identities.insert(identity, IdentityEntry::from_managed(&managed));
        id_cs.last_scanned_index = Some(7);

        let addr = PlatformAddress::P2pkh([42u8; 20]);
        let mut addr_cs = PlatformAddressChangeSet::default();
        addr_cs.addresses.insert(addr, 1_000);

        let mut tok_cs = TokenBalanceChangeSet::default();
        let token = Identifier::from([8u8; 32]);
        tok_cs.balances.insert((identity, token), 42);

        let cs = PlatformWalletChangeSet {
            identities: Some(id_cs),
            platform_addresses: Some(addr_cs),
            token_balances: Some(tok_cs),
            ..Default::default()
        };

        info.apply_changeset(&mut wallet, &cs).expect("first apply");
        info.apply_changeset(&mut wallet, &cs).expect("second apply");

        assert_eq!(info.identity_manager.identity_count(), 1);
        assert_eq!(info.identity_manager.last_scanned_index(), 7);
        assert_eq!(info.platform_address_balances.get(&addr), Some(&1_000));
        assert_eq!(info.token_balances.get(&(identity, token)), Some(&42));
    }
}
