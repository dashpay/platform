use crate::drive::votes::paths::{
    readiness_contract_tree_path, readiness_round_tree_path, READINESS_CURRENT_ROUND_POINTER_KEY,
    READINESS_ROUND_RECORD_KEY,
};
use crate::drive::votes::readiness::estimation_costs::ESTIMATED_READINESS_ROUND_RECORD_SIZE;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use crate::util::grove_operations::{BatchDeleteApplyType, DirectQueryType};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U32;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::payer::ReadinessPayer;
use dpp::voting::readiness::round::{ReadinessRound, ReadinessRoundOpening};
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
        let contract_path = readiness_contract_tree_path(&contract_id);
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
            // An estimate reads no state: it prices the pointer and record reads, then the
            // cancellation of the largest round shape (a crossed round, so its deadline entry
            // and time tree go too, with a fund to settle and a payer to refund) so the
            // estimate covers every case.
            let mut placeholder = ReadinessRound::new(
                self.config.network.magic(),
                ReadinessRoundOpening {
                    contract_id: Identifier::new(contract_id),
                    version: 0,
                    bundle_digest: [0u8; 32],
                    preparation_profile: 0,
                    accepted_at_ms: block_info.time_ms,
                    accepted_at_height: block_info.height,
                    payer: ReadinessPayer::Identity(Identifier::new([0u8; 32])),
                },
                platform_version,
            )?;
            placeholder.record_crossing(block_info.time_ms, 0, u64::MAX)?;
            self.grove_get_raw_optional(
                (&contract_path).into(),
                &[READINESS_CURRENT_ROUND_POINTER_KEY as u8],
                DirectQueryType::StatelessDirectQuery {
                    in_tree_type: TreeType::NormalTree,
                    query_target: QueryTargetValue(DEFAULT_HASH_SIZE_U32),
                },
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
            let round_id = placeholder.round_id();
            let round_path = readiness_round_tree_path(&contract_id, &round_id);
            self.grove_get_raw_optional(
                (&round_path).into(),
                &[READINESS_ROUND_RECORD_KEY],
                DirectQueryType::StatelessDirectQuery {
                    in_tree_type: TreeType::NormalTree,
                    query_target: QueryTargetValue(ESTIMATED_READINESS_ROUND_RECORD_SIZE),
                },
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
            placeholder
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

        if estimated_costs_only_with_layer_info.is_some() {
            return Ok((None, drive_operations));
        }
        Ok((Some(round), drive_operations))
    }
}
