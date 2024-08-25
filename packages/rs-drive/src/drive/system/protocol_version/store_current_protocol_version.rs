use crate::drive::system::protocol_version::PROTOCOL_VERSION_AUX_KEY;
use crate::drive::Drive;
use crate::error::Error;
use dpp::util::deserializer::ProtocolVersion;
use grovedb::TransactionArg;
use integer_encoding::VarInt;

impl Drive {
    /// Store the current protocol version in aux storage
    ///
    /// !!!DON'T CHANGE!!!!
    /// This function should never be changed !!! since it must always be compatible
    /// with fetch_current_protocol_version which is should never be changed.
    pub fn store_current_protocol_version(
        &self,
        protocol_version: ProtocolVersion,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        self.grove
            .put_aux(
                PROTOCOL_VERSION_AUX_KEY,
                &protocol_version.encode_var_vec(),
                None,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)
    }
}
