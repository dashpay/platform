mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations that hold `credits` in escrow for the new offer `order_id`. The caller
    /// removes the credits from the buyer's balance in the same batch: the escrow sits under the
    /// prefunded specialized balances, which the credit conservation check counts.
    pub fn add_to_credit_order_escrow_operations(
        &self,
        order_id: [u8; 32],
        credits: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .order_escrow
            .add_credits
        {
            Some(0) => self.add_to_credit_order_escrow_operations_v0(
                order_id,
                credits,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_to_credit_order_escrow_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "add_to_credit_order_escrow_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// The operations that set what the open offer `order_id` holds in escrow to `credits`,
    /// after a partial fill. An offer filled entirely or cancelled is removed instead.
    pub fn set_credit_order_escrow_operations(
        &self,
        order_id: [u8; 32],
        credits: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .order_escrow
            .set_credits
        {
            Some(0) => self.set_credit_order_escrow_operations_v0(
                order_id,
                credits,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "set_credit_order_escrow_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "set_credit_order_escrow_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// The operations that remove the escrow entry of the offer `order_id`, when it is filled
    /// entirely or cancelled.
    pub fn remove_credit_order_escrow_operations(
        &self,
        order_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .order_escrow
            .remove_credits
        {
            Some(0) => self.remove_credit_order_escrow_operations_v0(
                order_id,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_credit_order_escrow_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "remove_credit_order_escrow_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// What the offer `order_id` holds in escrow, `None` when it holds nothing.
    pub fn fetch_credit_order_escrow(
        &self,
        order_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Credits>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .order_escrow
            .fetch_credits
        {
            Some(0) => self.fetch_credit_order_escrow_v0(
                order_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_credit_order_escrow".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_credit_order_escrow".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
