mod v0;
//
// use dpp::block::block_info::BlockInfo;
// use crate::drive::identity::{identity_path_vec, IdentityRootStructure};
// use crate::drive::Drive;
// use crate::error::drive::DriveError;
// use crate::error::Error;
//
// use crate::fee::op::LowLevelDriveOperation;
// use grovedb::batch::KeyInfoPath;
// use crate::drive::identity::key::fetch::{
//     IdentityKeysRequest, KeyIDIdentityPublicKeyPairVec, KeyRequestType,
// };
// use dpp::fee::fee_result::FeeResult;
// use dpp::identity::{IdentityPublicKey, KeyID};
// use dpp::prelude::{Revision, TimestampMillis};
// use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
// use integer_encoding::VarInt;
// use std::collections::HashMap;
// use dpp::version::drive_versions::DriveVersion;
// use dpp::version::PlatformVersion;
//
// impl Drive {
//     /// Updates the revision for a specific identity. This function is version controlled.
//     ///
//     /// # Arguments
//     ///
//     /// * `identity_id` - The ID of the Identity whose revision is to be updated.
//     /// * `revision` - The revision to update to.
//     /// * `block_info` - The current block information.
//     /// * `apply` - Whether to apply the change.
//     /// * `transaction` - The current transaction.
//     /// * `drive_version` - The drive version.
//     ///
//     /// # Returns
//     ///
//     /// * `Result<FeeResult, Error>` - The resulting fee if successful, or an error.
//     pub fn update_identity_revision(
//         &self,
//         identity_id: [u8; 32],
//         revision: Revision,
//         block_info: &BlockInfo,
//         apply: bool,
//         transaction: TransactionArg,
//         platform_version: &PlatformVersion,
//     ) -> Result<FeeResult, Error> {
//         match platform_version.drive.methods.identity.update.update_identity_revision {
//             0 => self.update_identity_revision_v0(identity_id, revision, block_info, apply, transaction, platform_version),
//             version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
//                 method: "update_identity_revision".to_string(),
//                 known_versions: vec![0],
//                 received: version,
//             })),
//         }
//     }
//
//     /// Updates the revision operation of the identity. This function is version controlled.
//     ///
//     /// # Arguments
//     ///
//     /// * `identity_id` - The ID of the Identity whose revision operation is to be updated.
//     /// * `revision` - The revision to update to.
//     /// * `estimated_costs_only_with_layer_info` - The estimated costs with layer information.
//     ///
//     /// # Returns
//     ///
//     /// * `LowLevelDriveOperation` - The resulting low level drive operation.
//     pub(crate) fn update_identity_revision_operation(
//         &self,
//         identity_id: [u8; 32],
//         revision: Revision,
//         estimated_costs_only_with_layer_info: &mut Option<
//             HashMap<KeyInfoPath, EstimatedLayerInformation>,
//         >,
//         platform_version: &PlatformVersion,
//     ) -> LowLevelDriveOperation {
//         match platform_version.drive.methods.identity.update.update_identity_revision {
//             0 => self.update_identity_revision_operation_v0(identity_id, revision, estimated_costs_only_with_layer_info, platform_version),
//             version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
//                 method: "update_identity_revision_operation".to_string(),
//                 known_versions: vec![0],
//                 received: version,
//             })),
//         }
//     }
// }
