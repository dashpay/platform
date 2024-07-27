use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::serialization::PlatformSerializable;
use crate::util::hash::hash_double;
use crate::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::{Identifier, Value};
#[cfg(feature = "vote-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Encode, Decode, PlatformSerialize, PlatformDeserialize, PartialEq)]
#[cfg_attr(
    feature = "vote-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_serialize(limit = 100000)]
#[ferment_macro::export]
pub struct ContestedDocumentResourceVotePoll {
    pub contract_id: Identifier,
    pub document_type_name: String,
    pub index_name: String,
    pub index_values: Vec<Value>,
}

impl fmt::Display for ContestedDocumentResourceVotePoll {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format the index_values as a comma-separated list
        let index_values_str: Vec<String> =
            self.index_values.iter().map(|v| v.to_string()).collect();
        write!(
            f,
            "ContestedDocumentResourceVotePoll {{ contract_id: {}, document_type_name: {}, index_name: {}, index_values: [{}] }}",
            self.contract_id,
            self.document_type_name,
            self.index_name,
            index_values_str.join(", ")
        )
    }
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
        self.unique_id()
    }

    pub fn unique_id(&self) -> Result<Identifier, ProtocolError> {
        self.sha256_2_hash().map(Identifier::new)
    }
}
