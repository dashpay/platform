use crate::error::Error;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
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
    /// * `block_info` - An `ExtendedBlockInfo` reference containing block information.
    /// * `quorum_hash` - A `QuorumHash` reference.
    /// * `transaction` - A `Transaction` reference.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Returns an empty `Result` if the data is successfully stored, otherwise returns an `Error`.
    ///
    pub(crate) fn store_ephemeral_data(
        &self,
        block_info: &ExtendedBlockInfo,
        quorum_hash: &QuorumHash,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        // we need to serialize the block info
        let serialized_block_info = block_info.serialize()?;

        // next we need to store this data in grovedb
        self.drive
            .grove
            .put_aux(
                b"saved_state",
                &serialized_block_info,
                None,
                Some(transaction),
            )
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?;

        self.drive
            .grove
            .put_aux(
                b"saved_quorum_hash",
                &quorum_hash.into_inner(),
                None,
                Some(transaction),
            )
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?;

        Ok(())
    }
}
