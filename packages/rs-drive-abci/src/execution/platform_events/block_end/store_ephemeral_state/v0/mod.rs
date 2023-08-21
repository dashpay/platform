use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;

use crate::platform_types::platform_state::PlatformState;
use dpp::serialization::PlatformSerializable;

use drive::error::Error::GroveDB;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Stores ephemeral state data, including the block information and quorum hash in GroveDB.
    ///
    /// This function should be removed from the current location.
    ///
    /// # Arguments
    ///
    /// * `platform_state` - A `PlatformState` reference.
    /// * `transaction` - A `Transaction` reference.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Returns an empty `Result` if the data is successfully stored, otherwise returns an `Error`.
    ///
    pub(super) fn store_ephemeral_state_v0(
        &self,
        platform_state: &PlatformState,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        // we need to serialize the platform state
        let serialized_platform_state = platform_state.serialize()?;

        // next we need to store this data in grovedb
        //todo:: maybe this should be in actual state
        self.drive
            .grove
            .put_aux(
                b"saved_state",
                &serialized_platform_state,
                None,
                Some(transaction),
            )
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?;

        Ok(())
    }
}
