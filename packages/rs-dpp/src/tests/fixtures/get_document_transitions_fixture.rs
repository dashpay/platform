use crate::data_contract::document_type::DocumentType;
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

#[cfg(feature = "extended-document")]
pub fn get_extended_document_transitions_fixture(
    documents: impl IntoIterator<Item = (DocumentTransitionActionType, Vec<ExtendedDocument>)>,
) -> Vec<DocumentTransition> {
    let protocol_version = PlatformVersion::latest().protocol_version;
    let data_contract = get_data_contract_fixture(None, protocol_version).data_contract_owned();
    let document_factory = DocumentFactory::new(protocol_version, data_contract.clone())
        .expect("expected document factory");

    let mut documents_collected: HashMap<DocumentTransitionActionType, Vec<ExtendedDocument>> =
        documents.into_iter().collect();
    let create_documents = documents_collected
        .remove(&DocumentTransitionActionType::Create)
        .unwrap_or_else(|| {
            get_extended_documents_fixture(data_contract, protocol_version).unwrap()
        });
    let replace_documents = documents_collected
        .remove(&DocumentTransitionActionType::Replace)
        .unwrap_or_default();
    let delete_documents = documents_collected
        .remove(&DocumentTransitionActionType::Delete)
        .unwrap_or_default();

    document_factory
        .create_state_transition([
            (DocumentTransitionActionType::Create, create_documents),
            (DocumentTransitionActionType::Replace, replace_documents),
            (DocumentTransitionActionType::Delete, delete_documents),
        ])
        .expect("the transitions should be created")
        .transitions()
        .to_owned()
}

pub fn get_document_transitions_fixture<'a>(
    documents: impl IntoIterator<
        Item = (
            DocumentTransitionActionType,
            Vec<(Document, &'a DocumentType)>,
        ),
    >,
) -> Vec<DocumentTransition> {
    let protocol_version = PlatformVersion::latest().protocol_version;
    let data_contract = get_data_contract_fixture(None, protocol_version).data_contract_owned();
    let document_factory = DocumentFactory::new(protocol_version, data_contract.clone())
        .expect("expected to get document factory");

    let mut documents_collected: HashMap<
        DocumentTransitionActionType,
        Vec<(Document, &DocumentType)>,
    > = documents.into_iter().collect();

    let replace_documents = documents_collected
        .remove(&DocumentTransitionActionType::Replace)
        .unwrap_or_default();
    let delete_documents = documents_collected
        .remove(&DocumentTransitionActionType::Delete)
        .unwrap_or_default();

    //created documents are left
    let create_documents = documents_collected
        .remove(&DocumentTransitionActionType::Create)
        .unwrap_or_else(|| get_documents_fixture(data_contract, protocol_version).unwrap());

    document_factory
        .create_state_transition([
            (DocumentTransitionActionType::Create, create_documents),
            (DocumentTransitionActionType::Replace, replace_documents),
            (DocumentTransitionActionType::Delete, delete_documents),
        ])
        .expect("the transitions should be created")
        .transitions()
        .to_owned()
}
