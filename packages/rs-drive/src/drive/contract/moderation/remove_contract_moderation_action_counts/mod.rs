mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations that delete the moderation action counts of `identity_ids` on the
    /// elected contract `contract_id`: the reset a settle of the moderators pot makes after
    /// paying the pot out by them. Each count must exist. With layer information the
    /// operations are built for estimation only.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The elected contract.
    /// * `identity_ids`: The members whose counts go.
    /// * `estimated_costs_only_with_layer_info`: The estimation map for a dry run.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<LowLevelDriveOperation>)` with the operations.
    /// * `Err(Error)` when the version is unknown or GroveDB refuses an operation.
    pub fn remove_contract_moderation_action_counts_operations(
        &self,
        contract_id: Identifier,
        identity_ids: &[Identifier],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .remove_contract_moderation_action_counts
        {
            0 => self.remove_contract_moderation_action_counts_operations_v0(
                contract_id,
                identity_ids,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_contract_moderation_action_counts_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
