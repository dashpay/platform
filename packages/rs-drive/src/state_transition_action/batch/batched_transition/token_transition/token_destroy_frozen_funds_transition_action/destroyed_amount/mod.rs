use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::TokenDestroyFrozenFundsTransitionAction;
use dpp::balances::credits::TokenAmount;
use platform_version::version::PlatformVersion;

mod v0;
mod v1;

impl TokenDestroyFrozenFundsTransitionAction {
    /// What the transition's history and group-action events record as destroyed, as the
    /// converter generation `platform_version` selects
    /// (`token_destroy_frozen_funds_transition`): the frozen balance alone in generation 0, and
    /// the balance with what the listings hold in escrow from generation 1 (protocol version
    /// 15). The converter and the shared builders (`associated_token_event`,
    /// `build_historical_document`) all read it, so they record the same amount.
    pub fn destroyed_amount(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<TokenAmount, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .token_destroy_frozen_funds_transition
        {
            0 => Ok(self.destroyed_amount_v0()),
            1 => self.destroyed_amount_v1(),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "TokenDestroyFrozenFundsTransitionAction::destroyed_amount".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
