use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;
use crate::state_transition::documents_batch_transition::DocumentsBatchTransitionV0;

pub trait DocumentsBatchTransitionAccessorsV0 {
    fn transitions(&self) -> &Vec<DocumentTransition>;
    fn transitions_slice(&self) -> &[DocumentTransition];
}
