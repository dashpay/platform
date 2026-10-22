mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds the estimated layers of a write to one listing's escrowed tokens.
    pub(crate) fn add_estimation_costs_for_token_order_escrow(
        token_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .token
            .order_escrow
            .estimated_costs_for_tokens
        {
            Some(0) => {
                Self::add_estimation_costs_for_token_order_escrow_v0(
                    token_id,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_token_order_escrow".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "add_estimation_costs_for_token_order_escrow".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// Adds the estimated layers of a write to one offer's escrowed credits.
    pub(crate) fn add_estimation_costs_for_credit_order_escrow(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .token
            .order_escrow
            .estimated_costs_for_credits
        {
            Some(0) => {
                Self::add_estimation_costs_for_credit_order_escrow_v0(
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_credit_order_escrow".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "add_estimation_costs_for_credit_order_escrow".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
