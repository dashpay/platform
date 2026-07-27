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
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::accessors::v1::DataContractV1Setters;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;

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
}
