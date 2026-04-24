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

use super::IdentityManager;
use crate::changeset::{IdentityEntry, IdentityKeyEntry};
use crate::wallet::identity::state::managed_identity::{ManagedIdentity, PrivateKeyData};
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, KeyID};
use dpp::prelude::Identifier;
use key_wallet::bip32::{ChildNumber, DerivationPath, KeyDerivationType};
use key_wallet::dip9::{
    IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
};
use key_wallet::Network;
use std::collections::BTreeMap;

/// Rebuild the `m/9'/coin'/5'/0'/ECDSA'/identity_index'/key_index'`
/// DIP-9 identity authentication path for the given network + indices.
/// Factored out so both the apply path and the FFI layer can speak the
/// same derivation shape.
fn identity_auth_derivation_path(
    network: Network,
    identity_index: u32,
    key_index: u32,
) -> Option<DerivationPath> {
    let base: DerivationPath = match network {
        Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
        _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
    }
    .into();
    let key_type_index: u32 = KeyDerivationType::ECDSA.into();
    Some(base.extend([
        ChildNumber::from_hardened_idx(key_type_index).ok()?,
        ChildNumber::from_hardened_idx(identity_index).ok()?,
        ChildNumber::from_hardened_idx(key_index).ok()?,
    ]))
}

impl IdentityManager {
    /// Restore a single [`IdentityEntry`] into the manager.
    ///
    /// This is the apply-side counterpart to the scalar snapshot-
    /// emitting mutation methods on [`ManagedIdentity`]. It does NOT
    /// enforce invariants the mutation methods do at runtime (DPNS
    /// dedup, auto-establishment, fresh block-time stamps) — replay
    /// must be faithful to whatever the persisted entry carried.
    ///
    /// Contact state (`sent_contact_requests`, `incoming_contact_requests`,
    /// `established_contacts`) is NOT touched here — those live in
    /// [`ContactChangeSet`](crate::changeset::ContactChangeSet) and are
    /// applied separately by the caller. Identity keys live in
    /// [`IdentityKeysChangeSet`](crate::changeset::IdentityKeysChangeSet)
    /// and are layered in by [`Self::apply_identity_key_entry`].
    ///
    /// # Behaviour
    ///
    /// - If the identity is not yet present, a fresh DPP `Identity`
    ///   is constructed from the entry's scalar fields (empty
    ///   `public_keys` map — keys arrive separately via the
    ///   `IdentityKeysChangeSet` pass) and wrapped in a
    ///   `ManagedIdentity`.
    /// - If the identity is already present, persistable scalar fields
    ///   are updated in place. `balance` / `revision` are only
    ///   replaced when `entry.revision >= existing.identity.revision()`,
    ///   matching the merge policy on `IdentityChangeSet`.
    ///
    /// Idempotent: applying the same entry twice is the same as applying
    /// it once.
    pub(crate) fn apply_identity_entry(&mut self, entry: IdentityEntry) {
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::accessors::IdentitySettersV0;

        let id = entry.id;
        match self.identities.get_mut(&id) {
            Some(existing) => {
                if entry.revision >= existing.identity.revision() {
                    existing.identity.set_balance(entry.balance);
                    existing.identity.set_revision(entry.revision);
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
                // Same dedup-append policy for contested labels.
                // Resolved contests (won / locked) shrink the list
                // via a separate setter that emits a full snapshot.
                for label in entry.contested_dpns_names {
                    if !existing.contested_dpns_names.contains(&label) {
                        existing.contested_dpns_names.push(label);
                    }
                }
                existing.dashpay_payments.extend(entry.dashpay_payments);
            }
            None => {
                // Reconstruct a fresh DPP Identity from the scalar
                // fields. `public_keys` starts empty; any keys
                // belonging to this identity arrive separately via
                // `apply_identity_key_entry`.
                let identity = Identity::V0(IdentityV0 {
                    id: entry.id,
                    public_keys: BTreeMap::new(),
                    balance: entry.balance,
                    revision: entry.revision,
                });
                let mut managed = ManagedIdentity::new(identity, entry.identity_index);
                managed.label = entry.label;
                managed.last_updated_balance_block_time = entry.last_updated_balance_block_time;
                managed.last_synced_keys_block_time = entry.last_synced_keys_block_time;
                managed.status = entry.status;
                managed.wallet_id = entry.wallet_id;
                managed.dpns_names = entry.dpns_names;
                managed.contested_dpns_names = entry.contested_dpns_names;
                managed.dashpay_profile = entry.dashpay_profile;
                managed.dashpay_payments = entry.dashpay_payments;
                self.identities.insert(id, managed);
                if self.primary_identity_id.is_none() {
                    self.primary_identity_id = Some(id);
                }
            }
        }
    }

    /// Restore a single [`IdentityKeyEntry`] onto the matching
    /// `ManagedIdentity`.
    ///
    /// Layers the public-key record into the DPP `Identity`'s
    /// `public_keys` map (overwriting any existing slot with the same
    /// `KeyID`). When the entry carries both a `wallet_id` and
    /// `derivation_indices` the DIP-9 path is rebuilt from `network`
    /// and the matching `key_storage` slot is restored with
    /// `PrivateKeyData::AtWalletDerivationPath` — no private-key
    /// bytes ever cross this boundary, so a signer that needs the
    /// scalar will re-derive from the wallet seed on demand.
    ///
    /// Watch-only entries (no derivation breadcrumb) update the
    /// `public_keys` map but do not touch `key_storage`.
    ///
    /// If the owning identity isn't present in the manager (e.g. the
    /// keys changeset was persisted without its scalar sibling, or the
    /// owner was removed since), the entry is logged and skipped.
    pub(crate) fn apply_identity_key_entry(&mut self, entry: IdentityKeyEntry, network: Network) {
        use dpp::identity::accessors::IdentityGettersV0;

        if let Some(managed) = self.identities.get_mut(&entry.identity_id) {
            // `Identity::add_public_key` inserts by `key.id()` and
            // overwrites any prior slot — replay-safe.
            managed.identity.add_public_key(entry.public_key.clone());

            if let (Some(wallet_id), Some(indices)) = (entry.wallet_id, entry.derivation_indices) {
                if let Some(path) = identity_auth_derivation_path(
                    network,
                    indices.identity_index,
                    indices.key_index,
                ) {
                    managed.key_storage.insert(
                        entry.key_id,
                        (
                            entry.public_key,
                            PrivateKeyData::AtWalletDerivationPath {
                                wallet_id,
                                derivation_path: path,
                                identity_index: indices.identity_index,
                                key_index: indices.key_index,
                            },
                        ),
                    );
                } else {
                    tracing::warn!(
                        identity = %entry.identity_id,
                        key_id = entry.key_id,
                        identity_index = indices.identity_index,
                        key_index = indices.key_index,
                        "failed to rebuild DIP-9 path for identity key during apply; \
                         key recorded as public-only",
                    );
                }
            }
        } else {
            tracing::warn!(
                identity = %entry.identity_id,
                key_id = entry.key_id,
                "skipping identity key entry during apply: identity not in wallet",
            );
        }
    }

    /// Remove a single identity-key slot on the matching
    /// `ManagedIdentity`. Drops both the public-key map entry and any
    /// `key_storage` slot. Silent if the owner isn't present or the
    /// key id isn't known.
    pub(crate) fn apply_identity_key_removal(&mut self, identity_id: &Identifier, key_id: KeyID) {
        use dpp::identity::accessors::IdentityGettersV0;
        if let Some(managed) = self.identities.get_mut(identity_id) {
            managed.identity.public_keys_mut().remove(&key_id);
            managed.key_storage.remove(&key_id);
        }
    }
}
