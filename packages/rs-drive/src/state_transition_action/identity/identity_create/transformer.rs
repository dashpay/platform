use crate::state_transition_action::identity::identity_create::v0::IdentityCreateTransitionActionV0;
use crate::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use dpp::consensus::ConsensusError;
use dpp::dashcore::TxOut;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;

impl IdentityCreateTransitionAction {
    /// try from
    pub fn try_from(
        value: IdentityCreateTransition,
        output: TxOut,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityCreateTransition::V0(v0) => {
                Ok(IdentityCreateTransitionActionV0::try_from(v0, output)?.into())
            }
        }
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityCreateTransition,
        output: &TxOut,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityCreateTransition::V0(v0) => {
                Ok(IdentityCreateTransitionActionV0::try_from_borrowed(v0, output)?.into())
            }
        }
    }
}
