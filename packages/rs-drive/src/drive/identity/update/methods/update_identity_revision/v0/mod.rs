use dpp::block::block_info::BlockInfo;

use crate::drive::identity::{identity_path_vec, IdentityRootStructure};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use grovedb::batch::KeyInfoPath;

use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairVec, KeyRequestType,
};
use dpp::fee::fee_result::FeeResult;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::prelude::{Revision, TimestampMillis};
use dpp::version::drive_versions::DriveVersion;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use std::collections::HashMap;

impl Drive {
    //todo: this should probably not exist
    /// Update revision for specific identity
    pub fn update_identity_revision_v0(
        &self,
        identity_id: [u8; 32],
        revision: Revision,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<FeeResult, Error> {
        // TODO: In case of dry run we will get less because we replace the same bytes

        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = vec![self.update_identity_revision_operation(
            identity_id,
            revision,
            &mut estimated_costs_only_with_layer_info,
        )];

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
            drive_version,
        )?;

        let fees = Drive::calculate_fee(None, Some(drive_operations), &block_info.epoch)?;

        Ok(fees)
    }
}
