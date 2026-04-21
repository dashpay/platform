//! Core identity operations for ManagedIdentity

use super::key_storage::{DpnsNameInfo, IdentityStatus, PrivateKeyData};
use super::ManagedIdentity;
use crate::changeset::{
    IdentityChangeSet, IdentityEntry, IdentityKeyEntry, IdentityKeysChangeSet,
    PlatformWalletChangeSet,
};
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::signer::ManagedIdentitySigner;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID};
use dpp::prelude::Identifier;
use key_wallet::Network;
use key_wallet_manager::WalletManager;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    /// identity. Private-key material is populated from `key_storage`
    /// when available; unregistered watched keys emit `None` for the
    /// private-key half.
    pub(crate) fn keys_snapshot_changeset(&self) -> IdentityKeysChangeSet {
        let identity_id = self.id();
        let mut upserts = BTreeMap::new();
        for (key_id, pub_key) in self.identity.public_keys() {
            let private_key = self.key_storage.get(key_id).map(|(_, pk)| pk.clone());
            upserts.insert(
                (identity_id, *key_id),
                IdentityKeyEntry {
                    identity_id,
                    key_id: *key_id,
                    public_key: pub_key.clone(),
                    private_key,
                },
            );
        }
        IdentityKeysChangeSet {
            upserts,
            removed: Default::default(),
        }
    }

    /// Create a new managed identity with its BIP-9 HD identity index.
    pub fn new(identity: Identity, identity_index: u32) -> Self {
        Self {
            identity,
            identity_index,
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            label: None,
            established_contacts: Default::default(),
            sent_contact_requests: Default::default(),
            incoming_contact_requests: Default::default(),
            key_storage: Default::default(),
            status: Default::default(),
            dpns_names: Vec::new(),
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

    /// Store a private key entry in the key storage.
    ///
    /// Persists both the scalar identity snapshot (so any concurrent
    /// scalar changes observed by this call are captured) and a
    /// single-key [`IdentityKeysChangeSet`] upsert for the just-added
    /// key.
    pub fn add_key(
        &mut self,
        key_id: KeyID,
        public_key: IdentityPublicKey,
        private_key_data: PrivateKeyData,
        persister: &WalletPersister,
    ) {
        self.key_storage
            .insert(key_id, (public_key.clone(), private_key_data.clone()));
        let identity_id = self.id();
        let mut keys_cs = IdentityKeysChangeSet::default();
        keys_cs.upserts.insert(
            (identity_id, key_id),
            IdentityKeyEntry {
                identity_id,
                key_id,
                public_key,
                private_key: Some(private_key_data),
            },
        );
        let cs = PlatformWalletChangeSet {
            identities: Some(self.snapshot_changeset()),
            identity_keys: Some(keys_cs),
            ..Default::default()
        };
        if let Err(e) = persister.store(cs) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Look up private key data by key ID.
    pub fn private_key_data(&self, key_id: &KeyID) -> Option<&PrivateKeyData> {
        self.key_storage.get(key_id).map(|(_, pk)| pk)
    }

    /// Create a [`ManagedIdentitySigner`] for this identity.
    ///
    /// The signer resolves keys from this identity's `key_storage`. For keys
    /// stored with [`PrivateKeyData::AtWalletDerivationPath`] the wallet is
    /// used to derive the private key on demand. For keys not in the storage
    /// the signer falls back to the standard DIP-9 identity authentication
    /// path derivation.
    pub fn signer(
        &self,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        network: Network,
    ) -> ManagedIdentitySigner {
        ManagedIdentitySigner::new(
            self.key_storage.clone(),
            wallet_manager,
            wallet_id,
            self.identity_index,
            network,
        )
    }
}
