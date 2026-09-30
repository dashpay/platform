use crate::drive::votes::paths::{
    readiness_contract_tree_path, readiness_contract_tree_path_vec, readiness_contracts_tree_path,
    readiness_contracts_tree_path_vec, readiness_retired_rounds_tree_path,
    readiness_retired_rounds_tree_path_vec, readiness_round_reports_tree_path_vec,
    readiness_round_tree_path, readiness_round_tree_path_vec, READINESS_CURRENT_ROUND_POINTER_KEY,
    READINESS_ROUND_RECORD_KEY, READINESS_ROUND_REPORTS_TREE_KEY, READINESS_ROUND_SCAN_CURSOR_KEY,
};
use crate::drive::votes::readiness::cleanup_retired_readiness_round_operations::ReadinessCleanupOutcome;
use crate::drive::votes::readiness::estimation_costs::ESTIMATED_READINESS_REPORT_RECORD_SIZE;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::{CalculatedCostOperation, GroveOperation};
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use crate::util::grove_operations::{BatchDeleteApplyType, DirectQueryType};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U32;
use dpp::version::PlatformVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::SubelementsDeletionBehavior;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::query_result_type::QueryResultType;
use grovedb::{
    EstimatedLayerInformation, GroveDb, MaybeTree, PathQuery, Query, SizedQuery, TransactionArg,
    TreeType,
};
use grovedb_costs::CostContext;
use std::collections::HashMap;

