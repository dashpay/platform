use crate::drive::Drive;
use crate::util::storage_flags::StorageFlags;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use crate::drive::balances::total_tokens_root_supply_path_vec;
use crate::drive::tokens::paths::{
    token_balances_path_vec, token_balances_root_path, token_contract_infos_root_path,
    token_contract_lifecycles_root_path, token_identity_infos_root_path, token_statuses_root_path,
};
use crate::error::contract::DataContractError;
use crate::util::object_size_info::PathKeyElementInfo::PathKeyElement;
use crate::util::object_size_info::{DriveKeyInfo, PathKeyElementInfo};
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::serialization::{PlatformSerializable, PlatformSerializableWithPlatformVersion};
use dpp::tokens::contract_info::TokenContractInfo;
use dpp::tokens::contract_lifecycle::ContractTokenLifecycle;
use dpp::tokens::status::TokenStatus;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::Element::SumItem;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Insert a contract.
    ///
    /// Generation 2: the token handling of v1 plus the issuer's lifecycle record, seeded with
    /// the sum of the contract's base supplies, written in the same batch.
    #[inline(always)]
    pub(super) fn insert_contract_v2(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let storage_flags = if contract.config().can_be_deleted() || !contract.config().readonly() {
            Some(StorageFlags::new_single_epoch(
                block_info.epoch.index,
                Some(contract.owner_id().to_buffer()),
            ))
        } else {
            None
        };

        let serialized_contract =
            contract.serialize_to_bytes_with_platform_version(platform_version)?;

        if serialized_contract.len() as u64 > u32::MAX as u64
            || serialized_contract.len() as u32
                > platform_version.dpp.contract_versions.max_serialized_size
        {
            // This should normally be caught by DPP, but there is a rare possibility that the
            // re-serialized size is bigger than the original serialized data contract.
            return Err(Error::DataContract(DataContractError::ContractTooBig(format!("Trying to insert a data contract of size {} that is over the max allowed insertion size {}", serialized_contract.len(), platform_version.dpp.contract_versions.max_serialized_size))));
        }

        let contract_element = Element::Item(
            serialized_contract,
            StorageFlags::map_to_some_element_flags(storage_flags.as_ref()),
        );

        self.insert_contract_element_v2(
            contract_element,
            contract,
            &block_info,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )
    }

    /// Adds a contract to storage using `add_contract_to_storage`
    /// and inserts the empty trees which will be necessary to later insert documents.
    #[allow(clippy::too_many_arguments)]
    fn insert_contract_element_v2(
        &self,
        contract_element: Element,
        contract: &DataContract,
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
        let batch_operations = self.insert_contract_operations_v2(
            contract_element,
            contract,
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

    /// The operations for adding a contract.
    /// These operations add a contract to storage using `add_contract_to_storage`
    /// and insert the empty trees which will be necessary to later insert documents.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn insert_contract_add_operations_v2(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let batch_operations = self.insert_contract_operations_v2(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        drive_operations.extend(batch_operations);
        Ok(())
    }

    /// The operations for adding a contract.
    /// These operations add a contract to storage using `add_contract_to_storage`
    /// and insert the empty trees which will be necessary to later insert documents.
    fn insert_contract_operations_v2(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = self
            .insert_contract_operations_v0(
                contract_element,
                contract,
                block_info,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;

        if !contract.tokens().is_empty() {
            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Drive::add_estimation_costs_for_token_status_infos(
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;

                Drive::add_estimation_costs_for_token_contract_infos(
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;

                Drive::add_estimation_costs_for_token_contract_lifecycles(
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;
            }

            // The issuer's lifecycle record starts at the sum of the base supplies, which is
            // exactly what the supply leaves below are inserted with. A new contract has no
            // record yet, so this is a plain insert; the checked sum keeps two tokens at the
            // largest base supply from wrapping.
            let issued_supply = contract.tokens().values().try_fold(0u128, |sum, config| {
                sum.checked_add(config.base_supply() as u128)
                    .ok_or_else(|| {
                        Error::Protocol(Box::new(
                            ProtocolError::CriticalCorruptedCreditsCodeExecution(
                                "summed token base supplies overflow".to_string(),
                            ),
                        ))
                    })
            })?;
            let record_bytes = ContractTokenLifecycle::new(issued_supply, platform_version)?
                .serialize_consume_to_bytes()?;
            self.batch_insert(
                PathKeyElementInfo::PathFixedSizeKeyRefElement::<2>((
                    token_contract_lifecycles_root_path(),
                    contract.id().as_slice(),
                    Element::Item(record_bytes, None),
                )),
                &mut batch_operations,
                &platform_version.drive,
            )?;
        }

        for (token_pos, token_config) in contract.tokens() {
            let token_id = contract.token_id(*token_pos).ok_or(Error::DataContract(
                DataContractError::CorruptedDataContract(format!(
                    "data contract has a token at position {}, but can not find it",
                    token_pos
                )),
            ))?;

            let token_id_bytes = token_id.to_buffer();

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Drive::add_estimation_costs_for_token_balances(
                    token_id_bytes,
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;
                Drive::add_estimation_costs_for_token_identity_infos(
                    token_id_bytes,
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;
                Drive::add_estimation_costs_for_token_total_supply(
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;
            }

            self.batch_insert_empty_sum_tree(
                token_balances_root_path(),
                DriveKeyInfo::KeyRef(token_id_bytes.as_slice()),
                None,
                &mut batch_operations,
                &platform_version.drive,
            )?;

            self.batch_insert_empty_tree(
                token_identity_infos_root_path(),
                DriveKeyInfo::KeyRef(token_id_bytes.as_slice()),
                None,
                &mut batch_operations,
                &platform_version.drive,
            )?;

            if let Some(perpetual_distribution) =
                token_config.distribution_rules().perpetual_distribution()
            {
                self.add_perpetual_distribution(
                    token_id.to_buffer(),
                    perpetual_distribution,
                    estimated_costs_only_with_layer_info,
                    &mut batch_operations,
                    transaction,
                    platform_version,
                )?;
            }

            if token_config.start_as_paused() {
                // no status also means active.
                let starting_status = TokenStatus::new(true, platform_version)?;
                let token_status_bytes = starting_status.serialize_consume_to_bytes()?;

                self.batch_insert(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement::<2>((
                        token_statuses_root_path(),
                        token_id.as_slice(),
                        Element::Item(token_status_bytes, None),
                    )),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            }

            let token_contract_info =
                TokenContractInfo::new(contract.id(), *token_pos, platform_version)?;
            let token_contract_info_bytes = token_contract_info.serialize_consume_to_bytes()?;

            self.batch_insert(
                PathKeyElementInfo::PathFixedSizeKeyRefElement::<2>((
                    token_contract_infos_root_path(),
                    token_id.as_slice(),
                    Element::Item(token_contract_info_bytes, None),
                )),
                &mut batch_operations,
                &platform_version.drive,
            )?;

            if let Some(pre_programmed_distribution) = token_config
                .distribution_rules()
                .pre_programmed_distribution()
            {
                self.add_pre_programmed_distributions(
                    token_id.to_buffer(),
                    contract.owner_id().to_buffer(),
                    pre_programmed_distribution,
                    block_info,
                    estimated_costs_only_with_layer_info,
                    &mut batch_operations,
                    transaction,
                    platform_version,
                )?;
            }

            let path_holding_total_token_supply = total_tokens_root_supply_path_vec();

            if token_config.base_supply() > 0 {
                // We have a base supply that needs to be distributed on contract creation
                let destination_identity_id = token_config
                    .distribution_rules()
                    .new_tokens_destination_identity()
                    .copied()
                    .unwrap_or(contract.owner_id());
                let token_balance_path = token_balances_path_vec(token_id_bytes);

                if token_config.base_supply() > i64::MAX as u64 {
                    return Err(
                        ProtocolError::CriticalCorruptedCreditsCodeExecution(format!(
                            "Token base supply over i64 max, is {}",
                            token_config.base_supply()
                        ))
                        .into(),
                    );
                }
                self.batch_insert::<0>(
                    PathKeyElement((
                        token_balance_path,
                        destination_identity_id.to_vec(),
                        Element::new_sum_item(token_config.base_supply() as i64),
                    )),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
                self.batch_insert::<0>(
                    PathKeyElement((
                        path_holding_total_token_supply,
                        token_id.to_vec(),
                        Element::new_sum_item(token_config.base_supply() as i64),
                    )),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            } else {
                self.batch_insert::<0>(
                    PathKeyElement((
                        path_holding_total_token_supply,
                        token_id.to_vec(),
                        SumItem(0, None),
                    )),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            }
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

        if !contract.keywords().is_empty() {
            batch_operations.extend(self.add_new_contract_keywords_operations(
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
            batch_operations.extend(self.add_new_contract_description_operations(
                contract.id(),
                contract.owner_id(),
                description,
                false,
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
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::tokens::contract_lifecycle::v0::ContractTokenLifecycleV0Accessors;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_seed_the_lifecycle_record_with_the_summed_base_supplies() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let mut paused_config =
            TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
        paused_config.set_start_as_paused(true);
        contract.set_tokens(BTreeMap::from([
            (
                0,
                TokenConfiguration::V0(
                    TokenConfigurationV0::default_most_restrictive().with_base_supply(10),
                ),
            ),
            (1, paused_config),
            (
                2,
                TokenConfiguration::V0(
                    TokenConfigurationV0::default_most_restrictive().with_base_supply(500),
                ),
            ),
        ]));

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

        let record = drive
            .fetch_contract_token_lifecycle(contract.id().to_buffer(), None, platform_version)
            .expect("expected to read")
            .expect("expected a record");
        assert_eq!(record.issued_supply(), 510);
        assert!(!record.is_wiped());

        let totals = drive
            .calculate_total_tokens_balance(None, platform_version)
            .expect("expected totals");
        assert_eq!(totals.total_tokens_in_platform, 510);
        assert!(totals.ok().expect("expected a verdict"));
    }

    #[test]
    fn should_write_no_record_for_a_contract_without_tokens() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
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
            .expect("expected to insert the contract");

        assert_eq!(
            drive
                .fetch_contract_token_lifecycle(contract.id().to_buffer(), None, platform_version)
                .expect("expected to read"),
            None
        );
    }

    #[test]
    fn should_estimate_a_token_contract_without_touching_state() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_tokens(BTreeMap::from([(
            0,
            TokenConfiguration::V0(
                TokenConfigurationV0::default_most_restrictive().with_base_supply(500),
            ),
        )]));

        let fee = drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                false,
                None,
                platform_version,
            )
            .expect("expected an estimate");

        assert!(fee.processing_fee > 0 && fee.storage_fee > 0);
        assert_eq!(
            drive
                .fetch_contract_token_lifecycle(contract.id().to_buffer(), None, platform_version)
                .expect("expected to read"),
            None
        );
    }

    #[test]
    fn should_reject_a_base_supply_over_i64_max() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_tokens(BTreeMap::from([(
            0,
            TokenConfiguration::V0(
                TokenConfigurationV0::default_most_restrictive().with_base_supply(u64::MAX),
            ),
        )]));

        let result = drive.insert_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            platform_version,
        );

        assert!(
            matches!(
                &result,
                Err(crate::error::Error::Protocol(boxed))
                    if matches!(
                        boxed.as_ref(),
                        dpp::ProtocolError::CriticalCorruptedCreditsCodeExecution(_)
                    )
            ),
            "expected CriticalCorruptedCreditsCodeExecution, got: {:?}",
            result
        );
    }
}
