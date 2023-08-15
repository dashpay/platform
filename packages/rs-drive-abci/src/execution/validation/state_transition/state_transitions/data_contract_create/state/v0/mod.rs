use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use dpp::consensus::state::data_contract::data_contract_already_present_error::DataContractAlreadyPresentError;
use dpp::consensus::state::state_error::StateError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;

use dpp::version::{PlatformVersion, TryIntoPlatformVersioned};
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreateStateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractCreateStateTransitionStateValidationV0 for DataContractCreateTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;
        // Data contract shouldn't exist
        if drive
            .get_contract_with_fetch_info_and_fee(
                self.data_contract().id().to_buffer(),
                None,
                false,
                tx,
                platform_version,
            )?
            .1
            .is_some()
        {
            Ok(ConsensusValidationResult::new_with_errors(vec![
                StateError::DataContractAlreadyPresentError(DataContractAlreadyPresentError::new(
                    self.data_contract().id().to_owned(),
                ))
                .into(),
            ]))
        } else {
            self.transform_into_action_v0::<C>(platform_version)
        }
    }

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let create_action: DataContractCreateTransitionAction =
            self.try_into_platform_versioned(platform_version)?;
        let action: StateTransitionAction = create_action.into();
        Ok(action.into())
    }
}
