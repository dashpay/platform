use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use dpp::consensus::state::data_contract::data_contract_already_present_error::DataContractAlreadyPresentError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::state_transition::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionAction,
};
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransitionAction;
use drive::grovedb::TransactionArg;

pub(in crate::validation::state_transition) trait StateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionStateValidationV0 for DataContractCreateTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;
        // Data contract shouldn't exist
        if drive
            .get_contract_with_fetch_info_and_fee(
                self.data_contract.id.to_buffer(),
                None,
                false,
                tx,
            )?
            .1
            .is_some()
        {
            Ok(ConsensusValidationResult::new_with_errors(vec![
                StateError::DataContractAlreadyPresentError(DataContractAlreadyPresentError::new(
                    self.data_contract.id.to_owned(),
                ))
                .into(),
            ]))
        } else {
            self.transform_into_action_v0::<C>()
        }
    }

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let action: StateTransitionAction =
            Into::<DataContractCreateTransitionAction>::into(self).into();
        Ok(action.into())
    }
}
