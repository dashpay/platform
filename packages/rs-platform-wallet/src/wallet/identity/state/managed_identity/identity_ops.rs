//! Core identity operations for ManagedIdentity

use super::key_storage::{DpnsNameInfo, IdentityStatus};
use super::ManagedIdentity;
use crate::changeset::{IdentityChangeSet, IdentityEntry, IdentityKeyEntry, IdentityKeysChangeSet};
use crate::wallet::persister::WalletPersister;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use dpp::util::hash::ripemd160_sha256;
use std::collections::BTreeMap;

/// Compute the 20-byte RIPEMD160(SHA256) hash of the DPP public-key
/// bytes. Shared between `keys_snapshot_changeset` so the FFI surface
/// always carries a consistent pre-hashed form.
fn pubkey_hash_of(pub_key: &dpp::identity::IdentityPublicKey) -> [u8; 20] {
    let mut out = [0u8; 20];
    out.copy_from_slice(ripemd160_sha256(pub_key.data().as_slice()).as_slice());
    out
}

impl ManagedIdentity {
    /// Helper: produce an [`IdentityChangeSet`] containing a scalar-only
    /// [`IdentityEntry`] snapshot of `self`.
    ///
    /// Every scalar mutation method on `ManagedIdentity` calls this
    /// after mutating state so that the returned changeset faithfully
    /// describes the resulting identity. Public keys + private-key
    /// storage are snapshotted separately via
    /// [`Self::keys_snapshot_changeset`] so scalar mutations don't drag
    /// the full key payload through every persist.
    pub(crate) fn snapshot_changeset(&self) -> IdentityChangeSet {
        let mut cs = IdentityChangeSet::default();
        cs.identities
            .insert(self.id(), IdentityEntry::from_managed(self));
        cs
    }

    /// Helper: produce an [`IdentityKeysChangeSet`] containing one
    /// [`IdentityKeyEntry`] upsert per registered public key on this
    /// identity.
    ///
    /// Private-key bytes / derivation breadcrumbs no longer ride along
    /// here — `ManagedIdentity` doesn't carry `key_storage` anymore,
    /// so every emitted entry has `wallet_id == None` and
    /// `derivation_indices == None`. Callers that need the
    /// breadcrumb (e.g. registration / discovery) emit a dedicated
    /// `IdentityKeysChangeSet` themselves with the right
    /// `(wallet_id, identity_index, key_index)` pair.
    pub(crate) fn keys_snapshot_changeset(&self) -> IdentityKeysChangeSet {
        let identity_id = self.id();
        let mut upserts = BTreeMap::new();
        for (key_id, pub_key) in self.identity.public_keys() {
            upserts.insert(
                (identity_id, *key_id),
                IdentityKeyEntry {
                    identity_id,
                    key_id: *key_id,
                    public_key: pub_key.clone(),
                    public_key_hash: pubkey_hash_of(pub_key),
                    wallet_id: None,
                    derivation_indices: None,
                },
            );
        }
        IdentityKeysChangeSet {
            upserts,
            removed: Default::default(),
        }
    }

    /// Create a new wallet-owned managed identity with its BIP-9 HD
    /// identity index. The resulting `identity_index` field is
    /// `Some(identity_index)` — use [`Self::new_out_of_wallet`] for
    /// observed identities that have no derivation context.
    pub fn new(identity: Identity, identity_index: u32) -> Self {
        Self {
            identity,
            identity_index: Some(identity_index),
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            established_contacts: Default::default(),
            sent_contact_requests: Default::default(),
            incoming_contact_requests: Default::default(),
            rejected_contact_requests: Default::default(),
            status: Default::default(),
            dpns_names: Vec::new(),
            contested_dpns_names: Vec::new(),
            wallet_id: None,
            dashpay_profile: None,
            dashpay_payments: BTreeMap::new(),
        }
    }

    /// Create a new out-of-wallet managed identity (observed read-only).
    ///
    /// Out-of-wallet identities don't belong to any local wallet, so
    /// `identity_index` is `None` and `wallet_id` is left as `None` —
    /// signing / derivation paths must guard on these and reject the
    /// out-of-wallet case explicitly.
    pub fn new_out_of_wallet(identity: Identity) -> Self {
        Self {
            identity,
            identity_index: None,
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            established_contacts: Default::default(),
            sent_contact_requests: Default::default(),
            incoming_contact_requests: Default::default(),
            rejected_contact_requests: Default::default(),
            status: Default::default(),
            dpns_names: Vec::new(),
            contested_dpns_names: Vec::new(),
            wallet_id: None,
            dashpay_profile: None,
            dashpay_payments: BTreeMap::new(),
        }
    }

