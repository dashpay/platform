use crate::error::Error;
use crate::platform::state::PlatformState;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;

use dpp::serialization_traits::PlatformSerializable;
use drive::error::Error::GroveDB;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Stores ephemeral data, including the block information and quorum hash, in the GroveDB.
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
    pub(crate) fn store_ephemeral_data(
        &self,
        platform_state: &PlatformState,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        // we need to serialize the platform state
        let serialized_platform_state = platform_state.serialize()?;

        // next we need to store this data in grovedb
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
