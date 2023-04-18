use std::collections::HashMap;
use std::sync::Arc;

use crate::document::fetch_and_validate_data_contract::DataContractFetcherAndValidator;
use crate::document::ExtendedDocument;
use crate::document::{
    document_factory::DocumentFactory,
    document_transition::{Action, DocumentTransition},
};
use crate::state_repository::MockStateRepositoryLike;
use crate::tests::fixtures::get_extended_documents_fixture;
use crate::version::LATEST_VERSION;

use super::{get_data_contract_fixture, get_document_validator_fixture};

pub fn get_document_transitions_fixture(
    documents: impl IntoIterator<Item = (Action, Vec<ExtendedDocument>)>,
) -> Vec<DocumentTransition> {
    let document_factory = DocumentFactory::new(
        LATEST_VERSION,
        get_document_validator_fixture(),
        DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
    );

    let mut documents_collected: HashMap<Action, Vec<ExtendedDocument>> =
        documents.into_iter().collect();
    let create_documents = documents_collected
        .remove(&Action::Create)
        .unwrap_or_else(|| {
            get_extended_documents_fixture(get_data_contract_fixture(None)).unwrap()
        });
    let replace_documents = documents_collected
        .remove(&Action::Replace)
        .unwrap_or_default();
    let delete_documents = documents_collected
        .remove(&Action::Delete)
        .unwrap_or_default();

    document_factory
        .create_state_transition([
            (Action::Create, create_documents),
            (Action::Replace, replace_documents),
            (Action::Delete, delete_documents),
        ])
        .expect("the transitions should be created")
        .get_transitions()
        .to_owned()
}
