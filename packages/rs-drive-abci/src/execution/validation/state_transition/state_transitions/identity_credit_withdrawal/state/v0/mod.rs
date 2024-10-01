use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;

use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::identity_credit_withdrawal) trait IdentityCreditWithdrawalStateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreditWithdrawalStateTransitionStateValidationV0
    for IdentityCreditWithdrawalTransition
{
    fn validate_state_v0<C: CoreRPCLike>(
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
    }

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let consensus_validation_result =
            IdentityCreditWithdrawalTransitionAction::try_from_identity_credit_withdrawal(
                platform.drive,
                tx,
                self,
                block_info,
                platform_version,
            )
            .map(|consensus_validation_result| {
                consensus_validation_result.map(|withdrawal| withdrawal.into())
            })?;
        if consensus_validation_result.is_valid() {
            // If this is valid then we will apply the action and eventually perform network threshold signing
            execution_context.add_operation(ValidationOperation::PerformNetworkThresholdSigning);
        }
        Ok(consensus_validation_result)
    }
}
