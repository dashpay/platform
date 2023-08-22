use crate::drive::batch::GroveDbOpBatch;
use crate::drive::grove_operations::push_drive_operation_result;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::query::GroveError;
use grovedb::batch::estimated_costs::EstimatedCostsType::AverageCaseCostsType;
use grovedb::batch::{BatchApplyOptions, KeyInfoPath};
use grovedb::{EstimatedLayerInformation, GroveDb};
use std::collections::HashMap;

impl Drive {
    /// Gets the costs for the given groveDB op batch and passes them to `push_drive_operation_result`.
    pub(super) fn grove_batch_operations_costs_v0(
        &self,
        ops: GroveDbOpBatch,
        estimated_layer_info: HashMap<KeyInfoPath, EstimatedLayerInformation>,
        validate: bool,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        let cost_context = GroveDb::estimated_case_operations_for_batch(
            AverageCaseCostsType(estimated_layer_info),
            ops.operations,
            Some(BatchApplyOptions {
                validate_insertion_does_not_override: validate,
                validate_insertion_does_not_override_tree: validate,
                allow_deleting_non_empty_trees: false,
                deleting_non_empty_trees_returns_error: true,
                disable_operation_consistency_check: false,
                base_root_storage_is_free: true,
                batch_pause_height: None,
            }),
            |_, _, _| Ok(false),
            |_, _, _| Err(GroveError::InternalError("not implemented")),
        );
        push_drive_operation_result(cost_context, drive_operations)
    }
}
