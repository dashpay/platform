use crate::identifier::Identifier;
use crate::identity::{IdentityPublicKey, KeyID};
use crate::prelude::Revision;
use std::collections::BTreeMap;

/// An identity stub to get only data that is cared about from drive.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PartialIdentityInfo {
    pub id: Identifier,
    pub loaded_public_keys: BTreeMap<KeyID, IdentityPublicKey>,
    pub balance: Option<u64>,
    pub revision: Option<Revision>,
}
