mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// The operations that write `identity_id`'s count of moderation actions on the elected
    /// contract `contract_id` since the moderators pot was last settled: an insert for the
    /// member's first counted action of the period, a replacement of the same size after. With
    /// layer information the operations are built for estimation only.
    ///
    /// The count carries no storage flags: the member whose action writes it pays for it, and
    /// the settle that deletes it refunds nobody, as for a document a moderator deletes.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The elected contract.
    /// * `identity_id`: The member of the seated team that signed the action.
    /// * `count`: The count to store, the action included.
    /// * `estimated_costs_only_with_layer_info`: The estimation map for a dry run.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<LowLevelDriveOperation>)` with the operations.
    /// * `Err(Error)` when the version is unknown.
    pub fn set_contract_moderation_action_count_operations(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        count: u32,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .set_contract_moderation_action_count
        {
            0 => self.set_contract_moderation_action_count_operations_v0(
                contract_id,
                identity_id,
                count,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "set_contract_moderation_action_count_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
