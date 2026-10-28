use crate::util::batch::{DriveOperation, SystemOperationType};

use crate::drive::Drive;
use crate::error::{fee::FeeError, Error};
use crate::fees::op::LowLevelDriveOperation;

use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;

use crate::util::batch::drive_op_batch::finalize_task::{
    DriveOperationFinalizationTasks, DriveOperationFinalizeTask,
};
use crate::util::estimated_layers::merge_estimated_layer;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use std::collections::HashMap;

impl Drive {
    /// Generation 2 (protocol version 15): generation 1, with an estimate's layer descriptions
    /// merged per operation. A batch's estimate keeps one description per layer, and in
    /// generation 1 every operation's estimation writes into the same map, so the last one to
    /// describe a layer sets its level: a document, a nonce or a balance write built after a
    /// write deeper in the same layer (the token marketplace's escrows, in the Tokens and the
    /// prefunded specialized balances trees) lowered the batch's estimate of that layer. Here
    /// each operation is lowered with a map of its own, merged into the batch's as one
    /// description that covers both of two descriptions of one layer (`merge_estimated_layer`:
    /// the deeper depth, and at that depth the nodes GroveDB's propagation estimate prices
    /// dearer), so the estimate does not depend on the order of the batch's operations and no
    /// operation is priced below its own description. That raises the estimate of any batch
    /// whose operations describe a shared layer differently, not only the marketplace's: a
    /// document's nonce update describes the root a level deeper than the document does, so
    /// the root of every document batch is now estimated at the nonce's depth.
    ///
    /// Each document operation is also built against the operations of its GroveDB batch built
    /// before it ([`DriveOperation::append_low_level_drive_operations_after_ttl_drain`]): a
    /// document deletion sees the batch's earlier writes as pending, so two deletions emptying
    /// an index tree together remove it and refund it, and an insert into a tree an earlier
    /// deletion of the batch empties keeps the tree. Every other operation kind is built as
    /// generation 1 builds it, without the batch.
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
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        // The document operations of a moderator's deletion, whose removals refund nobody
        let mut forfeited_low_level_operations = vec![];

        let mut finalize_tasks: Vec<DriveOperationFinalizeTask> = Vec::new();

        for drive_op in operations {
            if let Some(tasks) = drive_op.finalization_tasks(platform_version)? {
                finalize_tasks.extend(tasks);
            }

            let forfeited = separates_forfeited_operations
                && matches!(drive_op, DriveOperation::DocumentOperation(_));
            // Each document operation is built against the operations of its GroveDB batch built
            // before it, and every operation is appended to them: a document deletion sees the
            // batch's other writes as pending. An estimate lowers each operation into layer descriptions of its own and
            // merges them into the batch's, one description covering both of two of one layer (the
            // deeper depth, with the nodes GroveDB prices dearer there): the batch's estimate is
            // the same whichever order its operations come in.
            let batch_operations = if forfeited {
                &mut forfeited_low_level_operations
            } else {
                &mut low_level_operations
            };
            match estimated_costs_only_with_layer_info.as_mut() {
                None => drive_op.append_low_level_drive_operations_after_ttl_drain(
                    self,
                    &mut None,
                    block_info,
                    batch_operations,
                    transaction,
                    platform_version,
                )?,
                Some(batch_layers) => {
                    let mut operation_layers = Some(HashMap::new());
                    drive_op.append_low_level_drive_operations_after_ttl_drain(
                        self,
                        &mut operation_layers,
                        block_info,
                        batch_operations,
                        transaction,
                        platform_version,
                    )?;
                    for (path, layer) in operation_layers.unwrap_or_default() {
                        merge_estimated_layer(batch_layers, path, layer, platform_version)?;
                    }
                }
            }
        }

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
    use crate::drive::Drive;
    use crate::util::batch::drive_op_batch::{
        PrefundedSpecializedBalanceOperationType, TokenOperationType,
    };
    use crate::util::batch::{DriveOperation, IdentityOperationType};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::fee::fee_result::FeeResult;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;

