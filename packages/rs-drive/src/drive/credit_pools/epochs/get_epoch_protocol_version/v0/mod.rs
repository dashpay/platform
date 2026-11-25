use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::push_drive_operation_result;

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
        self.get_epoch_protocol_version_operations_v0(
            epoch_tree,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    /// Returns the protocol version the epoch recorded and pushes the cost of reading it
    pub(super) fn get_epoch_protocol_version_operations_v0(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ProtocolVersion, Error> {
        let element = push_drive_operation_result(
            self.grove.get(
                &epoch_tree.get_path(),
                KEY_PROTOCOL_VERSION.as_slice(),
                transaction,
                &platform_version.drive.grove_version,
            ),
            drive_operations,
        )?;

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
