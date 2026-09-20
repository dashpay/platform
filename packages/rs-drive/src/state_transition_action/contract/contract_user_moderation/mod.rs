/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::contract::contract_user_moderation::v0::ContractUserModerationTransitionActionV0;
use derive_more::From;
use dpp::data_contract::config::moderation::ContractModerationStatus;
use dpp::platform_value::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationAction;

/// The action of a contract user moderation transition: one edit of a moderated contract's
/// banlist or suspension list, with the target's status as it was read when the transition was
/// validated.
#[derive(Debug, Clone, From)]
pub enum ContractUserModerationTransitionAction {
    /// v0
    V0(ContractUserModerationTransitionActionV0),
}

impl ContractUserModerationTransitionAction {
    /// The moderator that signed
    pub fn moderator_id(&self) -> Identifier {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.moderator_id,
        }
    }

    /// The moderated contract
    pub fn data_contract_id(&self) -> Identifier {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.data_contract_id,
        }
    }

    /// The moderator's nonce for the contract
    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.identity_contract_nonce,
        }
    }

    /// What is done, to whom
    pub fn action(&self) -> ContractUserModerationAction {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.action,
        }
    }

    /// The target's status as read when the transition was validated
    pub fn current_status(&self) -> ContractModerationStatus {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.current_status,
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.user_fee_increase,
        }
    }
}
