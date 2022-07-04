use crate::document::document_factory::DocumentFactory;

use super::{get_data_contract_fixture, get_documents_fixture};

fn get_document_transitions_fixture() {
    let data_contract = get_data_contract_fixture(None);
    let documents = get_documents_fixture(data_contract);
}
