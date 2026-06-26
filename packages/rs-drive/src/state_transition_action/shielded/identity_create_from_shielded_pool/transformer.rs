use crate::state_transition_action::shielded::identity_create_from_shielded_pool::v0::IdentityCreateFromShieldedPoolTransitionActionV0;
use crate::state_transition_action::shielded::identity_create_from_shielded_pool::IdentityCreateFromShieldedPoolTransitionAction;
use dpp::fee::Credits;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

impl IdentityCreateFromShieldedPoolTransitionAction {
    /// Transforms the state transition into an action.
    pub fn try_from_transition(
        value: &IdentityCreateFromShieldedPoolTransition,
        current_total_balance: Credits,
        fee_amount: Credits,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => {
                let action = IdentityCreateFromShieldedPoolTransitionActionV0::try_from_transition(
                    v0,
                    current_total_balance,
                    fee_amount,
                    platform_version,
                )?;
                Ok(action.into())
            }
        }
    }
}
