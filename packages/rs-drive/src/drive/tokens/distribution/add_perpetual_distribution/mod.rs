mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds a TokenPerpetualDistribution to the state tree.
    pub fn add_perpetual_distribution(
        &self,
        token_id: [u8; 32],
        distribution: &TokenPerpetualDistribution,
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
            .add_perpetual_distribution
        {
            0 => self.add_perpetual_distribution_v0(
                token_id,
                distribution,
                estimated_costs_only_with_layer_info,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_perpetual_distribution".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
