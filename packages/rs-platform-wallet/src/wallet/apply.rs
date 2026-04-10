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

/// Errors returned by [`PlatformWalletInfo::apply_changeset`] and the
/// `PlatformWallet::apply` async wrapper.
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

    /// The target wallet has been removed from the `WalletManager`
    /// between the caller obtaining the `Arc<PlatformWallet>` handle
    /// and calling `apply`. Returned by the `PlatformWallet::apply`
    /// async wrapper, never by `PlatformWalletInfo::apply_changeset`
    /// itself.
    #[error("wallet not found in manager: {0:?}")]
    WalletNotFound([u8; 32]),
}

impl PlatformWalletInfo {
    /// Apply a [`PlatformWalletChangeSet`] onto this `PlatformWalletInfo`,
    /// using `wallet` as the source of HD key material for any account
    /// types that need to be re-derived.
    ///
    /// Consumes the changeset by value so every owned field
    /// (`Identity` blobs, `KeyStorage`, `dpns_names`, `ContactRequest`s,
    /// `EstablishedContact`s, the `Transaction` inside each
    /// `AssetLockEntry`) moves directly into the in-memory maps with
    /// no clones. The borrow form was deliberately removed — the
    /// persister-load case (deserialize → apply once → drop) makes
    /// any clone pure waste, and a `&` variant existing alongside
    /// this one would just invite the wrong overload to be reached
    /// for. Tests that need to apply the same changeset twice
    /// `.clone()` it explicitly at the call site.
    ///
    /// See the module docs for invariants and ordering. Typical caller
    /// is the persister loader on startup, holding the
    /// `WalletManager` write lock to obtain the split borrow of
    /// `(&mut Wallet, &mut PlatformWalletInfo)`.
    pub fn apply_changeset(
        &mut self,
        wallet: &mut Wallet,
        cs: PlatformWalletChangeSet,
    ) -> Result<(), ApplyError> {
        let PlatformWalletChangeSet {
            core,
            identities,
            contacts,
            platform_addresses,
            asset_locks,
            token_balances,
        } = cs;

        // 1. Core wallet state — chain, accounts, UTXOs, transactions,
        //    addresses used, highest_used. Must run first so the per-
        //    account buckets exist before anything platform-side
        //    references them. The core changeset is moved by value
        //    into key-wallet's apply path; key-wallet itself drains
        //    the per-account buckets without clones.
        if let Some(core) = core {
            self.core_wallet
                .apply_changeset(wallet, core)
                .map_err(|e| ApplyError::CoreApply(e.to_string()))?;
        }

        // 2. Identities.
        if let Some(id_cs) = identities {
            // Stash the structural fields before draining `identities`
            // so the primary-identity fixup below can still see them.
            let removed = id_cs.removed;
            let new_primary = id_cs.primary_identity;
            let new_scan_index = id_cs.last_scanned_index;

            for (_id, entry) in id_cs.identities {
                self.identity_manager.apply_identity_entry(entry);
            }
            for removed_id in &removed {
                self.identity_manager.identities.shift_remove(removed_id);
            }
            // Primary-identity fixup: prefer an explicit selection from
            // the changeset; otherwise, if the current primary was just
            // removed, re-derive by picking the first remaining
            // identity (matches `IdentityManager::remove_identity`).
            // If the map is now empty the fallback yields `None`,
            // matching mutation-side semantics.
            if let Some(new_primary) = new_primary {
                self.identity_manager.primary_identity_id = Some(new_primary);
            } else if let Some(current) = self.identity_manager.primary_identity_id {
                if removed.contains(&current) {
                    self.identity_manager.primary_identity_id =
                        self.identity_manager.identities.keys().next().copied();
                }
            }
            if let Some(idx) = new_scan_index {
                self.identity_manager.last_scanned_index = idx;
            }
        }

        // 3. Contacts. Each entry routes to its owning ManagedIdentity by
        //    `(owner, contact)` key; orphans (owner not in the wallet)
        //    are logged and skipped. Trivial map ops (sent / incoming
        //    insert and remove) are inlined here — no helper earns its
        //    name for a single `insert` / `shift_remove` call. Only
        //    `apply_established_contact` is a method because it has
        //    real logic (drops both pending sides per the contract).
        if let Some(contact_cs) = contacts {
            let crate::changeset::ContactChangeSet {
                sent_requests,
                removed_sent,
                incoming_requests,
                removed_incoming,
                established,
            } = contact_cs;

            for ((owner, _contact), entry) in sent_requests {
                match self.identity_manager.managed_identity_mut(&owner) {
                    Some(managed) => {
                        managed
                            .sent_contact_requests
                            .insert(entry.request.recipient_id, entry.request);
                    }
                    None => tracing::warn!(
                        owner = %owner,
                        "skipping sent contact request during apply: owner identity not in wallet"
                    ),
                }
            }
            for ((owner, _contact), entry) in incoming_requests {
                match self.identity_manager.managed_identity_mut(&owner) {
                    Some(managed) => {
                        managed
                            .incoming_contact_requests
                            .insert(entry.request.sender_id, entry.request);
                    }
                    None => tracing::warn!(
                        owner = %owner,
                        "skipping incoming contact request during apply: owner identity not in wallet"
                    ),
                }
            }
            for (owner, contact) in removed_sent {
                if let Some(managed) = self.identity_manager.managed_identity_mut(&owner) {
                    managed.sent_contact_requests.remove(&contact);
                }
            }
            for (owner, contact) in removed_incoming {
                if let Some(managed) = self.identity_manager.managed_identity_mut(&owner) {
                    managed.incoming_contact_requests.remove(&contact);
                }
            }
            // Established promotions — drop any matching pending
            // entries on both sides per the auto-establishment contract.
            for ((owner, _contact), established) in established {
                match self.identity_manager.managed_identity_mut(&owner) {
                    Some(managed) => {
                        managed.apply_established_contact(established);
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
        if let Some(addr_cs) = platform_addresses {
            for (addr, credits) in addr_cs.addresses {
                self.platform_address_balances.insert(addr, credits);
            }
            for addr in addr_cs.removed {
                self.platform_address_balances.remove(&addr);
            }
        }

        // 5. Asset locks. Move each entry by value through the
        //    `AssetLockEntry → TrackedAssetLock` mapping — the only
        //    field difference is `amount_duffs` vs `amount`. The
        //    `Transaction` inside the entry transfers ownership
        //    directly into the wallet map with no clone.
        if let Some(al_cs) = asset_locks {
            for (out_point, entry) in al_cs.asset_locks {
                self.tracked_asset_locks.insert(
                    out_point,
                    TrackedAssetLock {
                        out_point: entry.out_point,
                        transaction: entry.transaction,
                        account_index: entry.account_index,
                        funding_type: entry.funding_type,
                        identity_index: entry.identity_index,
                        amount: entry.amount_duffs,
                        status: entry.status,
                        proof: entry.proof,
                    },
                );
            }
            for out_point in al_cs.removed {
                self.tracked_asset_locks.remove(&out_point);
            }
        }

        // 6. Token balances + watch registry deltas.
        if let Some(tok_cs) = token_balances {
            for (key, balance) in tok_cs.balances {
                self.token_balances.insert(key, balance);
            }
            for key in tok_cs.removed_balances {
                self.token_balances.remove(&key);
            }
            for (identity_id, tokens) in tok_cs.watched {
                self.token_watched
                    .entry(identity_id)
                    .or_default()
                    .extend(tokens);
            }
            for (identity_id, tokens) in tok_cs.unwatched {
                if let Some(set) = self.token_watched.get_mut(&identity_id) {
                    for token in &tokens {
                        set.remove(token);
                    }
                    if set.is_empty() {
                        self.token_watched.remove(&identity_id);
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
        info.apply_changeset(&mut wallet, cs).expect("apply");
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

        info.apply_changeset(&mut wallet, cs).expect("apply insert");
        assert_eq!(info.identity_manager.primary_identity_id, Some(id_a));
        assert_eq!(info.identity_manager.identity_count(), 2);

        // Remove the primary; apply should re-select the next available.
        let mut cs = PlatformWalletChangeSet::default();
        let mut id_cs = IdentityChangeSet::default();
        id_cs.removed.insert(id_a);
        cs.identities = Some(id_cs);

        info.apply_changeset(&mut wallet, cs).expect("apply remove");
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

        // Idempotent double-apply: clone explicitly because apply
        // consumes the changeset by value to avoid hidden clones in
        // the persister-load hot path.
        info.apply_changeset(&mut wallet, cs.clone()).expect("first apply");
        info.apply_changeset(&mut wallet, cs).expect("second apply");

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
        info.apply_changeset(&mut wallet, cs).expect("apply");
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
        info.apply_changeset(&mut wallet, cs).expect("apply identity");

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
        info.apply_changeset(&mut wallet, cs).expect("apply established");

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
        info.apply_changeset(&mut wallet, cs).expect("apply insert");
        assert_eq!(info.platform_address_balances.get(&addr1), Some(&100));
        assert_eq!(info.platform_address_balances.get(&addr2), Some(&200));

        // Remove one, update the other.
        let mut addr_cs = PlatformAddressChangeSet::default();
        addr_cs.addresses.insert(addr1, 150);
        addr_cs.removed.insert(addr2);
        let mut cs = PlatformWalletChangeSet::default();
        cs.platform_addresses = Some(addr_cs);
        info.apply_changeset(&mut wallet, cs).expect("apply remove");
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
        info.apply_changeset(&mut wallet, cs).expect("apply");
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
        info.apply_changeset(&mut wallet, cs).expect("apply remove");
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
        info.apply_changeset(&mut wallet, cs).expect("apply watch");
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
        info.apply_changeset(&mut wallet, cs).expect("apply unwatch");
        assert!(!info.token_balances.contains_key(&(identity, token)));
        assert!(!info.token_watched.contains_key(&identity));
    }

    // ----------------------------------------------------------------------
    // Round-trip tests (Phase 9a-4)
    //
    // The shape of every test is identical:
    //   1. Build two empty `PlatformWalletInfo`s — A is the wallet that gets
    //      mutated, B is the sibling that will receive the changeset.
    //   2. Mutate A via the new mutation methods (which now return sub-
    //      changesets).
    //   3. Wrap the returned sub-changeset into a `PlatformWalletChangeSet`
    //      and apply it to B.
    //   4. Assert B's state matches A's state on the field(s) the mutation
    //      touched.
    //
    // These verify the round-trip contract: changesets emitted by mutations
    // are faithful enough that apply rebuilds the same in-memory state. This
    // is what makes the persister adapter (Phase 9a-5) safe — it can
    // serialize the captured changeset and deserialize it back into a
    // sibling wallet on restart with no information loss.
    //
    // Out of scope: AssetLockManager / TokenWallet / PlatformAddressWallet
    // mutations are async and require an `Sdk` + broadcaster + Notify, so
    // they can't run as plain unit tests. Their round-trip coverage will
    // come from integration tests (Phase 9a-4 follow-up). The
    // synthesized-data tests above already cover the apply side; the gap
    // is verifying the *mutation side* emits a faithful changeset.
    // ----------------------------------------------------------------------

    /// Helper: wrap an `IdentityChangeSet` into a top-level
    /// `PlatformWalletChangeSet` for apply.
    fn wrap_id(id_cs: IdentityChangeSet) -> PlatformWalletChangeSet {
        PlatformWalletChangeSet {
            identities: Some(id_cs),
            ..Default::default()
        }
    }

    /// Helper: wrap a `ContactChangeSet` into a top-level
    /// `PlatformWalletChangeSet` for apply.
    fn wrap_contacts(contact_cs: ContactChangeSet) -> PlatformWalletChangeSet {
        PlatformWalletChangeSet {
            contacts: Some(contact_cs),
            ..Default::default()
        }
    }

    #[test]
    fn round_trip_add_identity() {
        let mut wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);

        // Mutate A.
        let id_cs = info_a
            .identity_manager
            .add_identity(make_test_identity(1, 1), 7)
            .expect("add");
        let id = Identifier::from([1u8; 32]);

        // Apply the captured changeset to B.
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply");

        // Both wallets should now hold the identity with matching fields.
        let a = info_a.identity_manager.managed_identity(&id).expect("a");
        let b = info_b.identity_manager.managed_identity(&id).expect("b");
        assert_eq!(a.identity_index, b.identity_index);
        assert_eq!(a.identity_index, 7);
        assert_eq!(
            info_a.identity_manager.primary_identity_id,
            info_b.identity_manager.primary_identity_id,
        );
        assert_eq!(info_a.identity_manager.primary_identity_id, Some(id));
    }

    #[test]
    fn round_trip_remove_identity_reselects_primary() {
        let mut wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);

        // Both A and B start with two identities (id1 primary, id2 second).
        for info in [&mut info_a, &mut info_b] {
            let _ = info
                .identity_manager
                .add_identity(make_test_identity(1, 1), 0)
                .expect("add 1");
            let _ = info
                .identity_manager
                .add_identity(make_test_identity(2, 1), 1)
                .expect("add 2");
        }
        let id1 = Identifier::from([1u8; 32]);
        let id2 = Identifier::from([2u8; 32]);
        assert_eq!(info_a.identity_manager.primary_identity_id, Some(id1));
        assert_eq!(info_b.identity_manager.primary_identity_id, Some(id1));

        // Remove the primary on A.
        let (_, id_cs) = info_a
            .identity_manager
            .remove_identity(&id1)
            .expect("remove");

        // Replay the changeset on B.
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply");

        // Both should converge: id1 gone, id2 is the new primary.
        assert_eq!(info_a.identity_manager.identity_count(), 1);
        assert_eq!(info_b.identity_manager.identity_count(), 1);
        assert_eq!(info_a.identity_manager.primary_identity_id, Some(id2));
        assert_eq!(info_b.identity_manager.primary_identity_id, Some(id2));
    }

    #[test]
    fn round_trip_set_label() {
        let mut wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);

        // Both start with the same identity.
        for info in [&mut info_a, &mut info_b] {
            let _ = info
                .identity_manager
                .add_identity(make_test_identity(1, 1), 0)
                .expect("add");
        }
        let id = Identifier::from([1u8; 32]);

        // Set a label on A via the manager method (returns a changeset).
        let id_cs = info_a
            .identity_manager
            .set_label(&id, "alice".into())
            .expect("set label");

        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply");

        let a = info_a.identity_manager.managed_identity(&id).expect("a");
        let b = info_b.identity_manager.managed_identity(&id).expect("b");
        assert_eq!(a.label, b.label);
        assert_eq!(b.label.as_deref(), Some("alice"));
    }

    #[test]
    fn round_trip_dpns_name_and_top_up() {
        use crate::wallet::identity::managed_identity::DpnsNameInfo;

        let mut wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);

        for info in [&mut info_a, &mut info_b] {
            let _ = info
                .identity_manager
                .add_identity(make_test_identity(1, 1), 0)
                .expect("add");
        }
        let id = Identifier::from([1u8; 32]);

        // Add a DPNS name on A's managed identity (returns IdentityChangeSet
        // via snapshot_changeset).
        let dpns_cs = info_a
            .identity_manager
            .managed_identity_mut(&id)
            .expect("a managed")
            .add_dpns_name(DpnsNameInfo {
                label: "alice".into(),
                acquired_at: Some(123_456),
            });

        // Record a top-up on the same identity.
        let top_up_cs = info_a
            .identity_manager
            .managed_identity_mut(&id)
            .expect("a managed")
            .record_top_up(0, 5_000_000);

        // Apply both changesets to B in order.
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(dpns_cs))
            .expect("apply dpns");
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(top_up_cs))
            .expect("apply top-up");

        let a = info_a.identity_manager.managed_identity(&id).expect("a");
        let b = info_b.identity_manager.managed_identity(&id).expect("b");
        assert_eq!(a.dpns_names.len(), b.dpns_names.len());
        assert_eq!(a.dpns_names.first().map(|n| n.label.as_str()), Some("alice"));
        assert_eq!(b.dpns_names.first().map(|n| n.label.as_str()), Some("alice"));
        assert_eq!(a.top_ups.get(&0), Some(&5_000_000));
        assert_eq!(b.top_ups.get(&0), Some(&5_000_000));
    }

    #[test]
    fn round_trip_block_time_updates() {
        use crate::wallet::identity::managed_identity::BlockTime;

        let mut wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);

        for info in [&mut info_a, &mut info_b] {
            let _ = info
                .identity_manager
                .add_identity(make_test_identity(1, 1), 0)
                .expect("add");
        }
        let id = Identifier::from([1u8; 32]);
        let bt = BlockTime::new(100, 200, 1_700_000_000);

        // Update both timestamps on A — each call returns a changeset.
        let cs1 = info_a
            .identity_manager
            .managed_identity_mut(&id)
            .expect("a")
            .update_balance_block_time(bt);
        let cs2 = info_a
            .identity_manager
            .managed_identity_mut(&id)
            .expect("a")
            .update_keys_sync_block_time(bt);

        info_b
            .apply_changeset(&mut wallet_b, wrap_id(cs1))
            .expect("apply balance bt");
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(cs2))
            .expect("apply keys bt");

        let a = info_a.identity_manager.managed_identity(&id).expect("a");
        let b = info_b.identity_manager.managed_identity(&id).expect("b");
        assert_eq!(a.last_updated_balance_block_time, b.last_updated_balance_block_time);
        assert_eq!(a.last_synced_keys_block_time, b.last_synced_keys_block_time);
        assert_eq!(b.last_updated_balance_block_time, Some(bt));
    }

