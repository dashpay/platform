use crate::drive::Drive;
use crate::error::drive::DriveError;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::identity::identity_nonce::MergeIdentityNonceResult;
use dpp::prelude::IdentityNonce;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

mod v0;

impl Drive {
    /// Merges the given revision into the identity contract pair nonce
    pub fn merge_identity_contract_nonce(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        revision_nonce: IdentityNonce,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<MergeIdentityNonceResult, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .contract_info
            .merge_identity_contract_nonce
        {
            0 => self.merge_identity_contract_nonce_v0(
                identity_id,
                contract_id,
                revision_nonce,
                block_info,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "merge_revision_nonce_for_identity_contract_pair".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Gives the operations of merging the given revision into the identity contract pair nonce
    pub fn merge_identity_contract_nonce_operations(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        revision_nonce: IdentityNonce,
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
            .contract_info
            .merge_identity_contract_nonce
        {
            0 => self.merge_identity_contract_nonce_operations_v0(
                identity_id,
                contract_id,
                revision_nonce,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "merge_revision_nonce_for_identity_contract_pair_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
