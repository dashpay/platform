use crate::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};

use crate::prelude::Revision;
use crate::ProtocolError;
use platform_value::Identifier;
use std::collections::{BTreeMap, HashSet};

/// Trait for getters in Identity
pub trait IdentityGettersV0 {
    /// Returns a reference to the public keys of the identity.
    fn public_keys(&self) -> &BTreeMap<KeyID, IdentityPublicKey>;
    /// Returns a mutable reference to the public keys of the identity.
    ///
    /// # Returns
    ///
    /// A mutable reference to a `BTreeMap` containing the `KeyID` as keys and `IdentityPublicKey` as values.
    fn public_keys_mut(&mut self) -> &mut BTreeMap<KeyID, IdentityPublicKey>;
    /// Consumes the `Identity` and returns the owned public keys.
    ///
    /// # Returns
    ///
    /// A `BTreeMap` containing the `KeyID` as keys and `IdentityPublicKey` as values.
    fn public_keys_owned(self) -> BTreeMap<KeyID, IdentityPublicKey>;

    /// Returns the balance of the identity.
    fn balance(&self) -> u64;

    /// Returns the revision of the identity.
    fn revision(&self) -> Revision;

    /// Returns the identifier of the identity.
    fn id(&self) -> Identifier;

    /// Returns a public key for a given id
    fn get_public_key_by_id(&self, key_id: KeyID) -> Option<&IdentityPublicKey>;
    /// Returns a public key for a given id
    fn get_public_key_by_id_mut(&mut self, key_id: KeyID) -> Option<&mut IdentityPublicKey>;
    /// Add identity public keys
    fn add_public_keys(&mut self, keys: impl IntoIterator<Item = IdentityPublicKey>);
    /// Get the biggest public KeyID
    fn get_public_key_max_id(&self) -> KeyID;
    /// Get first public key matching a purpose, security levels or key types
    fn get_first_public_key_matching(
        &self,
        purpose: Purpose,
        security_levels: HashSet<SecurityLevel>,
        key_types: HashSet<KeyType>,
        allow_disabled: bool,
    ) -> Option<&IdentityPublicKey>;
    /// Add an identity public key
    fn add_public_key(&mut self, key: IdentityPublicKey);
}

/// Trait for setters in Identity
pub trait IdentitySettersV0 {
    /// Sets the public keys of the identity.
    fn set_public_keys(&mut self, new_public_keys: BTreeMap<KeyID, IdentityPublicKey>);

    /// Sets the balance of the identity.
    fn set_balance(&mut self, new_balance: u64);

    /// Sets the revision of the identity.
    fn set_revision(&mut self, new_revision: Revision);
    /// Sets the revision of the identity.
    ///
    /// # Arguments
    ///
    /// * `new_revision` - The new revision as a `Revision`.
    fn bump_revision(&mut self);

    /// Sets the identifier of the identity.
    fn set_id(&mut self, new_id: Identifier);
    /// Increase Identity balance
    fn increase_balance(&mut self, amount: u64) -> u64;
    /// Reduce the Identity balance
    fn reduce_balance(&mut self, amount: u64) -> u64;
    /// Increment revision
    fn increment_revision(&mut self) -> Result<(), ProtocolError>;
}
