mod transformer;

use dpp::block::epoch::EpochIndex;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// action v0
#[derive(Debug, Clone)]
pub struct ContractFeeClaimTransitionActionV0 {
    /// the claimant that signed
    pub claimant_id: Identifier,
    /// the contract whose pot is paid out
    pub data_contract_id: Identifier,
    /// the claimant's nonce for the contract, used to prevent replay attacks
    pub identity_contract_nonce: IdentityNonce,
    /// the pot that is paid out
    pub pot: ContractFeePot,
    /// the epoch of the claim, recorded as the pot's last claim epoch
    pub epoch_index: EpochIndex,
    /// what each recipient is paid, as settled when the transition was validated: the whole
    /// owner pot to the contract owner, or an equal share of the moderators pot to every
    /// member of the moderation team. Never empty, and every amount is above zero.
    pub payouts: BTreeMap<Identifier, Credits>,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
