use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::system::misc_path;
use crate::drive::system::misc_tree_constants::PROTOCOL_VERSION_STORAGE_KEY;
use dpp::util::deserializer::ProtocolVersion;
use grovedb::{GroveDb, TransactionArg};
use integer_encoding::VarInt;

///
impl Drive {
    /// Gets the current protocol version from the backing store
    ///
    /// !!!DON'T CHANGE!!!!
    ///
    /// This function should never be changed !!! since it's using
    /// to get protocol version to read the state from the storage.
    /// In plain English, this is the first function that we call,
    /// so we don't know version yet.
    ///
    /// # Arguments
    ///
    /// * `transaction` - A `TransactionArg` object representing the transaction.
    ///
    /// # Returns
    ///
    /// * `Result<Option<ProtocolVersion>, Error>` - If successful, returns an `Ok(Option<ProtocolVersion>)`. If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the Drive version is unknown.
    pub fn fetch_current_protocol_version(
        &self,
        transaction: TransactionArg,
    ) -> Result<Option<ProtocolVersion>, Error> {
        Drive::fetch_current_protocol_version_with_grovedb(&self.grove, transaction)
    }

    pub(crate) fn fetch_current_protocol_version_with_grovedb(
        grove: &GroveDb,
        transaction: TransactionArg,
    ) -> Result<Option<ProtocolVersion>, Error> {
        let misc_path = misc_path();
        grove
            .get_raw_optional(
                (&misc_path).into(),
                PROTOCOL_VERSION_STORAGE_KEY,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)
            .map(|maybe_element| {
                maybe_element
                    .map(|e| {
                        let bytes = e.as_item_bytes()?;
                        let Some((protocol_version, _)) = ProtocolVersion::decode_var(bytes) else {
                            return Err(Error::Drive(DriveError::CorruptedSerialization(
                                String::from("protocol version incorrectly serialized"),
                            )));
                        };
                        Ok(protocol_version)
                    })
                    .transpose()
            })?
    }
}
