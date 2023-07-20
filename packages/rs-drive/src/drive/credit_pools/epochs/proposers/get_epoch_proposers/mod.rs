mod v0;

use grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee_pools::epochs::paths::EpochProposers;
use dpp::block::epoch::Epoch;
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;

impl Drive {
    /// Returns a list of the Epoch's block proposers
    ///
    /// # Arguments
    ///
    /// * `epoch_tree` - An Epoch instance.
    /// * `limit` - An Option containing the limit of proposers to be fetched.
    /// * `transaction` - A TransactionArg instance.
    /// * `platform_version` - A PlatformVersion instance representing the version of the drive.
    ///
    /// # Returns
    ///
    /// A Result containing a vector of tuples with proposers' transaction hashes and block counts or an Error.
    pub fn get_epoch_proposers(
        &self,
        epoch_tree: &Epoch,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Vec<u8>, u64)>, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .get_epoch_proposers
        {
            0 => self.get_epoch_proposers_v0(epoch_tree, limit, transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_epoch_proposers".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
