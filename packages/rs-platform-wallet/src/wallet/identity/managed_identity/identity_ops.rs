//! Core identity operations for ManagedIdentity

use super::key_storage::{DpnsNameInfo, IdentityStatus, PrivateKeyData};
use super::ManagedIdentity;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID};
use dpp::prelude::Identifier;
use std::collections::BTreeMap;

impl ManagedIdentity {
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
            wallet_seed_hash: None,
            top_ups: BTreeMap::new(),
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
    pub fn set_status(&mut self, status: IdentityStatus) {
        self.status = status;
    }

    /// Add a DPNS name associated with this identity.
    pub fn add_dpns_name(&mut self, name: DpnsNameInfo) {
        self.dpns_names.push(name);
    }

    /// Store a private key entry in the key storage.
    pub fn add_key(
        &mut self,
        key_id: KeyID,
        public_key: IdentityPublicKey,
        private_key_data: PrivateKeyData,
    ) {
        self.key_storage
            .insert(key_id, (public_key, private_key_data));
    }

    /// Look up private key data by key ID.
    pub fn private_key_data(&self, key_id: &KeyID) -> Option<&PrivateKeyData> {
        self.key_storage.get(key_id).map(|(_, pk)| pk)
    }

    /// Record a top-up by index and amount.
    pub fn record_top_up(&mut self, index: u32, amount: u64) {
        self.top_ups.insert(index, amount);
    }
}
