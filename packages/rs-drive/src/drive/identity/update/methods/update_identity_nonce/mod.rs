mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;

use dpp::prelude::IdentityNonce;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};

use std::collections::HashMap;

impl Drive {
    /// Updates the nonce for a specific identity. This function is version controlled.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity whose nonce is to be updated.
    /// * `nonce` - The nonce to update to.
    /// * `block_info` - The current block information.
    /// * `apply` - Whether to apply the change.
    /// * `transaction` - The current transaction.
    /// * `drive_version` - The drive version.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - The resulting fee if successful, or an error.
    pub fn update_identity_nonce(
        &self,
        identity_id: [u8; 32],
        nonce: IdentityNonce,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .update_identity_nonce
        {
            0 => self.update_identity_nonce_v0(
                identity_id,
                nonce,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_identity_nonce".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Updates the nonce operation of the identity. This function is version controlled.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity whose nonce operation is to be updated.
    /// * `nonce` - The nonce to update to.
    /// * `estimated_costs_only_with_layer_info` - The estimated costs with layer information.
    ///
    /// # Returns
    ///
    /// * `LowLevelDriveOperation` - The resulting low level drive operation.
    pub fn update_identity_nonce_operation(
        &self,
        identity_id: [u8; 32],
        nonce: IdentityNonce,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<LowLevelDriveOperation, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .update_identity_nonce
        {
            0 => self.update_identity_nonce_operation_v0(
                identity_id,
                nonce,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_identity_nonce_operation".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
