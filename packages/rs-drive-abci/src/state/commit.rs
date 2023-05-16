use crate::error::serialization::SerializationError;
use crate::error::Error;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use crate::state::PlatformState;
use dpp::block::block_info::ExtendedBlockInfo;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::QuorumHash;
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
        let mut platform_state = platform_state.clone();
        // we need to serialize the block info
        let mut serialized_platform_state = vec![];
        ciborium::ser::into_writer(&platform_state, &mut serialized_platform_state).map_err(
            |_| {
                SerializationError::CorruptedSerialization(format!(
                    "unable to encode PlatformState as cbor"
                ))
            },
        )?;

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
