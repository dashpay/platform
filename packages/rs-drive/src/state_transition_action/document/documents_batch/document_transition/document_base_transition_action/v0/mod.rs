pub mod transformer;

use std::sync::Arc;
use bincode::{Decode, Encode};
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use serde::{Deserialize, Serialize};
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use dpp::ProtocolError;
use crate::drive::contract::DataContractFetchInfo;

#[derive(Debug, Clone)]

pub struct DocumentBaseTransitionActionV0 {
    /// The document Id
    pub id: Identifier,
    /// Name of document type found int the data contract associated with the `data_contract_id`
    pub document_type_name: String,
    /// A potential data contract
    pub data_contract: Arc<DataContractFetchInfo>,
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
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo>;
}
