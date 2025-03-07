use crate::error::Error;
use crate::platform_types::platform::Platform;
use bincode::config;
use dpp::block::block_info::BlockInfo;
use dpp::serialization::{PlatformSerializable, ReducedPlatformSerializable};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::error::drive::DriveError::CorruptedDriveState;
use drive::query::TransactionArg;

impl<C> Platform<C> {
    pub(super) fn store_last_block_info_v0(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let config = config::standard().with_big_endian().with_no_limit();
        let block_info_bytes = bincode::encode_to_vec(block_info, config).map_err(|_| {
            Error::Drive(drive::error::Error::Drive(CorruptedDriveState(
                "corrupted last block_info serialisation".to_string(),
            )))
        })?;
        self.drive
            .store_last_block_info_bytes(block_info_bytes.as_slice(), transaction, platform_version)
            .map_err(Error::Drive)?;
        Ok(())
    }
}
