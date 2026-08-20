mod v0;
mod v1;
mod v2;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DocumentAndContractInfo;
use crate::util::object_size_info::PathInfo;
use crate::util::storage_flags::StorageFlags;

use dpp::data_contract::document_type::IndexLevel;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Removes indices for an index level and recurses.
    ///
    /// # Parameters
    /// * `document_and_contract_info`: The document and contract info.
    /// * `index_path_info`: The index path info.
    /// * `index_level`: The index level.
    /// * `any_fields_null`: Indicator if any fields are null.
    /// * `parent_value_tree_type`: Exact `TreeType` of the value tree
    ///   at `index_path_info`. Lets v1's cost-estimation arm emit
    ///   correct `EstimatedLayerInformation` for sum-bearing
    ///   parents (`SumTree` / `ProvableSumTree` / `CountSumTree` /
    ///   `ProvableCountSumTree` / `ProvableCountProvableSumTree`) —
    ///   the previous single-bool input collapsed all those to
    ///   `NormalTree`, under-charging dry-run delete fees. v0 only
    ///   ever sees `CountTree` / `NormalTree` (pre-v3 contracts),
    ///   so the dispatcher narrows the TreeType to a bool via
    ///   `matches!(_, TreeType::CountTree)` before calling v0.
    /// * `storage_flags`: The storage flags.
    /// * `previous_batch_operations`: Previous batch operations to include.
    /// * `estimated_costs_only_with_layer_info`: Estimated costs with layer info.
    /// * `event_id`: The event ID.
    /// * `transaction`: The transaction argument.
    /// * `batch_operations`: The batch operations to include.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn remove_indices_for_index_level_for_contract_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        index_level: &IndexLevel,
        any_fields_null: bool,
        all_fields_null: bool,
        parent_value_tree_type: TreeType,
        storage_flags: &Option<&StorageFlags>,
        previous_batch_operations: &Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        event_id: [u8; 32],
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .remove_indices_for_index_level_for_contract_operations
        {
            0 => {
                // v0 is stuck in time and only ever sees pre-v3
                // contracts whose value trees collapse to
                // `NormalTree` / `CountTree`. Narrow to bool here.
                let parent_value_tree_is_range_countable =
                    matches!(parent_value_tree_type, TreeType::CountTree);
                self.remove_indices_for_index_level_for_contract_operations_v0(
                    document_and_contract_info,
                    index_path_info,
                    index_level,
                    any_fields_null,
                    all_fields_null,
                    parent_value_tree_is_range_countable,
                    storage_flags,
                    previous_batch_operations,
                    estimated_costs_only_with_layer_info,
                    event_id,
                    transaction,
                    batch_operations,
                    platform_version,
                )
            }
            1 => self.remove_indices_for_index_level_for_contract_operations_v1(
                document_and_contract_info,
                index_path_info,
                index_level,
                any_fields_null,
                all_fields_null,
                parent_value_tree_type,
                storage_flags,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                event_id,
                transaction,
                batch_operations,
                platform_version,
            ),
            // v2 (platform v14+): tree types via the shared
            // continuation-demotion helper, mirroring the v2 insert walker.
            2 => self.remove_indices_for_index_level_for_contract_operations_v2(
                document_and_contract_info,
                index_path_info,
                index_level,
                any_fields_null,
                all_fields_null,
                parent_value_tree_type,
                storage_flags,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                event_id,
                transaction,
                batch_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_indices_for_index_level_for_contract_operations".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            })),
        }
    }
}
