use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::ConsensusError;
use dpp::fee::fee_result::FeeResult;
use dpp::validation::ValidationResult;
use drive::fee::result::FeeResult;

mod v0;

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
    ) -> Result<ValidationResult<FeeResult, ConsensusError>, Error> {
        //todo: use protocol version to determine version
        self.check_tx_v0(raw_tx)
    }
}
