use crate::abci::AbciError;
use crate::platform_types::state_transition_execution_result::StateTransitionExecutionResult;
use dpp::validation::SimpleValidationResult;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;

/// The outcome of the block execution, either by prepare proposal, or process proposal
#[derive(Clone)]
pub struct BlockExecutionOutcome {
    /// The app hash, also known as the commit hash, this is the root hash of grovedb
    /// after the block has been executed
    pub app_hash: [u8; 32],
    /// The results of the execution of each state transition
    pub state_transition_results: Vec<(Vec<u8>, StateTransitionExecutionResult)>,
    /// The changes to the validator set
    // TODO We should use another DTO, only abci module should deal with Tenderdash proto structures
    pub validator_set_update: Option<ValidatorSetUpdate>,
}

/// The outcome of the finalization of the block
pub struct BlockFinalizationOutcome {
    /// The validation result of the finalization of the block.
    /// Errors here can happen if the block that we receive to be finalized isn't actually
    /// the one we expect, this could be a replay attack or some other kind of attack.
    pub validation_result: SimpleValidationResult<AbciError>,
}

impl From<SimpleValidationResult<AbciError>> for BlockFinalizationOutcome {
    fn from(validation_result: SimpleValidationResult<AbciError>) -> Self {
        BlockFinalizationOutcome { validation_result }
    }
}
