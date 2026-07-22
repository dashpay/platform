//! Replay path — apply persisted
//! [`IdentityEntry`](crate::changeset::IdentityEntry) rows back onto
//! an in-memory [`IdentityManager`].
//!
//! The mutation methods elsewhere emit changeset entries as they
//! modify state; on restart, the persisted entries are replayed by
//! calling [`IdentityManager::apply_identity_entry`] for each one.
//!
//! Contact state is deliberately NOT touched here — it lives in
//! [`ContactChangeSet`](crate::changeset::ContactChangeSet) and is
//! applied by a separate code path. Identity keys (public keys +
//! private-key storage) are likewise NOT touched — they live in
//! [`IdentityKeysChangeSet`](crate::changeset::IdentityKeysChangeSet)
//! and are layered in by
//! [`IdentityManager::apply_identity_key_entry`].

use super::{IdentityLocation, IdentityManager};
use crate::changeset::{IdentityEntry, IdentityKeyEntry};
use crate::wallet::identity::state::managed_identity::ManagedIdentity;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, KeyID};
use dpp::prelude::Identifier;
use key_wallet::Network;
use std::collections::BTreeMap;

impl IdentityManager {
    /// Restore a single [`IdentityEntry`] into the manager.
    ///
    /// The entry's `wallet_id` decides which bucket it lands in:
    ///
    /// - `Some(wallet_id)` → `wallet_identities[wallet_id][identity_index]`,
    ///   denormalizing both keys onto the value.
    /// - `None` → `out_of_wallet_identities[identity_id]`.
    ///
    /// Idempotent: applying the same entry twice produces the same state
    /// as applying it once. If the identity already exists in either
    /// bucket, the scalar fields are updated in place; balance/revision
    /// are gated on `entry.revision >= existing.identity.revision()`
    /// matching the merge policy on `IdentityChangeSet`. Contested DPNS
    /// labels are a complete canonical snapshot and are assigned wholesale.
    pub(crate) fn apply_identity_entry(&mut self, entry: IdentityEntry) {
        use dpp::identity::accessors::IdentitySettersV0;

        let id = entry.id;

        // Updating an existing identity — find it across both buckets.
        if let Some(existing) = self.locate_mut(&id) {
            if entry.revision >= existing.identity.revision() {
                existing.identity.set_balance(entry.balance);
                existing.identity.set_revision(entry.revision);
            }
            existing.last_updated_balance_block_time = entry.last_updated_balance_block_time;
            existing.last_synced_keys_block_time = entry.last_synced_keys_block_time;
            existing.status = entry.status;
            *existing.dashpay_profile_mut() = entry.dashpay_profile;
            for name in entry.dpns_names {
                if !existing.dpns_names.iter().any(|n| n.label == name.label) {
                    existing.dpns_names.push(name);
                }
            }
            existing.contested_dpns_names = entry.contested_dpns_names;
            existing
                .dashpay_payments_mut()
                .extend(entry.dashpay_payments);
            existing
                .dashpay_contact_profiles_mut()
                .extend(entry.contact_profiles);
            // Ignored senders: union (un-ignore is carried by an explicit
            // `ContactChangeSet::unignored` removal, applied separately).
            for sender in entry.ignored_senders {
                existing.apply_ignored_sender(sender);
            }
            return;
        }

        // Fresh insert — reconstruct a DPP `Identity` from scalars and
        // wrap in a `ManagedIdentity`. Public keys arrive separately
        // via `apply_identity_key_entry`. Bucket placement is decided by
        // the entry's `wallet_id`; the corresponding `identity_index`
        // (Some/None) on the managed value is set by the constructor
        // we pick.
        let identity = Identity::V0(IdentityV0 {
            id: entry.id,
            public_keys: BTreeMap::new(),
            balance: entry.balance,
            revision: entry.revision,
        });

        match entry.wallet_id {
            Some(wallet_id) => {
                // Wallet-owned entries must carry a registration index;
                // the manager's invariant is that every entry in
                // `wallet_identities[wallet_id]` has a real bucket key.
                // A missing index here is a persistence bug — log and
                // skip rather than silently coercing to `0` (which would
                // collide with a real index-0 identity).
                let Some(registration_index) = entry.identity_index else {
                    tracing::warn!(
                        identity = %id,
                        wallet_id = %hex::encode(wallet_id),
                        "skipping wallet-owned identity entry with no identity_index during apply",
                    );
                    return;
                };

                let mut managed = ManagedIdentity::new(identity, registration_index);
                managed.last_updated_balance_block_time = entry.last_updated_balance_block_time;
                managed.last_synced_keys_block_time = entry.last_synced_keys_block_time;
                managed.status = entry.status;
                managed.wallet_id = Some(wallet_id);
                managed.dpns_names = entry.dpns_names;
                managed.contested_dpns_names = entry.contested_dpns_names;
                *managed.dashpay_profile_mut() = entry.dashpay_profile;
                *managed.dashpay_payments_mut() = entry.dashpay_payments;
                *managed.dashpay_contact_profiles_mut() = entry.contact_profiles;
                // Fresh-constructed identity: the ignored set starts empty,
                // so per-element apply reproduces the wholesale assign.
                for sender in entry.ignored_senders {
                    managed.apply_ignored_sender(sender);
                }

                self.wallet_identities
                    .entry(wallet_id)
                    .or_default()
                    .insert(registration_index, managed);
                // Lockstep with the bucket insert above so the
                // side-index invariant holds across the replay path.
                self.location_index_insert(
                    id,
                    IdentityLocation::InWallet {
                        wallet_id,
                        registration_index,
                    },
                );
            }
            None => {
                let mut managed = ManagedIdentity::new_out_of_wallet(identity);
                managed.last_updated_balance_block_time = entry.last_updated_balance_block_time;
                managed.last_synced_keys_block_time = entry.last_synced_keys_block_time;
                managed.status = entry.status;
                // `wallet_id` stays None — `new_out_of_wallet` already
                // initialized it that way.
                managed.dpns_names = entry.dpns_names;
                managed.contested_dpns_names = entry.contested_dpns_names;
                *managed.dashpay_profile_mut() = entry.dashpay_profile;
                *managed.dashpay_payments_mut() = entry.dashpay_payments;
                *managed.dashpay_contact_profiles_mut() = entry.contact_profiles;
                // Fresh-constructed identity: the ignored set starts empty,
                // so per-element apply reproduces the wholesale assign.
                for sender in entry.ignored_senders {
                    managed.apply_ignored_sender(sender);
                }

                self.out_of_wallet_identities.insert(id, managed);
                self.location_index_insert(id, IdentityLocation::OutOfWallet);
            }
        }
    }

