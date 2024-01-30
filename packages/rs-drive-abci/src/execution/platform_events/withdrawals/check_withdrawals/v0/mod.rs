use crate::abci::AbciError;
use crate::platform_types::platform::Platform;
use crate::platform_types::withdrawal::withdrawal_txs;
use crate::rpc::core::CoreRPCLike;
use dpp::bls_signatures;
use dpp::validation::SimpleValidationResult;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks if the received withdrawal transactions are correct and match the expected withdrawal transactions.
    ///
    /// This function compares the received withdrawal transactions with the expected ones. If they don't match,
    /// an error is returned. If a validator public key is provided, the function also verifies the withdrawal
    /// transactions' signatures.
    ///
    /// # Arguments
    ///
    /// * `received_withdrawals` - The withdrawal transactions received.
    /// * `our_withdrawals` - The expected withdrawal transactions.
    /// * `height` - The block height.
    /// * `round` - The consensus round.
    /// * `verify_with_validator_public_key` - An optional reference to a validator public key.
    /// * `quorum_hash` - An optional byte slice reference containing the quorum hash.
    ///
    /// # Returns
    ///
    /// * `SimpleValidationResult<AbciError>` - If the received withdrawal transactions match the expected ones
    ///   and the signatures are valid (if provided), it returns a default `SimpleValidationResult`. Otherwise,
    ///   it returns a `SimpleValidationResult` with an error.
    ///
    pub(super) fn check_withdrawals_v0(
        &self,
        received_withdrawals: &withdrawal_txs::v0::WithdrawalTxs,
        our_withdrawals: &withdrawal_txs::v0::WithdrawalTxs,
        height: u64,
        round: u32,
        verify_with_validator_public_key: Option<&bls_signatures::PublicKey>,
        quorum_hash: Option<&[u8]>,
    ) -> SimpleValidationResult<AbciError> {
        if received_withdrawals.ne(our_withdrawals) {
            return SimpleValidationResult::new_with_error(
                AbciError::VoteExtensionMismatchReceived {
                    got: received_withdrawals.to_string(),
                    expected: our_withdrawals.to_string(),
                },
            );
        }

        // we only verify if verify_with_validator_public_key exists
        if let Some(validator_public_key) = verify_with_validator_public_key {
            let quorum_hash = quorum_hash.expect("quorum hash is required to verify signature");
            let validation_result = received_withdrawals.verify_signatures(
                &self.config.abci.chain_id,
                self.config.validator_set_quorum_type(),
                quorum_hash,
                height,
                round,
                validator_public_key,
            );

            if validation_result.is_valid() {
                SimpleValidationResult::default()
            } else {
                SimpleValidationResult::new_with_error(
                    validation_result
                        .errors
                        .into_iter()
                        .next()
                        .expect("expected an error"),
                )
            }
        } else {
            SimpleValidationResult::default()
        }
    }
}
