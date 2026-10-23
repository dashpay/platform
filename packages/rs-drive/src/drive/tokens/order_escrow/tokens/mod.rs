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

impl Drive {
    /// The operations that hold `amount` of the token `token_id` in escrow for the new listing
    /// `order_id`, creating the token's escrow tree with its first listing. The caller removes
    /// the amount from the seller's balance in the same batch: escrow moves tokens, it does not
    /// create them, and the token conservation check counts the escrow.
    ///
    /// `previous_batch_operations` are the operations already queued for the same batch: a
    /// caller placing several listings of a token in one batch passes them, so the token's escrow
    /// tree is inserted once, by the first.
    #[allow(clippy::too_many_arguments)]
    pub fn add_to_token_order_escrow_operations(
        &self,
        token_id: [u8; 32],
        order_id: [u8; 32],
        amount: TokenAmount,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.token.order_escrow.add_tokens {
            Some(0) => self.add_to_token_order_escrow_operations_v0(
                token_id,
                order_id,
                amount,
                estimated_costs_only_with_layer_info,
                transaction,
                previous_batch_operations,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_to_token_order_escrow_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "add_to_token_order_escrow_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// The operations that set what the open listing `order_id` of `token_id` holds in escrow to
    /// `amount`, after a partial fill. `amount` is above zero: a listing filled entirely is
    /// removed instead.
    pub fn set_token_order_escrow_operations(
        &self,
        token_id: [u8; 32],
        order_id: [u8; 32],
        amount: TokenAmount,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.token.order_escrow.set_tokens {
            Some(0) => self.set_token_order_escrow_operations_v0(
                token_id,
                order_id,
                amount,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "set_token_order_escrow_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "set_token_order_escrow_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// The operations that remove the escrow entry of the listing `order_id` of `token_id`,
    /// when it is filled entirely or cancelled. The token's escrow tree stays.
    pub fn remove_token_order_escrow_operations(
        &self,
        token_id: [u8; 32],
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
            .remove_tokens
        {
            Some(0) => self.remove_token_order_escrow_operations_v0(
                token_id,
                order_id,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_token_order_escrow_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "remove_token_order_escrow_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// What the listing `order_id` of `token_id` holds in escrow, `None` when it holds nothing.
    pub fn fetch_token_order_escrow(
        &self,
        token_id: [u8; 32],
        order_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenAmount>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .order_escrow
            .fetch_tokens
        {
            Some(0) => self.fetch_token_order_escrow_v0(
                token_id,
                order_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_token_order_escrow".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_token_order_escrow".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
