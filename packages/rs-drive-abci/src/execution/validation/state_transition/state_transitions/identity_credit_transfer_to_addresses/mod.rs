mod balance;
mod nonce;
mod transform_into_action;

use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
use dpp::validation::ConsensusValidationResult;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::identity_credit_transfer_to_addresses::transform_into_action::v0::IdentityCreditTransferToAddressesStateTransitionStateValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformer;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

impl StateTransitionActionTransformer for IdentityCreditTransferToAddressesTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _block_info: &BlockInfo,
        _remaining_address_input_balances: &Option<
            BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        >,
        _validation_mode: ValidationMode,
        _execution_context: &mut StateTransitionExecutionContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_credit_transfer_to_addresses_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity credit transfer transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
