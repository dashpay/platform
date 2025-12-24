use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::epoch::Epoch;

use crate::drive::credit_pools::epochs::epoch_key_constants::KEY_PROTOCOL_VERSION;
use crate::drive::credit_pools::epochs::paths::EpochProposers;
use grovedb::{Element, TransactionArg};
use platform_version::version::{PlatformVersion, ProtocolVersion};

impl Drive {
    /// Returns the block height of the Epoch's start block
    pub(super) fn get_epoch_protocol_version_v0(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ProtocolVersion, Error> {
        let element = self
            .grove
            .get(
                &epoch_tree.get_path(),
                KEY_PROTOCOL_VERSION.as_slice(),
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::from)?;

        let Element::Item(encoded_protocol_version, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType(
                "protocol version  must be an item",
            )));
        };

        let protocol_version = ProtocolVersion::from_be_bytes(
            encoded_protocol_version
                .as_slice()
                .try_into()
                .map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(String::from(
                        "protocol version must be u32",
                    )))
                })?,
        );

        Ok(protocol_version)
    }
}
