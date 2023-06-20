use crate::abci::AbciError;
use dpp::validation::SimpleValidationResult;
use tenderdash_abci::proto::abci::{ExecTxResult, ValidatorSetUpdate};

/// The outcome of the block execution, either by prepare proposal, or process proposal
#[derive(Clone)]
pub struct BlockExecutionOutcome {
    /// The app hash, also known as the commit hash, this is the root hash of grovedb
    /// after the block has been executed
    pub app_hash: [u8; 32],
    /// The results of the execution of each transaction
    pub tx_results: Vec<(Vec<u8>, ExecTxResult)>,
    /// The changes to the validator set
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
