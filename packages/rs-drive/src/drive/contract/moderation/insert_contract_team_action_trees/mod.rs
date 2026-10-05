mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations creating the trees of the actions a contract's seated moderation team
    /// votes on: the tree of all of them (`[64, id, 2, 24]`) and, under it, the tree of the
    /// active actions (`M`) and of the closed ones (`X`), as a token group has.
    ///
    /// Called by a contract insertion when one of its document types sets
    /// `moderatorAbilities.deleteSettled`, whose settled documents the team deletes by such an
    /// action. No contract update adds such a type: the keyword needs the contract's elected
    /// declaration, which is fixed at creation, to give its team `deleteDocuments` on the type,
    /// and it can not name a type added later.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract the trees belong to.
    /// * `storage_flags`: The storage flags of the new trees.
    /// * `estimated_costs_only_with_layer_info`: The estimation map, when only estimating costs.
    /// * `transaction`: The GroveDB transaction.
    /// * `batch_operations`: The operations accumulator the tree inserts are appended to.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the tree inserts are appended to `batch_operations`.
    /// * `Err(Error)` when the method version is unknown or building an insert fails.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_contract_team_action_trees_operations(
        &self,
        contract_id: [u8; 32],
        storage_flags: Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .insert_contract_team_action_trees
        {
            0 => self.insert_contract_team_action_trees_operations_v0(
                contract_id,
                storage_flags,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_contract_team_action_trees_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
