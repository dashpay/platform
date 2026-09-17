use crate::drive::votes::paths::readiness_round_reports_tree_path_vec;
use crate::drive::votes::readiness::estimation_costs::ESTIMATED_READINESS_REPORT_RECORD_SIZE;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertApplyType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use crate::util::object_size_info::PathKeyElementInfo;
use crate::util::object_size_info::PathKeyElementInfo::{PathKeyElementSize, PathKeyRefElement};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::report_record::ReadinessReportRecord;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn insert_readiness_report_operations_v0(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        pro_tx_hash: [u8; 32],
        record: &ReadinessReportRecord,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(bool, Vec<LowLevelDriveOperation>), Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness(
                contract_id,
                round_id,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }
        let element = Element::new_item(record.serialize_to_bytes()?);
        let reports_path = readiness_round_reports_tree_path_vec(contract_id, round_id);
        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertApplyType::StatefulBatchInsert
        } else {
            BatchInsertApplyType::StatelessBatchInsert {
                in_tree_type: TreeType::CountTree,
                target: QueryTargetValue(ESTIMATED_READINESS_REPORT_RECORD_SIZE),
            }
        };
        let path_key_element_info: PathKeyElementInfo<'_, 0> =
            if estimated_costs_only_with_layer_info.is_none() {
                PathKeyRefElement((reports_path, pro_tx_hash.as_slice(), element))
            } else {
                PathKeyElementSize((
                    KeyInfoPath::from_known_owned_path(reports_path),
                    KeyInfo::MaxKeySize {
                        unique_id: pro_tx_hash.to_vec(),
                        max_size: DEFAULT_HASH_SIZE_U8,
                    },
                    element,
                ))
            };
        let inserted = self.batch_insert_if_not_exists(
            path_key_element_info,
            apply_type,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;
        Ok((inserted, drive_operations))
    }
}
