use derive_more::From;
use dpp::identifier::Identifier;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::{BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionAccessorsV0};

/// document transition
pub mod document_transition;
/// token transition
pub mod token_transition;

/// token action
#[derive(Debug, Clone, From)]
pub enum BatchedTransitionAction {
    /// document
    DocumentAction(DocumentTransitionAction),
    /// token
    TokenAction(TokenTransitionAction),
    /// bump identity data contract nonce
    BumpIdentityDataContractNonce(BumpIdentityDataContractNonceAction),
}

impl BatchedTransitionAction {
    /// Helper method to get the data contract id
    pub fn data_contract_id(&self) -> Identifier {
        match self {
            BatchedTransitionAction::DocumentAction(document_action) => {
                document_action.base().data_contract_id()
            }
            BatchedTransitionAction::TokenAction(token_action) => {
                token_action.base().data_contract_id()
            }
            BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action) => {
                bump_action.data_contract_id()
            }
        }
    }
}
