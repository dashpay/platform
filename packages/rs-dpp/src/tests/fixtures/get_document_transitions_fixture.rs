use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::document::document_factory::DocumentFactory;
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

use crate::document::Document;
use crate::state_transition::documents_batch_transition::document_transition::action_type::DocumentTransitionActionType;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;
use crate::tests::fixtures::get_documents_fixture;

#[cfg(feature = "extended-document")]
use crate::document::ExtendedDocument;
use crate::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
#[cfg(feature = "extended-document")]
use crate::tests::fixtures::get_extended_documents_fixture;

use super::get_data_contract_fixture;

pub fn get_document_transitions_fixture<'a>(
    documents: impl IntoIterator<
        Item = (
            DocumentTransitionActionType,
            Vec<(Document, DocumentTypeRef<'a>)>,
        ),
    >,
) -> Vec<DocumentTransition> {
    let protocol_version = PlatformVersion::latest().protocol_version;
    let data_contract = get_data_contract_fixture(None, protocol_version).data_contract_owned();
    let document_factory = DocumentFactory::new(protocol_version, data_contract.clone())
        .expect("expected to get document factory");

    document_factory
        .create_state_transition(documents)
        .expect("the transitions should be created")
        .transitions()
        .to_owned()
}
