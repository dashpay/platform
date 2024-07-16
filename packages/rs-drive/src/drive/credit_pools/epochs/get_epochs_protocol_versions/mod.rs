mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use std::collections::BTreeMap;

use dpp::block::epoch::EpochIndex;
use dpp::util::deserializer::ProtocolVersion;
use grovedb::TransactionArg;

use dpp::version::PlatformVersion;

impl Drive {
    /// Get multiple epoch protocol versions starting at a given epoch index
    pub fn get_epochs_protocol_versions(
        &self,
        start_epoch_index: u16,
        count: Option<u16>,
        ascending: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<EpochIndex, ProtocolVersion>, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .get_epochs_protocol_versions
        {
            0 => self.get_epochs_protocol_versions_v0(
                start_epoch_index,
                count,
                ascending,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_epochs_protocol_versions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
