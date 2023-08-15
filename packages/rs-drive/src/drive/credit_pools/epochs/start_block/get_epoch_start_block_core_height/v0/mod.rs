use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::epoch::Epoch;

use grovedb::{Element, TransactionArg};

use crate::fee_pools::epochs::epoch_key_constants::KEY_START_BLOCK_CORE_HEIGHT;
use crate::fee_pools::epochs::paths::EpochProposers;

impl Drive {
    /// Returns the core block height of the Epoch's start block
    pub(super) fn get_epoch_start_block_core_height_v0(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<u32, Error> {
        let element = self
            .grove
            .get(
                &epoch_tree.get_path(),
                KEY_START_BLOCK_CORE_HEIGHT.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        let Element::Item(encoded_start_block_core_height, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType("start block height must be an item")));
        };

        let start_block_core_height = u32::from_be_bytes(
            encoded_start_block_core_height
                .as_slice()
                .try_into()
                .map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "start block height must be u32",
                    ))
                })?,
        );

        Ok(start_block_core_height)
    }
}
