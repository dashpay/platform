use std::collections::BTreeMap;

use dpp::{
    document::Document,
    identity::KeyID,
    prelude::{DataContract, IdentityPublicKey, Revision},
};

pub type DataContractHistory = BTreeMap<u64, DataContract>;
pub type DataContracts = BTreeMap<[u8; 32], Option<DataContract>>;
pub type IdentityBalance = u64;
pub type IdentityBalanceAndRevision = (u64, Revision);
pub type IdentityPublicKeys = BTreeMap<KeyID, Option<IdentityPublicKey>>;
pub type Documents = Vec<Document>;
