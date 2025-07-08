use crate::error::Error;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::platform_value::Identifier;
use dpp::version::PlatformVersion;

mod batch_transition;
mod document;
mod token;

/// A converter that will get High Level Drive Operations from State transitions
pub trait DriveHighLevelBatchOperationConverter {
    /// This will get a list of atomic drive operations from a high level operations
    fn into_high_level_batch_drive_operations<'a>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error>;
}
