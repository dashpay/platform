use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::document_factory::DocumentFactory;
use platform_value::{Bytes32, Identifier};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

use crate::document::Document;
use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::batch_transition::batched_transition::BatchedTransition;
use crate::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;
use crate::tokens::token_payment_info::TokenPaymentInfo;

pub fn get_batched_transitions_fixture<'a>(
    documents: impl IntoIterator<
        Item = (
            DocumentTransitionActionType,
            Vec<(
                Document,
                DocumentTypeRef<'a>,
                Bytes32,
                Option<TokenPaymentInfo>,
            )>,
        ),
    >,
    nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>, //IdentityID/ContractID -> nonce
) -> Vec<BatchedTransition> {
    let protocol_version = PlatformVersion::latest().protocol_version;
    let document_factory =
        DocumentFactory::new(protocol_version).expect("expected to get document factory");

    document_factory
        .create_state_transition(documents, nonce_counter)
        .expect("the transitions should be created")
        .transitions_iter()
        .map(|batched_transition_ref| batched_transition_ref.to_owned_transition())
        .collect()
}
