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
//! 6. `cs.token_balances` — drained but **not replayed** here. The
//!    canonical home of token-balance state is the
//!    [`IdentitySyncManager`](crate::manager::identity_sync::IdentitySyncManager)
//!    cache, which is rebuilt by the next sync pass; the FFI persister
//!    surfaces upserts/tombstones to the Swift side via its own
//!    callback. There is nothing on `PlatformWalletInfo` to apply
//!    them onto.
//! 7. `update_balance()` — recompute the cached `WalletBalance` from
//!    the now-restored UTXO set; the returned changeset is discarded.

use key_wallet::wallet::Wallet;

use crate::changeset::PlatformWalletChangeSet;
use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
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
    /// Consumes the changeset by value; tests that replay must
    /// `.clone()` at the call site. See the module docs for
    /// invariants and ordering.
    pub fn apply_changeset(
        &mut self,
        wallet: &mut Wallet,
        cs: PlatformWalletChangeSet,
    ) -> Result<(), ApplyError> {
        let PlatformWalletChangeSet {
            core,
            identities,
            identity_keys,
            contacts,
            platform_addresses,
            asset_locks,
            token_balances,
            dashpay_profiles,
            dashpay_payments_overlay,
            // DashPay invitations (DIP-13) are persistence-only here: the
            // "Sent invitations" list is the Swift SwiftData mirror, and the
            // Rust manager holds no in-memory invitation state in v1 (reclaim
            // is future). Drop explicitly so future readers don't expect a
            // replay hook.
            invitations: _,
            // Registration-round metadata / per-account specs /
            // per-pool snapshots are persistence-only — the
            // canonical in-memory wallet state is built up at
            // creation time before this apply path ever runs.
            // Drop them explicitly so future readers don't expect
            // a replay hook here.
            wallet_metadata: _,
            account_registrations: _,
            provider_key_account_registrations: _,
            account_address_pools: _,
            // The deferred contact-crypto queue is persistence-only here too:
            // the in-memory queue is mutated directly at the enqueue (sweep)
            // and drain (signer-present) sites, and restored at load via the
            // start-state path. No changeset-replay hook in apply.
            pending_contact_crypto_added: _,
            pending_contact_crypto_cleared: _,
            // Shielded deltas are owned by `ShieldedWallet` (which
            // mutates its store directly during sync / spend); the
            // canonical in-memory state lives there and the
            // changeset is persistence-side only. Drop here.
            #[cfg(feature = "shielded")]
                shielded: _,
        } = cs;

        // 1. Core wallet state. In the new event-bus model, a
        //    `CoreChangeSet` flows OUT (event adapter → persister) but
        //    is never replayed back IN through this apply path —
        //    upstream `key_wallet`'s `process_block` keeps the
        //    in-memory `ManagedWalletInfo` up to date at runtime, and
        //    boot-time state restoration goes through
        //    `ClientStartState` (the persister's `load()` payload),
        //    not through changeset replay. The core field on `cs` is
        //    therefore informational here and intentionally not
        //    applied; we drop it explicitly so future readers don't
        //    expect a re-application path that no longer exists.
        drop(core);

        // 2. Identities.
        if let Some(id_cs) = identities {
            let crate::changeset::IdentityChangeSet {
                identities,
                removed,
            } = id_cs;

            for (_id, entry) in identities {
                self.identity_manager.apply_identity_entry(entry);
            }
            // Best-effort tombstones across both buckets. Routed
            // through `remove_for_apply` so the manager's side-index
            // stays in lockstep with the buckets without us having to
            // reach in and touch the index from out here.
            for removed_id in &removed {
                self.identity_manager.remove_for_apply(removed_id);
            }
        }

        // 2b. Identity keys. Runs after the scalar identity pass so
        //     the owning ManagedIdentity is guaranteed to exist before
        //     we layer keys into it. Upserts land first, then removals,
        //     matching the discipline used across the rest of this
        //     function. Orphan entries (owner not in the wallet) are
        //     logged and skipped by the per-entry apply helpers.
        if let Some(keys_cs) = identity_keys {
            let crate::changeset::IdentityKeysChangeSet { upserts, removed } = keys_cs;
            // Thread the wallet network through so the key-apply
            // path can reproduce DIP-9 derivation paths for any
            // entry that carries `(wallet_id, derivation_indices)`.
            let network = wallet.network;
            for (_key, entry) in upserts {
                self.identity_manager
                    .apply_identity_key_entry(entry, network);
            }
            for (identity_id, key_id) in removed {
                self.identity_manager
                    .apply_identity_key_removal(&identity_id, key_id);
            }
        }

        // 3. Contacts. Each entry routes to its owning ManagedIdentity by
        //    `(owner, contact)` key; orphans (owner not in the wallet)
        //    are logged and skipped. Every map mutation goes through the
        //    `apply_*` replay methods — the relationship maps are sealed
        //    to the state layer, and the replay methods reproduce
        //    persisted state without re-running the live invariants
        //    (`apply_established_contact` additionally drops both
        //    pending sides per the contract).
        if let Some(contact_cs) = contacts {
            let crate::changeset::ContactChangeSet {
                sent_requests,
                removed_sent,
                incoming_requests,
                removed_incoming,
                established,
                ignored,
                unignored,
            } = contact_cs;

            for (key, entry) in sent_requests {
                match self.identity_manager.managed_identity_mut(&key.owner_id) {
                    Some(managed) => {
                        managed.apply_sent_contact_request(entry.request);
                    }
                    None => tracing::warn!(
                        owner = %key.owner_id,
                        "skipping sent contact request during apply: owner identity not in wallet"
                    ),
                }
            }
            for (key, entry) in incoming_requests {
                match self.identity_manager.managed_identity_mut(&key.owner_id) {
                    Some(managed) => {
                        managed.apply_incoming_contact_request(entry.request);
                    }
                    None => tracing::warn!(
                        owner = %key.owner_id,
                        "skipping incoming contact request during apply: owner identity not in wallet"
                    ),
                }
            }
            for key in removed_sent {
                if let Some(managed) = self.identity_manager.managed_identity_mut(&key.owner_id) {
                    managed.apply_removed_sent(&key.recipient_id);
                }
            }
            for key in removed_incoming {
                if let Some(managed) = self.identity_manager.managed_identity_mut(&key.owner_id) {
                    managed.apply_removed_incoming(&key.sender_id);
                }
            }
            // Established promotions — drop any matching pending
            // entries on both sides per the auto-establishment contract.
            for (key, established) in established {
                match self.identity_manager.managed_identity_mut(&key.owner_id) {
                    Some(managed) => {
                        managed.apply_established_contact(established);
                    }
                    None => tracing::warn!(
                        owner = %key.owner_id,
                        "skipping established contact during apply: owner identity not in wallet"
                    ),
                }
            }
            // Ignored senders (per-sender mute, local-only). Restore the
            // in-memory suppression set so the sync ingest path won't
            // resurrect an ignored sender's requests after a restart.
            // `unignored` is applied AFTER `ignored` so an un-ignore in the
            // same delta wins (the sender ends up not ignored). Orphan
            // owners are logged and skipped.
            for (owner_id, sender_id) in ignored {
                match self.identity_manager.managed_identity_mut(&owner_id) {
                    Some(managed) => {
                        managed.apply_ignored_sender(sender_id);
                    }
                    None => tracing::warn!(
                        owner = %owner_id,
                        "skipping ignored sender during apply: owner identity not in wallet"
                    ),
                }
            }
            for (owner_id, sender_id) in unignored {
                if let Some(managed) = self.identity_manager.managed_identity_mut(&owner_id) {
                    managed.apply_unignored_sender(&sender_id);
                }
            }
        }

        // 3b. DashPay profile/payment overlays. Applied AFTER identities
        //     so the target ManagedIdentity exists. Only touches dashpay
        //     fields — does not require the identity blob.
        if let Some(profiles) = dashpay_profiles {
            for (id, profile) in profiles {
                if let Some(managed) = self.identity_manager.managed_identity_mut(&id) {
                    *managed.dashpay_profile_mut() = profile;
                }
            }
        }
        if let Some(payments) = dashpay_payments_overlay {
            for (id, payments_map) in payments {
                if let Some(managed) = self.identity_manager.managed_identity_mut(&id) {
                    managed.dashpay_payments_mut().extend(payments_map);
                }
            }
        }

        // 4. Platform address balances. Each entry carries its own
        //    account index so we route to the right
        //    ManagedPlatformAccount instead of lumping everything
        //    onto the first account. Callers are expected to pass a
        //    wallet-scoped changeset; we don't double-check
        //    `entry.wallet_id` because `PlatformWalletInfo` doesn't
        //    know its own id.
        if let Some(addr_cs) = platform_addresses {
            for entry in addr_cs.addresses {
                if let Some(account) = self
                    .core_wallet
                    .platform_payment_managed_account_at_index_mut(entry.account_index)
                {
                    account.set_address_credit_balance(entry.address, entry.funds.balance, None);
                    // Nonce isn't stored on `ManagedPlatformAccount`;
                    // callers that need it persist it via their own
                    // store (see evo-tool's platform_address_balances
                    // table which writes both `balance` and `nonce`
                    // from the changeset).
                }
            }
        }

        // 5. Asset locks. Move each entry by value through the
        //    `AssetLockEntry → TrackedAssetLock` mapping — the only
        //    field difference is `amount_duffs` vs `amount`. The
        //    `Transaction` inside the entry transfers ownership
        //    directly into the wallet map with no clone.
        //
        //    `Consumed` is the terminal post-consumption state: it
        //    means an identity registration / top-up has burned this
        //    asset lock. We drop the entry from the in-memory map
        //    (the wallet has no further use for it; nothing should be
        //    waiting on its proof) but the changeset's `asset_locks`
        //    entry still flows through to the Swift persister so the
        //    `PersistentAssetLock` row is upserted with `statusRaw=4`
        //    for historical lookups (e.g. the Transactions list
        //    rendering the original locked amount on a consumed
        //    funding tx).
        if let Some(al_cs) = asset_locks {
            for (out_point, entry) in al_cs.asset_locks {
                if entry.status == AssetLockStatus::Consumed {
                    self.tracked_asset_locks.remove(&out_point);
                } else {
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
            }
            for out_point in al_cs.removed {
                self.tracked_asset_locks.remove(&out_point);
            }
        }

        // 6. Token balances. The persistent cache lives entirely on
        //    the FFI / Swift side now; the in-memory canonical balance
        //    state lives on `IdentitySyncManager`, which gets rebuilt
        //    by the next sync pass rather than replayed from a
        //    changeset. Drop the field explicitly so future readers
        //    don't expect a mutation hook here.
        drop(token_balances);

        // 7. Recompute cached UI balance from the now-restored UTXO set.
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
        self.core_wallet.update_balance();
        // Mirror the recomputed balance into the lock-free Arc that the
        // UI reads.
        let core_balance = &self.core_wallet.balance;
        self.balance.set(
            core_balance.confirmed(),
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
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use dashcore::OutPoint;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::Identity;
    use dpp::prelude::Identifier;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;

    use crate::changeset::{
        AssetLockChangeSet, AssetLockEntry, ContactChangeSet, ContactRequestEntry,
        IdentityChangeSet, IdentityEntry, PlatformAddressChangeSet, PlatformWalletChangeSet,
        ReceivedContactRequestKey, SentContactRequestKey, TokenBalanceChangeSet,
    };
    use crate::wallet::asset_lock::tracked::AssetLockStatus;
    use crate::wallet::core::WalletBalance;
    use crate::wallet::identity::state::managed_identity::ManagedIdentity;
    use crate::wallet::identity::IdentityManager;
    use crate::wallet::identity::{ContactRequest, EstablishedContact};
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
    use key_wallet::wallet::Wallet;

    fn noop_persister() -> WalletPersister {
        WalletPersister::new([0u8; 32], Arc::new(NoPlatformPersistence))
    }

    fn build_test_wallet() -> Wallet {
        Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::None)
            .expect("test wallet")
    }

    fn empty_info(wallet: &Wallet) -> PlatformWalletInfo {
        PlatformWalletInfo {
            core_wallet: ManagedWalletInfo::from_wallet(wallet, 0),
            balance: std::sync::Arc::new(WalletBalance::new()),
            identity_manager: IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
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
    }

    #[test]
    fn apply_identity_insert_then_remove() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        let id_a = Identifier::from([1u8; 32]);
        let id_b = Identifier::from([2u8; 32]);

        // Insert two identities into the wallet bucket.
        let mut cs = PlatformWalletChangeSet::default();
        let mut id_cs = IdentityChangeSet::default();
        let mut managed_a = ManagedIdentity::new(make_test_identity(1, 0), 0);
        managed_a.wallet_id = Some([1u8; 32]);
        let mut managed_b = ManagedIdentity::new(make_test_identity(2, 0), 1);
        managed_b.wallet_id = Some([1u8; 32]);
        id_cs
            .identities
            .insert(id_a, IdentityEntry::from_managed(&managed_a));
        id_cs
            .identities
            .insert(id_b, IdentityEntry::from_managed(&managed_b));
        cs.identities = Some(id_cs);

        info.apply_changeset(&mut wallet, cs).expect("apply insert");
        assert_eq!(info.identity_manager.identity_count(), 2);

        // Remove id_a — apply walks both buckets, drops it, and leaves
        // id_b in place. No primary fixup; selection is a UI concern.
        let mut cs = PlatformWalletChangeSet::default();
        let mut id_cs = IdentityChangeSet::default();
        id_cs.removed.insert(id_a);
        cs.identities = Some(id_cs);

        info.apply_changeset(&mut wallet, cs).expect("apply remove");
        assert_eq!(info.identity_manager.identity_count(), 1);
        assert!(info.identity_manager.identity(&id_b).is_some());
    }

    #[test]
    fn apply_identity_double_apply_is_idempotent() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        let id = Identifier::from([7u8; 32]);
        // Wallet-owned identity at index 3 — verifies the bucket key
        // stays correct under double-apply.
        let mut managed = ManagedIdentity::new(make_test_identity(7, 1), 3);
        managed.wallet_id = Some([2u8; 32]);

        let mut cs = PlatformWalletChangeSet::default();
        let mut id_cs = IdentityChangeSet::default();
        id_cs
            .identities
            .insert(id, IdentityEntry::from_managed(&managed));
        cs.identities = Some(id_cs);

        // Idempotent double-apply: clone explicitly because apply
        // consumes the changeset by value to avoid hidden clones in
        // the persister-load hot path.
        info.apply_changeset(&mut wallet, cs.clone())
            .expect("first apply");
        info.apply_changeset(&mut wallet, cs).expect("second apply");

        let restored = info
            .identity_manager
            .managed_identity(&id)
            .expect("present");
        assert_eq!(restored.identity_index, Some(3));
        assert_eq!(restored.wallet_id, Some([2u8; 32]));
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
            SentContactRequestKey {
                owner_id: owner,
                recipient_id: other,
            },
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
        managed.apply_sent_contact_request(make_test_contact_request(1, 2));
        managed.apply_incoming_contact_request(make_test_contact_request(2, 1));
        id_cs
            .identities
            .insert(owner_id, IdentityEntry::from_managed(&managed));
        let mut cs = PlatformWalletChangeSet::default();
        cs.identities = Some(id_cs);
        info.apply_changeset(&mut wallet, cs)
            .expect("apply identity");

        // Now apply an established contact for the same pair.
        let established = EstablishedContact::new(
            other_id,
            make_test_contact_request(1, 2),
            make_test_contact_request(2, 1),
        );
        let mut contact_cs = ContactChangeSet::default();
        contact_cs.established.insert(
            SentContactRequestKey {
                owner_id,
                recipient_id: other_id,
            },
            established,
        );
        let mut cs = PlatformWalletChangeSet::default();
        cs.contacts = Some(contact_cs);
        info.apply_changeset(&mut wallet, cs)
            .expect("apply established");

        let managed = info
            .identity_manager
            .managed_identity(&owner_id)
            .expect("owner present");
        assert!(managed
            .dashpay()
            .established_contacts()
            .contains_key(&other_id));
        assert!(!managed
            .dashpay()
            .sent_contact_requests()
            .contains_key(&other_id));
        assert!(!managed
            .dashpay()
            .incoming_contact_requests()
            .contains_key(&other_id));
    }

    #[test]
    fn apply_platform_addresses_insert_and_remove() {
        use key_wallet::bip32::{ChildNumber, DerivationPath};
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType};
        use key_wallet::managed_account::managed_platform_account::ManagedPlatformAccount;
        use key_wallet::PlatformP2PKHAddress;

        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        // Set up a platform payment account so apply has somewhere to write.
        let base_path = DerivationPath::from(vec![
            ChildNumber::from_hardened_idx(9).unwrap(),
            ChildNumber::from_hardened_idx(1).unwrap(),
            ChildNumber::from_hardened_idx(17).unwrap(),
            ChildNumber::from_hardened_idx(0).unwrap(),
            ChildNumber::from_hardened_idx(0).unwrap(),
        ]);
        let pool = AddressPool::new_without_generation(
            base_path,
            AddressPoolType::Absent,
            20,
            Network::Testnet,
        );
        let platform_account = ManagedPlatformAccount::new(0, 0, pool, false);
        info.core_wallet
            .accounts
            .insert_platform_account(platform_account);

        let p2pkh1 = PlatformP2PKHAddress::new([10u8; 20]);
        let p2pkh2 = PlatformP2PKHAddress::new([20u8; 20]);

        use dash_sdk::platform::address_sync::AddressFunds;
        let funds = |balance, nonce| AddressFunds {
            balance,
            nonce,
            as_of_height: 0,
        };
        let wallet_id: crate::wallet::platform_wallet::WalletId = [0u8; 32];
        let entry = |address_index, address, funds| crate::PlatformAddressBalanceEntry {
            wallet_id,
            account_index: 0,
            address_index,
            address,
            funds,
        };

        // Insert two.
        let mut addr_cs = PlatformAddressChangeSet::default();
        addr_cs.addresses.push(entry(0, p2pkh1, funds(100, 0)));
        addr_cs.addresses.push(entry(1, p2pkh2, funds(200, 0)));
        let mut cs = PlatformWalletChangeSet::default();
        cs.platform_addresses = Some(addr_cs);
        info.apply_changeset(&mut wallet, cs).expect("apply insert");
        let account = info
            .core_wallet
            .first_platform_payment_managed_account()
            .unwrap();
        assert_eq!(account.address_credit_balance(&p2pkh1), 100);
        assert_eq!(account.address_credit_balance(&p2pkh2), 200);

        // Update one, drain the other to zero (per the new
        // PlatformAddressChangeSet model: drained addresses carry
        // balance 0 instead of being explicitly removed).
        let mut addr_cs = PlatformAddressChangeSet::default();
        addr_cs.addresses.push(entry(0, p2pkh1, funds(150, 0)));
        addr_cs.addresses.push(entry(1, p2pkh2, funds(0, 0)));
        let mut cs = PlatformWalletChangeSet::default();
        cs.platform_addresses = Some(addr_cs);
        info.apply_changeset(&mut wallet, cs).expect("apply remove");
        let account = info
            .core_wallet
            .first_platform_payment_managed_account()
            .unwrap();
        assert_eq!(account.address_credit_balance(&p2pkh1), 150);
        assert_eq!(account.address_credit_balance(&p2pkh2), 0);
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

    /// Token-balance changesets are accepted by `apply_changeset` for
    /// shape compatibility but are not replayed onto
    /// `PlatformWalletInfo` (which no longer has token_balances /
    /// token_watched fields). The canonical balance cache lives on
    /// `IdentitySyncManager` and is rebuilt by the next sync pass; the
    /// FFI persister surfaces the upserts/tombstones to the Swift side
    /// directly. This test pins the no-replay contract: applying a
    /// non-empty token-balance changeset must not error.
    #[test]
    fn apply_token_balance_changeset_is_noop_on_info() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        let identity = Identifier::from([5u8; 32]);
        let token = Identifier::from([10u8; 32]);

        let mut tok_cs = TokenBalanceChangeSet::default();
        tok_cs.balances.insert((identity, token), 999);
        tok_cs.removed_balances.insert((identity, token));
        let mut cs = PlatformWalletChangeSet::default();
        cs.token_balances = Some(tok_cs);
        info.apply_changeset(&mut wallet, cs).expect("apply token");

        // No assertion against `info` — the field is gone. The point
        // of this test is the call must not error.
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

    /// Wallet id used by every wallet-owned round-trip test below.
    /// Same value on A and B so the manager's two-bucket lookup hits
    /// the same slot on both sides.
    const ROUND_TRIP_WALLET_ID: [u8; 32] = [9u8; 32];

    #[test]
    fn round_trip_add_identity() {
        let wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);
        let p = noop_persister();

        // Mutate A (persister is a no-op, so state changes but nothing persists).
        info_a
            .identity_manager
            .add_identity(make_test_identity(1, 1), 7, ROUND_TRIP_WALLET_ID, &p)
            .expect("add");
        let id = Identifier::from([1u8; 32]);

        // Build the replay changeset from A's mutated state.
        let managed = info_a.identity_manager.managed_identity(&id).expect("a");
        let mut id_cs = IdentityChangeSet::default();
        id_cs
            .identities
            .insert(id, IdentityEntry::from_managed(managed));

        // Apply the constructed changeset to B.
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply");

        // Both wallets should now hold the identity with matching fields.
        let a = info_a.identity_manager.managed_identity(&id).expect("a");
        let b = info_b.identity_manager.managed_identity(&id).expect("b");
        assert_eq!(a.identity_index, b.identity_index);
        assert_eq!(a.identity_index, Some(7));
        assert_eq!(a.wallet_id, b.wallet_id);
        assert_eq!(a.wallet_id, Some(ROUND_TRIP_WALLET_ID));
        assert_eq!(
            info_a
                .identity_manager
                .highest_registration_index(&ROUND_TRIP_WALLET_ID),
            info_b
                .identity_manager
                .highest_registration_index(&ROUND_TRIP_WALLET_ID),
        );
    }

    #[test]
    fn round_trip_remove_identity() {
        let wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);
        let p = noop_persister();

        // Both A and B start with two identities in the wallet bucket.
        for info in [&mut info_a, &mut info_b] {
            info.identity_manager
                .add_identity(make_test_identity(1, 1), 0, ROUND_TRIP_WALLET_ID, &p)
                .expect("add 1");
            info.identity_manager
                .add_identity(make_test_identity(2, 1), 1, ROUND_TRIP_WALLET_ID, &p)
                .expect("add 2");
        }
        let id1 = Identifier::from([1u8; 32]);
        let id2 = Identifier::from([2u8; 32]);

        // Remove id1 on A.
        info_a
            .identity_manager
            .remove_identity(&id1, &p)
            .expect("remove");

        // Build the replay changeset: tombstone id1.
        let mut id_cs = IdentityChangeSet::default();
        id_cs.removed.insert(id1);

        // Replay the changeset on B.
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply");

        // Both converge: id1 gone, id2 still present.
        assert_eq!(info_a.identity_manager.identity_count(), 1);
        assert_eq!(info_b.identity_manager.identity_count(), 1);
        assert!(info_a.identity_manager.identity(&id2).is_some());
        assert!(info_b.identity_manager.identity(&id2).is_some());
    }

    #[test]
    fn round_trip_dpns_name() {
        use crate::wallet::identity::state::managed_identity::DpnsNameInfo;

        let wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);
        let p = noop_persister();

        for info in [&mut info_a, &mut info_b] {
            info.identity_manager
                .add_identity(make_test_identity(1, 1), 0, ROUND_TRIP_WALLET_ID, &p)
                .expect("add");
        }
        let id = Identifier::from([1u8; 32]);

        // Add a DPNS name on A's managed identity (persists internally).
        info_a
            .identity_manager
            .managed_identity_mut(&id)
            .expect("a managed")
            .add_dpns_name(
                DpnsNameInfo {
                    label: "alice".into(),
                    acquired_at: Some(123_456),
                },
                &p,
            );

        // Build the replay changeset from A's mutated state.
        let managed = info_a.identity_manager.managed_identity(&id).expect("a");
        let mut id_cs = IdentityChangeSet::default();
        id_cs
            .identities
            .insert(id, IdentityEntry::from_managed(managed));

        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply dpns");

        let a = info_a.identity_manager.managed_identity(&id).expect("a");
        let b = info_b.identity_manager.managed_identity(&id).expect("b");
        assert_eq!(a.dpns_names.len(), b.dpns_names.len());
        assert_eq!(
            a.dpns_names.first().map(|n| n.label.as_str()),
            Some("alice")
        );
        assert_eq!(
            b.dpns_names.first().map(|n| n.label.as_str()),
            Some("alice")
        );
    }

    #[test]
    fn round_trip_block_time_updates() {
        use crate::wallet::identity::state::managed_identity::BlockTime;

        let wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);
        let p = noop_persister();

        for info in [&mut info_a, &mut info_b] {
            info.identity_manager
                .add_identity(make_test_identity(1, 1), 0, ROUND_TRIP_WALLET_ID, &p)
                .expect("add");
        }
        let id = Identifier::from([1u8; 32]);
        let bt = BlockTime::new(100, 200, 1_700_000_000);

        // Update both timestamps on A (persists internally via noop persister).
        info_a
            .identity_manager
            .managed_identity_mut(&id)
            .expect("a")
            .update_balance_block_time(bt, &p);
        info_a
            .identity_manager
            .managed_identity_mut(&id)
            .expect("a")
            .update_keys_sync_block_time(bt, &p);

        // Build a single replay changeset from A's final state (both block times set).
        let managed = info_a.identity_manager.managed_identity(&id).expect("a");
        let mut id_cs = IdentityChangeSet::default();
        id_cs
            .identities
            .insert(id, IdentityEntry::from_managed(managed));

        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply block times");

        let a = info_a.identity_manager.managed_identity(&id).expect("a");
        let b = info_b.identity_manager.managed_identity(&id).expect("b");
        assert_eq!(
            a.last_updated_balance_block_time,
            b.last_updated_balance_block_time
        );
        assert_eq!(a.last_synced_keys_block_time, b.last_synced_keys_block_time);
        assert_eq!(b.last_updated_balance_block_time, Some(bt));
    }

    #[test]
    fn highest_registration_index_advances_on_add() {
        // Replaces the old `last_scanned_index` watermark test —
        // gap-limit scan resume is now derived from the highest
        // already-registered slot in the wallet bucket. See
        // `IdentityManager::highest_registration_index`.
        let wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let p = noop_persister();

        assert_eq!(
            info_a
                .identity_manager
                .highest_registration_index(&ROUND_TRIP_WALLET_ID),
            None
        );
        info_a
            .identity_manager
            .add_identity(make_test_identity(1, 1), 0, ROUND_TRIP_WALLET_ID, &p)
            .expect("add 0");
        info_a
            .identity_manager
            .add_identity(make_test_identity(2, 1), 5, ROUND_TRIP_WALLET_ID, &p)
            .expect("add 5");

        assert_eq!(
            info_a
                .identity_manager
                .highest_registration_index(&ROUND_TRIP_WALLET_ID),
            Some(5)
        );
    }

    #[test]
    fn round_trip_sent_contact_request_no_auto_establish() {
        let wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);
        let p = noop_persister();

        // Both wallets host the same owning identity so contact routing works.
        for info in [&mut info_a, &mut info_b] {
            info.identity_manager
                .add_identity(make_test_identity(1, 1), 0, ROUND_TRIP_WALLET_ID, &p)
                .expect("add owner");
        }
        let owner = Identifier::from([1u8; 32]);
        let recipient = Identifier::from([2u8; 32]);

        // Send a contact request on A (persists internally via noop persister).
        info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a owner")
            .add_sent_contact_request(make_test_contact_request(1, 2), &p)
            .expect("setup persists");

        // Build the replay changeset from A's state: the request ended up in
        // `sent_contact_requests` (no auto-establishment because there was no
        // matching incoming request).
        let request = make_test_contact_request(1, 2);
        let mut contact_cs = ContactChangeSet::default();
        contact_cs.sent_requests.insert(
            SentContactRequestKey {
                owner_id: owner,
                recipient_id: recipient,
            },
            ContactRequestEntry { request },
        );
        // Plain insert path — no matching incoming, so `established` is empty.
        assert!(contact_cs.established.is_empty());

        info_b
            .apply_changeset(&mut wallet_b, wrap_contacts(contact_cs))
            .expect("apply");

        let b_owner = info_b
            .identity_manager
            .managed_identity(&owner)
            .expect("b owner");
        assert!(b_owner
            .dashpay()
            .sent_contact_requests()
            .contains_key(&recipient));
        assert!(b_owner.dashpay().incoming_contact_requests().is_empty());
        assert!(b_owner.dashpay().established_contacts().is_empty());
    }

    #[test]
    fn round_trip_auto_establish_contact() {
        let wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);
        let p = noop_persister();

        for info in [&mut info_a, &mut info_b] {
            info.identity_manager
                .add_identity(make_test_identity(1, 1), 0, ROUND_TRIP_WALLET_ID, &p)
                .expect("add owner");
        }
        let owner = Identifier::from([1u8; 32]);
        let other = Identifier::from([2u8; 32]);

        // First the incoming arrives. Then the outgoing arrives — at which
        // point auto-establishment fires.
        info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a owner")
            .add_incoming_contact_request(make_test_contact_request(2, 1), &p)
            .expect("setup persists");

        // Snapshot the incoming request changeset for B replay step 1.
        let incoming_req = make_test_contact_request(2, 1);
        let mut cs_in = ContactChangeSet::default();
        cs_in.incoming_requests.insert(
            ReceivedContactRequestKey {
                owner_id: owner,
                sender_id: other,
            },
            ContactRequestEntry {
                request: incoming_req,
            },
        );

        // Now the outgoing arrives — auto-establishment fires in A.
        info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a owner")
            .add_sent_contact_request(make_test_contact_request(1, 2), &p)
            .expect("setup persists");

        // After auto-establishment: A's established_contacts should have `other`.
        let a_established = info_a
            .identity_manager
            .managed_identity(&owner)
            .expect("a owner")
            .dashpay()
            .established_contacts()
            .get(&other)
            .cloned()
            .expect("established in A");

        // Build the auto-establish changeset for B replay step 2.
        let mut cs_out = ContactChangeSet::default();
        cs_out.established.insert(
            SentContactRequestKey {
                owner_id: owner,
                recipient_id: other,
            },
            a_established,
        );
        // auto-establish path: sent_requests is empty (contact goes straight to established).
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
        assert!(a_owner
            .dashpay()
            .established_contacts()
            .contains_key(&other));
        assert!(b_owner
            .dashpay()
            .established_contacts()
            .contains_key(&other));
        assert!(a_owner.dashpay().sent_contact_requests().is_empty());
        assert!(b_owner.dashpay().sent_contact_requests().is_empty());
        assert!(a_owner.dashpay().incoming_contact_requests().is_empty());
        assert!(b_owner.dashpay().incoming_contact_requests().is_empty());
    }

    #[test]
    fn round_trip_remove_contact_request() {
        let wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);
        let p = noop_persister();

        for info in [&mut info_a, &mut info_b] {
            info.identity_manager
                .add_identity(make_test_identity(1, 1), 0, ROUND_TRIP_WALLET_ID, &p)
                .expect("add owner");
        }
        let owner = Identifier::from([1u8; 32]);
        let recipient = Identifier::from([2u8; 32]);

        // Both A and B start with a sent request — build the insert changeset
        // manually and apply it to both.
        info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a")
            .add_sent_contact_request(make_test_contact_request(1, 2), &p)
            .expect("setup persists");
        let mut insert_cs = ContactChangeSet::default();
        insert_cs.sent_requests.insert(
            SentContactRequestKey {
                owner_id: owner,
                recipient_id: recipient,
            },
            ContactRequestEntry {
                request: make_test_contact_request(1, 2),
            },
        );
        info_b
            .apply_changeset(&mut wallet_b, wrap_contacts(insert_cs))
            .expect("apply insert");

        // Now remove on A (returns changeset directly — `remove_sent_contact_request`
        // still returns a `ContactChangeSet` because it has no `persister` param).
        let (_, remove_cs) = info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a")
            .remove_sent_contact_request(&recipient);
        assert!(remove_cs.removed_sent.contains(&SentContactRequestKey {
            owner_id: owner,
            recipient_id: recipient
        }));

        info_b
            .apply_changeset(&mut wallet_b, wrap_contacts(remove_cs))
            .expect("apply remove");

        let a_owner = info_a.identity_manager.managed_identity(&owner).expect("a");
        let b_owner = info_b.identity_manager.managed_identity(&owner).expect("b");
        assert!(a_owner.dashpay().sent_contact_requests().is_empty());
        assert!(b_owner.dashpay().sent_contact_requests().is_empty());
    }

    // ----------------------------------------------------------------------
    // Reviewer test gaps (Phase 9a-3 followup)
    // ----------------------------------------------------------------------

    /// Reviewer #6a: removing the sole identity drops it cleanly.
    /// (Old test name referenced "primary" — primary selection no
    /// longer lives on the manager; the bucket just empties.)
    #[test]
    fn apply_remove_sole_identity_empties_bucket() {
        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        let id = Identifier::from([1u8; 32]);
        let mut id_cs = IdentityChangeSet::default();
        let mut managed = ManagedIdentity::new(make_test_identity(1, 0), 0);
        managed.wallet_id = Some([3u8; 32]);
        id_cs
            .identities
            .insert(id, IdentityEntry::from_managed(&managed));
        info.apply_changeset(&mut wallet, wrap_id(id_cs))
            .expect("apply insert");
        assert!(info.identity_manager.identity(&id).is_some());

        let mut id_cs = IdentityChangeSet::default();
        id_cs.removed.insert(id);
        info.apply_changeset(&mut wallet, wrap_id(id_cs))
            .expect("apply remove");

        assert_eq!(info.identity_manager.identity_count(), 0);
        assert!(info.identity_manager.is_empty());
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
        let mut m_a = ManagedIdentity::new(make_test_identity(1, 0), 0);
        m_a.wallet_id = Some([3u8; 32]);
        let mut m_b = ManagedIdentity::new(make_test_identity(2, 0), 1);
        m_b.wallet_id = Some([3u8; 32]);
        id_cs
            .identities
            .insert(id_a, IdentityEntry::from_managed(&m_a));
        id_cs
            .identities
            .insert(id_b, IdentityEntry::from_managed(&m_b));
        info.apply_changeset(&mut wallet, wrap_id(id_cs))
            .expect("apply insert");

        let mut tombstone = IdentityChangeSet::default();
        tombstone.removed.insert(id_a);
        let cs = wrap_id(tombstone);

        info.apply_changeset(&mut wallet, cs.clone())
            .expect("first remove");
        info.apply_changeset(&mut wallet, cs)
            .expect("second remove (no-op)");

        assert_eq!(info.identity_manager.identity_count(), 1);
        assert!(info.identity_manager.managed_identity(&id_a).is_none());
        assert!(info.identity_manager.managed_identity(&id_b).is_some());
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
        high_cs
            .identities
            .insert(id, IdentityEntry::from_managed(&high));
        info.apply_changeset(&mut wallet, wrap_id(high_cs))
            .expect("seed");

        // Stale revision 2 entry.
        use dpp::identity::accessors::IdentityGettersV0;
        let stale = ManagedIdentity::new(make_test_identity(1, 2), 0);
        let mut stale_cs = IdentityChangeSet::default();
        stale_cs
            .identities
            .insert(id, IdentityEntry::from_managed(&stale));
        info.apply_changeset(&mut wallet, wrap_id(stale_cs))
            .expect("stale apply");

        // The on-chain blob must still carry revision 5.
        let restored = info
            .identity_manager
            .managed_identity(&id)
            .expect("present");
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
        managed.apply_sent_contact_request(make_test_contact_request(1, 2));
        let mut id_cs = IdentityChangeSet::default();
        id_cs
            .identities
            .insert(owner, IdentityEntry::from_managed(&managed));
        info.apply_changeset(&mut wallet, wrap_id(id_cs))
            .expect("seed identity");

        // Apply a tombstone for that pair.
        let mut contact_cs = ContactChangeSet::default();
        contact_cs.removed_sent.insert(SentContactRequestKey {
            owner_id: owner,
            recipient_id: other,
        });
        info.apply_changeset(&mut wallet, wrap_contacts(contact_cs))
            .expect("apply tombstone");

        let restored = info
            .identity_manager
            .managed_identity(&owner)
            .expect("present");
        assert!(!restored
            .dashpay()
            .sent_contact_requests()
            .contains_key(&other));
    }

    #[test]
    fn round_trip_set_dashpay_profile() {
        use crate::wallet::identity::DashPayProfile;

        let wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);
        let p = noop_persister();

        for info in [&mut info_a, &mut info_b] {
            info.identity_manager
                .add_identity(make_test_identity(1, 1), 0, ROUND_TRIP_WALLET_ID, &p)
                .expect("add");
        }
        let id = Identifier::from([1u8; 32]);

        let profile = DashPayProfile {
            display_name: Some("alice".into()),
            bio: Some("test bio".into()),
            avatar_url: Some("https://example.com/avatar.png".into()),
            avatar_hash: Some([0xaa; 32]),
            avatar_fingerprint: Some([0xbb; 8]),
            public_message: Some("hello world".into()),
        };

        // Mutate A (persists internally via noop persister).
        info_a
            .identity_manager
            .managed_identity_mut(&id)
            .expect("a managed")
            .set_dashpay_profile(Some(profile.clone()), &p);

        // Build the replay changeset from A's mutated state.
        let managed = info_a.identity_manager.managed_identity(&id).expect("a");
        let mut id_cs = IdentityChangeSet::default();
        id_cs
            .identities
            .insert(id, IdentityEntry::from_managed(managed));

        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply");

        let a = info_a.identity_manager.managed_identity(&id).expect("a");
        let b = info_b.identity_manager.managed_identity(&id).expect("b");
        assert_eq!(a.dashpay().profile, b.dashpay().profile);
        assert_eq!(b.dashpay().profile, Some(profile));
    }

    #[test]
    fn round_trip_record_dashpay_payment() {
        use crate::wallet::identity::PaymentEntry;

        let wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);
        let p = noop_persister();

        for info in [&mut info_a, &mut info_b] {
            info.identity_manager
                .add_identity(make_test_identity(1, 1), 0, ROUND_TRIP_WALLET_ID, &p)
                .expect("add");
        }
        let owner = Identifier::from([1u8; 32]);
        let counterparty = Identifier::from([2u8; 32]);

        // Record a sent payment on A (persists internally via noop persister).
        let tx_id = "a".repeat(64);
        let payment = PaymentEntry::new_sent(counterparty, 12_000, Some("lunch".into()));
        info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a managed")
            .record_dashpay_payment(tx_id.clone(), payment.clone(), &p)
            .expect("record");

        // Build the replay changeset from A's mutated state.
        let managed = info_a.identity_manager.managed_identity(&owner).expect("a");
        let mut id_cs = IdentityChangeSet::default();
        id_cs
            .identities
            .insert(owner, IdentityEntry::from_managed(managed));

        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply");

        // B now carries the same payment entry on the same owner.
        let b_managed = info_b
            .identity_manager
            .managed_identity(&owner)
            .expect("b managed");
        assert_eq!(b_managed.dashpay().payments.get(&tx_id), Some(&payment));
    }

    #[test]
    fn round_trip_payment_status_update_overwrites() {
        use crate::wallet::identity::{PaymentEntry, PaymentStatus};

        let wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);
        let p = noop_persister();

        for info in [&mut info_a, &mut info_b] {
            info.identity_manager
                .add_identity(make_test_identity(1, 1), 0, ROUND_TRIP_WALLET_ID, &p)
                .expect("add");
        }
        let owner = Identifier::from([1u8; 32]);
        let counterparty = Identifier::from([2u8; 32]);
        let tx_id = "b".repeat(64);

        // Initial pending entry.
        let pending = PaymentEntry::new_sent(counterparty, 9_000, None);
        info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a managed")
            .record_dashpay_payment(tx_id.clone(), pending, &p)
            .expect("record");
        let managed = info_a.identity_manager.managed_identity(&owner).expect("a");
        let mut id_cs = IdentityChangeSet::default();
        id_cs
            .identities
            .insert(owner, IdentityEntry::from_managed(managed));
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply pending");

        // Overwrite with a confirmed entry under the same tx_id.
        let mut confirmed = PaymentEntry::new_sent(counterparty, 9_000, None);
        confirmed.status = PaymentStatus::Confirmed;
        info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a managed")
            .record_dashpay_payment(tx_id.clone(), confirmed.clone(), &p)
            .expect("record");
        let managed = info_a.identity_manager.managed_identity(&owner).expect("a");
        let mut id_cs = IdentityChangeSet::default();
        id_cs
            .identities
            .insert(owner, IdentityEntry::from_managed(managed));
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply confirmed");

        // B shows the confirmed status.
        let b_managed = info_b
            .identity_manager
            .managed_identity(&owner)
            .expect("b managed");
        assert_eq!(
            b_managed.dashpay().payments.get(&tx_id).map(|p| p.status),
            Some(PaymentStatus::Confirmed)
        );
    }

    /// A cached contact profile round-trips through the changeset
    /// (snapshot → apply), and a later update overwrites it (full-replace,
    /// last-write-wins per contact id) — so contact names/avatars survive
    /// relaunch instead of vanishing.
    #[test]
    fn round_trip_contact_profile_persists_and_overwrites() {
        use crate::wallet::identity::{ContactProfileEntry, DashPayProfile};

        let wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);
        let p = noop_persister();

        for info in [&mut info_a, &mut info_b] {
            info.identity_manager
                .add_identity(make_test_identity(1, 1), 0, ROUND_TRIP_WALLET_ID, &p)
                .expect("add");
        }
        let owner = Identifier::from([1u8; 32]);
        let contact = Identifier::from([2u8; 32]);

        // Cache a contact profile on A, snapshot, apply to B.
        info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a managed")
            .dashpay_contact_profiles_mut()
            .insert(
                contact,
                ContactProfileEntry {
                    profile: Some(DashPayProfile {
                        display_name: Some("Bob".into()),
                        avatar_url: Some("https://x/b.png".into()),
                        ..Default::default()
                    }),
                    checked_at_ms: 100,
                },
            );
        let managed = info_a.identity_manager.managed_identity(&owner).expect("a");
        let mut id_cs = IdentityChangeSet::default();
        id_cs
            .identities
            .insert(owner, IdentityEntry::from_managed(managed));
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply profile");

        let b_managed = info_b.identity_manager.managed_identity(&owner).expect("b");
        assert_eq!(
            b_managed
                .dashpay()
                .contact_profiles
                .get(&contact)
                .and_then(|e| e.profile.as_ref())
                .and_then(|pr| pr.display_name.as_deref()),
            Some("Bob"),
            "contact profile must survive the changeset round-trip"
        );

        // Contact updated their profile (removed the avatar) → overwrite.
        info_a
            .identity_manager
            .managed_identity_mut(&owner)
            .expect("a managed")
            .dashpay_contact_profiles_mut()
            .insert(
                contact,
                ContactProfileEntry {
                    profile: Some(DashPayProfile {
                        display_name: Some("Bob".into()),
                        avatar_url: None,
                        ..Default::default()
                    }),
                    checked_at_ms: 200,
                },
            );
        let managed = info_a.identity_manager.managed_identity(&owner).expect("a");
        let mut id_cs = IdentityChangeSet::default();
        id_cs
            .identities
            .insert(owner, IdentityEntry::from_managed(managed));
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply updated profile");

        let b_managed = info_b.identity_manager.managed_identity(&owner).expect("b");
        assert_eq!(
            b_managed
                .dashpay()
                .contact_profiles
                .get(&contact)
                .and_then(|e| e.profile.as_ref())
                .and_then(|pr| pr.avatar_url.clone()),
            None,
            "a removed avatar must be cleared on the apply side (full-replace)"
        );
    }

    #[test]
    fn round_trip_clear_dashpay_profile() {
        use crate::wallet::identity::DashPayProfile;

        let wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);
        let p = noop_persister();

        for info in [&mut info_a, &mut info_b] {
            info.identity_manager
                .add_identity(make_test_identity(1, 1), 0, ROUND_TRIP_WALLET_ID, &p)
                .expect("add");
            // Pre-seed with a profile so the clear has something to do.
            info.identity_manager
                .managed_identity_mut(&Identifier::from([1u8; 32]))
                .expect("managed")
                .set_dashpay_profile(
                    Some(DashPayProfile {
                        display_name: Some("seed".into()),
                        ..Default::default()
                    }),
                    &p,
                );
        }
        let id = Identifier::from([1u8; 32]);

        // Clear on A (persists internally via noop persister).
        info_a
            .identity_manager
            .managed_identity_mut(&id)
            .expect("a managed")
            .set_dashpay_profile(None, &p);

        // Build the replay changeset from A's mutated state.
        let managed = info_a.identity_manager.managed_identity(&id).expect("a");
        let mut id_cs = IdentityChangeSet::default();
        id_cs
            .identities
            .insert(id, IdentityEntry::from_managed(managed));

        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("apply");

        // Both wallets should have profile == None.
        assert_eq!(
            info_a
                .identity_manager
                .managed_identity(&id)
                .expect("a")
                .dashpay()
                .profile,
            None
        );
        assert_eq!(
            info_b
                .identity_manager
                .managed_identity(&id)
                .expect("b")
                .dashpay()
                .profile,
            None
        );
    }

    #[test]
    fn round_trip_double_apply_is_idempotent() {
        let wallet_a = build_test_wallet();
        let mut info_a = empty_info(&wallet_a);
        let mut wallet_b = build_test_wallet();
        let mut info_b = empty_info(&wallet_b);
        let p = noop_persister();

        // Single mutation: add a wallet-owned identity at a specific
        // index — covers the bucket-key denormalization on the value.
        info_a
            .identity_manager
            .add_identity(make_test_identity(1, 1), 0, ROUND_TRIP_WALLET_ID, &p)
            .expect("add");

        // Build the changeset from A's final state.
        let id = Identifier::from([1u8; 32]);
        let managed = info_a.identity_manager.managed_identity(&id).expect("a");
        let mut id_cs = IdentityChangeSet::default();
        id_cs
            .identities
            .insert(id, IdentityEntry::from_managed(managed));

        // Apply the changeset twice on B and verify state matches A.
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs.clone()))
            .expect("first apply");
        info_b
            .apply_changeset(&mut wallet_b, wrap_id(id_cs))
            .expect("second apply (idempotent)");

        let a = info_a.identity_manager.managed_identity(&id).expect("a");
        let b = info_b.identity_manager.managed_identity(&id).expect("b");
        assert_eq!(a.identity_index, b.identity_index);
        assert_eq!(a.wallet_id, b.wallet_id);
        assert_eq!(info_a.identity_manager.identity_count(), 1);
        assert_eq!(info_b.identity_manager.identity_count(), 1);
    }

    #[test]
    fn apply_double_apply_full_changeset_is_idempotent() {
        use key_wallet::bip32::{ChildNumber, DerivationPath};
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType};
        use key_wallet::managed_account::managed_platform_account::ManagedPlatformAccount;
        use key_wallet::PlatformP2PKHAddress;

        let mut wallet = build_test_wallet();
        let mut info = empty_info(&wallet);

        // Set up a platform payment account.
        let base_path = DerivationPath::from(vec![
            ChildNumber::from_hardened_idx(9).unwrap(),
            ChildNumber::from_hardened_idx(1).unwrap(),
            ChildNumber::from_hardened_idx(17).unwrap(),
            ChildNumber::from_hardened_idx(0).unwrap(),
            ChildNumber::from_hardened_idx(0).unwrap(),
        ]);
        let pool = AddressPool::new_without_generation(
            base_path,
            AddressPoolType::Absent,
            20,
            Network::Testnet,
        );
        let platform_account = ManagedPlatformAccount::new(0, 0, pool, false);
        info.core_wallet
            .accounts
            .insert_platform_account(platform_account);

        let identity = Identifier::from([3u8; 32]);
        let mut managed = ManagedIdentity::new(make_test_identity(3, 0), 0);
        managed.wallet_id = Some([4u8; 32]);

        let mut id_cs = IdentityChangeSet::default();
        id_cs
            .identities
            .insert(identity, IdentityEntry::from_managed(&managed));

        let addr = PlatformP2PKHAddress::new([42u8; 20]);
        let mut addr_cs = PlatformAddressChangeSet::default();
        addr_cs.addresses.push(crate::PlatformAddressBalanceEntry {
            wallet_id: [0u8; 32],
            account_index: 0,
            address_index: 0,
            address: addr,
            funds: dash_sdk::platform::address_sync::AddressFunds {
                balance: 1_000,
                nonce: 0,
                as_of_height: 0,
            },
        });

        // Token balance changesets are accepted for shape compat but
        // no longer drive `PlatformWalletInfo` state — the manager
        // owns the balance cache. Include one anyway to confirm the
        // double-apply still works once the field has been replaced
        // with a `drop`.
        let mut tok_cs = TokenBalanceChangeSet::default();
        let token = Identifier::from([8u8; 32]);
        tok_cs.balances.insert((identity, token), 42);

        let cs = PlatformWalletChangeSet {
            identities: Some(id_cs),
            platform_addresses: Some(addr_cs),
            token_balances: Some(tok_cs),
            ..Default::default()
        };

        info.apply_changeset(&mut wallet, cs.clone())
            .expect("first apply");
        info.apply_changeset(&mut wallet, cs).expect("second apply");

        assert_eq!(info.identity_manager.identity_count(), 1);
        let account = info
            .core_wallet
            .first_platform_payment_managed_account()
            .unwrap();
        assert_eq!(
            account.address_credit_balance(&PlatformP2PKHAddress::new([42u8; 20])),
            1_000
        );
    }
}
