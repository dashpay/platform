use dpp::block::block_info::BlockInfo;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use drive::grovedb::TransactionArg;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::data_contract_update::identity_contract_nonce::v0::DataContractUpdateStateTransitionIdentityContractNonceV0;
use crate::execution::validation::state_transition::processor::v0::{StateTransitionNonceValidationV0};
use crate::platform_types::platform::{PlatformStateRef};

pub(crate) mod v0;

impl StateTransitionNonceValidationV0 for DataContractUpdateTransition {
    fn validate_nonces(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .nonce
        {
            Some(0) => self.validate_identity_contract_nonce_v0(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: validate_identity_contract_nonce"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "data contract update transition: validate_identity_contract_nonce"
                    .to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
