use crate::identity::{IdentityPublicKey, Purpose};
use platform_value::Identifier;
use std::collections::BTreeMap;

pub type IdentitiesContractKeys =
    BTreeMap<Identifier, BTreeMap<Purpose, Option<IdentityPublicKey>>>;
