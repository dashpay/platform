use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use bincode::config;
use dpp::block::block_info::BlockInfo;
use dpp::reduced_platform_state::v0::ReducedPlatformStateForSavingV0;
use dpp::reduced_platform_state::ReducedPlatformStateForSaving;
use dpp::serialization::{
    PlatformDeserializableFromVersionedStructure, ReducedPlatformDeserializable,
};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::drive::Drive;
use drive::error::drive::DriveError::CorruptedDriveState;
use drive::query::TransactionArg;

impl<C> Platform<C> {
    pub(super) fn fetch_last_block_info_v0(
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<BlockInfo>, Error> {
        drive
            .fetch_last_block_info_bytes(transaction, platform_version)
            .map_err(Error::Drive)?
            .map(|bytes| {
                let config = config::standard().with_big_endian().with_no_limit();
                let (block_info, _): (BlockInfo, _) = bincode::decode_from_slice(&bytes, config)
                    .map_err(|_|
                        Error::Drive(drive::error::Error::Drive(CorruptedDriveState("corrupted last block_info serialisation".to_string())))
                    )?;

                Ok(block_info)
            })
            .transpose() // Converts Option<Result<T, E>> to Result<Option<T>, E>
    }
}
