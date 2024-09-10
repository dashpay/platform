mod v0;

use grovedb::TransactionArg;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::epoch::Epoch;

use crate::query::proposer_block_count_query::ProposerQueryType;
use dpp::version::PlatformVersion;

impl Drive {
    /// Generates a GroveDB proof of the block proposers for a given epoch.
    ///
    /// This function retrieves the list of block proposers for a specific epoch and returns
    /// a proof that can be verified externally. The query can be limited by either a range or a set of proposer IDs.
    ///
    /// # Parameters
    ///
    /// - `epoch_tree`: A reference to an `Epoch` instance representing the epoch for which to retrieve proposers.
    /// - `query_type`: The type of query to perform (`ProposerQueryType`), which can either be a range query with
    ///   an optional limit and starting point or a query by specific proposer IDs.
    /// - `transaction`: A `TransactionArg` representing the transaction context for the query.
    /// - `platform_version`: The version of the platform, represented by a `PlatformVersion` instance, to ensure compatibility.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// - `Vec<u8>`: A byte vector representing the GroveDB proof of the proposers for the epoch.
    /// - `Error`: If the proof generation fails due to invalid data, unsupported platform version, or other errors.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// - The platform version is unknown or unsupported.
    /// - There is an issue generating the GroveDB proof for the epoch proposers.
    /// - The epoch or query type is invalid.
    pub fn prove_epoch_proposers(
        &self,
        epoch_tree: &Epoch,
        query_type: ProposerQueryType,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .prove_epoch_proposers
        {
            0 => {
                self.prove_epoch_proposers_v0(epoch_tree, query_type, transaction, platform_version)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_epoch_proposers".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
