use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
use crate::rpc::core::CoreRPCLike;
use dpp::bls_signatures;
use dpp::validation::SimpleValidationResult;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::abci::ExtendVoteExtension;
use tenderdash_abci::proto::types::VoteExtension;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Version-dependent method that checks the validity of withdrawal transactions.
    ///
    /// Based on the `platform_version` passed, this function will route to the appropriate versioned
    /// implementation of the `check_withdrawals` method. Each implementation will compare the received
    /// withdrawal transactions with the expected ones, returning an error if they do not match.
    ///
    /// If a validator public key is provided, each versioned method also verifies the withdrawal
    /// transactions' signatures. The `platform_version` parameter dictates which version of the
    /// method to call. If an unsupported version is passed, the function will return an
    /// `Error::Execution` with an `ExecutionError::UnknownVersionMismatch` error.
    ///
    /// # Arguments
    ///
    /// * `received_withdrawals` - The withdrawal transactions received.
    /// * `our_withdrawals` - The expected withdrawal transactions.
    /// * `height` - The block height.
    /// * `round` - The consensus round.
    /// * `verify_with_validator_public_key` - An optional reference to a validator public key.
    /// * `quorum_hash` - An optional byte slice reference containing the quorum hash.
    /// * `platform_version` - A `PlatformVersion` reference dictating which version of the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleValidationResult<AbciError>, Error>` - On success, a `SimpleValidationResult`
    ///   containing an `AbciError` is returned. On error, an `Error` is returned.
    ///
    pub(crate) fn check_withdrawals(
        &self,
        received_withdrawals: &Vec<VoteExtension>,
        our_withdrawals: &UnsignedWithdrawalTxs,
        height: u64,
        round: u32,
        verify_with_validator_public_key: Option<&bls_signatures::PublicKey>,
        quorum_hash: Option<&[u8]>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleValidationResult<AbciError>, Error> {
        // TODO: Revisit this function. Do we even need it?
        match platform_version
            .drive_abci
            .methods
            .withdrawals
            .check_withdrawals
        {
            0 => Ok(self.check_withdrawals_v0(
                received_withdrawals,
                our_withdrawals,
                height,
                round,
                verify_with_validator_public_key,
                quorum_hash,
            )),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "check_withdrawals".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
