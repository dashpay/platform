use crate::identity::{Identity, IdentityPublicKey, KeyID};
use crate::metadata::Metadata;
use crate::prelude::{AssetLockProof, Revision};
use platform_value::Identifier;
use std::collections::BTreeMap;

impl Identity {
    /// Returns a reference to the public keys of the identity.
    ///
    /// # Returns
    ///
    /// A reference to a `BTreeMap` containing the `KeyID` as keys and `IdentityPublicKey` as values.
    pub fn public_keys(&self) -> &BTreeMap<KeyID, IdentityPublicKey> {
        match self {
            Identity::V0(identity) => &identity.public_keys,
        }
    }

    /// Returns a mutable reference to the public keys of the identity.
    ///
    /// # Returns
    ///
    /// A mutable reference to a `BTreeMap` containing the `KeyID` as keys and `IdentityPublicKey` as values.
    pub fn public_keys_mut(&mut self) -> &mut BTreeMap<KeyID, IdentityPublicKey> {
        match self {
            Identity::V0(identity) => &mut identity.public_keys,
        }
    }

    /// Consumes the `Identity` and returns the owned public keys.
    ///
    /// # Returns
    ///
    /// A `BTreeMap` containing the `KeyID` as keys and `IdentityPublicKey` as values.
    pub fn public_keys_owned(self) -> BTreeMap<KeyID, IdentityPublicKey> {
        match self {
            Identity::V0(identity) => identity.public_keys,
        }
    }

    /// Sets the public keys of the identity.
    ///
    /// # Arguments
    ///
    /// * `new_public_keys` - A `BTreeMap` containing the new `KeyID` as keys and `IdentityPublicKey` as values.
    pub fn set_public_keys(&mut self, new_public_keys: BTreeMap<KeyID, IdentityPublicKey>) {
        match self {
            Identity::V0(identity) => identity.public_keys = new_public_keys,
        }
    }

    /// Returns the balance of the identity.
    ///
    /// # Returns
    ///
    /// The balance as a `u64`.
    pub fn balance(&self) -> u64 {
        match self {
            Identity::V0(identity) => identity.balance,
        }
    }

    /// Sets the balance of the identity.
    ///
    /// # Arguments
    ///
    /// * `new_balance` - The new balance as a `u64`.
    pub fn set_balance(&mut self, new_balance: u64) {
        match self {
            Identity::V0(identity) => identity.balance = new_balance,
        }
    }

    /// Returns the revision of the identity.
    ///
    /// # Returns
    ///
    /// The revision as a `Revision`.
    pub fn revision(&self) -> Revision {
        match self {
            Identity::V0(identity) => identity.revision,
        }
    }

    /// Sets the revision of the identity.
    ///
    /// # Arguments
    ///
    /// * `new_revision` - The new revision as a `Revision`.
    pub fn set_revision(&mut self, new_revision: Revision) {
        match self {
            Identity::V0(identity) => identity.revision = new_revision,
        }
    }

    /// Returns a reference to the asset lock proof of the identity.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the `AssetLockProof`, if it exists.
    pub fn asset_lock_proof(&self) -> Option<&AssetLockProof> {
        match self {
            Identity::V0(identity) => identity.asset_lock_proof.as_ref(),
        }
    }

    /// Returns a mutable reference to the asset lock proof of the identity.
    ///
    /// # Returns
    ///
    /// An `Option` containing a mutable reference to the `AssetLockProof`, if it exists.
    pub fn asset_lock_proof_mut(&mut self) -> Option<&mut AssetLockProof> {
        match self {
            Identity::V0(identity) => identity.asset_lock_proof.as_mut(),
        }
    }

    /// Consumes the `Identity` and returns the owned asset lock proof.
    ///
    /// # Returns
    ///
    /// An `Option` containing the `AssetLockProof`, if it exists.
    pub fn asset_lock_proof_owned(self) -> Option<AssetLockProof> {
        match self {
            Identity::V0(identity) => identity.asset_lock_proof,
        }
    }

    /// Sets the asset lock proof of the identity.
    ///
    /// # Arguments
    ///
    /// * `new_asset_lock_proof` - An `Option` containing the new `AssetLockProof`, if it exists.
    pub fn set_asset_lock_proof(&mut self, new_asset_lock_proof: AssetLockProof) {
        match self {
            Identity::V0(identity) => identity.asset_lock_proof = Some(new_asset_lock_proof),
        }
    }

    /// Remove the asset lock proof of the identity.
    pub fn remove_asset_lock_proof(&mut self) {
        match self {
            Identity::V0(identity) => identity.asset_lock_proof = None,
        }
    }

    /// Returns a reference to the metadata of the identity.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the `Metadata`, if it exists.
    pub fn metadata(&self) -> Option<&Metadata> {
        match self {
            Identity::V0(identity) => identity.metadata.as_ref(),
        }
    }

    /// Returns a mutable reference to the metadata of the identity.
    ///
    /// # Returns
    ///
    /// An `Option` containing a mutable reference to the `Metadata`, if it exists.
    pub fn metadata_mut(&mut self) -> Option<&mut Metadata> {
        match self {
            Identity::V0(identity) => identity.metadata.as_mut(),
        }
    }

    /// Consumes the `Identity` and returns the owned metadata.
    ///
    /// # Returns
    ///
    /// An `Option` containing the `Metadata`, if it exists.
    pub fn metadata_owned(self) -> Option<Metadata> {
        match self {
            Identity::V0(identity) => identity.metadata,
        }
    }

    /// Sets the metadata of the identity.
    ///
    /// # Arguments
    ///
    /// * `new_metadata` - An `Option` containing the new `Metadata`, if it exists.
    pub fn set_metadata(&mut self, new_metadata: Option<Metadata>) {
        match self {
            Identity::V0(identity) => identity.metadata = new_metadata,
        }
    }

    /// Returns the identifier of the identity.
    ///
    /// # Returns
    ///
    /// The identifier as an `Identifier`.
    pub fn id(&self) -> Identifier {
        match self {
            Identity::V0(identity) => identity.id,
        }
    }

    /// Consumes the `Identity` and returns the owned identifier.
    ///
    /// # Returns
    ///
    /// The identifier as an `Identifier`.
    pub fn id_owned(self) -> Identifier {
        match self {
            Identity::V0(identity) => identity.id,
        }
    }

    /// Sets the identifier of the identity.
    ///
    /// # Arguments
    ///
    /// * `new_id` - The new identifier as an `Identifier`.
    pub fn set_id(&mut self, new_id: Identifier) {
        match self {
            Identity::V0(identity) => identity.id = new_id,
        }
    }
}
