use crate::data_contract::DataContract;
use crate::document::document_transition::document_base_transition::DocumentBaseTransition;
use crate::identifier::Identifier;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DocumentBaseTransitionAction {
    /// The document ID
    #[serde(rename = "$id")]
    pub id: Identifier,
    /// Name of document type found int the data contract associated with the `data_contract_id`
    #[serde(rename = "$type")]
    pub document_type: String,
    /// Data contract ID generated from the data contract's `owner_id` and `entropy`
    #[serde(rename = "$dataContractId")]
    pub data_contract_id: Identifier,
    #[serde(skip)]
    pub data_contract: DataContract,
}

impl From<DocumentBaseTransition> for DocumentBaseTransitionAction {
    fn from(value: DocumentBaseTransition) -> Self {
        let DocumentBaseTransition {
            id,
            document_type,
            data_contract_id,
            data_contract,
            ..
        } = value;
        DocumentBaseTransitionAction {
            id,
            document_type,
            data_contract_id,
            data_contract,
        }
    }
}

impl From<&DocumentBaseTransition> for DocumentBaseTransitionAction {
    fn from(value: &DocumentBaseTransition) -> Self {
        let DocumentBaseTransition {
            id,
            document_type,
            data_contract_id,
            data_contract,
            ..
        } = value;
        DocumentBaseTransitionAction {
            id: *id,
            document_type: document_type.clone(),
            data_contract_id: *data_contract_id,
            data_contract: data_contract.clone(),
        }
    }
}
