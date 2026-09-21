use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::DataContract;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::tokens::calculate_token_id;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Protocol 15 contract updates, including pools for newly added shielded tokens.
    #[inline(always)]
    pub(super) fn update_contract_v3(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
        platform_version: &PlatformVersion,
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

        self.update_contract_element_v3(
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
            .insert_rewritten(updated_contract_fetch_info, transaction.is_some());

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
    pub(super) fn update_contract_element_v3(
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
        let batch_operations = self.update_contract_operations_v3(
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
    pub(super) fn update_contract_add_operations_v3(
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
        let batch_operations = self.update_contract_operations_v3(
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

    /// Protocol 14 update operations plus shielded pools for newly added tokens.
    #[allow(clippy::too_many_arguments)]
    fn update_contract_operations_v3(
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
        let mut batch_operations = vec![];
        self.update_contract_add_operations_v2(
            contract_element,
            contract,
            original_contract,
            block_info,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
            platform_version,
        )?;

        for (position, configuration) in contract.tokens() {
            if original_contract.tokens().contains_key(position)
                || !configuration.has_shielded_pool()
            {
                continue;
            }
            let token_id = calculate_token_id(contract.id().as_bytes(), *position);
            batch_operations.extend(self.create_token_shielded_pool_trees_operations(
                token_id,
                true,
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
    use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
    use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_create_token_pools_only_from_protocol_15_for_registration_and_updates() {
        for platform_version in [PlatformVersion::get(14).unwrap(), PlatformVersion::latest()] {
            for add_by_update in [false, true] {
                let drive = setup_drive_with_initial_state_structure(None);
                let mut contract =
                    get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
                        .data_contract_owned();
                contract.config_mut().set_readonly(false);

                if add_by_update {
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
                    contract.increment_version();
                }

                let mut configuration = TokenConfiguration::V0(
                    TokenConfigurationV0::default_most_restrictive().with_base_supply(100),
                );
                configuration.set_has_shielded_pool(true);
                contract.set_tokens(BTreeMap::from([(0, configuration)]));
                if add_by_update {
                    drive
                        .update_contract(
                            &contract,
                            BlockInfo::default(),
                            true,
                            None,
                            platform_version,
                            None,
                        )
                        .expect("add token through contract update");
                } else {
                    drive
                        .apply_contract(
                            &contract,
                            BlockInfo::default(),
                            true,
                            StorageFlags::optional_default_as_cow(),
                            None,
                            platform_version,
                        )
                        .expect("register token contract");
                }

                let token_id = contract.token_id(0).expect("token id").to_buffer();
                assert_eq!(
                    drive
                        .has_token_shielded_pool(token_id, None, &mut vec![], platform_version)
                        .expect("check token pool"),
                    platform_version.protocol_version >= 15,
                );
                assert_eq!(
                    drive
                        .fetch_token_total_supply(token_id, None, platform_version)
                        .expect("read total supply"),
                    Some(100)
                );

                // Updating an existing token must not recreate its pool or mint its supply again.
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
                    .expect("update existing token without recreating storage");
                assert_eq!(
                    drive
                        .fetch_token_total_supply(token_id, None, platform_version)
                        .expect("read unchanged total supply"),
                    Some(100)
                );
            }
        }
    }
}
