use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::consensus::state::contract_moderation::{
    ContractFeeClaimNotAllowedError, ContractFeesAlreadyClaimedThisEpochError,
    ContractFeesNothingToClaimError,
};
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::prelude::{ConsensusValidationResult, Identifier};
use dpp::state_transition::contract_fee_claim_transition::accessors::ContractFeeClaimTransitionAccessorsV0;
use dpp::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;
use dpp::state_transition::StateTransitionOwned;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::contract_fee_claim::ContractFeeClaimTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::StateTransitionAction;
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::execution::validation::state_transition::state_transitions::contract_fee_claim) trait ContractFeeClaimStateTransitionStateValidationV0
{
    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ContractFeeClaimStateTransitionStateValidationV0 for ContractFeeClaimTransition {
    /// Reads the contract and the pot and settles the payout: the signer is a recipient of the
    /// pot, the pot was not claimed in this epoch yet, and it holds enough to pay every
    /// recipient something. Every refusal after the contract is found is paid for by bumping
    /// the signer's contract nonce, and a refused claim leaves the pot's last claim epoch
    /// alone.
    ///
    /// The action carries what each recipient is paid, so Drive pays the pot out without
    /// reading it again, and the mempool, which transforms without a state validation stage,
    /// refuses with the same consensus codes as a block.
    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let contract_id = self.data_contract_id();
        let claimant_id = self.owner_id();
        let pot = self.pot();

        let Some(contract_fetch_info) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                contract_id.to_buffer(),
                Some(&block_info.epoch),
                false,
                tx,
                platform_version,
            )?
            .1
        else {
            return Ok(ConsensusValidationResult::new_with_error(
                DataContractNotPresentError::new(contract_id).into(),
            ));
        };
        if let Some(fee) = contract_fetch_info.fee.clone() {
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
        }

        let bump_action = || {
            StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_contract_fee_claim_transition(
                    self,
                ),
            )
        };
        let refuse = |error: ConsensusError| {
            Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action(),
                vec![error],
            ))
        };

        // Only who a payout of the pot goes to may claim it: the contract owner for the owner
        // pot, a member of the moderation team for the moderators pot. A contract that
        // declares no moderation has no team, so nobody claims its moderators pot.
        let recipients = pot.recipients(&contract_fetch_info.contract);
        if !recipients.contains(&claimant_id) {
            return refuse(
                ContractFeeClaimNotAllowedError::new(contract_id, pot, claimant_id).into(),
            );
        }

        let (fee, fee_pot) = platform.drive.fetch_contract_fee_pot_with_fee(
            contract_id,
            pot,
            &block_info.epoch,
            tx,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

        let epoch_index = block_info.epoch.index;
        if fee_pot.last_claim_epoch == Some(epoch_index) {
            return refuse(
                ContractFeesAlreadyClaimedThisEpochError::new(contract_id, pot, epoch_index).into(),
            );
        }

        let Some(payouts) = split_between(&recipients, fee_pot.credits) else {
            return refuse(ContractFeesNothingToClaimError::new(contract_id, pot).into());
        };

        Ok(ConsensusValidationResult::new_with_data(
            ContractFeeClaimTransitionAction::from_borrowed_transition_with_payouts(
                self,
                epoch_index,
                payouts,
            )
            .into(),
        ))
    }
}

/// What each of `recipients` is paid out of `credits`: an equal share each, `None` when there
/// is nobody to pay or the share rounds down to nothing. What the equal split leaves over, less
/// than one credit per recipient, stays in the pot for the next claim, so no recipient is
/// favoured by the order of their identity ids.
///
/// For the owner pot the only recipient is the contract owner, whose share is the whole pot.
fn split_between(
    recipients: &BTreeSet<Identifier>,
    credits: Credits,
) -> Option<BTreeMap<Identifier, Credits>> {
    let share = credits.checked_div(recipients.len() as Credits)?;
    if share == 0 {
        return None;
    }
    Some(
        recipients
            .iter()
            .map(|recipient| (*recipient, share))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(seeds: &[u8]) -> BTreeSet<Identifier> {
        seeds
            .iter()
            .map(|seed| Identifier::from([*seed; 32]))
            .collect()
    }

    #[test]
    fn should_pay_a_single_recipient_the_whole_pot() {
        let payouts = split_between(&ids(&[1]), 1_001).expect("expected a payout");
        assert_eq!(
            payouts,
            BTreeMap::from([(Identifier::from([1; 32]), 1_001)])
        );
    }

    #[test]
    fn should_pay_equal_shares_and_leave_the_remainder_in_the_pot() {
        let payouts = split_between(&ids(&[1, 2, 3]), 1_001).expect("expected a payout");
        assert_eq!(payouts.len(), 3);
        assert!(payouts.values().all(|share| *share == 333));
        let paid_out: Credits = payouts.values().sum();
        assert_eq!(1_001 - paid_out, 2);
    }

    #[test]
    fn should_pay_nothing_when_there_is_nobody_to_pay_or_less_than_a_credit_each() {
        assert_eq!(split_between(&ids(&[]), 1_000), None);
        assert_eq!(split_between(&ids(&[1, 2, 3]), 2), None);
        assert_eq!(split_between(&ids(&[1]), 0), None);
    }
}
