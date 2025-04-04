use derive_more::From;
use dpp::identifier::Identifier;
use dpp::ProtocolError;
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
    /// as_document_action
    pub fn as_document_action(&self) -> Result<&DocumentTransitionAction, ProtocolError> {
        match self {
            BatchedTransitionAction::DocumentAction(action) => Ok(action),
            other => Err(ProtocolError::InvalidBatchedTransitionActionVariant {
                expected: "DocumentAction",
                found: other.variant_name(),
            }),
        }
    }

    /// as_token_action
    pub fn as_token_action(&self) -> Result<&TokenTransitionAction, ProtocolError> {
        match self {
            BatchedTransitionAction::TokenAction(action) => Ok(action),
            other => Err(ProtocolError::InvalidBatchedTransitionActionVariant {
                expected: "TokenAction",
                found: other.variant_name(),
            }),
        }
    }

    /// as_bump_identity_nonce_action
    pub fn as_bump_identity_nonce_action(
        &self,
    ) -> Result<&BumpIdentityDataContractNonceAction, ProtocolError> {
        match self {
            BatchedTransitionAction::BumpIdentityDataContractNonce(action) => Ok(action),
            other => Err(ProtocolError::InvalidBatchedTransitionActionVariant {
                expected: "BumpIdentityDataContractNonce",
                found: other.variant_name(),
            }),
        }
    }

    /// Helper method to get the variant name for diagnostics.
    fn variant_name(&self) -> &'static str {
        match self {
            BatchedTransitionAction::DocumentAction(_) => "DocumentAction",
            BatchedTransitionAction::TokenAction(_) => "TokenAction",
            BatchedTransitionAction::BumpIdentityDataContractNonce(_) => {
                "BumpIdentityDataContractNonce"
            }
        }
    }
}
