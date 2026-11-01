use crate::util::batch::{DriveOperation, SystemOperationType};

use crate::drive::Drive;
use crate::error::{drive::DriveError, fee::FeeError, Error};
use crate::fees::op::LowLevelDriveOperation;

use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use grovedb::TransactionArg;

use dpp::version::PlatformVersion;

use crate::drive::RootTree;
use crate::util::batch::drive_op_batch::finalize_task::{
    DriveOperationFinalizationTasks, DriveOperationFinalizeTask,
};
use crate::util::estimated_layers::EstimatedLayers;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;

/// The level of the root tree each root key sits on, as `drive/mod.rs` draws the root tree and
/// `grovedb-structure.json` records it (`layer_shapes.root`, kept in step with a real GroveDB by
/// `structure::tests`).
const ROOT_KEY_LEVELS: [(u8, u32); 18] = [
    (RootTree::DataContractDocuments as u8, 0),
    (RootTree::Identities as u8, 1),
    (RootTree::Balances as u8, 1),
    (RootTree::Tokens as u8, 2),
    (RootTree::Pools as u8, 2),
    (RootTree::WithdrawalTransactions as u8, 2),
    (RootTree::Votes as u8, 2),
    (RootTree::NonUniquePublicKeyKeyHashesToIdentities as u8, 3),
    (RootTree::UniquePublicKeyHashesToIdentities as u8, 3),
    (RootTree::PreFundedSpecializedBalances as u8, 3),
    (RootTree::AddressBalances as u8, 3),
    (RootTree::SpentAssetLockTransactions as u8, 3),
    (RootTree::GroupActions as u8, 3),
    (RootTree::Misc as u8, 3),
    (RootTree::Versions as u8, 3),
    (RootTree::SavedBlockTransactions as u8, 4),
    (RootTree::ShieldedBalances as u8, 4),
    (RootTree::ContractGroups as u8, 4),
];

/// The deepest level of the root tree a root key `operations` write under sits on: the key
/// opening an operation's path, or the key of an operation at the root itself.
fn deepest_root_key_level<'a>(
    operations: impl IntoIterator<Item = &'a LowLevelDriveOperation>,
) -> Option<u32> {
    operations
        .into_iter()
        .filter_map(|operation| match operation {
            LowLevelDriveOperation::GroveOperation(operation)
            | LowLevelDriveOperation::EphemeralGroveOperation(operation, _) => Some(operation),
            _ => None,
        })
        .filter_map(|operation| match operation.path.0.first() {
            Some(first) => Some(first),
            None => operation.key.as_ref(),
        })
        .filter_map(|root_key| match root_key {
            KnownKey(key) => match key.as_slice() {
                [key] => ROOT_KEY_LEVELS
                    .iter()
                    .find(|(root_key, _)| root_key == key)
                    .map(|(_, level)| *level),
                _ => None,
            },
            _ => None,
        })
        .max()
}

