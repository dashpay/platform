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
//! applied by a separate code path.

use super::IdentityManager;
use crate::changeset::IdentityEntry;
use crate::wallet::identity::state::managed_identity::ManagedIdentity;

impl IdentityManager {
    /// Restore a single [`IdentityEntry`] into the manager.
    ///
    /// This is the apply-side counterpart to the snapshot-emitting
    /// mutation methods on [`ManagedIdentity`]. It does NOT enforce
    /// invariants the mutation methods do at runtime (DPNS dedup,
    /// auto-establishment, fresh block-time stamps) — replay must be
    /// faithful to whatever the persisted entry carried.
    ///
    /// Contact state (`sent_contact_requests`, `incoming_contact_requests`,
    /// `established_contacts`) is NOT touched here — those live in
    /// [`ContactChangeSet`](crate::changeset::ContactChangeSet) and are
    /// applied separately by the caller.
    ///
    /// # Behaviour
    ///
    /// - If the identity is not yet present, a fresh `ManagedIdentity` is
    ///   constructed and every persistable field is copied from the entry.
    /// - If the identity is already present, persistable fields are
    ///   updated in place. The on-chain `Identity` blob is only replaced
    ///   when `entry.identity.revision() >= existing.identity.revision()`,
    ///   matching the merge policy on `IdentityChangeSet`.
    ///
    /// Idempotent: applying the same entry twice is the same as applying
    /// it once.
    pub(crate) fn apply_identity_entry(&mut self, entry: IdentityEntry) {
        use dpp::identity::accessors::IdentityGettersV0;
        let id = entry.identity.id();
        match self.identities.get_mut(&id) {
            Some(existing) => {
                if entry.identity.revision() >= existing.identity.revision() {
                    existing.identity = entry.identity;
                }
                // identity_index is immutable per identity (it's the
                // BIP-9 HD derivation index used during registration),
                // so we don't overwrite it here. This matches
                // `IdentityChangeSet::merge`'s policy.
                existing.label = entry.label;
                existing.last_updated_balance_block_time = entry.last_updated_balance_block_time;
                existing.last_synced_keys_block_time = entry.last_synced_keys_block_time;
                existing.status = entry.status;
                existing.wallet_id = entry.wallet_id;
                existing.dashpay_profile = entry.dashpay_profile;
                // Append new DPNS names by label, preserving any
                // pre-existing entries the changeset didn't carry.
                for name in entry.dpns_names {
                    if !existing.dpns_names.iter().any(|n| n.label == name.label) {
                        existing.dpns_names.push(name);
                    }
                }
                existing.key_storage.extend(entry.key_storage);
                existing.dashpay_payments.extend(entry.dashpay_payments);
            }
            None => {
                let mut managed = ManagedIdentity::new(entry.identity, entry.identity_index);
                managed.label = entry.label;
                managed.last_updated_balance_block_time = entry.last_updated_balance_block_time;
                managed.last_synced_keys_block_time = entry.last_synced_keys_block_time;
                managed.status = entry.status;
                managed.wallet_id = entry.wallet_id;
                managed.dpns_names = entry.dpns_names;
                managed.key_storage = entry.key_storage;
                managed.dashpay_profile = entry.dashpay_profile;
                managed.dashpay_payments = entry.dashpay_payments;
                self.identities.insert(id, managed);
                if self.primary_identity_id.is_none() {
                    self.primary_identity_id = Some(id);
                }
            }
        }
    }
}
