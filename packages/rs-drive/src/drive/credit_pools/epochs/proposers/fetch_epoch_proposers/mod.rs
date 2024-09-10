mod v0;

use grovedb::TransactionArg;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::epoch::Epoch;

use crate::query::proposer_block_count_query::ProposerQueryType;
use dpp::version::PlatformVersion;

impl Drive {
    /// Retrieves the list of block proposers for a given epoch.
    ///
    /// This function fetches the proposers for a specified epoch, returning their transaction hashes and
    /// the number of blocks they proposed. The query can either be limited by a range or a set of proposer IDs.
    ///
    /// # Parameters
    ///
    /// - `epoch_tree`: A reference to an `Epoch` instance representing the epoch for which to retrieve proposers.
    /// - `query_type`: The type of query to perform (`ProposerQueryType`), which can either specify a range of
    ///   proposers or fetch specific proposers by their IDs.
    /// - `transaction`: A `TransactionArg` that provides the transaction context for the query execution.
    /// - `platform_version`: The version of the platform, represented by a `PlatformVersion` instance, ensuring compatibility
    ///   between different platform versions.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// - `Vec<(Vec<u8>, u64)>`: A vector of tuples where each tuple contains:
    ///   - A byte vector (`Vec<u8>`) representing the proposer's transaction hash.
    ///   - A `u64` representing the number of blocks proposed by that proposer.
    /// - `Error`: An error if the query fails due to an invalid platform version, transaction issues, or invalid epoch data.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// - The platform version is unknown or unsupported.
    /// - There is an issue with retrieving the proposers, such as a database or transaction error.
    /// - The provided epoch or query type is invalid.
    pub fn fetch_epoch_proposers(
        &self,
        epoch_tree: &Epoch,
        query_type: ProposerQueryType,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Vec<u8>, u64)>, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .fetch_epoch_proposers
        {
            0 => {
                self.fetch_epoch_proposers_v0(epoch_tree, query_type, transaction, platform_version)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_epoch_proposers".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
