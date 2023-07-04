use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;

pub trait DocumentsBatchTransitionV0Methods {
    fn get_transitions(&self) -> &Vec<DocumentTransition>;
    fn get_transitions_slice(&self) -> &[DocumentTransition];
}

impl DocumentsBatchTransitionV0Methods for DataContractCreateTransitionV0 {
    fn get_transitions(&self) -> &Vec<DocumentTransition> {
        &self.transitions
    }

    fn get_transitions_slice(&self) -> &[DocumentTransition] {
        self.transitions.as_slice()
    }
}
