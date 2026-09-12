mod v0;
mod v1;
mod v2;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DocumentAndContractInfo;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds indices for the top index level and calls for lower levels.
    ///
    /// Version dispatch:
    /// - **v0** (consensus-locked for protocol ≤ v11): emits
    ///   `EstimatedLayerSizes::AllSubtrees(.., NoSumTrees, ..)` for every
    ///   layer. Correct for pre-v12 contracts whose value trees are
    ///   always NormalTree (the sum/range flags didn't exist).
    /// - **v1** (active at protocol v12+): emits the matching
    ///   `SomeSumTrees` shortcut for the property-name layer's actual
    ///   `value_tree_type`, so dry-run cost estimation includes the
    ///   per-node aggregate bytes for SumTree / ProvableSumTree /
    ///   CountSumTree / ProvableCountSumTree / PCPS value trees.
    ///   Unblocked by grovedb #674.
    /// - **v2** (active at protocol v14+): shared-prefix aggregate fix —
    ///   tree types derive through the shared continuation-demotion
    ///   helper so aggregating prefix indexes with compound siblings
    ///   become insertable. Bit-identical to v1 for shapes v1 could
    ///   insert.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_indices_for_top_index_level_for_contract_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .insert
            .add_indices_for_top_index_level_for_contract_operations
        {
            0 => self.add_indices_for_top_index_level_for_contract_operations_v0(
                document_and_contract_info,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                platform_version,
            ),
            1 => self.add_indices_for_top_index_level_for_contract_operations_v1(
                document_and_contract_info,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                platform_version,
            ),
            2 => self.add_indices_for_top_index_level_for_contract_operations_v2(
                document_and_contract_info,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                block_time_ms,
                transaction,
                batch_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_indices_for_top_index_level_for_contract_operations".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            })),
        }
    }
}