    /// A write that describes the root layer shallowly (an identity contract nonce, level 0)
    /// and one that describes it deeper (a prefunded specialized balance, level 3).
    fn shallow() -> DriveOperation<'static> {
        DriveOperation::IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce {
            identity_id: [1; 32],
            contract_id: [2; 32],
            nonce: 3,
        })
    }

    fn deep() -> DriveOperation<'static> {
        DriveOperation::PrefundedSpecializedBalanceOperation(
            PrefundedSpecializedBalanceOperationType::CreateNewPrefundedBalance {
                prefunded_specialized_balance_id: Identifier::from([4; 32]),
                add_balance: 1_000,
            },
        )
    }

    fn estimate(
        drive: &Drive,
        operations: Vec<DriveOperation<'static>>,
        platform_version: &PlatformVersion,
    ) -> FeeResult {
        drive
            .apply_drive_operations(
                operations,
                false,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected an estimate")
    }

    /// A token mint and a prefunded balance write both describe the root at depth 3, with
    /// different key sizes and sum trees: the batch's estimate keeps the nodes GroveDB prices
    /// dearer, whichever comes first.
    #[test]
    fn should_estimate_two_descriptions_of_one_depth_alike_in_either_order() {
        let latest = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(latest));
        let mint = || {
            DriveOperation::TokenOperation(TokenOperationType::TokenMint {
                token_id: Identifier::from([5; 32]),
                identity_balance_holder_id: Identifier::from([6; 32]),
                mint_amount: 100,
                allow_first_mint: true,
                allow_saturation: false,
            })
        };
        assert_eq!(
            estimate(&drive, vec![mint(), deep()], latest),
            estimate(&drive, vec![deep(), mint()], latest)
        );
    }

    /// From protocol version 15 a batch's estimate does not depend on the order of its
    /// operations: built either way, the shallow and the deep write are estimated alike, at the
    /// deeper root. Generation 1 (protocol version 14) estimated the batch by whichever write
    /// came last, so the two orders differed.
    #[test]
    fn should_estimate_a_batch_alike_in_either_order() {
        let latest = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(latest));
        let shallow_first = estimate(&drive, vec![shallow(), deep()], latest);
        let deep_first = estimate(&drive, vec![deep(), shallow()], latest);
        assert_eq!(shallow_first, deep_first);

        let before = PlatformVersion::get(14).expect("expected protocol version 14");
        let shallow_first_before = estimate(&drive, vec![shallow(), deep()], before);
        let deep_first_before = estimate(&drive, vec![deep(), shallow()], before);
        assert_ne!(shallow_first_before, deep_first_before);
        // The order-independent estimate is the conservative one of the two.
        assert!(
            shallow_first.processing_fee
                >= shallow_first_before
                    .processing_fee
                    .max(deep_first_before.processing_fee)
        );
    }

    mod batch_document_writes {
        use crate::drive::{Drive, RootTree};
        use crate::util::batch::drive_op_batch::{
            DocumentOperation, DocumentOperationsForContractDocumentType,
        };
        use crate::util::batch::{DocumentOperationType, DriveOperation};
        use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
        use crate::util::object_size_info::{
            DataContractInfo, DocumentAndContractInfo, DocumentTypeInfo, OwnedDocumentInfo,
        };
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
        use crate::util::test_helpers::subtree_paths;
        use dpp::block::block_info::BlockInfo;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::DataContract;
        use dpp::document::Document;
        use dpp::identifier::Identifier;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use dpp::tokens::token_order::TokenOrder;
        use dpp::tokens::token_order_side::TokenOrderSide;
        use dpp::version::PlatformVersion;
        use std::collections::BTreeSet;

        const OWNER: [u8; 32] = [7; 32];

        fn market() -> (Drive, DataContract) {
            let platform_version = PlatformVersion::latest();
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));
            let contract =
                load_system_data_contract(SystemDataContract::TokenMarket, platform_version)
                    .expect("expected the token market contract");
            drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    None,
                    None,
                    platform_version,
                )
                .expect("expected to store the token market contract");
            (drive, contract)
        }

        /// An `ask` of `OWNER` for 10 of `token` at price 100 and decimals 8, as a placement
        /// stores it.
        fn ask(id: u8, token: u8) -> Document {
            TokenOrder {
                id: Identifier::from([id; 32]),
                side: TokenOrderSide::Ask,
                maker_id: Identifier::from(OWNER),
                token_id: Identifier::from([token; 32]),
                price: 100,
                amount: 10,
                remaining: 10,
                escrow: 0,
                decimals: 8,
            }
            .new_document(&BlockInfo {
                time_ms: 1_000,
                ..BlockInfo::default()
            })
            .expect("expected the ask document")
        }

        fn store(drive: &Drive, contract: &DataContract, document: &Document) {
            let platform_version = PlatformVersion::latest();
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((document, None)),
                            owner_id: Some(OWNER),
                        },
                        contract,
                        document_type: contract
                            .document_type_for_name("ask")
                            .expect("expected the ask type"),
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected to store the ask");
        }

        fn delete(contract: &DataContract, id: u8) -> DriveOperation<'static> {
            DriveOperation::DocumentOperation(DocumentOperationType::DeleteDocument {
                document_id: Identifier::from([id; 32]),
                contract_info: DataContractInfo::OwnedDataContract(contract.clone()),
                document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr("ask"),
            })
        }

        fn apply(drive: &Drive, operations: Vec<DriveOperation>) {
            drive
                .apply_drive_operations(
                    operations,
                    true,
                    &BlockInfo::default(),
                    None,
                    PlatformVersion::latest(),
                    None,
                )
                .expect("expected the batch to apply");
        }

        /// Every tree and item under the `ask` document type, by path.
        fn ask_trees(drive: &Drive) -> BTreeSet<Vec<Vec<u8>>> {
            subtree_paths(
                drive,
                vec![
                    vec![RootTree::DataContractDocuments as u8],
                    SystemDataContract::TokenMarket.id().to_vec(),
                    vec![1u8],
                    b"ask".to_vec(),
                ],
                PlatformVersion::latest(),
            )
        }

        /// One owner's asks of two tokens share the owner's `byOwner` bucket. Deleted by two
        /// operations of one batch, each built against the other, the bucket is removed: the
        /// `ask` trees left are those deleting the two in two batches leaves.
        #[test]
        fn should_remove_a_tree_two_operations_of_one_batch_empty_together() {
            let (drive, contract) = market();
            store(&drive, &contract, &ask(1, 0xA1));
            store(&drive, &contract, &ask(2, 0xA2));
            let with_asks = ask_trees(&drive);
            apply(&drive, vec![delete(&contract, 1), delete(&contract, 2)]);

            let (separately, contract) = market();
            store(&separately, &contract, &ask(1, 0xA1));
            store(&separately, &contract, &ask(2, 0xA2));
            apply(&separately, vec![delete(&contract, 1)]);
            apply(&separately, vec![delete(&contract, 2)]);

            let left = ask_trees(&drive);
            assert_eq!(left, ask_trees(&separately));
            assert!(left.len() < with_asks.len());
            assert!(
                !left
                    .iter()
                    .any(|path| path.iter().any(|key| key.as_slice() == OWNER.as_slice())),
                "no tree keyed by the owner stays"
            );
        }

        /// As `should_insert_into_a_tree_an_earlier_deletion_of_the_batch_empties`, with the
        /// insert sent as a group of operations for one document type: a group's operations are
        /// built against the batch's earlier ones too.
        #[test]
        fn should_insert_through_a_group_into_a_tree_an_earlier_deletion_empties() {
            let (drive, contract) = market();
            store(&drive, &contract, &ask(1, 0xA1));
            let replacement = ask(2, 0xA1);
            apply(
                &drive,
                vec![
                    delete(&contract, 1),
                    DriveOperation::DocumentOperation(
                        DocumentOperationType::MultipleDocumentOperationsForSameContractDocumentType {
                            document_operations: DocumentOperationsForContractDocumentType {
                                operations: vec![DocumentOperation::AddOperation {
                                    owned_document_info: OwnedDocumentInfo {
                                        document_info: DocumentRefInfo((&replacement, None)),
                                        owner_id: Some(OWNER),
                                    },
                                    override_document: false,
                                }],
                                contract: &contract,
                                document_type: contract
                                    .document_type_for_name("ask")
                                    .expect("expected the ask type"),
                            },
                        },
                    ),
                ],
            );

            let (only_replacement, contract) = market();
            store(&only_replacement, &contract, &replacement);
            assert_eq!(ask_trees(&drive), ask_trees(&only_replacement));
        }

        /// The deletions of `should_remove_a_tree_two_operations_of_one_batch_empty_together`
        /// through the converter that returns a batch instead of applying it
        /// (`convert_drive_operations_to_grove_operations` 2, protocol version 15): applied, its
        /// batch leaves the trees deleting the two in two batches leaves. It sits here, beside
        /// the helpers it shares.
        #[test]
        fn should_remove_a_shared_tree_through_the_raw_converter() {
            let platform_version = PlatformVersion::latest();
            let (drive, contract) = market();
            store(&drive, &contract, &ask(1, 0xA1));
            store(&drive, &contract, &ask(2, 0xA2));
            let transaction = drive.grove.start_transaction();
            let batch = drive
                .convert_drive_operations_to_grove_operations(
                    vec![delete(&contract, 1), delete(&contract, 2)],
                    &BlockInfo::default(),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected the conversion");
            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("expected the batch to apply");
            drive
                .commit_transaction(transaction, &platform_version.drive)
                .expect("expected to commit");

            let (separately, contract) = market();
            store(&separately, &contract, &ask(1, 0xA1));
            store(&separately, &contract, &ask(2, 0xA2));
            apply(&separately, vec![delete(&contract, 1)]);
            apply(&separately, vec![delete(&contract, 2)]);

            let left = ask_trees(&drive);
            assert_eq!(left, ask_trees(&separately));
            assert!(
                !left
                    .iter()
                    .any(|path| path.iter().any(|key| key.as_slice() == OWNER.as_slice())),
                "no tree keyed by the owner stays"
            );
        }

        /// A deletion that empties a bucket and an insert into the same bucket in one batch: the
        /// insert cancels the removal, and the `ask` trees are those of the new ask alone.
        #[test]
        fn should_insert_into_a_tree_an_earlier_deletion_of_the_batch_empties() {
            let (drive, contract) = market();
            store(&drive, &contract, &ask(1, 0xA1));
            let replacement = ask(2, 0xA1);
            apply(
                &drive,
                vec![
                    delete(&contract, 1),
                    DriveOperation::DocumentOperation(DocumentOperationType::AddDocument {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&replacement, None)),
                            owner_id: Some(OWNER),
                        },
                        contract_info: DataContractInfo::OwnedDataContract(contract.clone()),
                        document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr("ask"),
                        override_document: false,
                    }),
                ],
            );

            let (only_replacement, contract) = market();
            store(&only_replacement, &contract, &replacement);
            assert_eq!(ask_trees(&drive), ask_trees(&only_replacement));
        }
    }
}
