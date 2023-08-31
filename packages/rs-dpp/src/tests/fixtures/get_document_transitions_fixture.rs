use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::document_factory::DocumentFactory;
use platform_value::Bytes32;
use platform_version::version::PlatformVersion;

use crate::document::Document;
use crate::state_transition::documents_batch_transition::document_transition::action_type::DocumentTransitionActionType;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;

use crate::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;

use super::get_data_contract_fixture;

pub fn get_document_transitions_fixture<'a>(
    documents: impl IntoIterator<
        Item = (
            DocumentTransitionActionType,
            Vec<(Document, DocumentTypeRef<'a>, Bytes32)>,
        ),
    >,
) -> Vec<DocumentTransition> {
    let protocol_version = PlatformVersion::latest().protocol_version;
    let document_factory =
        DocumentFactory::new(protocol_version).expect("expected to get document factory");

    document_factory
        .create_state_transition(documents)
        .expect("the transitions should be created")
        .transitions()
        .to_owned()
}