    /// Set the DashPay profile for this identity.
    ///
    /// Persists the resulting changeset via `persister` and returns `()`.
    /// Pass `None` to clear the profile.
    ///
    /// TODO: This is `pub` transitionally — evo-tool's backend tasks
    /// still fetch/create DashPay profiles themselves and push results
    /// back here. `IdentityWallet::sync_profiles` now covers the
    /// happy path; once evo-tool's `platform_wallet_cache.rs` migrates
    /// off the direct setter, this should drop to `pub(crate)`.
    pub fn set_dashpay_profile(
        &mut self,
        profile: Option<crate::wallet::identity::DashPayProfile>,
        persister: &WalletPersister,
    ) {
        self.dashpay_profile = profile;
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Record a DashPay payment under its transaction id.
    ///
    /// If an entry with the same `tx_id` already exists, it is
    /// overwritten (last-write-wins) — the expected use case is
    /// updating the status field from `Pending` to `Confirmed` or
    /// `Failed` after broadcast.
    ///
    /// Persists the resulting changeset via `persister` and returns `()`.
    ///
    /// TODO: Same transitional `pub` as `set_dashpay_profile` — see
    /// that method's doc comment for rationale.
    pub fn record_dashpay_payment(
        &mut self,
        tx_id: String,
        entry: crate::wallet::identity::PaymentEntry,
        persister: &WalletPersister,
    ) {
        self.dashpay_payments.insert(tx_id, entry);
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Get the identity ID
    pub fn id(&self) -> Identifier {
        self.identity.id()
    }

    /// Get the identity's balance
    pub fn balance(&self) -> u64 {
        self.identity.balance()
    }

    /// Get the identity's revision
    pub fn revision(&self) -> u64 {
        self.identity.revision()
    }

    /// Set the identity lifecycle status.
    ///
    /// Persists the resulting changeset via `persister` and returns `()`.
    pub fn set_status(&mut self, status: IdentityStatus, persister: &WalletPersister) {
        self.status = status;
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Add a DPNS name associated with this identity.
    ///
    /// Persists the resulting changeset via `persister` and returns `()`.
    pub fn add_dpns_name(&mut self, name: DpnsNameInfo, persister: &WalletPersister) {
        self.dpns_names.push(name);
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Append a contested DPNS label this identity is contending for.
    ///
    /// Dedup is enforced — the same label isn't added twice. When a
    /// contest resolves (won), the caller should move the label from
    /// `contested_dpns_names` onto `dpns_names` via a separate
    /// mutation; on lock / loss, the label just gets dropped. Both
    /// cases are out of scope for this method.
    ///
    /// Persists the resulting snapshot via `persister`.
    pub fn add_contested_dpns_name(&mut self, label: String, persister: &WalletPersister) {
        if self.contested_dpns_names.contains(&label) {
            return;
        }
        self.contested_dpns_names.push(label);
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Replace the contested-name list wholesale.
    ///
    /// Use this when a sync round pulls the canonical set of
    /// contested names from Platform — the merge-time dedup-append
    /// policy on `IdentityChangeSet` would otherwise accumulate
    /// stale labels (contests that resolved but still appear in
    /// the local cache). Emitting a full snapshot here + running
    /// the sync path on identity reapply bakes the authoritative
    /// set into state.
    pub fn set_contested_dpns_names(&mut self, labels: Vec<String>, persister: &WalletPersister) {
        self.contested_dpns_names = labels;
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Layer an `IdentityPublicKey` onto this identity's
    /// `public_keys` map and emit a single-key
    /// [`IdentityKeysChangeSet`] upsert carrying the full derivation
    /// breadcrumb.
    ///
    /// `wallet_id` and `(identity_index, key_index)` together let the
    /// client (iOS Keychain) re-derive the matching private key from
    /// the wallet seed at the DIP-9 identity authentication path.
    /// Pass `None` for a watch-only key the wallet didn't derive.
    pub fn add_key(
        &mut self,
        public_key: dpp::identity::IdentityPublicKey,
        derivation_breadcrumb: Option<([u8; 32], u32, u32)>,
        persister: &WalletPersister,
    ) {
        use dpp::identity::accessors::IdentitySettersV0;

        let key_id = public_key.id();
        let public_key_hash = pubkey_hash_of(&public_key);

        // Layer onto the DPP `Identity` itself — that's what every
        // signing / introspection path reads.
        let mut keys = self.identity.public_keys().clone();
        keys.insert(key_id, public_key.clone());
        self.identity.set_public_keys(keys);

        let identity_id = self.id();
        let (wallet_id, derivation_indices) = match derivation_breadcrumb {
            Some((wallet_id, identity_index, key_index)) => (
                Some(wallet_id),
                Some(crate::changeset::IdentityKeyDerivationIndices {
                    identity_index,
                    key_index,
                }),
            ),
            None => (None, None),
        };
        let mut keys_cs = IdentityKeysChangeSet::default();
        keys_cs.upserts.insert(
            (identity_id, key_id),
            IdentityKeyEntry {
                identity_id,
                key_id,
                public_key,
                public_key_hash,
                wallet_id,
                derivation_indices,
            },
        );
        let cs = crate::changeset::PlatformWalletChangeSet {
            identities: Some(self.snapshot_changeset()),
            identity_keys: Some(keys_cs),
            ..Default::default()
        };
        if let Err(e) = persister.store(cs) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }
}
