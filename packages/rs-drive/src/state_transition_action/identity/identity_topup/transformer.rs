use crate::state_transition_action::identity::identity_topup::v0::IdentityTopUpTransitionActionV0;
use crate::state_transition_action::identity::identity_topup::IdentityTopUpTransitionAction;
use dpp::consensus::ConsensusError;
use dpp::dashcore::TxOut;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;

impl IdentityTopUpTransitionAction {
    /// try from
    pub fn try_from(value: IdentityTopUpTransition, output: TxOut) -> Result<Self, ConsensusError> {
        match value {
            IdentityTopUpTransition::V0(v0) => {
                Ok(IdentityTopUpTransitionActionV0::try_from(v0, output)?.into())
            }
        }
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityTopUpTransition,
        output: &TxOut,
    ) -> Result<Self, ConsensusError> {
        match value {
            IdentityTopUpTransition::V0(v0) => {
                Ok(IdentityTopUpTransitionActionV0::try_from_borrowed(v0, output)?.into())
            }
        }
    }
}
