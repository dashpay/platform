use crate::drive::credit_pools::epochs::epoch_key_constants::KEY_POOL_PROCESSING_FEES;
use crate::drive::credit_pools::epochs::paths::EpochProposers;
use crate::drive::votes::paths::{
    readiness_deadline_tree_path_vec, readiness_retired_rounds_tree_path_vec,
};
use crate::drive::votes::readiness::retire_readiness_round_operations::ReadinessRetirement;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::{BatchDeleteApplyType, DirectQueryType};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U32;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::round::ReadinessRound;
use dpp::ProtocolError;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, MaybeTree, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn retire_readiness_round_operations_v0(
        &self,
        round: &ReadinessRound,
        cleanup_reserve: Credits,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(ReadinessRetirement, Vec<LowLevelDriveOperation>), Error> {
        let contract_id = round.contract_id().to_buffer();
        let round_id = round.round_id();
        let mut drive_operations = vec![];

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness(
                contract_id,
                round_id,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        // 1. Queue the round for the bounded cleanup. Its subtree stays under its own key.
        drive_operations.push(GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
            readiness_retired_rounds_tree_path_vec(),
            round_id.to_vec(),
            Element::new_item(contract_id.to_vec()),
        )));

        // 2. Drop the deadline entry of a crossed round. The per-time tree stays: another
        //    round may share the time, and an empty time tree costs a delete-up-tree walk the
        //    deferred cleanup is not asked to do; the block event skips stale entries.
        if let Some(deadline_ms) = round.deadline_ms() {
            let deadline_path = readiness_deadline_tree_path_vec(deadline_ms);
            let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                let present = self.grove_has_raw(
                    deadline_path.as_slice().into(),
                    &contract_id,
                    DirectQueryType::StatefulDirectQuery,
                    transaction,
                    &mut drive_operations,
                    &platform_version.drive,
                )?;
                if present {
                    Some(BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                    })
                } else {
                    None
                }
            } else {
                Some(BatchDeleteApplyType::StatelessBatchDelete {
                    in_tree_type: TreeType::NormalTree,
                    estimated_key_size: DEFAULT_HASH_SIZE_U32,
                    estimated_value_size: DEFAULT_HASH_SIZE_U32,
                })
            };
            if let Some(apply_type) = apply_type {
                self.batch_delete(
                    deadline_path.as_slice().into(),
                    &contract_id,
                    apply_type,
                    transaction,
                    &mut drive_operations,
                    &platform_version.drive,
                )?;
            }
        }

        // 3. Empty the fund. In estimation mode the helper prices the delete and reports a
        //    zero balance; the settlement below is then zero as well.
        let (fund_balance, fund_operations) = self.empty_readiness_fund_operations(
            round.funding_id(),
            false,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        drive_operations.extend(fund_operations);

        // 4. Charge the cleanup reserve to the epoch's processing pool, capped at the fund.
        let cleanup_charged = cleanup_reserve.min(fund_balance);
        let refund = fund_balance
            .checked_sub(cleanup_charged)
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "cleanup charge exceeds the fund it was capped at",
            )))?;
        if cleanup_charged > 0 {
            let mut read_operations = vec![];
            let pool_operation = self.add_readiness_pool_credit_operation(
                block_info,
                cleanup_charged,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                &mut read_operations,
                platform_version,
            )?;
            drive_operations.extend(read_operations);
            drive_operations.push(pool_operation);
        }

        Ok((
            ReadinessRetirement {
                fund_balance,
                cleanup_charged,
                refund,
            },
            drive_operations,
        ))
    }

    /// The operation that adds `amount` to the block epoch's processing fee pool, reading
    /// the current pool value in the same transaction. In estimation mode the read is
    /// priced and the write is priced as an insert of a sum item.
    pub(crate) fn add_readiness_pool_credit_operation(
        &self,
        block_info: &BlockInfo,
        amount: Credits,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<LowLevelDriveOperation, Error> {
        let epoch_tree_path = block_info.epoch.get_path();
        let existing_value = if apply {
            match self.grove_get_raw_optional(
                (&epoch_tree_path).into(),
                KEY_POOL_PROCESSING_FEES.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                drive_operations,
                &platform_version.drive,
            )? {
                None => 0,
                Some(Element::SumItem(existing_value, _)) => existing_value,
                Some(_) => {
                    return Err(Error::Drive(DriveError::UnexpectedElementType(
                        "epochs processing fee must be a sum item",
                    )))
                }
            }
        } else {
            0
        };
        if amount > i64::MAX as u64 {
            return Err(Error::Protocol(Box::new(ProtocolError::Overflow(
                "adding over i64::MAX to the processing fee pool",
            ))));
        }
        let updated_value = existing_value
            .checked_add(amount as i64)
            .ok_or(ProtocolError::Overflow("overflow when adding to the processing fee pool"))?;
        Ok(LowLevelDriveOperation::insert_for_known_path_key_element(
            epoch_tree_path.iter().map(|segment| segment.to_vec()).collect(),
            KEY_POOL_PROCESSING_FEES.to_vec(),
            Element::new_sum_item(updated_value),
        ))
    }
}
