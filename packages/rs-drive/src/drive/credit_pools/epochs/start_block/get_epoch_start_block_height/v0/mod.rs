use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::epoch::Epoch;

use crate::drive::credit_pools::epochs::epoch_key_constants::KEY_START_BLOCK_HEIGHT;
use crate::drive::credit_pools::epochs::paths::EpochProposers;
use grovedb::{Element, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Returns the block height of the Epoch's start block
    pub(super) fn get_epoch_start_block_height_v0(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let element = self
            .grove
            .get(
                &epoch_tree.get_path(),
                KEY_START_BLOCK_HEIGHT.as_slice(),
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        let Element::Item(encoded_start_block_height, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType(
                "start block height must be an item",
            )));
        };

        let start_block_height = u64::from_be_bytes(
            encoded_start_block_height
                .as_slice()
                .try_into()
                .map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(String::from(
                        "start block height must be u64",
                    )))
                })?,
        );

        Ok(start_block_height)
    }
}
