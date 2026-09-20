use crate::state_transition_action::contract::contract_fee_claim::v0::ContractFeeClaimTransitionActionV0;
use dpp::block::epoch::EpochIndex;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::state_transition::contract_fee_claim_transition::v0::ContractFeeClaimTransitionV0;
use std::collections::BTreeMap;

impl ContractFeeClaimTransitionActionV0 {
    /// The action of a borrowed transition, carrying the epoch of the claim and what each
    /// recipient is paid
    pub fn from_borrowed_transition_with_payouts(
        value: &ContractFeeClaimTransitionV0,
        epoch_index: EpochIndex,
        payouts: BTreeMap<Identifier, Credits>,
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
            payouts,
            user_fee_increase: *user_fee_increase,
        }
    }
}
