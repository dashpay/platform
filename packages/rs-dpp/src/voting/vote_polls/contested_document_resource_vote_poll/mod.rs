use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
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
