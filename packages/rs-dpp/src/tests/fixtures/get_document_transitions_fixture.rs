use std::collections::HashMap;

use crate::version::LATEST_VERSION;
use crate::{
    document::{
        document_factory::DocumentFactory,
        document_transition::{Action, DocumentTransition},
        Document,
    },
    mocks,
};

use super::{get_data_contract_fixture, get_document_validator_fixture, get_documents_fixture};

pub fn get_document_transitions_fixture(
    documents: impl IntoIterator<Item = (Action, Vec<Document>)>,
) -> Vec<DocumentTransition> {
    let document_factory = DocumentFactory::new(
        LATEST_VERSION,
        get_document_validator_fixture(),
        mocks::FetchAndValidateDataContract {},
    );

    let mut documents_collected: HashMap<Action, Vec<Document>> = documents.into_iter().collect();
    let create_documents = documents_collected
        .remove(&Action::Create)
        .unwrap_or_else(|| get_documents_fixture(get_data_contract_fixture(None)).unwrap());
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
        .expect("the transitions should be crated")
        .get_transitions()
        .to_owned()
}
