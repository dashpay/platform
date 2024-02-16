
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::validation::SimpleConsensusValidationResult;
use drive::grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::identity_credit_withdrawal::nonce::v0::IdentityCreditWithdrawalTransitionIdentityContractNonceV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionNonceValidationV0;
use crate::platform_types::platform::PlatformStateRef;

pub(crate) mod v0;
impl StateTransitionNonceValidationV0 for IdentityCreditWithdrawalTransition {
    fn validate_nonces(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_credit_withdrawal_state_transition
            .nonce
        {
            Some(0) => {
                self.validate_nonce_v0(platform, block_info, tx, platform_version)
            }
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity credit withdrawal transition: validate_nonces"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity credit withdrawal transition: validate_nonces"
                    .to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
