mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::epoch::EpochIndex;
use dpp::block::finalized_epoch_info::FinalizedEpochInfo;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Retrieves finalized epoch information for a specified range of epochs.
    ///
    /// This method fetches finalized epoch details between `start_epoch_index` (inclusive or exclusive)
    /// and `end_epoch_index` (inclusive or exclusive) and returns them as a collection.
    ///
    /// The method first determines the correct versioned implementation to invoke, based on the
    /// provided `platform_version`. Currently, only version `0` is supported.
    ///
    /// ## Parameters
    ///
    /// - `start_epoch_index` (`u16`):  
    ///   The starting epoch index for the query.
    /// - `start_epoch_index_included` (`bool`):  
    ///   If `true`, includes `start_epoch_index` in the results.
    /// - `end_epoch_index` (`u16`):  
    ///   The ending epoch index for the query.
    /// - `end_epoch_index_included` (`bool`):  
    ///   If `true`, includes `end_epoch_index` in the results.
    /// - `transaction` (`TransactionArg`):  
    ///   The current database transaction for querying storage.
    /// - `platform_version` (`&PlatformVersion`):  
    ///   The platform version to use for method dispatch.
    ///
    /// ## Returns
    ///
    /// - `Ok(T)`: A collection (`T`) of `(EpochIndex, FinalizedEpochInfo)` tuples,
    ///   where `T` implements `FromIterator<(EpochIndex, FinalizedEpochInfo)>`.
    /// - `Err(Error)`: An error if querying fails due to version mismatch or storage issues.
    ///
    /// ## Errors
    ///
    /// - Returns `DriveError::UnknownVersionMismatch` if an unsupported `platform_version` is provided.
    /// - Any errors returned by `get_finalized_epoch_infos_v0` if the query fails.
    pub fn get_finalized_epoch_infos<T: FromIterator<(EpochIndex, FinalizedEpochInfo)>>(
        &self,
        start_epoch_index: u16,
        start_epoch_index_included: bool,
        end_epoch_index: u16,
        end_epoch_index_included: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<T, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .get_finalized_epoch_infos
        {
            0 => self.get_finalized_epoch_infos_v0(
                start_epoch_index,
                start_epoch_index_included,
                end_epoch_index,
                end_epoch_index_included,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_finalized_epoch_infos".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
