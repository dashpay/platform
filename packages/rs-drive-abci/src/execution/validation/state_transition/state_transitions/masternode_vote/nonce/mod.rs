use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::masternode_vote::nonce::v0::MasternodeVoteTransitionIdentityNonceV0;
use crate::execution::validation::state_transition::masternode_vote::nonce::v1::MasternodeVoteTransitionIdentityNonceV1;
use crate::execution::validation::state_transition::processor::v0::StateTransitionNonceValidationV0;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

pub(crate) mod v0;
pub(crate) mod v1;

impl StateTransitionNonceValidationV0 for MasternodeVoteTransition {
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
            .masternode_vote_state_transition
            .nonce
        {
            Some(0) => self.validate_nonce_v0(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            Some(1) => self.validate_nonce_v1(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "masternode vote transition: validate_nonces".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "masternode vote transition: validate_nonces".to_string(),
                known_versions: vec![0, 1],
            })),
        }
    }
}
