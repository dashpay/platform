use crate::state_transition_action::identity::identity_create::v0::IdentityCreateTransitionActionV0;
use crate::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;

impl IdentityCreateTransitionAction {
    /// try from
    pub fn try_from(
        value: IdentityCreateTransition,
        initial_balance_amount: Credits,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityCreateTransition::V0(v0) => {
                Ok(IdentityCreateTransitionActionV0::try_from(v0, initial_balance_amount)?.into())
            }
        }
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityCreateTransition,
        initial_balance_amount: Credits,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityCreateTransition::V0(v0) => Ok(
                IdentityCreateTransitionActionV0::try_from_borrowed(v0, initial_balance_amount)?
                    .into(),
            ),
        }
    }
}
