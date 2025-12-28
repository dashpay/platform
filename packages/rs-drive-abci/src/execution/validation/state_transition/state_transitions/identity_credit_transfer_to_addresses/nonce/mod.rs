use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::identity_credit_transfer_to_addresses::nonce::v0::IdentityCreditTransferToAddressesTransitionIdentityNonceV0;
use crate::execution::validation::state_transition::processor::identity_nonces::StateTransitionIdentityNonceValidationV0;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

pub(crate) mod v0;
impl StateTransitionIdentityNonceValidationV0 for IdentityCreditTransferToAddressesTransition {
    fn validate_identity_nonces(
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
            .identity_credit_transfer_to_addresses_state_transition
            .nonce
        {
            Some(0) => self.validate_nonce_v0(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity credit transfer transition: validate_nonces".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity credit transfer transition: validate_nonces".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
