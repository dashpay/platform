use serde::{Deserialize, Serialize};
use crate::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionV0;

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
