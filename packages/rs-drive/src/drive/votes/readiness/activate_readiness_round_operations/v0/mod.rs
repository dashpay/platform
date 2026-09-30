use crate::drive::votes::paths::{
    readiness_contract_tree_path, READINESS_CURRENT_ROUND_POINTER_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
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
    #[allow(clippy::too_many_arguments)]
    pub(super) fn activate_readiness_round_operations_v0(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        cleanup_reserve: Credits,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(ReadinessRound, Vec<LowLevelDriveOperation>), Error> {
        let mut drive_operations = vec![];
        let round = if estimated_costs_only_with_layer_info.is_none() {
            let round = self
                .fetch_readiness_round_operations(
                    contract_id,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?
                .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                    "activating a readiness round of a contract that has none".to_string(),
                )))?;
            if round.round_id() != round_id {
                return Err(Error::Drive(DriveError::CorruptedDriveState(
                    "activating a readiness round that is not the contract's current round"
                        .to_string(),
                )));
            }
            if round.is_pending() {
                return Err(Error::Drive(DriveError::CorruptedDriveState(
                    "activating a readiness round that has not crossed".to_string(),
                )));
            }
            round
        } else {
            // An estimate reads no state: it prices the pointer and record reads, then the
            // activation of the largest round shape so the estimate covers every case.
            self.estimate_current_readiness_round_v0(
                contract_id,
                block_info,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
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

        // An estimate always prices the credit: the placeholder's fund reads as empty.
        if retirement.refund > 0 || estimated_costs_only_with_layer_info.is_some() {
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

        Ok((round, drive_operations))
    }
}
