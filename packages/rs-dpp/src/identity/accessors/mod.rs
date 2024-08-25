use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};

use crate::prelude::Revision;
use crate::ProtocolError;
use platform_value::Identifier;
use std::collections::{BTreeMap, HashSet};
pub use v0::{IdentityGettersV0, IdentitySettersV0};

mod v0;

impl IdentityGettersV0 for Identity {
    /// Returns a reference to the public keys of the identity.
    ///
    /// # Returns
    ///
    /// A reference to a `BTreeMap` containing the `KeyID` as keys and `IdentityPublicKey` as values.
    fn public_keys(&self) -> &BTreeMap<KeyID, IdentityPublicKey> {
        match self {
            Identity::V0(identity) => &identity.public_keys,
        }
    }

    /// Returns a mutable reference to the public keys of the identity.
    ///
    /// # Returns
    ///
    /// A mutable reference to a `BTreeMap` containing the `KeyID` as keys and `IdentityPublicKey` as values.
    fn public_keys_mut(&mut self) -> &mut BTreeMap<KeyID, IdentityPublicKey> {
        match self {
            Identity::V0(identity) => &mut identity.public_keys,
        }
    }

    /// Consumes the `Identity` and returns the owned public keys.
    ///
    /// # Returns
    ///
    /// A `BTreeMap` containing the `KeyID` as keys and `IdentityPublicKey` as values.
    fn public_keys_owned(self) -> BTreeMap<KeyID, IdentityPublicKey> {
        match self {
            Identity::V0(identity) => identity.public_keys,
        }
    }

    /// Returns the balance of the identity.
    ///
    /// # Returns
    ///
    /// The balance as a `u64`.
    fn balance(&self) -> u64 {
        match self {
            Identity::V0(identity) => identity.balance,
        }
    }

    /// Returns the revision of the identity.
    ///
    /// # Returns
    ///
    /// The revision as a `Revision`.
    fn revision(&self) -> Revision {
        match self {
            Identity::V0(identity) => identity.revision,
        }
    }

    /// Returns the identifier of the identity.
    ///
    /// # Returns
    ///
    /// The identifier as an `Identifier`.
    fn id(&self) -> Identifier {
        match self {
            Identity::V0(identity) => identity.id,
        }
    }

    /// Returns a public key for a given id
    fn get_public_key_by_id(&self, key_id: KeyID) -> Option<&IdentityPublicKey> {
        match self {
            Identity::V0(identity) => identity.public_keys.get(&key_id),
        }
    }

    /// Returns a public key for a given id
    fn get_public_key_by_id_mut(&mut self, key_id: KeyID) -> Option<&mut IdentityPublicKey> {
        match self {
            Identity::V0(identity) => identity.public_keys.get_mut(&key_id),
        }
    }

    /// Get the biggest public KeyID
    fn get_public_key_max_id(&self) -> KeyID {
        match self {
            Identity::V0(identity) => identity
                .public_keys
                .keys()
                .copied()
                .max()
                .unwrap_or_default(),
        }
    }

    /// Add an identity public key
    fn add_public_key(&mut self, key: IdentityPublicKey) {
        match self {
            Identity::V0(identity) => identity.public_keys.insert(key.id(), key),
        };
    }

    /// Add identity public keys
    fn add_public_keys(&mut self, keys: impl IntoIterator<Item = IdentityPublicKey>) {
        match self {
            Identity::V0(identity) => identity
                .public_keys
                .extend(keys.into_iter().map(|a| (a.id(), a))),
        }
    }

    /// Get first public key matching a purpose, security levels, or key types, optionally allowing disabled keys
    fn get_first_public_key_matching(
        &self,
        purpose: Purpose,
        security_levels: HashSet<SecurityLevel>,
        key_types: HashSet<KeyType>,
        allow_disabled: bool,
    ) -> Option<&IdentityPublicKey> {
        match self {
            Identity::V0(identity) => identity.public_keys.values().find(|key| {
                key.purpose() == purpose
                    && security_levels.contains(&key.security_level())
                    && key_types.contains(&key.key_type())
                    && (allow_disabled || !key.is_disabled())
            }),
        }
    }
}

impl IdentitySettersV0 for Identity {
    /// Sets the public keys of the identity.
    ///
    /// # Arguments
    ///
    /// * `new_public_keys` - A `BTreeMap` containing the new `KeyID` as keys and `IdentityPublicKey` as values.
    fn set_public_keys(&mut self, new_public_keys: BTreeMap<KeyID, IdentityPublicKey>) {
        match self {
            Identity::V0(identity) => identity.public_keys = new_public_keys,
        }
    }

    /// Sets the balance of the identity.
    ///
    /// # Arguments
    ///
    /// * `new_balance` - The new balance as a `u64`.
    fn set_balance(&mut self, new_balance: u64) {
        match self {
            Identity::V0(identity) => identity.balance = new_balance,
        }
    }

    /// Sets the revision of the identity.
    ///
    /// # Arguments
    ///
    /// * `new_revision` - The new revision as a `Revision`.
    fn set_revision(&mut self, new_revision: Revision) {
        match self {
            Identity::V0(identity) => identity.revision = new_revision,
        }
    }

    /// Sets the revision of the identity to +1.
    ///
    fn bump_revision(&mut self) {
        match self {
            Identity::V0(identity) => identity.revision += 1,
        }
    }

    /// Sets the identifier of the identity.
    ///
    /// # Arguments
    ///
    /// * `new_id` - The new identifier as an `Identifier`.
    fn set_id(&mut self, new_id: Identifier) {
        match self {
            Identity::V0(identity) => identity.id = new_id,
        }
    }

    /// Increase Identity balance
    fn increase_balance(&mut self, amount: u64) -> u64 {
        match self {
            Identity::V0(identity) => {
                identity.balance += amount;
                identity.balance
            }
        }
    }

    /// Reduce the Identity balance
    fn reduce_balance(&mut self, amount: u64) -> u64 {
        match self {
            Identity::V0(identity) => {
                identity.balance -= amount;
                identity.balance
            }
        }
    }

    /// Increment revision
    fn increment_revision(&mut self) -> Result<(), ProtocolError> {
        match self {
            Identity::V0(identity) => {
                let result = identity
                    .revision
                    .checked_add(1)
                    .ok_or(ProtocolError::Generic(
                        "identity revision is at max level".to_string(),
                    ))?;

                identity.revision = result;

                Ok(())
            }
        }
    }
}
