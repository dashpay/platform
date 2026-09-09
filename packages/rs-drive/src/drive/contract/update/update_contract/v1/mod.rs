use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use dpp::serialization::PlatformSerializableWithPlatformVersion;

use crate::error::contract::DataContractError;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Updates a data contract.
    ///
    /// This function updates a given data contract in the storage. The fee for updating
    /// the contract is also calculated and returned.
    ///
    /// # Arguments
    ///
    /// * `contract` - A reference to the `DataContract` to be updated.
    /// * `block_info` - A `BlockInfo` object containing information about the block where
    ///   the contract is being updated.
    /// * `apply` - A boolean indicating whether the contract update should be applied (`true`) or not (`false`). Passing `false` would only tell the fees but won't interact with the state.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for updating the contract.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - If successful, returns a `FeeResult` representing the fee
    ///   for updating the contract. If an error occurs during the contract update or fee calculation,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract update or fee calculation fails.
    #[inline(always)]
    pub(super) fn update_contract_v1(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        if !apply {
            return self.insert_contract(
                contract,
                block_info,
                false,
                transaction,
                platform_version,
            );
        }

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let contract_bytes = contract.serialize_to_bytes_with_platform_version(platform_version)?;

        // Since we can update the contract by definition it already has storage flags
        let storage_flags = Some(StorageFlags::new_single_epoch(
            block_info.epoch.index,
            Some(contract.owner_id().to_buffer()),
        ));

        let contract_element = Element::Item(
            contract_bytes,
            StorageFlags::map_to_some_element_flags(storage_flags.as_ref()),
        );

        let original_contract_fetch_info = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contract.id().to_buffer(),
                Some(&block_info.epoch),
                true,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "contract should exist",
            )))?;

        if original_contract_fetch_info.contract.config().readonly() {
            return Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableContract(
                "original contract is readonly",
            )));
        }

        self.update_contract_element_v1(
            contract_element,
            contract,
            &original_contract_fetch_info.contract,
            &block_info,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        // Update DataContracts cache with the new contract
        let updated_contract_fetch_info = self
            .fetch_contract_and_add_operations(
                contract.id().to_buffer(),
                Some(&block_info.epoch),
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "contract should exist",
            )))?;

        self.cache
            .data_contracts
            .insert(updated_contract_fetch_info, transaction.is_some());

        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            previous_fee_versions,
        )
    }

    /// Updates a contract.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_contract_element_v1(
        &self,
        contract_element: Element,
        contract: &DataContract,
        original_contract: &DataContract,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info =
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>;
        let batch_operations = self.update_contract_operations_v1(
            contract_element,
            contract,
            original_contract,
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

    /// Updates a contract.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn update_contract_add_operations_v1(
        &self,
        contract_element: Element,
        contract: &DataContract,
        original_contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let batch_operations = self.update_contract_operations_v1(
            contract_element,
            contract,
            original_contract,
            block_info,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        drive_operations.extend(batch_operations);
        Ok(())
    }

    /// operations for updating a contract.
    #[allow(clippy::too_many_arguments)]
    fn update_contract_operations_v1(
        &self,
        contract_element: Element,
        contract: &DataContract,
        original_contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = self
            .update_contract_operations_v0(
                contract_element,
                contract,
                original_contract,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?;

        for (token_pos, configuration) in contract.tokens() {
            let token_id = contract.token_id(*token_pos).ok_or(Error::DataContract(
                DataContractError::CorruptedDataContract(format!(
                    "data contract has a token at position {}, but it can not be found",
                    token_pos
                )),
            ))?;

            batch_operations.extend(self.create_token_trees_operations(
                contract.id(),
                *token_pos,
                token_id.to_buffer(),
                configuration.start_as_paused(),
                true,
                &mut None,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        if !contract.groups().is_empty() {
            batch_operations.extend(self.add_new_groups_operations(
                contract.id(),
                contract.groups(),
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        // Skipping an empty keyword set is load-bearing, but it is a shield
        // rather than a fix, and both halves matter to anyone changing it.
        //
        // What it prevents: the keyword update emits its deletes blind to each
        // other in one batch, so several of them jointly emptying the shared
        // `byContractId/<contractId>` group would leave that group tree behind
        // with nothing in it — and emptying the group without refilling it
        // requires exactly this empty-set case.
        //
        // What it costs: the previous keyword documents are not deleted either,
        // so a contract that clears its keywords advertises none while keyword
        // search still returns it under the old ones. Removing this guard to fix
        // that trades a stale index for a stranded group tree; the deletes have
        // to become sibling-aware first. Both halves are pinned —
        // `clearing_a_contracts_keywords_leaves_the_old_ones_indexed` and
        // `clearing_every_keyword_leaves_an_empty_by_contract_id_group_behind`.
        if !contract.keywords().is_empty() {
            batch_operations.extend(self.update_contract_keywords_operations(
                contract.id(),
                contract.owner_id(),
                contract.keywords(),
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        if let Some(description) = contract.description() {
            batch_operations.extend(self.update_contract_description_operations(
                contract.id(),
                contract.owner_id(),
                description,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        Ok(batch_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::accessors::v1::DataContractV1Setters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::prelude::Identifier;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    /// Exercises `update_contract_operations_v1` when the updated contract
    /// gains tokens that weren't in the original. This covers the loop that
    /// calls `create_token_trees_operations` for each token.
    /// PR #3516 inserts contracts with tokens but does not exercise an
    /// UPDATE that adds tokens.
    #[test]
    fn test_update_contract_v1_adds_tokens_creates_token_trees() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Original: no tokens.
        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.config_mut().set_readonly(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("insert initial contract without tokens");

        // Updated: add a token configuration. The update path exercises the
        // `create_token_trees_operations` call in update_contract_operations_v1.
        let token_config = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(0),
        );
        contract.set_tokens(BTreeMap::from([(0, token_config)]));
        contract.increment_version();

        drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("update adding tokens should succeed");
    }

    /// Exercises `update_contract_operations_v1` where the updated contract
    /// gains groups that weren't in the original. This covers the
    /// `if !contract.groups().is_empty()` true branch inside
    /// `update_contract_operations_v1`, invoking `add_new_groups_operations`.
    #[test]
    fn test_update_contract_v1_adds_groups() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("insert");

        // Add a group.
        let member = Identifier::random();
        let group = Group::V0(GroupV0 {
            members: BTreeMap::from([(member, 1)]),
            required_power: 1,
        });
        contract.set_groups(BTreeMap::from([(0, group)]));
        contract.increment_version();

        drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("update adding groups should succeed");
    }

    /// Exercises `update_contract_operations_v1`'s keyword-update branch:
    /// update a contract that starts with some keywords to a new set of
    /// keywords (different set), routed through the full `update_contract_v1`
    /// path rather than the dedicated `update_contract_keywords` API.
    /// PR #3516 covers the dedicated API but not the embedded path invoked
    /// via `update_contract`.
    #[test]
    fn test_update_contract_v1_keyword_delta_via_update_contract() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Insert the keyword_search system contract first (required because
        // update_contract_v1 calls update_contract_keywords_operations).
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

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_keywords(vec!["initial_a".to_string(), "initial_b".to_string()]);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("initial insert with keywords");

        // Now change keywords entirely.
        contract.set_keywords(vec!["new_x".to_string(), "new_y".to_string()]);
        contract.increment_version();

        drive
            .update_contract(
                &contract,
                BlockInfo {
                    time_ms: 2000,
                    height: 10,
                    core_height: 5,
                    epoch: Default::default(),
                },
                true,
                None,
                platform_version,
                None,
            )
            .expect("update keyword delta via update_contract should succeed");
    }

    /// The keywords the keyword search index currently returns for `contract_id`.
    fn indexed_keywords(
        drive: &crate::drive::Drive,
        keyword_search: &dpp::prelude::DataContract,
        contract_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Vec<String> {
        use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
        use crate::query::{DriveDocumentQuery, WhereClause, WhereOperator};
        use dpp::document::DocumentV0Getters;
        use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
        use dpp::platform_value::Value;

        let document_type = keyword_search
            .document_type_for_name("contractKeywords")
            .expect("contractKeywords doctype");
        let mut query = DriveDocumentQuery::all_items_query(keyword_search, document_type, None);
        query.internal_clauses.equal_clauses.insert(
            "contractId".to_string(),
            WhereClause {
                field: "contractId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(contract_id.to_buffer()),
            },
        );
        let mut keywords: Vec<String> = drive
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
                document
                    .properties()
                    .get_string("keyword")
                    .expect("every keyword document carries a keyword")
            })
            .collect();
        keywords.sort();
        keywords
    }

    /// **This test asserts a defect, not the desired behaviour**, and it is the
    /// other half of the empty-keyword-set skip above.
    ///
    /// Clearing a contract's keywords does not delete its keyword documents: an
    /// empty set skips the keyword update entirely, so the previous documents
    /// survive and stay indexed. The contract then advertises no keywords while
    /// keyword search still returns it under the old ones, permanently.
    ///
    /// The skip is a shield, not a fix. It is what keeps the deletes from
    /// jointly emptying the shared `byContractId` group and stranding it — see
    /// `clearing_every_keyword_leaves_an_empty_by_contract_id_group_behind` —
    /// so removing it to make this test go green trades a stale index for an
    /// empty group tree. Making the deletes sibling-aware has to come first.
    #[test]
    fn clearing_a_contracts_keywords_leaves_the_old_ones_indexed() {
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

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
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
            .expect("initial insert with keywords");

        assert_eq!(
            indexed_keywords(&drive, &keyword_search, contract.id(), platform_version),
            vec!["alpha".to_string(), "bravo".to_string()],
            "baseline: both keywords are indexed"
        );

        contract.set_keywords(vec![]);
        contract.increment_version();
        drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("clearing keywords via update_contract should succeed");

        assert_eq!(
            indexed_keywords(&drive, &keyword_search, contract.id(), platform_version),
            vec!["alpha".to_string(), "bravo".to_string()],
            "the old keyword documents are expected to survive: an empty keyword set skips \
             the keyword update rather than performing it"
        );
    }

    /// Exercises `update_contract_operations_v1`'s description-update branch:
    /// changing contract description routes through
    /// `update_contract_description_operations`. Covers the `if let Some(description)`
    /// true branch specifically from the v1 update path (not the dedicated update
    /// description API).
    #[test]
    fn test_update_contract_v1_description_via_update_contract() {
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

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_description(Some("initial description".to_string()));

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("initial insert with description");

        contract.set_description(Some("updated description text".to_string()));
        contract.increment_version();

        drive
            .update_contract(
                &contract,
                BlockInfo {
                    time_ms: 3000,
                    height: 20,
                    core_height: 7,
                    epoch: Default::default(),
                },
                true,
                None,
                platform_version,
                None,
            )
            .expect("update description via update_contract should succeed");
    }
}
