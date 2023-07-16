#[cfg(feature = "state-transition-transformers")]
pub mod transformer;

use crate::data_contract::DataContract;
use crate::identifier::Identifier;
use serde::{Deserialize, Serialize};
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

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

pub trait DocumentBaseTransitionActionAccessorsV0 {
    /// The document Id
    fn id(&self) -> Identifier;
    /// Name of document type found int the data contract associated with the `data_contract_id`
    fn document_type_name(&self) -> &String;
    fn document_type_name_owned(self) -> String;
    /// Data contract ID generated from the data contract's `owner_id` and `entropy`
    fn data_contract_id(&self) -> Identifier;
    /// Data contract
    fn data_contract(&self) -> &DataContract;
}
