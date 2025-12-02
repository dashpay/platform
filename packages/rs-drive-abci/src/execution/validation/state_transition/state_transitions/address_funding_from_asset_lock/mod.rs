mod tests;
mod transform_into_action;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use dpp::validation::ConsensusValidationResult;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::address_funding_from_asset_lock::transform_into_action::v0::AddressFundingFromAssetLockStateTransitionTransformIntoActionValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

/// A trait to transform into an action for address funding from asset lock
pub trait StateTransitionAddressFundingFromAssetLockTransitionActionTransformer {
    /// Transform into an action for address funding from asset lock
    fn transform_into_action_for_address_funding_from_asset_lock_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionAddressFundingFromAssetLockTransitionActionTransformer
    for AddressFundingFromAssetLockTransition
{
    fn transform_into_action_for_address_funding_from_asset_lock_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .address_funds_from_asset_lock
            .transform_into_action
        {
            0 => self.transform_into_action_v0(
                platform,
                signable_bytes,
                inputs_with_remaining_balance,
                validation_mode,
                execution_context,
                tx,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "address funding from asset lock transition: transform_into_action"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
