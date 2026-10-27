use crate::drive::document::expiration::paths::documents_expirations_at_time_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteUpTreeApplyType;
use crate::util::type_constants::{DEFAULT_HASH_SIZE_U8, U64_SIZE_U8};
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::ApproximateElements;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, MaybeTree, TransactionArg, TreeType};
use intmap::IntMap;
use std::collections::HashMap;

/// Where the removal stops climbing: it removes the entry (a key of `Misc / E / <time>`) and,
/// once that is empty, the tree of its time (a key of `Misc / E`), but never `Misc / E`
/// itself, the key of the one-element path `Misc`.
const DOCUMENTS_EXPIRATIONS_TREE_HEIGHT: u16 = 1;

impl Drive {
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn remove_document_expiration_operations_v0(
        &self,
        document_id: [u8; 32],
        expires_at_ms: TimestampMillis,
        entry_value_size: u32,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        check_existing_operations: &Option<&mut Vec<LowLevelDriveOperation>>,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let apply_type = if let Some(estimated_costs_only_with_layer_info) =
            estimated_costs_only_with_layer_info
        {
            Self::add_estimation_costs_for_document_expiration(
                expires_at_ms,
                entry_value_size,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            // The worst case: the entry was its time's last, so the time's tree goes too.
            // Keyed by the height of the key each deletion removes: the entry among the
            // entries of its time, then the time among the times.
            BatchDeleteUpTreeApplyType::StatelessBatchDelete {
                estimated_layer_info: IntMap::from_iter([
                    (
                        DOCUMENTS_EXPIRATIONS_TREE_HEIGHT,
                        EstimatedLayerInformation {
                            tree_type: TreeType::NormalTree,
                            estimated_layer_count: ApproximateElements(6_307_200),
                            estimated_layer_sizes: AllSubtrees(U64_SIZE_U8, NoSumTrees, None),
                        },
                    ),
                    (
                        DOCUMENTS_EXPIRATIONS_TREE_HEIGHT + 1,
                        EstimatedLayerInformation {
                            tree_type: TreeType::NormalTree,
                            estimated_layer_count: ApproximateElements(16),
                            estimated_layer_sizes: AllItems(
                                DEFAULT_HASH_SIZE_U8,
                                entry_value_size,
                                None,
                            ),
                        },
                    ),
                ]),
            }
        } else {
            BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
            }
        };
        self.batch_delete_up_tree_while_empty(
            KeyInfoPath::from_known_owned_path(documents_expirations_at_time_path_vec(
                expires_at_ms,
            )),
            document_id.as_slice(),
            Some(DOCUMENTS_EXPIRATIONS_TREE_HEIGHT),
            apply_type,
            transaction,
            check_existing_operations,
            batch_operations,
            &platform_version.drive,
        )
    }
}
