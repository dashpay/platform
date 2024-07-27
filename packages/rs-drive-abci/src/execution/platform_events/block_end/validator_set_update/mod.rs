mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::execution::types::block_execution_context::BlockExecutionContext;

use crate::platform_types::platform::Platform;

use crate::platform_types::platform_state::PlatformState;

use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for validator set rotations and performs rotations if necessary.
    ///
    /// This function is a version handler that directs to specific version implementations
    /// of the validator_set_update function.
    ///
    /// # Arguments
    ///
    /// * `platform_state` - A `PlatformState` reference.
    /// * `block_execution_context` - A mutable `BlockExecutionContext` reference.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<Option<ValidatorSetUpdate>, Error>` - If the rotation is successful, it returns `Ok(Some(ValidatorSetUpdate))`
    ///   If there is no update, it returns `Ok(None)`. If there is an error, it returns `Error`.
    ///
    pub fn validator_set_update(
        &self,
        proposer_pro_tx_hash: [u8; 32],
        platform_state: &PlatformState,
        block_execution_context: &mut BlockExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ValidatorSetUpdate>, Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .validator_set_update
        {
            0 => self.validator_set_update_v0(
                proposer_pro_tx_hash,
                platform_state,
                block_execution_context,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "validator_set_update".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