impl Drive {
    /// Generation 2 (protocol version 15): generation 1, with a resolved estimate. GroveDB prices
    /// every operation at a layer's path with one description, and in generation 1 the last write
    /// to describe a layer sets it, whether that write is another operation of the batch or a
    /// nested estimator of the same operation: an identity contract nonce update (the root at
    /// level 1) built after a prefunded specialized balance write (the root at level 3) lowered
    /// the batch's estimate of the root, and a token mint describes the root at level 3 for its
    /// supply and then at level 2 for its balance. The contract fee pot estimator writes the root
    /// (at level 3) only when nothing described it, so an action fee's pot write, which comes
    /// after the transition's own operations, was priced at the root its identity balance write
    /// described (level 1).
    ///
    /// Here, once the whole batch is lowered, the estimate is resolved
    /// ([`EstimatedLayers::resolve`]): each layer is described by the costliest of every
    /// description its writes gave it, as `costliest_layer` picks it. The root is also kept at the
    /// level of the deepest root key the batch writes under ([`ROOT_KEY_LEVELS`]), as some root
    /// descriptions name a shallower one (the credit shielded pool's, at level 3, for
    /// `ShieldedBalances` at level 4). The estimate does not depend on the order of the batch's operations, and no
    /// write is priced at a shallower depth than its own description. Applying a batch is
    /// generation 1's.
    #[inline(always)]
    pub(crate) fn apply_drive_operations_v2(
        &self,
        operations: Vec<DriveOperation>,
        apply: bool,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        DriveOperation::refuse_repeated_token_balance_writes(&operations)?;
        self.refuse_repeated_counter_moves(&operations, block_info, transaction, platform_version)?;
        let mut operations = DriveOperation::merge_balance_writes(operations)?;
        let mut processing_pool_credits: Credits = 0;
        for operation in operations.extract_if(.., |operation| {
            matches!(
                operation,
                DriveOperation::SystemOperation(
                    SystemOperationType::AddToCurrentEpochProcessingPool { .. }
                )
            )
        }) {
            if let DriveOperation::SystemOperation(
                SystemOperationType::AddToCurrentEpochProcessingPool { amount },
            ) = operation
            {
                processing_pool_credits =
                    processing_pool_credits
                        .checked_add(amount)
                        .ok_or(Error::Fee(FeeError::Overflow(
                            "overflow when accumulating processing pool credits",
                        )))?;
            }
        }
        if operations.is_empty() && (!apply || processing_pool_credits == 0) {
            return Ok(FeeResult::default());
        }
        let forfeits_storage_refunds = operations
            .iter()
            .any(DriveOperation::forfeits_storage_refunds);
        // The document operations of a moderator's deletion go in a batch of their own only
        // when something else in the batch may free bytes someone is owed.
        let separates_forfeited_operations = forfeits_storage_refunds
            && operations
                .iter()
                .any(DriveOperation::refunds_moderation_storage);
        // With no caller transaction, TTL preparation (direct drainage
        // writes), conversion reads, and the batch apply would each commit
        // on their own, so a conversion error after preparation would leave
        // drained buckets committed without the write. Span all of it with
        // one owned transaction and commit only once the batch applied.
        let caller_transaction = transaction;
        let owned_transaction =
            (apply && transaction.is_none()).then(|| self.grove.start_transaction());
        let transaction = owned_transaction.as_ref().or(caller_transaction);
        if apply {
            self.prepare_drive_operations_time_range_ttl(
                &operations,
                block_info,
                transaction,
                platform_version,
            )?;
        }
        let mut low_level_operations = vec![];
        let mut estimated_costs_only_with_layer_info = (!apply).then(EstimatedLayers::new);

        // The document operations of a moderator's deletion, whose removals refund nobody
        let mut forfeited_low_level_operations = vec![];

        let mut finalize_tasks: Vec<DriveOperationFinalizeTask> = Vec::new();

        for drive_op in operations {
            if let Some(tasks) = drive_op.finalization_tasks(platform_version)? {
                finalize_tasks.extend(tasks);
            }

            let forfeited = separates_forfeited_operations
                && matches!(drive_op, DriveOperation::DocumentOperation(_));
            let mut converted = drive_op.into_low_level_drive_operations_after_ttl_drain(
                self,
                &mut estimated_costs_only_with_layer_info,
                block_info,
                transaction,
                platform_version,
            )?;
            if forfeited {
                forfeited_low_level_operations.append(&mut converted);
            } else {
                low_level_operations.append(&mut converted);
            }
        }

        // A conversion that left no estimate behind would have the batch applied instead of
        // estimated, so it is refused.
        if !apply && estimated_costs_only_with_layer_info.is_none() {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "an estimated operation leaves the estimate's layer descriptions behind",
            )));
        }
        if let Some(estimated_costs_only_with_layer_info) =
            estimated_costs_only_with_layer_info.as_mut()
        {
            if let Some(level) = deepest_root_key_level(
                low_level_operations
                    .iter()
                    .chain(forfeited_low_level_operations.iter()),
            ) {
                estimated_costs_only_with_layer_info
                    .keep_at_level(&KeyInfoPath::from_known_path([]), level);
            }
        }
        let estimated_costs_only_with_layer_info = estimated_costs_only_with_layer_info
            .map(|layers| layers.resolve(platform_version))
            .transpose()?;

        let repaid_identity_debt =
            LowLevelDriveOperation::take_repaid_identity_debt(&mut low_level_operations)?;

        let mut cost_operations = vec![];

        let forfeited_estimated_costs_only_with_layer_info =
            if forfeited_low_level_operations.is_empty() {
                None
            } else {
                Some(estimated_costs_only_with_layer_info.clone())
            };
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            low_level_operations,
            &mut cost_operations,
            &platform_version.drive,
        )?;
        if let Some(estimated_costs_only_with_layer_info) =
            forfeited_estimated_costs_only_with_layer_info
        {
            let mut forfeited_cost_operations = vec![];
            self.apply_batch_low_level_drive_operations(
                estimated_costs_only_with_layer_info,
                transaction,
                forfeited_low_level_operations,
                &mut forfeited_cost_operations,
                &platform_version.drive,
            )?;
            super::v1::forfeit_storage_refunds(&mut forfeited_cost_operations);
            cost_operations.append(&mut forfeited_cost_operations);
        } else if forfeits_storage_refunds {
            // One batch, in which only the document operations free bytes: it forfeits whole.
            super::v1::forfeit_storage_refunds(&mut cost_operations);
        }
        self.apply_repaid_identity_debt_to_processing_pool(
            repaid_identity_debt,
            &block_info.epoch,
            transaction,
            platform_version,
        )?;
        // Read the pool after ordinary writes and debt repayment so neither can overwrite
        // the returned allocation. This transfer shares their transaction and is not billed.
        if apply && processing_pool_credits > 0 {
            let pool_operation = self.add_epoch_processing_credits_for_distribution_operation(
                &block_info.epoch,
                processing_pool_credits,
                transaction,
                platform_version,
            )?;
            self.apply_batch_low_level_drive_operations(
                None,
                transaction,
                vec![pool_operation],
                &mut vec![],
                &platform_version.drive,
            )?;
        }
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, &platform_version.drive)?;
        }

        // Execute drive operation callbacks after updating state. Nothing was written when
        // only estimating, so there is nothing to finalize. The tasks read through the
        // caller's transaction; an owned one was committed just above, and `caller_transaction`
        // is `None` exactly then, so they read committed state.
        if apply {
            for task in finalize_tasks {
                task.execute(self, caller_transaction, platform_version)?;
            }
        }

        Drive::calculate_fee(
            None,
            Some(cost_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            previous_fee_versions,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{deep, estimate, mint, shallow, system_credits};
    use super::{deepest_root_key_level, ROOT_KEY_LEVELS};
    use crate::drive::Drive;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::batch::DriveOperation;
    use crate::util::estimated_layers::EstimatedLayers;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::version::PlatformVersion;
    use grovedb::batch::KeyInfoPath;
    use grovedb::Element;

    /// The root layer a single operation's estimate is priced with, lowered into an estimate and
    /// resolved as generation 2 resolves a batch's.
    fn root_level(drive: &Drive, operation: DriveOperation<'static>) -> u32 {
        let platform_version = PlatformVersion::latest();
        let mut estimate = Some(EstimatedLayers::new());
        operation
            .into_low_level_drive_operations_after_ttl_drain(
                drive,
                &mut estimate,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected to lower the operation");
        estimate
            .expect("expected the estimate")
            .resolve(platform_version)
            .expect("expected to resolve the estimate")
            .get(&KeyInfoPath::from_known_path([]))
            .expect("expected the root to be described")
            .estimated_layer_count
            .estimate_levels()
    }

    /// One token mint describes the root twice: its supply estimator at level 3, then its
    /// balance estimator at level 2. The resolved estimate prices the mint's root at the
    /// supply's level 3.
    #[test]
    fn should_price_a_composite_operation_by_every_description_of_its_writes() {
        let drive = setup_drive_with_initial_state_structure(Some(PlatformVersion::latest()));
        assert_eq!(root_level(&drive, mint()), 3);
    }

    /// A token mint (whose supply describes the root at depth 3) and a prefunded balance write
    /// (the root at depth 3 too, with other key sizes and sum trees): the batch's estimate keeps
    /// the nodes GroveDB prices dearer, whichever comes first.
    #[test]
    fn should_estimate_two_descriptions_of_one_depth_alike_in_either_order() {
        let latest = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(latest));
        assert_eq!(
            estimate(&drive, vec![mint(), deep()], latest).expect("expected an estimate"),
            estimate(&drive, vec![deep(), mint()], latest).expect("expected an estimate")
        );
    }

    /// A system credits update and a token mint both describe `Misc`, the mint as holding the
    /// token supplies tree, whose new root key GroveDB prices with the subtree sizes: the batch
    /// is estimated alike whichever comes first.
    #[test]
    fn should_estimate_a_layer_holding_items_and_subtrees_in_either_order() {
        let latest = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(latest));
        assert_eq!(
            estimate(&drive, vec![system_credits(), mint()], latest).expect("expected an estimate"),
            estimate(&drive, vec![mint(), system_credits()], latest).expect("expected an estimate")
        );
    }

    /// A batch's estimate does not depend on the order of its operations: built either way, the
    /// shallow and the deep write are estimated alike, at the deeper root.
    #[test]
    fn should_estimate_a_batch_alike_in_either_order() {
        let latest = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(latest));
        assert_eq!(
            estimate(&drive, vec![shallow(), deep()], latest).expect("expected an estimate"),
            estimate(&drive, vec![deep(), shallow()], latest).expect("expected an estimate")
        );
    }

    /// The levels are the recorded root shape's, every root key included.
    #[test]
    fn should_take_root_levels_from_the_recorded_root_shape() {
        fn walk(node: &serde_json::Value, level: u32, levels: &mut Vec<(u8, u32)>) {
            if let Some(hex) = node["hex"].as_str() {
                let key = u8::from_str_radix(hex, 16).expect("expected a one-byte root key");
                levels.push((key, level));
                walk(&node["left"], level + 1, levels);
                walk(&node["right"], level + 1, levels);
            }
        }
        let structure = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/grovedb-structure.json"
        ))
        .expect("expected the recorded structure");
        let structure: serde_json::Value =
            serde_json::from_str(&structure).expect("expected valid json");
        let mut recorded = vec![];
        walk(&structure["layer_shapes"]["root"]["tree"], 0, &mut recorded);
        recorded.sort_unstable();
        let mut levels = ROOT_KEY_LEVELS.to_vec();
        levels.sort_unstable();
        assert_eq!(levels, recorded);
    }

    /// The deepest root key a batch writes under: the key opening an operation's path, or the
    /// key of an operation at the root itself; a path under an unknown key counts for nothing.
    #[test]
    fn should_find_the_deepest_root_key_the_operations_write_under() {
        let at = |path: Vec<Vec<u8>>, key: Vec<u8>| {
            LowLevelDriveOperation::insert_for_known_path_key_element(
                path,
                key,
                Element::new_item(vec![1]),
            )
        };
        let documents = at(vec![vec![64], vec![1]], vec![2]);
        let shielded = at(vec![vec![52], vec![1]], vec![2]);
        let root_misc = at(vec![], vec![104]);
        assert_eq!(deepest_root_key_level([&documents]), Some(0));
        assert_eq!(deepest_root_key_level([&root_misc]), Some(3));
        assert_eq!(
            deepest_root_key_level([&documents, &shielded, &root_misc]),
            Some(4)
        );
        assert_eq!(deepest_root_key_level([&at(vec![vec![7]], vec![1])]), None);
        assert_eq!(deepest_root_key_level([]), None);
    }
}
