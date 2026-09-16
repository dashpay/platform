mod v0;
mod v1;
mod v2;

use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::util::object_size_info::{DocumentAndContractInfo, PathInfo};

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::document_type::IndexLevel;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;

use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Adds indices for an index level and recurses.
    ///
    /// `parent_value_tree_type` is the exact `TreeType` of the value
    /// tree at `index_path_info` — `NormalTree` for non-terminator /
    /// non-aggregating levels, or one of the aggregating variants
    /// (`CountTree` / `ProvableCountTree` / `SumTree` / `ProvableSumTree` /
    /// `CountSumTree` / `ProvableCountSumTree` /
    /// `ProvableCountProvableSumTree`) when the parent index opts into
    /// count and/or sum aggregation. The v0 implementation uses this
    /// to pick the correct wrapper variant
    /// (`NonCounted` / `NotSummed` / `NotCountedOrSummed`) for child
    /// continuation property-name trees so they contribute 0 to the
    /// parent's per-axis aggregates.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_indices_for_index_level_for_contract_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        index_level: &IndexLevel,
        any_fields_null: bool,
        all_fields_null: bool,
        parent_value_tree_type: TreeType,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        storage_flags: &Option<&StorageFlags>,
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
            .insert
            .add_indices_for_index_level_for_contract_operations
        {
            0 => {
                // v0 predates the sum-tree feature and accepted only a
                // `parent_value_tree_is_count_tree: bool`. Convert from
                // the wider `parent_value_tree_type` the dispatcher
                // signature carries today — for pre-v3 contracts the
                // only aggregating variant v0 ever saw was
                // `CountTree`, so a `matches!` collapse is exact.
                // (Sum-side variants would never reach v0: the v3
                // sum-tree feature lights up under v1 only.)
                let parent_value_tree_is_count_tree =
                    matches!(parent_value_tree_type, TreeType::CountTree);
                self.add_indices_for_index_level_for_contract_operations_v0(
                    document_and_contract_info,
                    index_path_info,
                    index_level,
                    any_fields_null,
                    all_fields_null,
                    parent_value_tree_is_count_tree,
                    previous_batch_operations,
                    storage_flags,
                    estimated_costs_only_with_layer_info,
                    event_id,
                    transaction,
                    batch_operations,
                    platform_version,
                )
            }
            1 => self.add_indices_for_index_level_for_contract_operations_v1(
                document_and_contract_info,
                index_path_info,
                index_level,
                any_fields_null,
                all_fields_null,
                parent_value_tree_type,
                previous_batch_operations,
                storage_flags,
                estimated_costs_only_with_layer_info,
                event_id,
                transaction,
                batch_operations,
                platform_version,
            ),
            // v2 (platform v14+): shared-prefix aggregate layouts become
            // insertable — continuation demotion + completed wrapper matrix.
            2 => self.add_indices_for_index_level_for_contract_operations_v2(
                document_and_contract_info,
                index_path_info,
                index_level,
                any_fields_null,
                all_fields_null,
                parent_value_tree_type,
                previous_batch_operations,
                storage_flags,
                estimated_costs_only_with_layer_info,
                event_id,
                transaction,
                batch_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_indices_for_index_level_for_contract_operations".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            })),
        }
    }
}
