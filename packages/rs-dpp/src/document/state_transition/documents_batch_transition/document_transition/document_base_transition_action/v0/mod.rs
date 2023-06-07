use crate::data_contract::DataContract;
use crate::document::document_transition::document_base_transition::{
    DocumentBaseTransition, DocumentBaseTransitionV0,
};
use crate::identifier::Identifier;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DocumentBaseTransitionActionV0 {
    /// The document Id
    #[serde(rename = "$id")]
    pub id: Identifier,
    /// Name of document type found int the data contract associated with the `data_contract_id`
    #[serde(rename = "$type")]
    pub document_type_name: String,
    /// Data contract ID generated from the data contract's `owner_id` and `entropy`
    #[serde(rename = "$dataContractId")]
    pub data_contract_id: Identifier,
    #[serde(skip)]
    pub data_contract: DataContract,
}

impl From<DocumentBaseTransitionV0> for DocumentBaseTransitionActionV0 {
    fn from(value: DocumentBaseTransitionV0) -> Self {
        let DocumentBaseTransitionV0 {
            id,
            document_type_name,
            data_contract_id,
            data_contract,
            ..
        } = value;
        DocumentBaseTransitionActionV0 {
            id,
            document_type_name,
            data_contract_id,
            data_contract,
        }
    }
}

impl From<&DocumentBaseTransitionV0> for DocumentBaseTransitionActionV0 {
    fn from(value: &DocumentBaseTransitionV0) -> Self {
        let DocumentBaseTransitionV0 {
            id,
            document_type_name,
            data_contract_id,
            data_contract,
            ..
        } = value;
        DocumentBaseTransitionActionV0 {
            id: *id,
            document_type_name: document_type_name.clone(),
            data_contract_id: *data_contract_id,
            data_contract: data_contract.clone(),
        }
    }
}
