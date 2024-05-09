use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::serialization::PlatformSerializable;
use crate::util::hash::hash_double;
use crate::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, PlatformSerialize, PlatformDeserialize, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_serialize(limit = 100000)]
pub struct ContestedDocumentResourceVotePoll {
    pub contract_id: Identifier,
    pub document_type_name: String,
    pub index_name: String,
    pub index_values: Vec<Value>,
}

impl Default for ContestedDocumentResourceVotePoll {
    fn default() -> Self {
        ContestedDocumentResourceVotePoll {
            contract_id: Default::default(),
            document_type_name: "".to_string(),
            index_name: "".to_string(),
            index_values: vec![],
        }
    }
}

impl ContestedDocumentResourceVotePoll {
    pub fn sha256_2_hash(&self) -> Result<[u8; 32], ProtocolError> {
        let encoded = self.serialize_to_bytes()?;
        Ok(hash_double(encoded))
    }

    pub fn specialized_balance_id(&self) -> Result<Identifier, ProtocolError> {
        self.sha256_2_hash().map(|id| Identifier::new(id))
    }
}
