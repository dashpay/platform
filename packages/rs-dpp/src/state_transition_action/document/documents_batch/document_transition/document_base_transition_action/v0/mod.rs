#[cfg(feature = "state-transition-transformers")]
pub mod transformer;

use bincode::{Decode, Encode};
use crate::data_contract::DataContract;
use crate::identifier::Identifier;
use serde::{Deserialize, Serialize};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::ProtocolError;

#[derive(Debug, Clone, Serialize, Deserialize, PlatformSerialize, PlatformDeserialize)]

pub struct DocumentBaseTransitionActionV0 {
    /// The document Id
    pub id: Identifier,
    /// Name of document type found int the data contract associated with the `data_contract_id`
    pub document_type_name: String,
    /// Data contract ID generated from the data contract's `owner_id` and `entropy`
    pub data_contract_id: Identifier,
    /// A potential data contract
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
