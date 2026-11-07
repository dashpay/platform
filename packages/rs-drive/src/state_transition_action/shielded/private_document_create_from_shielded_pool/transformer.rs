use crate::state_transition_action::shielded::private_document_create_from_shielded_pool::v0::PrivateDocumentCreateFromShieldedPoolTransitionActionV0;
use crate::state_transition_action::shielded::private_document_create_from_shielded_pool::PrivateDocumentCreateFromShieldedPoolTransitionAction;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;

impl PrivateDocumentCreateFromShieldedPoolTransitionAction {
    /// Transforms the state transition into an action, once the transformer has checked the
    /// pool, the target type and the entries' size against state
    pub fn try_from_transition(
        value: &PrivateDocumentCreateFromShieldedPoolTransition,
        current_total_balance: Credits,
        fee_amount: Credits,
    ) -> ConsensusValidationResult<Self> {
        match value {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => {
                PrivateDocumentCreateFromShieldedPoolTransitionActionV0::try_from_transition(
                    v0,
                    current_total_balance,
                    fee_amount,
                )
                .map(|action| action.into())
            }
        }
    }
}
