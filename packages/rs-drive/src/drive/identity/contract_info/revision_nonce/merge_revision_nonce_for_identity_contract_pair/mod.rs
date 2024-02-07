use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::identity::identity_contract_nonce::MergeIdentityContractNonceResult;
use dpp::prelude::IdentityContractNonce;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

mod v0;

pub(crate) trait MergeIdentityContractNonceResultToResult {
    fn to_result(self) -> Result<(), Error>;
}

impl MergeIdentityContractNonceResultToResult for MergeIdentityContractNonceResult {
    /// Gives a result from the enum
    fn to_result(self) -> Result<(), Error> {
        if let Some(error_message) = self.error_message() {
            Err(Error::Identity(
                IdentityError::IdentityContractRevisionNonceError(error_message),
            ))
        } else {
            Ok(())
        }
    }
}

impl Drive {
    /// Merges the given revision into the identity contract pair nonce
    pub fn merge_revision_nonce_for_identity_contract_pair(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        revision_nonce: IdentityContractNonce,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<MergeIdentityContractNonceResult, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .contract_info
            .merge_revision_nonce_for_identity_contract_pair
        {
            0 => self.merge_revision_nonce_for_identity_contract_pair_v0(
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
    pub fn merge_revision_nonce_for_identity_contract_pair_operations(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        revision_nonce: IdentityContractNonce,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            MergeIdentityContractNonceResult,
            Vec<LowLevelDriveOperation>,
        ),
        Error,
    > {
        match platform_version
            .drive
            .methods
            .identity
            .contract_info
            .merge_revision_nonce_for_identity_contract_pair
        {
            0 => self.merge_revision_nonce_for_identity_contract_pair_operations_v0(
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
