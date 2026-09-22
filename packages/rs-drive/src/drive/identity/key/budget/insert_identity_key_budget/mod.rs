mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::IdentityPublicKey;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Writes the full budget of a newly added key as its remaining budget, creating the
    /// identity's key budgets subtree when this is its first budgeted key. A key without a
    /// budget adds no operations.
    ///
    /// # Parameters
    /// - `identity_id`: The identity the key is added to.
    /// - `identity_key`: The key being added.
    /// - `estimated_costs_only_with_layer_info`: `Some` when only estimating costs.
    /// - `transaction`: The transaction the subtree's existence is checked in.
    /// - `drive_operations`: The operations the writes are appended to.
    /// - `platform_version`: The platform version selecting the implementation.
    ///
    /// # Returns
    /// - `Ok(())`, or a version error when the key budgets subtree is not active.
    pub(crate) fn insert_identity_key_budget_operations(
        &self,
        identity_id: [u8; 32],
        identity_key: &IdentityPublicKey,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .keys
            .budget
            .insert_identity_key_budget
        {
            Some(0) => self.insert_identity_key_budget_operations_v0(
                identity_id,
                identity_key,
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_identity_key_budget_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "insert_identity_key_budget_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
