use crate::drive::system::misc_path;
use crate::drive::system::misc_tree_constants::NEXT_PROTOCOL_VERSION_STORAGE_KEY;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::util::deserializer::ProtocolVersion;

use grovedb::TransactionArg;
use integer_encoding::VarInt;

impl Drive {
    /// Gets the next protocol version from the backing store
    #[inline(always)]
    pub(super) fn fetch_next_protocol_version_v0(
        &self,
        transaction: TransactionArg,
    ) -> Result<Option<ProtocolVersion>, Error> {
        let misc_path = misc_path();
        self.grove
            .get_raw_optional(
                (&misc_path).into(),
                NEXT_PROTOCOL_VERSION_STORAGE_KEY,
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
