mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use drive::error::Error::GroveDB;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Stores ephemeral state data, including the block information and quorum hash in GroveDB.
    ///
    /// This function is a version handler that directs to specific version implementations
    /// of the store_ephemeral_state function.
    ///
    /// # Arguments
    ///
    /// * `platform_state` - A `PlatformState` reference.
    /// * `transaction` - A `Transaction` reference.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Returns an empty `Result` if the data is successfully stored, otherwise returns an `Error`.
    ///
    pub fn store_ephemeral_state(
        &self,
        platform_state: &PlatformState,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .store_ephemeral_state
        {
            0 => self.store_ephemeral_state_v0(platform_state, transaction),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "store_ephemeral_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
