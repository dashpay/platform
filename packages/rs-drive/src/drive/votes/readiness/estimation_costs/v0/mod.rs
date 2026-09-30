use crate::drive::constants::AVERAGE_BALANCE_SIZE;
use crate::drive::credit_pools::epochs::paths::EpochProposers;
use crate::drive::credit_pools::pools_vec_path;
use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_for_readiness_path_vec, prefunded_specialized_balances_path,
};
use crate::drive::votes::paths::{
    readiness_contract_tree_path, readiness_round_tree_path, READINESS_CURRENT_ROUND_POINTER_KEY,
    READINESS_ROUND_RECORD_KEY,
};
use crate::drive::votes::paths::{
    readiness_contract_tree_path_vec, readiness_contracts_tree_path_vec,
    readiness_deadline_tree_path_vec, readiness_deadlines_tree_path_vec,
    readiness_retired_rounds_tree_path_vec, readiness_round_reports_tree_path_vec,
    readiness_round_tree_path_vec, readiness_tree_path_vec, vote_root_path_vec,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use crate::util::type_constants::{
    DEFAULT_HASH_SIZE_U32, DEFAULT_HASH_SIZE_U8, U64_SIZE_U8, U8_SIZE_U8,
};
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::payer::ReadinessPayer;
use dpp::voting::readiness::round::{ReadinessRound, ReadinessRoundOpening};
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::{AllSumTrees, NoSumTrees, SomeSumTrees};
use grovedb::{EstimatedLayerInformation, GroveDb, TransactionArg, TreeType};
use grovedb_costs::CostContext;
use std::collections::HashMap;

/// The largest serialized size of a readiness round record (223 bytes), rounded up: a
/// versioned enum around two identifiers, two 32-byte digests, a handful of integers, the
/// crossed status with two timestamps, the evaluation mark and the typed payer, every integer
/// at its widest varint.
pub(crate) const ESTIMATED_READINESS_ROUND_RECORD_SIZE: u32 = 224;

/// The largest serialized size of a readiness report record (13 bytes), rounded up: a
/// versioned enum around a height and a profile.
pub(crate) const ESTIMATED_READINESS_REPORT_RECORD_SIZE: u32 = 16;

/// The largest serialized size of a readiness scan cursor (59 bytes), rounded up: a versioned
/// enum around two counters, the pagination key and three counts.
pub(crate) const ESTIMATED_READINESS_SCAN_CURSOR_SIZE: u32 = 64;

/// The number of readiness rounds we expect to be pending at once (one per contract with a
/// pending bundle).
const ESTIMATED_PENDING_ROUNDS: u32 = 1_024;

/// The number of reports we expect a round to hold at most: every evonode of the network.
const ESTIMATED_REPORTS_PER_ROUND: u32 = 4_096;

/// The number of entries of the pools tree: one epoch tree per epoch of the perpetual
/// storage window (50 eras of 40 epochs at the default era length) beside the pool items.
const ESTIMATED_POOL_ENTRIES: u32 = 2_048;

/// The number of entries of an epoch's tree: the proposers tree beside the pool, start and
/// multiplier items.
const ESTIMATED_EPOCH_ENTRIES: u32 = 9;

impl Drive {
    /// Describes every layer a readiness round write can touch: the root, the votes tree, the
    /// readiness tree, the contracts tree, the contract's tree (pointer item beside round
    /// trees), the round tree (record item, count tree, cursor item), the reports count tree,
    /// the deadlines tree and the retired rounds tree.
    pub(super) fn add_estimation_costs_for_readiness_v0(
        contract_id: [u8; 32],
        round_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(vote_root_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_tree_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((1, NoSumTrees, None, 3)),
                    items_size: Some((1, DEFAULT_HASH_SIZE_U8 as u32, None, 1)),
                    references_size: None,
                    items_with_sum_item_size: None,
                    references_with_sum_item_size: None,
                },
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_contracts_tree_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(ESTIMATED_PENDING_ROUNDS),
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        // A contract's tree holds the pointer item and normally one round tree, two during
        // the batch that replaces a round.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_contract_tree_path_vec(contract_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(3),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((DEFAULT_HASH_SIZE_U8, NoSumTrees, None, 2)),
                    items_size: Some((U8_SIZE_U8, DEFAULT_HASH_SIZE_U8 as u32, None, 1)),
                    references_size: None,
                    items_with_sum_item_size: None,
                    references_with_sum_item_size: None,
                },
            },
        );

        // A round's tree holds the record item, the reports count tree and possibly the
        // cursor item.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_round_tree_path_vec(
                contract_id,
                round_id,
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(3),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((
                        U8_SIZE_U8,
                        SomeSumTrees {
                            sum_trees_weight: 0,
                            big_sum_trees_weight: 0,
                            count_trees_weight: 1,
                            count_sum_trees_weight: 0,
                            non_sum_trees_weight: 0,
                            provable_sum_trees_weight: 0,
                            provable_count_trees_weight: 0,
                            provable_count_sum_trees_weight: 0,
                            provable_count_provable_sum_trees_weight: 0,
                        },
                        None,
                        1,
                    )),
                    items_size: Some((U8_SIZE_U8, ESTIMATED_READINESS_ROUND_RECORD_SIZE, None, 2)),
                    references_size: None,
                    items_with_sum_item_size: None,
                    references_with_sum_item_size: None,
                },
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_round_reports_tree_path_vec(
                contract_id,
                round_id,
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::CountTree,
                estimated_layer_count: ApproximateElements(ESTIMATED_REPORTS_PER_ROUND),
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    ESTIMATED_READINESS_REPORT_RECORD_SIZE,
                    None,
                ),
            },
        );

        // Deadlines: one subtree per distinct deadline time holding contract id -> round id
        // items. The block time granularity makes distinct times plentiful.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_deadlines_tree_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(ESTIMATED_PENDING_ROUNDS),
                estimated_layer_sizes: AllSubtrees(U64_SIZE_U8, NoSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_retired_rounds_tree_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(ESTIMATED_PENDING_ROUNDS),
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    DEFAULT_HASH_SIZE_U8 as u32,
                    None,
                ),
            },
        );
    }

    /// Describes the per-time tree of one activation deadline: contract id keys with round
    /// id items, a few per distinct time.
    pub(super) fn add_estimation_costs_for_readiness_deadline_v0(
        deadline_ms: u64,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_deadline_tree_path_vec(deadline_ms)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(2),
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    DEFAULT_HASH_SIZE_U8 as u32,
                    None,
                ),
            },
        );
    }

    /// Describes the layers a readiness fund write touches, mirroring the voting fund
    /// estimation on the sibling tree.
    pub(super) fn add_estimation_costs_for_readiness_fund_update_v0(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 1,
                        big_sum_trees_weight: 0,
                        count_trees_weight: 0,
                        count_sum_trees_weight: 0,
                        non_sum_trees_weight: 1,
                        provable_sum_trees_weight: 0,
                        provable_count_trees_weight: 0,
                        provable_count_sum_trees_weight: 0,
                        provable_count_provable_sum_trees_weight: 0,
                    },
                    None,
                ),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(prefunded_specialized_balances_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(1, AllSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(
                prefunded_specialized_balances_for_readiness_path_vec(),
            ),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, AVERAGE_BALANCE_SIZE, None),
            },
        );
    }

    /// Describes the layers a credit to an epoch's processing fee pool touches: the root
    /// (only when no earlier estimation described it), the pools sum tree of epoch trees and
    /// the epoch's sum tree, whose pool is a sum item beside the other epoch items.
    pub(super) fn add_estimation_costs_for_readiness_pool_credit_v0(
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        estimated_costs_only_with_layer_info
            .entry(KeyInfoPath::from_known_path([]))
            .or_insert(EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 1,
                        big_sum_trees_weight: 0,
                        count_trees_weight: 0,
                        count_sum_trees_weight: 0,
                        non_sum_trees_weight: 1,
                        provable_sum_trees_weight: 0,
                        provable_count_trees_weight: 0,
                        provable_count_sum_trees_weight: 0,
                        provable_count_provable_sum_trees_weight: 0,
                    },
                    None,
                ),
            });

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(pools_vec_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: ApproximateElements(ESTIMATED_POOL_ENTRIES),
                estimated_layer_sizes: AllSubtrees(2, AllSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(epoch.get_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: ApproximateElements(ESTIMATED_EPOCH_ENTRIES),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((U8_SIZE_U8, NoSumTrees, None, 1)),
                    items_size: Some((U8_SIZE_U8, U64_SIZE_U8 as u32, None, 8)),
                    references_size: None,
                    items_with_sum_item_size: None,
                    references_with_sum_item_size: None,
                },
            },
        );
    }

    /// Prices the reads of a contract's current readiness round without state and returns a
    /// placeholder for the largest round shape it could be: a crossed round (with a deadline
    /// entry and time tree to drop) funded by an identity (with a payer to refund). An
    /// estimate that retires the current round prices that placeholder's retirement so it
    /// covers every stored round.
    pub(in crate::drive::votes::readiness) fn estimate_current_readiness_round_v0(
        &self,
        contract_id: [u8; 32],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ReadinessRound, Error> {
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
        let contract_path = readiness_contract_tree_path(&contract_id);
        self.grove_get_raw_optional(
            (&contract_path).into(),
            &[READINESS_CURRENT_ROUND_POINTER_KEY as u8],
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTargetValue(DEFAULT_HASH_SIZE_U32),
            },
            transaction,
            drive_operations,
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
            drive_operations,
            &platform_version.drive,
        )?;
        Ok(placeholder)
    }

    /// Prices the merk walk of one estimated report delete from a round's count tree.
    ///
    /// The batch estimate charges the propagation of a layer once, while the merk walks and
    /// rehashes the path of every deleted key: this prices that walk per key, with
    /// propagation. Applied deletes in one batch share ancestors they rewrite once, so the
    /// sum over a batch is an upper bound (about ten times a 512-delete batch); its margin
    /// also covers a caller's handful of fixed reads. The caller describes the readiness
    /// layers first.
    pub(in crate::drive::votes::readiness) fn add_estimated_readiness_report_delete_walk_v0(
        contract_id: [u8; 32],
        round_id: [u8; 32],
        key: &KeyInfo,
        estimated_costs_only_with_layer_info: &HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let reports_path = KeyInfoPath::from_known_owned_path(
            readiness_round_reports_tree_path_vec(contract_id, round_id),
        );
        let reports_layer = estimated_costs_only_with_layer_info
            .get(&reports_path)
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "the readiness estimation describes the reports tree",
            )))?;
        let CostContext { value, cost } = GroveDb::average_case_merk_delete_element(
            key,
            reports_layer,
            true,
            &platform_version.drive.grove_version,
        );
        value?;
        drive_operations.push(CalculatedCostOperation(cost));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::serialization::PlatformSerializable;
    use dpp::voting::readiness::report_record::ReadinessReportRecord;
    use dpp::voting::readiness::round::ReadinessEvaluation;
    use dpp::voting::readiness::scan_cursor::ReadinessScanCursor;

    #[test]
    fn should_cover_the_widest_serialized_readiness_records() {
        let platform_version = PlatformVersion::latest();

        // Every integer past u32::MAX takes the widest varint; the round is crossed and
        // evaluated so that every optional field is present.
        let mut round = ReadinessRound::new(
            u32::MAX,
            ReadinessRoundOpening {
                contract_id: Identifier::new([0xFFu8; 32]),
                version: u32::MAX,
                bundle_digest: [0xFFu8; 32],
                preparation_profile: u16::MAX,
                accepted_at_ms: 1 << 40,
                accepted_at_height: u64::MAX,
                payer: ReadinessPayer::Identity(Identifier::new([0xFFu8; 32])),
            },
            platform_version,
        )
        .expect("round");
        round.set_evaluation(
            ReadinessEvaluation {
                core_height: u32::MAX,
                raw_count: u64::MAX,
            },
            true,
        );
        round
            .record_crossing(1 << 41, 1 << 40, 1 << 40)
            .expect("crossing");
        let round_size = round.serialize_to_bytes().expect("round bytes").len();
        assert_eq!(round_size, 223);
        assert!(ESTIMATED_READINESS_ROUND_RECORD_SIZE as usize >= round_size);

        let report =
            ReadinessReportRecord::new(u64::MAX, u16::MAX, platform_version).expect("report");
        let report_size = report.serialize_to_bytes().expect("report bytes").len();
        assert_eq!(report_size, 13);
        assert!(ESTIMATED_READINESS_REPORT_RECORD_SIZE as usize >= report_size);

        let mut cursor =
            ReadinessScanCursor::new(u32::MAX, u32::MAX, platform_version).expect("cursor");
        cursor
            .advance([0xFFu8; 32], u32::MAX, u32::MAX, u32::MAX)
            .expect("advance");
        let cursor_size = cursor.serialize_to_bytes().expect("cursor bytes").len();
        assert_eq!(cursor_size, 59);
        assert!(ESTIMATED_READINESS_SCAN_CURSOR_SIZE as usize >= cursor_size);
    }
}
