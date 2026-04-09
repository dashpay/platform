//! Core identity operations for ManagedIdentity

use super::key_storage::{DpnsNameInfo, IdentityStatus, PrivateKeyData};
use super::ManagedIdentity;
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
