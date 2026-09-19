mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::data_contract::config::moderation::ContractModerationConfig;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds the operations that create the moderation list trees a contract's config declares
    /// and that do not exist yet: the banlist under key `3`, the suspension list under key
    /// `4`. Called by the contract insertion only: the lists a contract keeps are fixed when it is
    /// created, so a contract update never adds one.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract.
    /// * `moderation`: The contract's moderation declaration.
    /// * `storage_flags`: The flags of the contract's trees.
    /// * `estimated_costs_only_with_layer_info`: The estimation map for a dry run.
    /// * `transaction`: The GroveDB transaction.
    /// * `batch_operations`: The operations accumulator.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` with the tree creations appended.
    /// * `Err(Error)` when the version is unknown or GroveDB refuses an operation.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_contract_moderation_trees_operations(
        &self,
        contract_id: [u8; 32],
        moderation: &ContractModerationConfig,
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
            .insert_contract_moderation_trees
        {
            0 => self.insert_contract_moderation_trees_operations_v0(
                contract_id,
                moderation,
                storage_flags,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_contract_moderation_trees_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
