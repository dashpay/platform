use crate::platform::Platform;
use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransitionAction;
use dpp::identity::state_transition::identity_topup_transition::IdentityTopUpTransitionAction;
use dpp::state_transition::{StateTransition, StateTransitionAction};

impl Platform {
    pub(crate) fn convert_state_transition(
        &self,
        state_transition: &StateTransition,
    ) -> StateTransitionAction {
        match state_transition {
            StateTransition::DataContractCreate(data_contract_create) => {
                StateTransitionAction::DataContractCreateAction(data_contract_create.into())
            }
            StateTransition::DataContractUpdate(data_contract_update) => {
                StateTransitionAction::DataContractUpdateAction(data_contract_update.into())
            }
            StateTransition::DocumentsBatch(documents_batch) => {
                StateTransitionAction::DocumentsBatchAction(documents_batch.into())
            }
            StateTransition::IdentityCreate(identity_create) => {
                //todo: figure out balance
                StateTransitionAction::IdentityCreateAction(
                    IdentityCreateTransitionAction::from_borrowed(identity_create, top_up_balance),
                )
            }
            StateTransition::IdentityTopUp(identity_top_up) => {
                //todo: figure out balance
                StateTransitionAction::IdentityTopUpAction(
                    IdentityTopUpTransitionAction::from_borrowed(identity_top_up, top_up_balance),
                )
            }
            StateTransition::IdentityCreditWithdrawal(identity_credit_withdrawal) => {
                StateTransitionAction::IdentityCreditWithdrawalAction(
                    identity_credit_withdrawal.into(),
                )
            }
            StateTransition::IdentityUpdate(identity_update) => {
                StateTransitionAction::IdentityUpdateAction(identity_update.into())
            }
        }
    }
    pub(crate) fn convert_state_transitions(
        &self,
        state_transitions: &Vec<StateTransition>,
    ) -> Vec<StateTransitionAction> {
        state_transitions
            .iter()
            .map(|state_transition| self.convert_state_transition(state_transition))
            .collect()
    }
}
