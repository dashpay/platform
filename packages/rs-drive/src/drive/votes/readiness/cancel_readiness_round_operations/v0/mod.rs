use crate::drive::votes::paths::{readiness_contract_tree_path, READINESS_CURRENT_ROUND_POINTER_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteApplyType;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U32;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::round::ReadinessRound;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, MaybeTree, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn cancel_readiness_round_operations_v0(
        &self,
        contract_id: [u8; 32],
        cleanup_reserve: Credits,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<ReadinessRound>, Vec<LowLevelDriveOperation>), Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness(
                contract_id,
                [0u8; 32],
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }
        let round = if estimated_costs_only_with_layer_info.is_none() {
            match self.fetch_readiness_round_operations(
                contract_id,
                transaction,
                &mut drive_operations,
                platform_version,
            )? {
                Some(round) => round,
                None => return Ok((None, drive_operations)),
            }
        } else {
            // Estimation has no round to read; nothing below reads state either, and the
            // fixed-size operations are priced against the layer map.
            return Ok((None, drive_operations));
        };

        let (retirement, retire_operations) = self.retire_readiness_round_operations(
            &round,
            cleanup_reserve,
            block_info,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        drive_operations.extend(retire_operations);

        let contract_path = readiness_contract_tree_path(&contract_id);
        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchDeleteApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
            }
        } else {
            BatchDeleteApplyType::StatelessBatchDelete {
                in_tree_type: TreeType::NormalTree,
                estimated_key_size: 1,
                estimated_value_size: DEFAULT_HASH_SIZE_U32,
            }
        };
        self.batch_delete(
            (&contract_path).into(),
            &[READINESS_CURRENT_ROUND_POINTER_KEY as u8],
            apply_type,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        if retirement.refund > 0 {
            if let Some(payer_id) = round.payer().identity_id() {
                drive_operations.extend(self.add_to_identity_balance_operations(
                    payer_id.to_buffer(),
                    retirement.refund,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?);
            }
        }

        Ok((Some(round), drive_operations))
    }
}
