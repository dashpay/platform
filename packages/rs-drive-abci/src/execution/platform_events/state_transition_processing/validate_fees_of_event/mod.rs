mod v0;
mod v1;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Validates the fees of a given `ExecutionEvent`.
    ///
    /// # Arguments
    ///
    /// * `event` - The `ExecutionEvent` instance to validate.
    /// * `block_info` - Information about the current block.
    /// * `transaction` - The transaction arguments for the given event.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<FeeResult>, Error>` - On success, returns a
    ///   `ConsensusValidationResult` containing an `FeeResult`. On error, returns an `Error`.
    ///
    /// # Errors
    ///
    /// * This function may return an `Error::Execution` if the identity balance is not found.
    /// * This function may return an `Error::Drive` if there's an issue with applying drive operations.
    pub(in crate::execution) fn validate_fees_of_event(
        &self,
        event: &ExecutionEvent,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<ConsensusValidationResult<FeeResult>, Error> {
        // Fee validation with `transaction: None` is the CheckTx estimation mode: it runs
        // OUTSIDE any block transaction, so it must NEVER mutate committed GroveDB state —
        // an eager write inside an op converter here commits straight to disk and halts the
        // chain (devnet paloma, height 788 — see test::helpers::state_mutation_guard). In
        // tests, every transactionless call asserts the committed root hash is byte-identical
        // around the call, so every event/op shape estimated in tests picks up the invariant
        // automatically.
        #[cfg(test)]
        let committed_root_hash_before_estimation = transaction.is_none().then(|| {
            crate::test::helpers::state_mutation_guard::committed_root_hash(
                &self.drive,
                platform_version,
            )
        });

        let result = match platform_version
            .drive_abci
            .methods
            .state_transition_processing
            .validate_fees_of_event
        {
            0 => self.validate_fees_of_event_v0(
                event,
                block_info,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            1 => self.validate_fees_of_event_v1(
                event,
                block_info,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "validate_fees_of_event".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        };

        #[cfg(test)]
        if let Some(root_hash_before) = committed_root_hash_before_estimation {
            assert_eq!(
                root_hash_before,
                crate::test::helpers::state_mutation_guard::committed_root_hash(
                    &self.drive,
                    platform_version,
                ),
                "validate_fees_of_event with transaction = None mutated committed grovedb \
                 state: an eager write on the estimation path commits straight to disk, \
                 diverging the root from the signed app hash and halting the chain (devnet \
                 paloma, height 788)"
            );
        }

        result
    }
}
