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
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use crate::util::grove_operations::{BatchDeleteUpTreeApplyType, DirectQueryType};
use crate::util::type_constants::{
    DEFAULT_HASH_SIZE_U32, DEFAULT_HASH_SIZE_U8, U64_SIZE_U32, U64_SIZE_U8,
};
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::round::ReadinessRound;
use dpp::ProtocolError;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::EstimatedLayerCount::ApproximateElements;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{Element, EstimatedLayerInformation, MaybeTree, TransactionArg, TreeType};
use intmap::IntMap;
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
            if let Some(deadline_ms) = round.deadline_ms() {
                Self::add_estimation_costs_for_readiness_deadline(
                    deadline_ms,
                    estimated_costs_only_with_layer_info,
                    platform_version,
                )?;
            }
        }

        // 1. Queue the round for the bounded cleanup. Its subtree stays under its own key.
        drive_operations.push(GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
            readiness_retired_rounds_tree_path_vec(),
            round_id.to_vec(),
            Element::new_item(contract_id.to_vec()),
        )));

        // 2. Drop the deadline entry of a crossed round, and its per-time tree when that
        //    entry was the last one: the due query walks the time keys in order under a
        //    limit, and an empty time tree still spends it, so a run of emptied trees ahead
        //    of a live deadline would hide that deadline from every block. The walk up stops
        //    at the deadlines tree and deletes the time tree only when nothing else is in it
        //    (another round sharing the time keeps it). It sees this batch's own operations
        //    only, which the one retirement per batch rule already requires.
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
                    Some(BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                    })
                } else {
                    None
                }
            } else {
                // Price both levels, keyed by the index of the deleted key in the path: the
                // entry in the time tree (3), then the time key in the deadlines tree (2).
                Some(BatchDeleteUpTreeApplyType::StatelessBatchDelete {
                    estimated_layer_info: IntMap::from_iter([
                        (
                            3,
                            EstimatedLayerInformation {
                                tree_type: TreeType::NormalTree,
                                estimated_layer_count: ApproximateElements(2),
                                estimated_layer_sizes: AllItems(
                                    DEFAULT_HASH_SIZE_U8,
                                    DEFAULT_HASH_SIZE_U32,
                                    None,
                                ),
                            },
                        ),
                        (
                            2,
                            EstimatedLayerInformation {
                                tree_type: TreeType::NormalTree,
                                estimated_layer_count: ApproximateElements(1_024),
                                estimated_layer_sizes: AllSubtrees(U64_SIZE_U8, NoSumTrees, None),
                            },
                        ),
                    ]),
                })
            };
            if let Some(apply_type) = apply_type {
                self.batch_delete_up_tree_while_empty(
                    KeyInfoPath::from_known_owned_path(deadline_path),
                    &contract_id,
                    // Stop at the readiness tree's path (length 2): the deadlines tree stays.
                    Some(2),
                    apply_type,
                    transaction,
                    &None,
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
        //    An estimate read no fund, so it prices the charge of the whole reserve.
        let cleanup_charged = cleanup_reserve.min(fund_balance);
        let refund = fund_balance
            .checked_sub(cleanup_charged)
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "cleanup charge exceeds the fund it was capped at",
            )))?;
        let priced_charge = if estimated_costs_only_with_layer_info.is_none() {
            cleanup_charged
        } else {
            cleanup_reserve
        };
        if priced_charge > 0 {
            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Self::add_estimation_costs_for_readiness_pool_credit(
                    &block_info.epoch,
                    estimated_costs_only_with_layer_info,
                    platform_version,
                )?;
            }
            let mut read_operations = vec![];
            let pool_operation = self.add_readiness_pool_credit_operation(
                block_info,
                priced_charge,
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
    /// priced without state and the write is priced as an insert of a sum item; the caller
    /// describes the pool layers first.
    ///
    /// Readiness credits reach the pool this way (the cleanup reserve at retirement, the
    /// membership lookup fees of the block event) so that every credit leaving a readiness
    /// fund lands in a bucket conservation counts. The write is an absolute rewrite of the
    /// pool item, so one such operation per applied batch.
    ///
    /// # Parameters
    ///
    /// * `block_info` - The block whose epoch receives the credits.
    /// * `amount` - The credits to add.
    /// * `apply` - Whether to read state (`true`) or only estimate the cost (`false`).
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The accumulator the read cost is appended to.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The operation that writes the pool item.
    pub fn add_readiness_pool_credit_operation(
        &self,
        block_info: &BlockInfo,
        amount: Credits,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<LowLevelDriveOperation, Error> {
        let epoch_tree_path = block_info.epoch.get_path();
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::SumTree,
                query_target: QueryTargetValue(U64_SIZE_U32),
            }
        };
        let existing_value = match self.grove_get_raw_optional(
            (&epoch_tree_path).into(),
            KEY_POOL_PROCESSING_FEES.as_slice(),
            direct_query_type,
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
        };
        if amount > i64::MAX as u64 {
            return Err(Error::Protocol(Box::new(ProtocolError::Overflow(
                "adding over i64::MAX to the processing fee pool",
            ))));
        }
        let updated_value =
            existing_value
                .checked_add(amount as i64)
                .ok_or(ProtocolError::Overflow(
                    "overflow when adding to the processing fee pool",
                ))?;
        Ok(LowLevelDriveOperation::insert_for_known_path_key_element(
            epoch_tree_path
                .iter()
                .map(|segment| segment.to_vec())
                .collect(),
            KEY_POOL_PROCESSING_FEES.to_vec(),
            Element::new_sum_item(updated_value),
        ))
    }
}
