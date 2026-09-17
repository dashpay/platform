mod v0;

pub(crate) use v0::{ESTIMATED_READINESS_REPORT_RECORD_SIZE, ESTIMATED_READINESS_SCAN_CURSOR_SIZE};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds the layer estimation for writes under the readiness subtrees of one contract and
    /// round, and under the readiness fund tree.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract whose readiness tree is written.
    /// * `round_id` - The round whose tree is written.
    /// * `estimated_costs_only_with_layer_info` - The estimation map to fill.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(())` on success.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub(crate) fn add_estimation_costs_for_readiness(
        contract_id: [u8; 32],
        round_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .readiness
            .estimation_costs
        {
            Some(0) => {
                Self::add_estimation_costs_for_readiness_v0(
                    contract_id,
                    round_id,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "add_estimation_costs_for_readiness".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_readiness".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds the layer estimation for the per-time tree of one activation deadline.
    ///
    /// # Parameters
    ///
    /// * `deadline_ms` - The deadline whose tree is written.
    /// * `estimated_costs_only_with_layer_info` - The estimation map to fill.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(())` on success.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub(crate) fn add_estimation_costs_for_readiness_deadline(
        deadline_ms: u64,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .readiness
            .estimation_costs
        {
            Some(0) => {
                Self::add_estimation_costs_for_readiness_deadline_v0(
                    deadline_ms,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "add_estimation_costs_for_readiness_deadline".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_readiness_deadline".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds the layer estimation for a readiness fund write.
    ///
    /// # Parameters
    ///
    /// * `estimated_costs_only_with_layer_info` - The estimation map to fill.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(())` on success.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub(crate) fn add_estimation_costs_for_readiness_fund_update(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .readiness
            .estimation_costs
        {
            Some(0) => {
                Self::add_estimation_costs_for_readiness_fund_update_v0(
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "add_estimation_costs_for_readiness_fund_update".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_readiness_fund_update".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
