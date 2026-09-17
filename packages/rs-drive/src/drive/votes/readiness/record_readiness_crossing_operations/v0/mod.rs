use crate::drive::votes::paths::{
    readiness_deadline_tree_path_vec, readiness_deadlines_tree_path_vec,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::common::encode::encode_u64;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use crate::util::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType};
use crate::util::object_size_info::PathKeyElementInfo::{PathKeyElementSize, PathKeyRefElement};
use crate::util::object_size_info::{DriveKeyInfo, PathInfo, PathKeyElementInfo};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::round::ReadinessRound;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn record_readiness_crossing_operations_v0(
        &self,
        round: &mut ReadinessRound,
        crossing_ms: u64,
        min_wait_ms: u64,
        max_wait_ms: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(u64, Vec<LowLevelDriveOperation>), Error> {
        if !round.is_pending() {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "recording a crossing on a round that already crossed",
            )));
        }
        let contract_id = round.contract_id().to_buffer();
        let round_id = round.round_id();
        let deadline_ms = round.record_crossing(crossing_ms, min_wait_ms, max_wait_ms)?;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness_deadline(
                deadline_ms,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        let mut drive_operations = self.update_readiness_round_evaluation_operations(
            round,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        drive_operations.extend(self.clear_readiness_scan_cursor_operations(
            contract_id,
            round_id,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?);

        // The deadline tree at this time may already exist (another round crossed with the
        // same deadline in this or an earlier block), so insert it only if absent.
        let deadline_key = DriveKeyInfo::Key(encode_u64(deadline_ms));
        let path_key_info = deadline_key
            .add_path_info::<0>(PathInfo::PathAsVec(readiness_deadlines_tree_path_vec()));
        let tree_apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: 0,
            }
        };
        self.batch_insert_empty_tree_if_not_exists(
            path_key_info,
            TreeType::NormalTree,
            None,
            tree_apply_type,
            transaction,
            &mut None,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        let entry = Element::new_item(round_id.to_vec());
        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertApplyType::StatefulBatchInsert
        } else {
            BatchInsertApplyType::StatelessBatchInsert {
                in_tree_type: TreeType::NormalTree,
                target: QueryTargetValue(
                    entry.serialized_size(&platform_version.drive.grove_version)? as u32,
                ),
            }
        };
        let deadline_path = readiness_deadline_tree_path_vec(deadline_ms);
        let path_key_element_info: PathKeyElementInfo<'_, 0> =
            if estimated_costs_only_with_layer_info.is_none() {
                PathKeyRefElement((deadline_path, contract_id.as_slice(), entry))
            } else {
                PathKeyElementSize((
                    KeyInfoPath::from_known_owned_path(deadline_path),
                    KeyInfo::MaxKeySize {
                        unique_id: contract_id.to_vec(),
                        max_size: DEFAULT_HASH_SIZE_U8,
                    },
                    entry,
                ))
            };
        self.batch_insert_if_not_exists(
            path_key_element_info,
            apply_type,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;
        Ok((deadline_ms, drive_operations))
    }
}
