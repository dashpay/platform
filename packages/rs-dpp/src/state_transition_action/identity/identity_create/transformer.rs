use crate::consensus::ConsensusError;
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::state_transition_action::identity::identity_create::v0::IdentityCreateTransitionActionV0;
use crate::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;

impl IdentityCreateTransitionAction {
    pub fn try_from(
        value: IdentityCreateTransition,
        initial_balance_amount: u64,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityCreateTransition::V0(v0) => {
                Ok(IdentityCreateTransitionActionV0::try_from(v0, initial_balance_amount)?.into())
            }
        }
    }

    pub fn try_from_borrowed(
        value: &IdentityCreateTransition,
        initial_balance_amount: u64,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityCreateTransition::V0(v0) => Ok(
                IdentityCreateTransitionActionV0::try_from_borrowed(v0, initial_balance_amount)?
                    .into(),
            ),
        }
    }
}