    /// Restore a single [`IdentityKeyEntry`] onto the matching
    /// `ManagedIdentity`.
    ///
    /// Layers the public-key record into the DPP `Identity`'s
    /// `public_keys` map (overwriting any existing slot with the same
    /// `KeyID`). Private-key data is no longer kept on
    /// `ManagedIdentity` (it lives in the iOS Keychain on the client
    /// side); the `(wallet_id, derivation_indices)` breadcrumb on the
    /// entry tells the client how to re-derive the scalar.
    ///
    /// If the owning identity isn't present in the manager (e.g. the
    /// keys changeset was persisted without its scalar sibling, or the
    /// owner was removed since), the entry is logged and skipped.
    pub(crate) fn apply_identity_key_entry(&mut self, entry: IdentityKeyEntry, _network: Network) {
        // `add_public_key` lives on `IdentityFactory` / V0 setter trait;
        // bring it into scope here.
        use dpp::identity::accessors::IdentitySettersV0;

        if let Some(managed) = self.locate_mut(&entry.identity_id) {
            // Insert into the DPP `Identity`'s `public_keys` map by id;
            // replay-safe (idempotent overwrite).
            let mut keys = managed.identity.public_keys().clone();
            keys.insert(entry.public_key.id(), entry.public_key.clone());
            managed.identity.set_public_keys(keys);
        } else {
            tracing::warn!(
                identity = %entry.identity_id,
                key_id = entry.key_id,
                "skipping identity key entry during apply: identity not in wallet",
            );
        }
    }

    /// Remove a single identity-key slot on the matching
    /// `ManagedIdentity`. Drops the public-key map entry. Silent if the
    /// owner isn't present or the key id isn't known.
    pub(crate) fn apply_identity_key_removal(&mut self, identity_id: &Identifier, key_id: KeyID) {
        if let Some(managed) = self.locate_mut(identity_id) {
            managed.identity.public_keys_mut().remove(&key_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changeset::IdentityEntry;
    use crate::wallet::identity::state::managed_identity::IdentityStatus;
    use std::collections::{BTreeMap, BTreeSet};

    fn entry(id: Identifier, labels: &[&str]) -> IdentityEntry {
        IdentityEntry {
            id,
            balance: 0,
            revision: 0,
            identity_index: None,
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            dpns_names: Vec::new(),
            contested_dpns_names: labels.iter().map(|label| (*label).to_owned()).collect(),
            status: IdentityStatus::Unknown,
            wallet_id: None,
            dashpay_profile: None,
            dashpay_payments: BTreeMap::new(),
            contact_profiles: BTreeMap::new(),
            ignored_senders: BTreeSet::new(),
        }
    }

    #[test]
    fn contested_dpns_apply_replaces_canonical_snapshot_and_allows_empty() {
        let id = Identifier::from([0x52; 32]);
        let mut manager = IdentityManager::default();
        manager.apply_identity_entry(entry(id, &["old", "retained"]));
        manager.apply_identity_entry(entry(id, &["retained", "new"]));
        assert_eq!(
            manager
                .locate_mut(&id)
                .expect("identity must exist")
                .contested_dpns_names,
            ["retained", "new"]
        );

        manager.apply_identity_entry(entry(id, &[]));
        assert!(manager
            .locate_mut(&id)
            .expect("identity must exist")
            .contested_dpns_names
            .is_empty());
    }
}
