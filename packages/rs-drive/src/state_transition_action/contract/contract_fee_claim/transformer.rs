use crate::state_transition_action::contract::contract_fee_claim::v0::ContractFeeClaimTransitionActionV0;
use crate::state_transition_action::contract::contract_fee_claim::ContractFeeClaimTransitionAction;
use dpp::block::epoch::EpochIndex;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;
use std::collections::BTreeMap;

impl ContractFeeClaimTransitionAction {
    /// The action of a borrowed transition, carrying the epoch and the block time of the claim,
    /// what each recipient is paid and whose moderation action counts the claim resets
    pub fn from_borrowed_transition_with_payouts(
        value: &ContractFeeClaimTransition,
        epoch_index: EpochIndex,
        time_ms: TimestampMillis,
        payouts: BTreeMap<Identifier, Credits>,
        settled_action_counts: Vec<Identifier>,
    ) -> Self {
        match value {
            ContractFeeClaimTransition::V0(v0) => {
                ContractFeeClaimTransitionActionV0::from_borrowed_transition_with_payouts(
                    v0,
                    epoch_index,
                    time_ms,
                    payouts,
                    settled_action_counts,
                )
                .into()
            }
        }
    }
}
