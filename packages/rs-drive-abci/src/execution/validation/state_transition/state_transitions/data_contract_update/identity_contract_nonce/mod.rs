use dpp::block::block_info::BlockInfo;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use drive::grovedb::TransactionArg;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::data_contract_update::identity_contract_nonce::v0::DataContractUpdateStateTransitionIdentityContractNonceV0;
use crate::execution::validation::state_transition::data_contract_update::identity_contract_nonce::v1::DataContractUpdateStateTransitionIdentityContractNonceV1;
use crate::execution::validation::state_transition::data_contract_update::unsupported_delta_error;
use crate::execution::validation::state_transition::processor::identity_nonces::StateTransitionIdentityNonceValidationV0;
use crate::platform_types::platform::{PlatformStateRef};

pub(crate) mod v0;
pub(crate) mod v1;

impl StateTransitionIdentityNonceValidationV0 for DataContractUpdateTransition {
    fn validate_identity_nonces(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let version = platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .nonce;
        // Generation 0 reads the owner and the contract id from the contract an update
        // embeds. A delta, which embeds none, is not active at the protocol versions that
        // select it and is refused at decode there; one that got here anyway is refused as an
        // unsupported version, a consensus error, before it runs
        if matches!(self, DataContractUpdateTransition::V1(_)) && version == Some(0) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                unsupported_delta_error(self, platform_version),
            ));
        }
        match version {
            Some(0) => self.validate_identity_contract_nonce_v0(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            Some(1) => self.validate_identity_contract_nonce_v1(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: validate_identity_contract_nonce"
                    .to_string(),
                known_versions: vec![0, 1],
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