impl Drive {
    pub(super) fn cleanup_retired_readiness_round_operations_v0(
        &self,
        round_id: [u8; 32],
        contract_id: [u8; 32],
        max_deletes: u16,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(ReadinessCleanupOutcome, Vec<LowLevelDriveOperation>), Error> {
        let mut drive_operations = vec![];

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness(
                contract_id,
                round_id,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
            // An estimate prices the worst step: `max_deletes` report deletes plus the
            // fixed tail that removes the round, so it is at or above any applied step.
            let reports_path_vec = readiness_round_reports_tree_path_vec(contract_id, round_id);
            let reports_path = KeyInfoPath::from_known_owned_path(reports_path_vec.clone());
            let reports_layer = estimated_costs_only_with_layer_info
                .get(&reports_path)
                .cloned()
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "the readiness estimation describes the reports tree",
                )))?;
            let read_report = DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::CountTree,
                query_target: QueryTargetValue(ESTIMATED_READINESS_REPORT_RECORD_SIZE),
            };
            for index in 0..max_deletes {
                // The applied step reads each report twice before the batch: once through
                // the bounded query and once to build its delete.
                for _ in 0..2 {
                    self.grove_get_raw_optional(
                        reports_path_vec.as_slice().into(),
                        &[0u8; 32],
                        read_report,
                        transaction,
                        &mut drive_operations,
                        &platform_version.drive,
                    )?;
                }
                let key = KeyInfo::MaxKeySize {
                    unique_id: index.to_be_bytes().to_vec(),
                    max_size: DEFAULT_HASH_SIZE_U32 as u8,
                };
                // The batch estimate charges the propagation of a layer once, while the merk
                // walks and rehashes the path of every deleted key: price that walk per key.
                // The applied step deletes the lowest keys, whose shared ancestors it rewrites
                // once, so this is an upper bound (about ten times a 512-delete step); its
                // margin also covers the step's handful of fixed reads.
                let CostContext { value, cost } = GroveDb::average_case_merk_delete_element(
                    &key,
                    &reports_layer,
                    true,
                    &platform_version.drive.grove_version,
                );
                value?;
                drive_operations.push(CalculatedCostOperation(cost));
                drive_operations.push(GroveOperation(QualifiedGroveDbOp::delete_estimated_op(
                    reports_path.clone(),
                    key,
                )));
            }
            Self::add_readiness_cleanup_tail_estimated_operations(
                contract_id,
                round_id,
                &mut drive_operations,
            );
            return Ok((
                ReadinessCleanupOutcome {
                    reports_deleted: max_deletes as u64,
                    finished: true,
                },
                drive_operations,
            ));
        }

        // Does the round tree still exist? A queue entry may outlive it only if a previous
        // step removed the tree but the batch failed before the entry; then just drop the
        // entry.
        let round_path = readiness_round_tree_path(&contract_id, &round_id);
        let contract_path = readiness_contract_tree_path(&contract_id);
        let round_tree_exists = self.grove_has_raw(
            (&contract_path).into(),
            &round_id,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        let mut reports_deleted = 0u64;
        let mut remaining = 0u64;
        if round_tree_exists {
            // Delete up to `max_deletes` reports through the bounded path-query delete.
            if max_deletes > 0 {
                let mut query = Query::new_with_direction(true);
                query.insert_all();
                let path_query = PathQuery::new(
                    readiness_round_reports_tree_path_vec(contract_id, round_id),
                    SizedQuery::new(query, Some(max_deletes), None),
                );
                let before = drive_operations.len();
                self.batch_delete_items_in_path_query(
                    &path_query,
                    false,
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                    },
                    transaction,
                    &mut drive_operations,
                    &platform_version.drive,
                )?;
                reports_deleted = drive_operations[before..]
                    .iter()
                    .filter(|operation| matches!(operation, GroveOperation(_)))
                    .count() as u64;
            }
            let raw_count = self.fetch_readiness_round_raw_count_operations(
                contract_id,
                round_id,
                transaction,
                &mut drive_operations,
                platform_version,
            )?;
            remaining = raw_count.saturating_sub(reports_deleted);
        }

        if remaining > 0 {
            return Ok((
                ReadinessCleanupOutcome {
                    reports_deleted,
                    finished: false,
                },
                drive_operations,
            ));
        }

        let delete_item = BatchDeleteApplyType::StatefulBatchDelete {
            is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
        };

        if round_tree_exists {
            // Record, cursor (if any) and the now-empty count tree.
            self.batch_delete(
                (&round_path).into(),
                &[READINESS_ROUND_RECORD_KEY],
                delete_item,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
            if self.grove_has_raw(
                (&round_path).into(),
                &[READINESS_ROUND_SCAN_CURSOR_KEY],
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )? {
                self.batch_delete(
                    (&round_path).into(),
                    &[READINESS_ROUND_SCAN_CURSOR_KEY],
                    delete_item,
                    transaction,
                    &mut drive_operations,
                    &platform_version.drive,
                )?;
            }
            // The count tree: its reports were all deleted in this batch or earlier ones,
            // so the emptiness check of the delete (which counts same-batch deletes as gone)
            // passes and the tree element goes with no cleanup.
            self.batch_delete(
                (&round_path).into(),
                &[READINESS_ROUND_REPORTS_TREE_KEY],
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some(MaybeTree::Tree(TreeType::CountTree)),
                },
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
            // The round tree itself, emptied by the three deletes above.
            self.batch_delete(
                (&contract_path).into(),
                &round_id,
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some(MaybeTree::Tree(TreeType::NormalTree)),
                },
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
            // The contract tree when nothing else lives there: no pointer (a live round
            // would have one) and no other round tree.
            let pointer_present = self.grove_has_raw(
                (&contract_path).into(),
                &[READINESS_CURRENT_ROUND_POINTER_KEY as u8],
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
            if !pointer_present {
                let mut query = Query::new_with_direction(true);
                query.insert_all();
                let path_query = PathQuery::new(
                    readiness_contract_tree_path_vec(contract_id),
                    SizedQuery::new(query, Some(2), None),
                );
                let siblings = self
                    .grove_get_raw_path_query(
                        &path_query,
                        transaction,
                        QueryResultType::QueryKeyElementPairResultType,
                        &mut drive_operations,
                        &platform_version.drive,
                    )?
                    .0
                    .to_keys();
                let only_this_round = siblings.iter().all(|key| key.as_slice() == round_id);
                if only_this_round {
                    let contracts_path = readiness_contracts_tree_path();
                    self.batch_delete(
                        (&contracts_path).into(),
                        &contract_id,
                        BatchDeleteApplyType::StatefulBatchDelete {
                            is_known_to_be_subtree_with_sum: Some(MaybeTree::Tree(
                                TreeType::NormalTree,
                            )),
                        },
                        transaction,
                        &mut drive_operations,
                        &platform_version.drive,
                    )?;
                }
            }
        }

        // The queue entry.
        let retired_path = readiness_retired_rounds_tree_path();
        self.batch_delete(
            (&retired_path).into(),
            &round_id,
            delete_item,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Ok((
            ReadinessCleanupOutcome {
                reports_deleted,
                finished: true,
            },
            drive_operations,
        ))
    }

    /// The estimated operations of the tail of a cleanup step: the record, cursor, count
    /// tree, round tree, contract tree and queue entry deletes.
    fn add_readiness_cleanup_tail_estimated_operations(
        contract_id: [u8; 32],
        round_id: [u8; 32],
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) {
        let round_path = KeyInfoPath::from_known_owned_path(readiness_round_tree_path_vec(
            contract_id,
            round_id,
        ));
        for key in [READINESS_ROUND_RECORD_KEY, READINESS_ROUND_SCAN_CURSOR_KEY] {
            drive_operations.push(GroveOperation(QualifiedGroveDbOp::delete_estimated_op(
                round_path.clone(),
                KeyInfo::KnownKey(vec![key]),
            )));
        }
        drive_operations.push(GroveOperation(
            QualifiedGroveDbOp::delete_estimated_tree_op(
                round_path,
                KeyInfo::KnownKey(vec![READINESS_ROUND_REPORTS_TREE_KEY]),
                TreeType::CountTree,
                SubelementsDeletionBehavior::DontCheckWithNoCleanup,
            ),
        ));
        let contract_path =
            KeyInfoPath::from_known_owned_path(readiness_contract_tree_path_vec(contract_id));
        drive_operations.push(GroveOperation(
            QualifiedGroveDbOp::delete_estimated_tree_op(
                contract_path,
                KeyInfo::KnownKey(round_id.to_vec()),
                TreeType::NormalTree,
                SubelementsDeletionBehavior::DontCheckWithNoCleanup,
            ),
        ));
        let contracts_path =
            KeyInfoPath::from_known_owned_path(readiness_contracts_tree_path_vec());
        drive_operations.push(GroveOperation(
            QualifiedGroveDbOp::delete_estimated_tree_op(
                contracts_path,
                KeyInfo::KnownKey(contract_id.to_vec()),
                TreeType::NormalTree,
                SubelementsDeletionBehavior::DontCheckWithNoCleanup,
            ),
        ));
        let retired_path =
            KeyInfoPath::from_known_owned_path(readiness_retired_rounds_tree_path_vec());
        drive_operations.push(GroveOperation(QualifiedGroveDbOp::delete_estimated_op(
            retired_path,
            KeyInfo::KnownKey(round_id.to_vec()),
        )));
    }
}