    #[test]
    fn round_trip_last_scanned_index_watermark() {
        let mut wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);

        let id_cs = info_a.identity_manager.set_last_scanned_index(42);
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply");

        assert_eq!(info_a.identity_manager.last_scanned_index(), 42);
        assert_eq!(info_b.identity_manager.last_scanned_index(), 42);
    }

    #[test]
    fn round_trip_sent_contact_request_no_auto_establish() {
        let mut wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);

        // Both wallets host the same owning identity so contact routing works.
        for info in [&mut info_a, &mut info_b] {
            let _ = info
                .identity_manager
                .add_identity(make_test_identity(1, 1), 0)
                .expect("add owner");
        }
        let owner = Identifier::from([1u8; 32]);
        let recipient = Identifier::from([2u8; 32]);

        // Send a contact request on A; capture the emitted ContactChangeSet.
        let contact_cs = info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a owner")
            .add_sent_contact_request(make_test_contact_request(1, 2));
        // Plain insert path — no matching incoming, so `established` is empty.
        assert!(contact_cs.established.is_empty());
        assert!(contact_cs.sent_requests.contains_key(&(owner, recipient)));

        info_b
            .apply_changeset(&mut wallet_b, wrap_contacts(contact_cs))
            .expect("apply");

        let b_owner = info_b
            .identity_manager
            .managed_identity(&owner)
            .expect("b owner");
        assert!(b_owner.sent_contact_requests.contains_key(&recipient));
        assert!(b_owner.incoming_contact_requests.is_empty());
        assert!(b_owner.established_contacts.is_empty());
    }

    #[test]
    fn round_trip_auto_establish_contact() {
        let mut wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);

        for info in [&mut info_a, &mut info_b] {
            let _ = info
                .identity_manager
                .add_identity(make_test_identity(1, 1), 0)
                .expect("add owner");
        }
        let owner = Identifier::from([1u8; 32]);
        let other = Identifier::from([2u8; 32]);

        // First the incoming arrives. Then the outgoing arrives — at which
        // point auto-establishment fires and emits an `established` entry.
        let cs_in = info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a owner")
            .add_incoming_contact_request(make_test_contact_request(2, 1));
        let cs_out = info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a owner")
            .add_sent_contact_request(make_test_contact_request(1, 2));

        // The second changeset (the auto-establish trigger) carries the full
        // `EstablishedContact`.
        assert!(cs_out.established.contains_key(&(owner, other)));
        assert!(cs_out.sent_requests.is_empty());

        // Replay both onto B in mutation order.
        info_b
            .apply_changeset(&mut wallet_b, wrap_contacts(cs_in))
            .expect("apply incoming");
        info_b
            .apply_changeset(&mut wallet_b, wrap_contacts(cs_out))
            .expect("apply auto-establish");

        // B converges: established present, both pending sets drained.
        let a_owner = info_a
            .identity_manager
            .managed_identity(&owner)
            .expect("a owner");
        let b_owner = info_b
            .identity_manager
            .managed_identity(&owner)
            .expect("b owner");
        assert!(a_owner.established_contacts.contains_key(&other));
        assert!(b_owner.established_contacts.contains_key(&other));
        assert!(a_owner.sent_contact_requests.is_empty());
        assert!(b_owner.sent_contact_requests.is_empty());
        assert!(a_owner.incoming_contact_requests.is_empty());
        assert!(b_owner.incoming_contact_requests.is_empty());
    }

    #[test]
    fn round_trip_remove_contact_request() {
        let mut wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);

        for info in [&mut info_a, &mut info_b] {
            let _ = info
                .identity_manager
                .add_identity(make_test_identity(1, 1), 0)
                .expect("add owner");
        }
        let owner = Identifier::from([1u8; 32]);
        let recipient = Identifier::from([2u8; 32]);

        // Both A and B start with a sent request — apply the same insert
        // changeset to both.
        let insert_cs = info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a")
            .add_sent_contact_request(make_test_contact_request(1, 2));
        info_b
            .apply_changeset(&mut wallet_b, wrap_contacts(insert_cs))
            .expect("apply insert");

        // Now remove on A.
        let (_, remove_cs) = info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a")
            .remove_sent_contact_request(&recipient);
        assert!(remove_cs.removed_sent.contains(&(owner, recipient)));

        info_b
            .apply_changeset(&mut wallet_b, wrap_contacts(remove_cs))
            .expect("apply remove");

        let a_owner = info_a
            .identity_manager
            .managed_identity(&owner)
            .expect("a");
        let b_owner = info_b
            .identity_manager
            .managed_identity(&owner)
            .expect("b");
        assert!(a_owner.sent_contact_requests.is_empty());
        assert!(b_owner.sent_contact_requests.is_empty());
    }

    // ----------------------------------------------------------------------
    // Reviewer test gaps (Phase 9a-3 followup)
    // ----------------------------------------------------------------------

    /// Reviewer #6a: removing the sole identity must clear the primary
    /// to `None`. The fallback `identities.keys().next()` returns
    /// `None` on an empty map, matching the mutation-side semantics.
    #[test]
    fn apply_remove_sole_identity_clears_primary() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        let id = Identifier::from([1u8; 32]);
        let mut id_cs = IdentityChangeSet::default();
        let managed = ManagedIdentity::new(make_test_identity(1, 0), 0);
        id_cs.identities.insert(id, IdentityEntry::from_managed(&managed));
        id_cs.primary_identity = Some(id);
        info.apply_changeset(&mut wallet, wrap_id(id_cs)).expect("apply insert");
        assert_eq!(info.identity_manager.primary_identity_id, Some(id));

        let mut id_cs = IdentityChangeSet::default();
        id_cs.removed.insert(id);
        info.apply_changeset(&mut wallet, wrap_id(id_cs)).expect("apply remove");

        assert_eq!(info.identity_manager.identity_count(), 0);
        assert_eq!(info.identity_manager.primary_identity_id, None);
    }

    /// Reviewer #6b: applying a tombstone twice is a no-op (the second
    /// apply must not error or panic on the missing-key remove).
    #[test]
    fn apply_remove_tombstone_double_apply_is_noop() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        // Insert two so the removal isn't a complete-clear case.
        let id_a = Identifier::from([1u8; 32]);
        let id_b = Identifier::from([2u8; 32]);
        let mut id_cs = IdentityChangeSet::default();
        let m_a = ManagedIdentity::new(make_test_identity(1, 0), 0);
        let m_b = ManagedIdentity::new(make_test_identity(2, 0), 1);
        id_cs.identities.insert(id_a, IdentityEntry::from_managed(&m_a));
        id_cs.identities.insert(id_b, IdentityEntry::from_managed(&m_b));
        id_cs.primary_identity = Some(id_a);
        info.apply_changeset(&mut wallet, wrap_id(id_cs)).expect("apply insert");

        let mut tombstone = IdentityChangeSet::default();
        tombstone.removed.insert(id_a);
        let cs = wrap_id(tombstone);

        // Idempotent tombstone replay: explicit clone for the first
        // apply.
        info.apply_changeset(&mut wallet, cs.clone()).expect("first remove");
        info.apply_changeset(&mut wallet, cs).expect("second remove (no-op)");

        assert_eq!(info.identity_manager.identity_count(), 1);
        assert!(info.identity_manager.managed_identity(&id_a).is_none());
        assert_eq!(info.identity_manager.primary_identity_id, Some(id_b));
    }

    /// Reviewer #6c: a stale entry with a lower revision must NOT
    /// clobber an in-place identity blob with a higher revision. This
    /// locks in the merge-policy invariant on `apply_identity_entry`
    /// (matching `IdentityChangeSet::merge`).
    #[test]
    fn apply_lower_revision_does_not_overwrite_higher() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        // Seed with revision 5.
        let id = Identifier::from([1u8; 32]);
        let high = ManagedIdentity::new(make_test_identity(1, 5), 0);
        let mut high_cs = IdentityChangeSet::default();
        high_cs.identities.insert(id, IdentityEntry::from_managed(&high));
        info.apply_changeset(&mut wallet, wrap_id(high_cs)).expect("seed");

        // Stale revision 2 entry.
        use dpp::identity::accessors::IdentityGettersV0;
        let stale = ManagedIdentity::new(make_test_identity(1, 2), 0);
        let mut stale_cs = IdentityChangeSet::default();
        stale_cs.identities.insert(id, IdentityEntry::from_managed(&stale));
        info.apply_changeset(&mut wallet, wrap_id(stale_cs)).expect("stale apply");

        // The on-chain blob must still carry revision 5.
        let restored = info.identity_manager.managed_identity(&id).expect("present");
        assert_eq!(restored.identity.revision(), 5);
    }

    /// Reviewer #6d: contact tombstone for a present (non-orphan) owner
    /// must drop the matching pending request — happy-path coverage
    /// previously only existed via the orphan-skip test.
    #[test]
    fn apply_contact_tombstone_drops_pending_for_present_owner() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        // Set up an owner with a pending sent request.
        let owner = Identifier::from([1u8; 32]);
        let other = Identifier::from([2u8; 32]);
        let mut managed = ManagedIdentity::new(make_test_identity(1, 0), 0);
        managed
            .sent_contact_requests
            .insert(other, make_test_contact_request(1, 2));
        let mut id_cs = IdentityChangeSet::default();
        id_cs.identities.insert(owner, IdentityEntry::from_managed(&managed));
        info.apply_changeset(&mut wallet, wrap_id(id_cs)).expect("seed identity");

        // Apply a tombstone for that pair.
        let mut contact_cs = ContactChangeSet::default();
        contact_cs.removed_sent.insert((owner, other));
        info.apply_changeset(&mut wallet, wrap_contacts(contact_cs)).expect("apply tombstone");

        let restored = info.identity_manager.managed_identity(&owner).expect("present");
        assert!(!restored.sent_contact_requests.contains_key(&other));
    }

    #[test]
    fn round_trip_double_apply_is_idempotent() {
        let mut wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);

        // Compose a multi-field mutation: add identity + label + dpns + top-up.
        let id_cs1 = info_a
            .identity_manager
            .add_identity(make_test_identity(1, 1), 0)
            .expect("add");
        let id_cs2 = info_a
            .identity_manager
            .set_label(&Identifier::from([1u8; 32]), "alice".into())
            .expect("label");

        // Apply both changesets twice on B and verify state matches A.
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs1.clone()))
            .expect("first add");
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs2.clone()))
            .expect("first label");
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs1))
            .expect("second add");
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs2))
            .expect("second label");

        let id = Identifier::from([1u8; 32]);
        let a = info_a.identity_manager.managed_identity(&id).expect("a");
        let b = info_b.identity_manager.managed_identity(&id).expect("b");
        assert_eq!(a.label, b.label);
        assert_eq!(b.label.as_deref(), Some("alice"));
        assert_eq!(info_a.identity_manager.identity_count(), 1);
        assert_eq!(info_b.identity_manager.identity_count(), 1);
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

        // Idempotent double-apply: explicit clone for the first call.
        info.apply_changeset(&mut wallet, cs.clone()).expect("first apply");
        info.apply_changeset(&mut wallet, cs).expect("second apply");

        assert_eq!(info.identity_manager.identity_count(), 1);
        assert_eq!(info.identity_manager.last_scanned_index(), 7);
        assert_eq!(info.platform_address_balances.get(&addr), Some(&1_000));
        assert_eq!(info.token_balances.get(&(identity, token)), Some(&42));
    }
}
