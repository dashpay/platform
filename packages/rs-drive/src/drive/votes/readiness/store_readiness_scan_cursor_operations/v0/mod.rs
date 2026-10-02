use crate::drive::votes::paths::{readiness_round_tree_path_vec, READINESS_ROUND_SCAN_CURSOR_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::scan_cursor::ReadinessScanCursor;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    pub(super) fn store_readiness_scan_cursor_operations_v0(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        cursor: &ReadinessScanCursor,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness(
                contract_id,
                round_id,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }
        // Insert-or-replace: the cursor grows as the walk advances (the optional key
        // becomes present), so this is priced as an insert, never as a same-size replace.
        let op = QualifiedGroveDbOp::insert_or_replace_op(
            readiness_round_tree_path_vec(contract_id, round_id),
            vec![READINESS_ROUND_SCAN_CURSOR_KEY],
            Element::new_item(cursor.serialize_to_bytes()?),
        );
        Ok(vec![GroveOperation(op)])
    }
}
