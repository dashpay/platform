mod transformer;

use dpp::block::epoch::EpochIndex;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::prelude::{IdentityNonce, TimestampMillis, UserFeeIncrease};
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
    /// the epoch of the claim, recorded with the pot's last claim
    pub epoch_index: EpochIndex,
    /// the time of the block the claim executes in, recorded with the pot's last claim
    pub time_ms: TimestampMillis,
    /// what each recipient is paid, as settled when the transition was validated: the whole
    /// owner pot to the contract owner, an equal share of the moderators pot to every member
    /// of a declared moderation team, or a seated team's pot split by its proposal's reward
    /// split. Never empty, and every amount is above zero.
    pub payouts: BTreeMap<Identifier, Credits>,
    /// the members of an elected contract's seated team whose moderation action counts the
    /// claim of its moderators pot split the pot by and resets; empty for every other claim
    pub settled_action_counts: Vec<Identifier>,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
