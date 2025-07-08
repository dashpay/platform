use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0Getters};
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;

impl DocumentBaseTransitionV0 {
    pub(in crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_base_transition) fn from_document(
        document: &Document,
        document_type: DocumentTypeRef,
        identity_contract_nonce: IdentityNonce,
    ) -> Self {
        DocumentBaseTransitionV0 {
            id: document.id(),
            identity_contract_nonce,
            document_type_name: document_type.name().to_string(),
            data_contract_id: document_type.data_contract_id(),
        }
    }
}
