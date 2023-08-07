use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentType;
use crate::document::{Document, DocumentV0Getters};
use crate::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;

impl DocumentBaseTransitionV0 {
    pub(in crate::state_transition::state_transitions::document::documents_batch_transition::document_transition::document_base_transition) fn from_document(
        document: &Document,
        document_type: &DocumentType,
    ) -> Self {
        DocumentBaseTransitionV0 {
            id: document.id(),
            document_type_name: document_type.name().to_string(),
            data_contract_id: document_type.data_contract_id(),
        }
    }
}
