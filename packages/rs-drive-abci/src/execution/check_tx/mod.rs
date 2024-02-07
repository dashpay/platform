use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::abci::AbciError;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::ConsensusError;
use dpp::fee::fee_result::FeeResult;
use dpp::validation::ValidationResult;

mod v0;

// @append_only
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum CheckTxLevel {
    FirstTimeCheck,
    Recheck,
}

impl TryFrom<u8> for CheckTxLevel {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CheckTxLevel::FirstTimeCheck),
            1 => Ok(CheckTxLevel::Recheck),
            value => Err(Error::Abci(AbciError::BadRequest(format!(
                "Invalid value for CheckTxLevel {}",
                value
            )))),
        }
    }
}

impl TryFrom<i32> for CheckTxLevel {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CheckTxLevel::FirstTimeCheck),
            1 => Ok(CheckTxLevel::Recheck),
            value => Err(Error::Abci(AbciError::BadRequest(format!(
                "Invalid value for CheckTxLevel {}",
                value
            )))),
        }
    }
}

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks a state transition to determine if it should be added to the mempool.
    ///
    /// This function performs a few checks, including validating the state transition and ensuring that the
    /// user can pay for it. It may be inaccurate in rare cases, so the proposer needs to re-check transactions
    /// before proposing a block.
    ///
    /// # Arguments
    ///
    /// * `raw_tx` - A raw transaction represented as a vector of bytes.
    ///
    /// # Returns
    ///
    /// * `Result<ValidationResult<FeeResult, ConsensusError>, Error>` - If the state transition passes all
    ///   checks, it returns a `ValidationResult` with fee information. If any check fails, it returns an `Error`.
    pub fn check_tx(
        &self,
        raw_tx: &[u8],
        check_tx_level: CheckTxLevel,
    ) -> Result<ValidationResult<Option<FeeResult>, ConsensusError>, Error> {
        let state = self.state.read().expect("expected to get state");
        let platform_version = state.current_platform_version()?;
        match platform_version.drive_abci.methods.engine.check_tx {
            0 => self.check_tx_v0(raw_tx, check_tx_level, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "check_tx".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
