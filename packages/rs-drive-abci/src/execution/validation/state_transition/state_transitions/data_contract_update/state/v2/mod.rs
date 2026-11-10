use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::state_transitions::data_contract_update::delta::{
    as_delta_result, merge_delta_onto_stored_contract, MergedDelta,
};
use crate::execution::validation::state_transition::state_transitions::data_contract_update::state::v1::DataContractUpdateStateTransitionStateValidationV1;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionStateValidationV2
{
    fn validate_state_v2<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractUpdateStateTransitionStateValidationV2 for DataContractUpdateTransition {
    /// Generation 2 (protocol version 15): generation 1 for an update that embeds its
    /// contract, and a delta, which carries only what it changes, is validated.
    ///
    /// A delta is merged onto the stored contract, and the update embedding the merged
    /// contract is validated by generation 1 in the delta's place: the update rules against
    /// the stored contract, the size caps, the identities new groups and tokens name, token
    /// costs, references and moderators. Nothing of that is restated for a delta, so the two
    /// forms cannot drift apart. A delta that does not merge, or whose merged contract breaks
    /// a structure rule, is refused with a nonce bump before generation 1 runs.
    ///
    /// The stored contract is read twice, to merge the delta and by generation 1, and both
    /// reads are billed. The action stores the merged contract and pays the registration cost
    /// of what the delta registers.
    fn validate_state_v2<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let DataContractUpdateTransition::V1(delta) = self else {
            return self.validate_state_v1(
                platform,
                block_info,
                validation_mode,
                execution_context,
                tx,
                platform_version,
            );
        };

        match merge_delta_onto_stored_contract(
            self,
            delta,
            platform,
            block_info,
            validation_mode,
            execution_context,
            tx,
            platform_version,
        )? {
            MergedDelta::Refused(result) => Ok(result),
            MergedDelta::Merged {
                full_contract_update,
                registration_cost,
            } => Ok(as_delta_result(
                full_contract_update.validate_state_v1(
                    platform,
                    block_info,
                    validation_mode,
                    execution_context,
                    tx,
                    platform_version,
                )?,
                registration_cost,
            )),
        }
    }
}
