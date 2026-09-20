mod transformer;

use dpp::data_contract::config::moderation::ContractModerationStatus;
use dpp::identifier::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationAction;

/// action v0
#[derive(Debug, Clone)]
pub struct ContractUserModerationTransitionActionV0 {
    /// the moderator that signed
    pub moderator_id: Identifier,
    /// the moderated contract
    pub data_contract_id: Identifier,
    /// the moderator's nonce for the contract, used to prevent replay attacks
    pub identity_contract_nonce: IdentityNonce,
    /// what is done, to whom
    pub action: ContractUserModerationAction,
    /// the target's status as read when the transition was validated, so that Drive knows
    /// whether a suspend replaces an entry and whether a ban also removes a suspension,
    /// without reading again
    pub current_status: ContractModerationStatus,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
