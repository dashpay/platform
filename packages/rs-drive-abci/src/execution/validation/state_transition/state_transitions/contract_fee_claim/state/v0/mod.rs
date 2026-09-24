use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::seated_moderation_charter::{
    fetch_seated_moderation_charter, SeatedModerationCharter,
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
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
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
    /// recipient something. Every refusal, a contract that does not exist included, is paid for
    /// by bumping the signer's contract nonce, and a refused claim leaves the pot's last claim
    /// epoch alone.
    ///
    /// The moderators pot of an elected contract is read first for its seated charter (the
    /// `byTargetContract` query, billed) whoever claims, since any identity may be on a seated
    /// team: with one seated, the claim is the team's (see `claim_seated_moderators_pot_v0`),
    /// and without, the interim team's as the declaration names it. A claim refused for a
    /// signer who is neither therefore pays that query too.
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

        let (contract_fetch_fee, maybe_contract_fetch_info) =
            platform.drive.get_contract_with_fetch_info_and_fee(
                contract_id.to_buffer(),
                Some(&block_info.epoch),
                false,
                tx,
                platform_version,
            )?;
        // The read is billed from the fee this call returns, whether the contract was pulled
        // from disk, was in the cache or does not exist. The fee a cached fetch info carries is
        // only there when that entry was built with an epoch, which differs from node to node,
        // so billing it would make the fee, and the app hash, depend on the cache.
        let contract_fetch_fee =
            contract_fetch_fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "fee must exist for the contract fetch of a contract fee claim transition",
            )))?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(
            contract_fetch_fee,
        ));

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

        // Paid like every other refusal: the signer is authenticated and the lookup happened,
        // as for a contract update of a contract that does not exist.
        let Some(contract_fetch_info) = maybe_contract_fetch_info else {
            return refuse(DataContractNotPresentError::new(contract_id).into());
        };

        // An elected contract's moderators pot is its seated team's once a charter is seated
        // (decentralized moderation teams): the leader or an active member claims it for the
        // team, and it is split by the proposal's reward split. Whether one is seated is read,
        // billed. Until then it is its interim team's, as the declaration names it.
        let elected = contract_fetch_info
            .contract
            .config()
            .moderation()
            .and_then(|moderation| moderation.moderators.elected());
        if let (ContractFeePot::Moderators, Some(elected)) = (pot, elected) {
            if let Some(charter) = fetch_seated_moderation_charter(
                platform.drive,
                contract_id,
                &block_info.epoch,
                execution_context,
                tx,
                platform_version,
            )? {
                return claim_seated_moderators_pot_v0(
                    self,
                    platform,
                    block_info,
                    &charter,
                    elected.max_added_moderators,
                    execution_context,
                    tx,
                    platform_version,
                );
            }
        }

        // Only who a payout of the pot goes to may claim it: the contract owner for the owner
        // pot, a member of the moderation team for the moderators pot. A contract that
        // declares no moderation has no team, so nobody claims its moderators pot. The
        // recipients of an elected contract's moderators pot are its interim team, who claim it
        // only until a charter is seated: from then on it is the seated team's (above), and it
        // accumulates for that team, unsettled, as it does under an interim that names nobody.
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
        if fee_pot.last_claim_epoch() == Some(epoch_index) {
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
                block_info.time_ms,
                payouts,
                vec![],
            )
            .into(),
        ))
    }
}

/// The claim of the moderators pot of an elected contract with a seated charter: the
/// signer is the leader or an active member of the seated team, the pot was not claimed in
/// this epoch yet, and the proposal's reward split pays someone at least a credit. The team
/// is read once (the charter's removals and additions), then the pot, the proposal and the
/// team's moderation action counts, all billed, and the claim resets the counts. Every
/// refusal is paid for by bumping the signer's contract nonce.
#[allow(clippy::too_many_arguments)]
fn claim_seated_moderators_pot_v0<C: CoreRPCLike>(
    transition: &ContractFeeClaimTransition,
    platform: &PlatformRef<C>,
    block_info: &BlockInfo,
    charter: &SeatedModerationCharter,
    max_added_moderators: u16,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
    let contract_id = transition.data_contract_id();
    let claimant_id = transition.owner_id();
    let pot = ContractFeePot::Moderators;
    let epoch = &block_info.epoch;
    let refuse = |error: ConsensusError| {
        Ok(ConsensusValidationResult::new_with_data_and_errors(
            StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_contract_fee_claim_transition(
                    transition,
                ),
            ),
            vec![error],
        ))
    };

    // The team is read once: whether the claimant is on it, and who the split pays. The
    // interim moderators, the owner among them, claim no more once a charter is seated.
    let members = charter.fetch_active_members(
        platform.drive,
        max_added_moderators,
        epoch,
        execution_context,
        tx,
        platform_version,
    )?;
    if claimant_id != charter.leader_id && !members.contains(&claimant_id) {
        return refuse(ContractFeeClaimNotAllowedError::new(contract_id, pot, claimant_id).into());
    }

    let (fee, fee_pot) = platform.drive.fetch_contract_fee_pot_with_fee(
        contract_id,
        pot,
        epoch,
        tx,
        platform_version,
    )?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

    let epoch_index = epoch.index;
    if fee_pot.last_claim_epoch() == Some(epoch_index) {
        return refuse(
            ContractFeesAlreadyClaimedThisEpochError::new(contract_id, pot, epoch_index).into(),
        );
    }

    let settlement = charter.settle_moderators_pot_among(
        &members,
        platform.drive,
        contract_id,
        fee_pot.credits,
        max_added_moderators,
        epoch,
        execution_context,
        tx,
        platform_version,
    )?;
    // A claim that would pay nobody a credit is refused, as for a declared team; what the
    // split leaves over waits in the pot for the next settle.
    if settlement.payouts.is_empty() {
        return refuse(ContractFeesNothingToClaimError::new(contract_id, pot).into());
    }

    Ok(ConsensusValidationResult::new_with_data(
        ContractFeeClaimTransitionAction::from_borrowed_transition_with_payouts(
            transition,
            epoch_index,
            block_info.time_ms,
            settlement.payouts,
            settlement.settled_action_counts,
        )
        .into(),
    ))
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
