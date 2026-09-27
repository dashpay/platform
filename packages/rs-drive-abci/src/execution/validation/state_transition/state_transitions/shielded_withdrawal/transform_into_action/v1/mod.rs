use super::v0::ShieldedWithdrawalStateTransitionTransformIntoActionValidationV0;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::stamp_withdrawal_document;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shielded_withdrawal::ShieldedWithdrawalTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::shielded_withdrawal) trait ShieldedWithdrawalStateTransitionTransformIntoActionValidationV1
{
    fn transform_into_action_v1(
        &self,
        drive: &Drive,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ShieldedWithdrawalStateTransitionTransformIntoActionValidationV1
    for ShieldedWithdrawalTransition
{
    fn transform_into_action_v1(
        &self,
        drive: &Drive,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        self.transform_into_action_v0(drive, block_info, transaction, platform_version)
            .and_then(|result| {
                result.map_result(|mut action| {
                    match &mut action {
                        StateTransitionAction::ShieldedWithdrawalAction(
                            ShieldedWithdrawalTransitionAction::V0(withdrawal),
                        ) => stamp_withdrawal_document(
                            &mut withdrawal.prepared_withdrawal_document,
                            platform_version,
                        ),
                        _ => {
                            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                                "shielded withdrawal transformer returned an unrelated action",
                            )));
                        }
                    }
                    Ok(action)
                })
            })
    }
}
