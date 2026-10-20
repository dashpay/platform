use super::v0::IdentityCreditWithdrawalStateTransitionStateValidationV0;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::state_transitions::stamp_withdrawal_document;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::identity_credit_withdrawal) trait IdentityCreditWithdrawalStateTransitionStateValidationV1 {
    fn transform_into_action_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreditWithdrawalStateTransitionStateValidationV1
    for IdentityCreditWithdrawalTransition
{
    fn transform_into_action_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        self.transform_into_action_v0(
            platform,
            block_info,
            execution_context,
            tx,
            platform_version,
        )
        .and_then(|result| {
            result.map_result(|mut action| {
                match &mut action {
                    StateTransitionAction::IdentityCreditWithdrawalAction(
                        IdentityCreditWithdrawalTransitionAction::V0(withdrawal),
                    ) => stamp_withdrawal_document(
                        &mut withdrawal.prepared_withdrawal_document,
                        platform_version,
                    ),
                    _ => {
                        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "identity withdrawal transformer returned an unrelated action",
                        )));
                    }
                }
                Ok(action)
            })
        })
    }
}
