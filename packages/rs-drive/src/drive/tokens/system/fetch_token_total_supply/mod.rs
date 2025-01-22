mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::TokenAmount;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;

impl Drive {
    /// Fetches token's total supply
    pub fn fetch_token_total_supply(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenAmount>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .fetch
            .token_total_supply
        {
            0 => self.fetch_token_total_supply_v0(token_id, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_token_total_supply".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
    /// Fetches token's total supply and returns cost of doing so
    pub fn fetch_token_total_supply_with_cost(
        &self,
        token_id: [u8; 32],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<TokenAmount>, FeeResult), Error> {
        match platform_version
            .drive
            .methods
            .token
            .fetch
            .token_total_supply
        {
            0 => self.fetch_token_total_supply_with_cost_v0(token_id, block_info, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_token_total_supply".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds the operations of fetching the token total supply
    pub fn fetch_token_total_supply_add_to_operations(
        &self,
        token_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenAmount>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .fetch
            .token_total_supply
        {
            0 => self.fetch_token_total_supply_add_to_operations_v0(
                token_id,
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_token_total_supply_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
