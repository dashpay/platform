mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds a list of pre-programmed distributions to the state tree.
    ///
    /// This function inserts pre-programmed token distributions, ensuring they are properly structured
    /// within the storage tree. It creates necessary subtrees, validates input values, and associates
    /// each distribution entry with the appropriate identifiers and timestamps.
    ///
    /// # Parameters
    /// - `token_id`: The unique identifier of the token for which distributions are being added.
    /// - `owner_id`: The identifier of the entity that owns the distributions.
    /// - `distribution`: A `TokenPreProgrammedDistribution` containing the scheduled distributions.
    /// - `block_info`: Metadata about the current block, including epoch information.
    /// - `estimated_costs_only_with_layer_info`: If provided, stores estimated cost calculations
    ///   instead of applying the changes.
    /// - `previous_batch_operations`: If provided, contains previously executed batch operations
    ///   that should be taken into account.
    /// - `batch_operations`: The list of low-level operations to be executed as a batch.
    /// - `transaction`: The transaction context for this operation.
    /// - `platform_version`: The version of the platform to determine the correct function variant.
    ///
    /// # Behavior
    /// - Ensures that the root path for pre-programmed distributions exists.
    /// - Inserts a new distribution entry if one does not already exist.
    /// - Stores distributions as sum trees, allowing for quick retrieval of total distributions at
    ///   a given time.
    /// - Uses reference paths to map distributions to their corresponding execution times.
    /// - Prevents overflow errors by ensuring token amounts do not exceed `i64::MAX`.
    ///
    /// # Returns
    /// - `Ok(())` if the distributions are successfully added.
    /// - `Err(Error::Drive(DriveError::UnknownVersionMismatch))` if an unsupported platform version
    ///   is encountered.
    /// - `Err(Error::Protocol(ProtocolError::Overflow))` if a distribution amount exceeds the
    ///   maximum allowed value.
    pub fn add_pre_programmed_distributions(
        &self,
        token_id: [u8; 32],
        owner_id: [u8; 32],
        distribution: TokenPreProgrammedDistribution,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .token
            .distribution
            .add_pre_programmed_distributions
        {
            0 => self.add_pre_programmed_distributions_v0(
                token_id,
                owner_id,
                distribution,
                block_info,
                estimated_costs_only_with_layer_info,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_pre_programmed_distributions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
