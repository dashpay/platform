use crate::state_transition_action::contract::contract_fee_claim::v0::ContractFeeClaimTransitionActionV0;
use dpp::block::epoch::EpochIndex;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::state_transition::contract_fee_claim_transition::v0::ContractFeeClaimTransitionV0;
use std::collections::BTreeMap;

impl ContractFeeClaimTransitionActionV0 {
    /// The action of a borrowed transition, carrying the epoch and the block time of the claim,
    /// what each recipient is paid and whose moderation action counts the claim resets
    pub fn from_borrowed_transition_with_payouts(
        value: &ContractFeeClaimTransitionV0,
        epoch_index: EpochIndex,
        time_ms: TimestampMillis,
        payouts: BTreeMap<Identifier, Credits>,
        settled_action_counts: Vec<Identifier>,
    ) -> Self {
        let ContractFeeClaimTransitionV0 {
            owner_id,
            data_contract_id,
            identity_contract_nonce,
            pot,
            user_fee_increase,
            ..
        } = value;
        ContractFeeClaimTransitionActionV0 {
            claimant_id: *owner_id,
            data_contract_id: *data_contract_id,
            identity_contract_nonce: *identity_contract_nonce,
            pot: *pot,
            epoch_index,
            time_ms,
            payouts,
            settled_action_counts,
            user_fee_increase: *user_fee_increase,
        }
    }
}
