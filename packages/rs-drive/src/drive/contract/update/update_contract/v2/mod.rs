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
    /// Generation 2: the same update as v1, with the accumulated batch handed to the token
    /// tree creation of every added token so the issuer's lifecycle record is written once
    /// per update, whatever the number of tokens added.
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
    pub(super) fn update_contract_v2(
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

        self.update_contract_element_v2(
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
    pub(super) fn update_contract_element_v2(
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
        let batch_operations = self.update_contract_operations_v2(
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
    pub(super) fn update_contract_add_operations_v2(
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
        let batch_operations = self.update_contract_operations_v2(
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
    fn update_contract_operations_v2(
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

            // The accumulated batch is handed to the token tree creation so the issuer's
            // lifecycle record, which the first added token inserts, is seen by the tokens
            // that follow in the same update; v1 passed no batch and so emitted the record
            // once per token, which the batch consistency check rejects.
            let token_operations = self.create_token_trees_operations(
                contract.id(),
                *token_pos,
                token_id.to_buffer(),
                configuration.start_as_paused(),
                true,
                &mut Some(&mut batch_operations),
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?;
            batch_operations.extend(token_operations);
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
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::prelude::{DataContract, Identifier};
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::tokens::contract_lifecycle::v0::ContractTokenLifecycleV0Accessors;
    use dpp::tokens::contract_lifecycle::TokenLifecycle;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn zero_supply_token() -> TokenConfiguration {
        TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive().with_base_supply(0))
    }

    /// A mutable contract without tokens, inserted at the latest version.
    fn inserted_contract_without_tokens(drive: &crate::drive::Drive) -> DataContract {
        let platform_version = PlatformVersion::latest();
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
            .expect("expected to insert the contract");
        contract
    }

    #[test]
    fn should_create_the_token_trees_and_the_lifecycle_record_when_a_token_is_added() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut contract = inserted_contract_without_tokens(&drive);

        contract.set_tokens(BTreeMap::from([(0, zero_supply_token())]));
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
            .expect("expected the update adding a token to succeed");

        let token_id = contract.token_id(0).expect("expected a token id");
        assert_eq!(
            drive
                .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
                .expect("expected to fetch supply"),
            Some(0)
        );
        let record = drive
            .fetch_contract_token_lifecycle(contract.id().to_buffer(), None, platform_version)
            .expect("expected to read")
            .expect("expected a record");
        assert_eq!(record.issued_supply(), 0);
        assert!(!record.is_wiped());
    }

    /// v1 emitted the record once per added token, which the batch consistency check
    /// rejects; v2 hands the accumulated batch to every token so the record is written once.
    #[test]
    fn should_write_the_lifecycle_record_once_when_several_tokens_are_added() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut contract = inserted_contract_without_tokens(&drive);

        contract.set_tokens(BTreeMap::from([
            (0, zero_supply_token()),
            (1, zero_supply_token()),
            (2, zero_supply_token()),
        ]));
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
            .expect("expected the update adding three tokens to succeed");

        let token_ids: Vec<[u8; 32]> = (0..3u16)
            .map(|position| {
                contract
                    .token_id(position)
                    .expect("expected a token id")
                    .to_buffer()
            })
            .collect();
        let lifecycles = drive
            .fetch_token_lifecycles(&token_ids, None, platform_version)
            .expect("expected to resolve the tokens");
        assert_eq!(lifecycles.len(), 3);
        assert!(lifecycles
            .values()
            .all(|lifecycle| *lifecycle == TokenLifecycle::Live));
        drive.assert_token_rollups_consistent(None, platform_version);
    }

    #[test]
    fn should_keep_the_rollup_of_an_issuer_adding_a_token() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut contract = inserted_contract_without_tokens(&drive);

        contract.set_tokens(BTreeMap::from([(0, zero_supply_token())]));
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
            .expect("expected the first update to succeed");
        let first_token = contract.token_id(0).expect("expected a token id");
        drive
            .token_mint(
                first_token.to_buffer(),
                contract.owner_id().to_buffer(),
                640,
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to mint");

        contract.set_tokens(BTreeMap::from([
            (0, zero_supply_token()),
            (1, zero_supply_token()),
        ]));
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
            .expect("expected the second update to succeed");

        let record = drive
            .fetch_contract_token_lifecycle(contract.id().to_buffer(), None, platform_version)
            .expect("expected to read")
            .expect("expected a record");
        assert_eq!(record.issued_supply(), 640);
        drive.assert_token_rollups_consistent(None, platform_version);
    }

    #[test]
    fn should_refuse_to_add_a_token_to_a_destroyed_issuer() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut contract = inserted_contract_without_tokens(&drive);

        contract.set_tokens(BTreeMap::from([(0, zero_supply_token())]));
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
            .expect("expected the first update to succeed");
        drive
            .destroy_token_issuer(
                contract.id().to_buffer(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to destroy the issuer");

        contract.set_tokens(BTreeMap::from([
            (0, zero_supply_token()),
            (1, zero_supply_token()),
        ]));
        contract.increment_version();
        let result = drive.update_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        );

        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::CorruptedDriveState(_)))
            ),
            "expected the update to be refused, got {result:?}"
        );
        let second_token = contract.token_id(1).expect("expected a token id");
        assert_eq!(
            drive
                .fetch_token_total_supply(second_token.to_buffer(), None, platform_version)
                .expect("expected to fetch supply"),
            None
        );
    }

    #[test]
    fn should_estimate_an_update_adding_tokens_without_touching_state() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut contract = inserted_contract_without_tokens(&drive);
        let root_hash_before = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected a root hash");

        contract.set_tokens(BTreeMap::from([
            (0, zero_supply_token()),
            (1, zero_supply_token()),
        ]));
        contract.increment_version();
        let estimated = drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                false,
                None,
                platform_version,
                None,
            )
            .expect("expected an estimate");

        assert!(estimated.processing_fee > 0);
        let root_hash_after = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected a root hash");
        assert_eq!(root_hash_before, root_hash_after);

        let applied = drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to apply");
        assert!(
            estimated.total_base_fee() >= applied.total_base_fee(),
            "estimated total {} is below applied total {}",
            estimated.total_base_fee(),
            applied.total_base_fee()
        );
    }

    #[test]
    fn should_add_groups_through_the_update() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut contract = inserted_contract_without_tokens(&drive);

        let member = Identifier::from([42u8; 32]);
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
            .expect("expected the update adding groups to succeed");
    }
}
