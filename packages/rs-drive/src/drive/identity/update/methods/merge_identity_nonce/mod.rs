mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;

use dpp::prelude::IdentityNonce;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};

use crate::error::identity::IdentityError;
use dpp::identity::identity_nonce::MergeIdentityNonceResult;
use std::collections::HashMap;

pub(crate) trait MergeIdentityContractNonceResultToResult {
    fn to_result(self) -> Result<(), Error>;
}

impl MergeIdentityContractNonceResultToResult for MergeIdentityNonceResult {
    /// Gives a result from the enum
    fn to_result(self) -> Result<(), Error> {
        if let Some(error_message) = self.error_message() {
            Err(Error::Identity(IdentityError::IdentityNonceError(
                error_message,
            )))
        } else {
            Ok(())
        }
    }
}

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
    pub fn merge_identity_nonce(
        &self,
        identity_id: [u8; 32],
        nonce: IdentityNonce,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(MergeIdentityNonceResult, Option<FeeResult>), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .merge_identity_nonce
        {
            0 => self.merge_identity_nonce_v0(
                identity_id,
                nonce,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "merge_identity_nonce".to_string(),
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
    pub fn merge_identity_nonce_operations(
        &self,
        identity_id: [u8; 32],
        nonce: IdentityNonce,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(MergeIdentityNonceResult, Vec<LowLevelDriveOperation>), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .merge_identity_nonce
        {
            0 => self.merge_identity_nonce_operations_v0(
                identity_id,
                nonce,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "merge_identity_nonce_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
