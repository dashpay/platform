use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::{DriveDocumentQuery, WhereClause, WhereOperator};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::DocumentV0Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::Value;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, HashMap};

impl Drive {
    /// Updates the documents in the Keyword Search contract for the contract
    /// update keywords and returns the fee result
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_contract_keywords_v0(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        keywords: &[String],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.update_contract_keywords_add_to_operations_v0(
            contract_id,
            owner_id,
            keywords,
            block_info,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok(fees)
    }

    /// Creates and applies the LowLeveLDriveOperations needed to update
    /// the documents in the Keyword Search contract for the contract keywords
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_contract_keywords_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        keywords: &[String],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.update_contract_keywords_operations(
            contract_id,
            owner_id,
            keywords,
            block_info,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )
    }

    /// Creates and returns the LowLeveLDriveOperations needed to update
    /// the documents in the Keyword Search contract for the contract keywords
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_contract_keywords_operations_v0(
        &self,
        contract_id: Identifier,
        owner_id: Identifier,
        keywords: &[String],
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut operations: Vec<LowLevelDriveOperation> = vec![];

        // First get the existing keywords so we know which ones we need to delete and which new ones we need to add
        let contract = self
            .cache
            .system_data_contracts
            .load_keyword_search(platform_version)?;
        let document_type = contract.document_type_for_name("contractKeywords")?;

        let mut query = DriveDocumentQuery::all_items_query(&contract, document_type, None);
        query.internal_clauses.equal_clauses.insert(
            "contractId".to_string(),
            WhereClause {
                field: "contractId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(contract_id.to_buffer()),
            },
        );

        // todo: deal with cost of this operation
        let query_outcome = self.query_documents(
            query,
            Some(&block_info.epoch),
            false,
            transaction,
            Some(platform_version.protocol_version),
        )?;

        let mut existing: BTreeMap<String, Identifier> = BTreeMap::new();
        for doc in query_outcome.documents_owned() {
            let kw = doc.properties().get_string("keyword")?;
            existing.insert(kw, doc.id());
        }

        // If an existing keyword is not in the new keyword set, we delete it
        for (kw, doc_id) in &existing {
            if !keywords.contains(kw) {
                operations.extend(self.force_delete_document_for_contract_operations(
                    *doc_id,
                    &contract,
                    document_type,
                    None,
                    estimated_costs_only_with_layer_info,
                    block_info.time_ms,
                    transaction,
                    platform_version,
                )?);
            }
        }

        // Finally, add the new ones
        let mut keywords_to_add: Vec<String> = Vec::new();
        for kw in keywords {
            if !existing.contains_key(kw) {
                keywords_to_add.push(kw.clone());
            }
        }

        if !keywords_to_add.is_empty() {
            operations.extend(self.add_new_contract_keywords_operations(
                contract_id,
                owner_id,
                &keywords_to_add,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        Ok(operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
    use crate::drive::Drive;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::query::{DriveDocumentQuery, WhereClause, WhereOperator};
    use crate::util::grove_operations::DirectQueryType;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::accessors::v1::DataContractV1Setters;
    use dpp::document::DocumentV0Getters;
    use dpp::identifier::Identifier;
    use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
    use dpp::platform_value::Value;
    use dpp::prelude::DataContract;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use grovedb::batch::GroveOp;
    use grovedb::query_result_type::QueryResultType;
    use grovedb::{PathQuery, Query};

    /// Exercises `update_contract_keywords_v0` in apply=false estimation mode
    /// with a fresh contract (no prior keywords). The inner
    /// `update_contract_keywords_operations_v0` takes the path:
    /// - `existing` is empty
    /// - `keywords_to_add` contains all new keywords
    /// - `add_new_contract_keywords_operations` is called with
    ///   `estimated_costs_only_with_layer_info = Some(..)`.
    /// PR #3516 covers apply=true for this scenario but not estimate mode.
    #[test]
    fn test_update_contract_keywords_v0_estimate_only_add_only() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let keyword_search =
            load_system_data_contract(SystemDataContract::KeywordSearch, platform_version)
                .expect("load keyword_search");
        drive
            .apply_contract(
                &keyword_search,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("apply keyword_search");

        let contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();

        let fee = drive
            .update_contract_keywords(
                contract.id(),
                contract.owner_id(),
                &["est_kw_1".to_string(), "est_kw_2".to_string()],
                &BlockInfo::default(),
                false, // estimation
                None,
                platform_version,
            )
            .expect("estimate-only keyword update should succeed");

        assert!(fee.processing_fee > 0 || fee.storage_fee > 0);
    }

    /// Exercises `update_contract_keywords_v0` overlap path: existing keywords
    /// = {"A","B"}, new = {"A","B"} (identical set). The inner
    /// `update_contract_keywords_operations_v0` should take neither the delete
    /// nor the add sub-path — `keywords_to_add` ends up empty because
    /// every new keyword is already in `existing`, and nothing in existing
    /// fails `keywords.contains(kw)`.
    /// This covers the "both sets equal -> no-op" combined branch which is
    /// distinct from PR #3516's "add-only" and "remove-all" scenarios.
    #[test]
    fn test_update_contract_keywords_v0_identical_keywords_noop() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let keyword_search =
            load_system_data_contract(SystemDataContract::KeywordSearch, platform_version)
                .expect("load keyword_search");
        drive
            .apply_contract(
                &keyword_search,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("apply keyword_search");

        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_keywords(vec!["same1".to_string(), "same2".to_string()]);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("insert contract with keywords");

        // Pass the same keyword set: no deletes, no adds should happen.
        let fee = drive
            .update_contract_keywords(
                contract.id(),
                contract.owner_id(),
                &["same1".to_string(), "same2".to_string()],
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("identical-keywords update should succeed");

        // With no grove mutations, processing_fee should be zero (the only
        // grove work is the query, which is accounted in query cost).
        assert_eq!(
            fee.processing_fee, 0,
            "identical keyword update should produce zero processing fee"
        );
    }

    /// Exercises the intersecting-sets branch of
    /// `update_contract_keywords_operations_v0`: existing = {"A","B","C"},
    /// new = {"B","D"}. The code path:
    /// - delete loop removes "A" (existing_but_not_in_new)
    /// - delete loop removes "C"
    /// - `keywords_to_add` = ["D"] (new_but_not_in_existing; B skipped)
    /// - `add_new_contract_keywords_operations` called for ["D"] only.
    ///
    /// PR #3516's test covers add-only and remove-all, NOT simultaneous
    /// partial add + partial remove with overlap.
    #[test]
    fn test_update_contract_keywords_v0_partial_add_and_remove_with_overlap() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let keyword_search =
            load_system_data_contract(SystemDataContract::KeywordSearch, platform_version)
                .expect("load keyword_search");
        drive
            .apply_contract(
                &keyword_search,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("apply keyword_search");

        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_keywords(vec!["A".to_string(), "B".to_string(), "C".to_string()]);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("insert contract");

        let new_keywords = vec!["B".to_string(), "D".to_string()];
        let fee = drive
            .update_contract_keywords(
                contract.id(),
                contract.owner_id(),
                &new_keywords,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("partial update should succeed");

        // Both deletes and adds happened, so fee must be non-zero.
        assert!(
            fee.processing_fee > 0,
            "partial keyword update should produce non-zero fee"
        );
    }

    /// The `contractKeywords` doctype tree of the keyword search contract.
    fn contract_keywords_path(keyword_search: &DataContract) -> Vec<Vec<u8>> {
        vec![
            vec![crate::drive::RootTree::DataContractDocuments as u8],
            keyword_search.id().as_bytes().to_vec(),
            vec![1],
            b"contractKeywords".to_vec(),
        ]
    }

    /// The subtree of `byContractId` references belonging to one contract:
    /// the group tree the deletes and the adds of a keyword replacement share.
    fn by_contract_id_reference_path(
        keyword_search: &DataContract,
        contract_id: Identifier,
    ) -> Vec<Vec<u8>> {
        let mut path = contract_keywords_path(keyword_search);
        path.push(b"contractId".to_vec());
        path.push(contract_id.as_bytes().to_vec());
        path.push(vec![0]);
        path
    }

    /// The keys directly under `path`, sorted. Every caller compares the
    /// result against an exact expectation, so a missing or empty subtree
    /// fails the caller's assertion rather than passing quietly.
    fn subtree_keys(
        drive: &Drive,
        path: Vec<Vec<u8>>,
        platform_version: &PlatformVersion,
    ) -> Vec<Vec<u8>> {
        let (result, _) = drive
            .grove
            .query_raw(
                &PathQuery::new_unsized(path, Query::new_range_full()),
                false,
                true,
                true,
                QueryResultType::QueryKeyElementPairResultType,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("the subtree read must succeed");
        let mut keys: Vec<Vec<u8>> = result
            .to_key_elements()
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        keys.sort();
        keys
    }

    /// Is there a `byKeyword` group tree for `keyword`?
    fn by_keyword_group_exists(
        drive: &Drive,
        keyword_search: &DataContract,
        keyword: &str,
        platform_version: &PlatformVersion,
    ) -> bool {
        let mut path = contract_keywords_path(keyword_search);
        path.push(b"keyword".to_vec());
        let path_refs: Vec<&[u8]> = path.iter().map(|v| v.as_slice()).collect();
        drive
            .grove_get_raw_optional(
                path_refs.as_slice().into(),
                keyword.as_bytes(),
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("the raw read must succeed")
            .is_some()
    }

    /// The keyword documents currently indexed under `byContractId` for
    /// `contract_id`, read back through that index — the same query
    /// `update_contract_keywords_operations_v0` uses to find what exists.
    /// Returned as `(keyword, document id)` sorted by keyword, because the
    /// document ids are what the raw tree-level assertions need.
    fn keywords_indexed_by_contract_id(
        drive: &Drive,
        contract_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Vec<(String, Identifier)> {
        let keyword_search = drive
            .cache
            .system_data_contracts
            .load_keyword_search(platform_version)
            .expect("load keyword_search");
        let document_type = keyword_search
            .document_type_for_name("contractKeywords")
            .expect("contractKeywords doctype");

        let mut query = DriveDocumentQuery::all_items_query(&keyword_search, document_type, None);
        query.internal_clauses.equal_clauses.insert(
            "contractId".to_string(),
            WhereClause {
                field: "contractId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(contract_id.to_buffer()),
            },
        );

        let mut keywords: Vec<(String, Identifier)> = drive
            .query_documents(
                query,
                None,
                false,
                None,
                Some(platform_version.protocol_version),
            )
            .expect("the byContractId query must succeed")
            .documents_owned()
            .into_iter()
            .map(|document| {
                (
                    document
                        .properties()
                        .get_string("keyword")
                        .expect("every keyword document carries a keyword"),
                    document.id(),
                )
            })
            .collect();
        keywords.sort();
        keywords
    }

    /// The keywords alone, for the assertions that do not care about ids.
    fn keyword_names(indexed: &[(String, Identifier)]) -> Vec<String> {
        indexed.iter().map(|(keyword, _)| keyword.clone()).collect()
    }

    /// Document ids sorted the way grovedb orders subtree keys.
    fn sorted_document_keys(indexed: &[(String, Identifier)]) -> Vec<Vec<u8>> {
        let mut keys: Vec<Vec<u8>> = indexed
            .iter()
            .map(|(_, id)| id.as_bytes().to_vec())
            .collect();
        keys.sort();
        keys
    }

    /// Replacing a contract's entire keyword set deletes every existing
    /// keyword document and adds the new ones in **one** grovedb batch, all
    /// sharing the single `byContractId/<contractId>` group tree. Nothing in
    /// that batch is aware of its siblings, so the deletes each decide the
    /// group is not yet empty and leave it standing — which happens to be the
    /// correct answer, because the adds land in the same group in the same
    /// batch.
    ///
    /// That coincidence is what keeps the path correct, so it is worth
    /// executing rather than reasoning about: if the group tree were ever
    /// removed here, the new keyword documents would be written into a tree
    /// the same batch deleted. The case where the coincidence runs out is
    /// [`clearing_every_keyword_leaves_an_empty_by_contract_id_group_behind`].
    #[test]
    fn replacing_a_contracts_whole_keyword_set_keeps_the_by_contract_id_group() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let keyword_search =
            load_system_data_contract(SystemDataContract::KeywordSearch, platform_version)
                .expect("load keyword_search");
        drive
            .apply_contract(
                &keyword_search,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("apply keyword_search");

        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_keywords(vec!["alpha".to_string(), "bravo".to_string()]);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("insert contract with keywords");

        let before = keywords_indexed_by_contract_id(&drive, contract.id(), platform_version);
        assert_eq!(
            keyword_names(&before),
            vec!["alpha".to_string(), "bravo".to_string()],
            "baseline: both original keywords are indexed under byContractId"
        );

        let replacement = vec!["charlie".to_string(), "delta".to_string()];
        let group_path = by_contract_id_reference_path(&keyword_search, contract.id());

        // The premise the correctness of this path rests on: a *single*
        // operations vector carries both the deletes and the adds over that one
        // group tree. Building the operations does not mutate anything, so this
        // can be checked before applying them.
        let operations = drive
            .update_contract_keywords_operations(
                contract.id(),
                contract.owner_id(),
                &replacement,
                &BlockInfo::default(),
                &mut None,
                None,
                platform_version,
            )
            .expect("building the keyword update operations must succeed");
        let (deletes, inserts) = operations
            .iter()
            .fold((0, 0), |(deletes, inserts), operation| match operation {
                LowLevelDriveOperation::GroveOperation(op) if op.path.to_path() == group_path => {
                    match op.op {
                        GroveOp::Delete | GroveOp::DeleteTree(..) => (deletes + 1, inserts),
                        _ => (deletes, inserts + 1),
                    }
                }
                _ => (deletes, inserts),
            });
        assert_eq!(
            (deletes, inserts),
            (2, 2),
            "one operations vector must carry both keyword deletes and both keyword adds \
             over the shared byContractId group; if these ever split into separate batches \
             this test no longer exercises the shape it exists to document"
        );

        // The disjoint replacement, applied.
        drive
            .update_contract_keywords(
                contract.id(),
                contract.owner_id(),
                &replacement,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("replacing the whole keyword set should succeed");

        let after = keywords_indexed_by_contract_id(&drive, contract.id(), platform_version);
        assert_eq!(
            keyword_names(&after),
            replacement,
            "the byContractId group must survive holding exactly the new keywords"
        );

        // The group tree itself, read raw rather than through a query: its
        // children must be exactly the two new documents. This is the assertion
        // that would catch a surviving reference to a deleted document, a
        // duplicate, or a group that was removed and rebuilt with the wrong
        // membership.
        assert_eq!(
            subtree_keys(&drive, group_path, platform_version),
            sorted_document_keys(&after),
            "the byContractId group's references must be exactly the new keyword documents"
        );

        // Primary document storage: the index can only show what it references,
        // so orphaned documents would be invisible to the assertions above and
        // to verify_grovedb alike.
        let mut storage_path = contract_keywords_path(&keyword_search);
        storage_path.push(vec![0]);
        assert_eq!(
            subtree_keys(&drive, storage_path, platform_version),
            sorted_document_keys(&after),
            "the deleted keyword documents must be gone from primary storage, not merely \
             unreferenced"
        );

        // The other index over the same documents. A delete that maintained
        // byContractId but not byKeyword would leave keyword search — the
        // contract's entire purpose — returning dead entries.
        for stale in ["alpha", "bravo"] {
            assert!(
                !by_keyword_group_exists(&drive, &keyword_search, stale, platform_version),
                "the byKeyword group for the removed keyword {stale} must be gone"
            );
        }
        for fresh in ["charlie", "delta"] {
            assert!(
                by_keyword_group_exists(&drive, &keyword_search, fresh, platform_version),
                "the byKeyword group for the new keyword {fresh} must exist"
            );
        }

        let issues = drive
            .grove
            .verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("verify_grovedb must run");
        assert!(
            issues.is_empty(),
            "grovedb integrity verification reported issues: {issues:?}"
        );
    }

    /// **This test asserts a defect, not the desired behaviour**, and it exists
    /// to put a price on one line in the caller.
    ///
    /// The deletes this function emits are blind to each other
    /// (`previous_batch_operations` is `None`), so when two or more of them
    /// jointly empty the shared `byContractId/<contractId>` group each sees the
    /// other's reference still committed, and the group tree survives with
    /// nothing behind it. `verify_grovedb` cannot see that — primary and
    /// secondary agree the empty group exists — so it is permanent state.
    ///
    /// Emptying the group without refilling it requires the *new* keyword set
    /// to be empty: deletes cover `existing - new` and adds cover
    /// `new - existing`, so if every existing document is deleted then
    /// `existing` and `new` are disjoint, and the adds are empty only when
    /// `new` is. `update_contract_v1` never calls into here with an empty set —
    /// it guards the call with `!contract.keywords().is_empty()` — which is the
    /// only reason a `DataContractUpdate` cannot produce this. The guard is
    /// load-bearing, not an optimisation; this is what it is worth.
    ///
    /// `contractKeywords` has no ranked index, so the residue is state bloat
    /// and a broken "a group tree exists therefore the group is non-empty"
    /// invariant rather than a wrong query answer. It would not stay that way
    /// if the same shape were ever built over a ranked index.
    #[test]
    fn clearing_every_keyword_leaves_an_empty_by_contract_id_group_behind() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let keyword_search =
            load_system_data_contract(SystemDataContract::KeywordSearch, platform_version)
                .expect("load keyword_search");
        drive
            .apply_contract(
                &keyword_search,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("apply keyword_search");

        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_keywords(vec!["alpha".to_string(), "bravo".to_string()]);
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("insert contract with keywords");

        // Two blind deletes, no adds — reachable only by calling this API
        // directly, which is what makes the caller's guard the real protection.
        drive
            .update_contract_keywords(
                contract.id(),
                contract.owner_id(),
                &[],
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("clearing every keyword should succeed");

        assert!(
            keywords_indexed_by_contract_id(&drive, contract.id(), platform_version).is_empty(),
            "every keyword document must be gone"
        );

        let mut storage_path = contract_keywords_path(&keyword_search);
        storage_path.push(vec![0]);
        assert!(
            subtree_keys(&drive, storage_path, platform_version).is_empty(),
            "primary document storage must be empty"
        );
        for stale in ["alpha", "bravo"] {
            assert!(
                !by_keyword_group_exists(&drive, &keyword_search, stale, platform_version),
                "the byKeyword group for {stale} must be gone — those groups hold one \
                 document each, so their deletes are never blind to a sibling"
            );
        }

        // And the residue itself. If this ever starts returning `None` the
        // defect has been fixed: delete this test and relax the guard note on
        // `update_contract_v1`'s call site.
        let mut group_level = contract_keywords_path(&keyword_search);
        group_level.push(b"contractId".to_vec());
        let path_refs: Vec<&[u8]> = group_level.iter().map(|v| v.as_slice()).collect();
        let group = drive
            .grove_get_raw_optional(
                path_refs.as_slice().into(),
                contract.id().as_bytes(),
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("the raw read must succeed");
        assert!(
            group.is_some(),
            "the emptied byContractId group tree is expected to survive — two blind deletes \
             each conclude the group is not yet empty"
        );

        let issues = drive
            .grove
            .verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("verify_grovedb must run");
        assert!(
            issues.is_empty(),
            "integrity verification cannot see the residue; got {issues:?}"
        );
    }
}
