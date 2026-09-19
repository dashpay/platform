use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use dpp::serialization::PlatformSerializableWithPlatformVersion;

use crate::drive::balances::total_tokens_root_supply_path_vec;
use crate::drive::tokens::paths::token_balances_path_vec;
use crate::error::contract::DataContractError;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v1::TokenDistributionRulesV1Getters;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
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
    ///
    /// The v1 operations, plus the perpetual, pre-programmed and
    /// once-per-identity distribution storage of every token the update adds:
    /// the same storage `insert_contract` creates for a token present at
    /// registration. v1
    /// created none of it, so the first claim on such a token wrote its claim
    /// record under a tree that did not exist and failed as an internal error,
    /// leaving the distribution unclaimable.
    ///
    /// It also mints the base supply of a token the update adds, to the same
    /// identity `insert_contract` credits at registration. v1 left such a
    /// token at a total supply of zero with nobody holding any of it.
    ///
    /// An update whose config declares a moderation list the stored contract does not keep yet
    /// also creates that list's tree.
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
        let storage_flags = StorageFlags::map_some_element_flags_ref(contract_element.get_flags())?;

        let mut batch_operations: Vec<LowLevelDriveOperation> = self
            .update_contract_operations_v1(
                contract_element,
                contract,
                original_contract,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?;

        for (token_pos, configuration) in contract.tokens() {
            // Only a token absent from the original contract is new to state.
            // A token the contract already had keeps the supply and the
            // distribution storage it has: its base supply was minted when it
            // was added, and both distribution helpers error when the token's
            // tree already exists. That covers a token config update too,
            // which reaches this method with its token present in the original
            // contract. An update can not remove a token, so a token is new
            // exactly once.
            if original_contract.tokens().contains_key(token_pos) {
                continue;
            }

            let token_id = contract.token_id(*token_pos).ok_or(Error::DataContract(
                DataContractError::CorruptedDataContract(format!(
                    "data contract has a token at position {}, but it can not be found",
                    token_pos
                )),
            ))?;

            if configuration.base_supply() > 0 {
                let destination_identity_id = configuration
                    .distribution_rules()
                    .new_tokens_destination_identity()
                    .copied()
                    .unwrap_or(contract.owner_id());

                Self::mint_base_supply_of_added_token(
                    token_id.to_buffer(),
                    configuration.base_supply(),
                    destination_identity_id.to_buffer(),
                    &mut batch_operations,
                )?;
            }

            if let Some(perpetual_distribution) =
                configuration.distribution_rules().perpetual_distribution()
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

            if let Some(pre_programmed_distribution) = configuration
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

            // The once-per-identity claims subtree, as `insert_contract` creates it for the
            // tokens of a new contract; without it every claim would insert under a path
            // that does not exist.
            if configuration
                .distribution_rules()
                .once_per_identity_distribution()
                .is_some()
            {
                self.add_once_per_identity_distribution(
                    token_id.to_buffer(),
                    estimated_costs_only_with_layer_info,
                    &mut batch_operations,
                    transaction,
                    platform_version,
                )?;
            }
        }

        if let Some(moderation) = contract.config().moderation() {
            // The list trees already on disk are kept (the insert is `if not exists`); only a
            // list the update turns on gets a new tree.
            self.insert_contract_moderation_trees_operations(
                contract.id().to_buffer(),
                moderation,
                storage_flags.as_ref(),
                estimated_costs_only_with_layer_info,
                transaction,
                &mut batch_operations,
                platform_version,
            )?;
        }

        Ok(batch_operations)
    }

    /// Turns the operations creating the trees of a token the update adds into
    /// ones that also mint its base supply, the way `insert_contract` v1 does
    /// at registration: `destination_identity_id` is credited `base_supply`
    /// and the token's total supply starts at `base_supply`.
    ///
    /// The v1 operations already hold the insert of a zero
    /// total supply, and a batch may hold only one operation per path and
    /// key, so that insert is replaced rather than followed by a second one.
    /// The supply helpers used by a mint transition do not fit here: they
    /// read the current supply and balance from state, where this token's
    /// trees do not exist until the batch is applied.
    fn mint_base_supply_of_added_token(
        token_id: [u8; 32],
        base_supply: TokenAmount,
        destination_identity_id: [u8; 32],
        token_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        // The update transition's basic structure validation rejects such a
        // base supply as a consensus error, so this is not reachable from a
        // state transition.
        if base_supply > i64::MAX as u64 {
            return Err(
                ProtocolError::CriticalCorruptedCreditsCodeExecution(format!(
                    "Token base supply over i64 max, is {}",
                    base_supply
                ))
                .into(),
            );
        }

        let zero_total_supply_insert = LowLevelDriveOperation::insert_for_known_path_key_element(
            total_tokens_root_supply_path_vec(),
            token_id.to_vec(),
            Element::new_sum_item(0),
        );

        // The token is new to the contract and a token id is unique to its
        // contract and position, so no supply entry can exist yet and the
        // insert has to be there. Minting on top of an existing entry would
        // overwrite a live supply.
        let total_supply_insert = token_operations
            .iter_mut()
            .find(|operation| **operation == zero_total_supply_insert)
            .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                "a token new to the contract already has a total supply".to_string(),
            )))?;

        *total_supply_insert = LowLevelDriveOperation::insert_for_known_path_key_element(
            total_tokens_root_supply_path_vec(),
            token_id.to_vec(),
            Element::new_sum_item(base_supply as i64),
        );

        token_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
            token_balances_path_vec(token_id),
            destination_identity_id.to_vec(),
            Element::new_sum_item(base_supply as i64),
        ));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::balances::total_tokens_root_supply_path_vec;
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v1::TokenDistributionRulesV1Setters;
    use dpp::data_contract::associated_token::token_once_per_identity_distribution::v0::TokenOncePerIdentityDistributionV0;
    use dpp::data_contract::associated_token::token_once_per_identity_distribution::TokenOncePerIdentityDistribution;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::prelude::{DataContract, Identifier};
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::version::PlatformVersion;
    use dpp::ProtocolError;
    use grovedb::Element;
    use std::collections::BTreeMap;

    const DISTRIBUTION_RECIPIENT: [u8; 32] = [7; 32];

    fn block_based_distribution_type() -> RewardDistributionType {
        RewardDistributionType::BlockBasedDistribution {
            interval: 10,
            function: DistributionFunction::FixedAmount { amount: 50 },
        }
    }

    /// A token paying `DISTRIBUTION_RECIPIENT` 50 tokens every 10 blocks and,
    /// once, 445 tokens at time 100.
    fn token_with_both_distributions() -> TokenConfiguration {
        let mut configuration = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(0),
        );
        let recipient = Identifier::from(DISTRIBUTION_RECIPIENT);
        configuration
            .distribution_rules_mut()
            .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                TokenPerpetualDistributionV0 {
                    distribution_type: block_based_distribution_type(),
                    distribution_recipient: TokenDistributionRecipient::Identity(recipient),
                },
            )));
        configuration
            .distribution_rules_mut()
            .set_pre_programmed_distribution(Some(TokenPreProgrammedDistribution::V0(
                TokenPreProgrammedDistributionV0 {
                    distributions: BTreeMap::from([(100, BTreeMap::from([(recipient, 445)]))]),
                },
            )));
        configuration
    }

    /// Registers a contract without tokens, then adds
    /// `token_with_both_distributions` at position 0 through `update_contract`.
    /// Returns the updated contract and the id of the added token.
    fn add_token_with_distributions_by_update(
        drive: &Drive,
        platform_version: &PlatformVersion,
    ) -> (DataContract, [u8; 32]) {
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

        contract.set_tokens(BTreeMap::from([(0, token_with_both_distributions())]));
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
            .expect("update adding the token should succeed");

        let token_id = contract
            .token_id(0)
            .expect("expected the token added at position 0")
            .to_buffer();

        (contract, token_id)
    }

    /// Writes what a perpetual claim at block 40 writes.
    fn record_perpetual_claim(
        drive: &Drive,
        token_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let operations = drive.mark_perpetual_release_as_distributed_operations(
            token_id,
            DISTRIBUTION_RECIPIENT,
            RewardDistributionMoment::BlockBasedMoment(40),
            &mut None,
            platform_version,
        )?;
        drive.apply_batch_low_level_drive_operations(
            None,
            None,
            operations,
            &mut vec![],
            &platform_version.drive,
        )
    }

    /// Writes what a claim of the pre-programmed release at time 100 writes.
    fn record_pre_programmed_claim(
        drive: &Drive,
        token_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let operations = drive.mark_pre_programmed_release_as_distributed_operations(
            token_id,
            DISTRIBUTION_RECIPIENT,
            100,
            &BlockInfo::default(),
            &mut None,
            None,
            platform_version,
        )?;
        drive.apply_batch_low_level_drive_operations(
            None,
            None,
            operations,
            &mut vec![],
            &platform_version.drive,
        )
    }

    /// A token added by an update whose rules carry a once-per-identity distribution gets its
    /// claims subtree, so a claim can be recorded under it; a later update that adds nothing
    /// leaves the existing subtree alone.
    #[test]
    fn should_create_once_per_identity_distribution_storage_for_token_added_by_update() {
        let drive = setup_drive_with_initial_state_structure(None);
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
            .expect("insert initial contract without tokens");

        let mut token_config = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(0),
        );
        token_config
            .distribution_rules_mut()
            .set_once_per_identity_distribution(Some(TokenOncePerIdentityDistribution::V0(
                TokenOncePerIdentityDistributionV0 { amount: 100 },
            )));
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
            .expect("update adding the token should succeed");

        let token_id = contract.token_id(0).expect("expected the token id");
        let claimant = Identifier::random();

        let operations = drive
            .mark_once_per_identity_release_as_distributed_operations(
                token_id.to_buffer(),
                claimant.to_buffer(),
                1_000,
                &BlockInfo::default(),
                &mut None,
                platform_version,
            )
            .expect("expected the claim operations");
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("the claim must insert under the token's claims subtree");

        assert_eq!(
            drive
                .fetch_once_per_identity_distribution_claim(
                    token_id.to_buffer(),
                    claimant,
                    None,
                    platform_version,
                )
                .expect("expected to fetch the claim"),
            Some(1_000)
        );

        // The token now belongs to the original contract, so its subtree is not added again.
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
            .expect("a later update should leave the claims subtree alone");
    }

    #[test]
    fn should_create_perpetual_distribution_storage_for_token_added_by_update() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let (_, token_id) = add_token_with_distributions_by_update(&drive, platform_version);

        record_perpetual_claim(&drive, token_id, platform_version)
            .expect("a perpetual claim on the added token should be recordable");

        let last_paid_moment = drive
            .fetch_perpetual_distribution_last_paid_moment(
                token_id,
                Identifier::from(DISTRIBUTION_RECIPIENT),
                &block_based_distribution_type(),
                None,
                platform_version,
            )
            .expect("expected to fetch the last paid moment");
        assert_eq!(
            last_paid_moment,
            Some(RewardDistributionMoment::BlockBasedMoment(40))
        );
    }

    #[test]
    fn should_create_pre_programmed_distribution_storage_for_token_added_by_update() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let (_, token_id) = add_token_with_distributions_by_update(&drive, platform_version);

        let distributions = drive
            .fetch_token_pre_programmed_distributions(token_id, None, None, None, platform_version)
            .expect("expected to fetch the pre-programmed distributions");
        assert_eq!(
            distributions,
            BTreeMap::from([(
                100,
                BTreeMap::from([(Identifier::from(DISTRIBUTION_RECIPIENT), 445)])
            )])
        );

        record_pre_programmed_claim(&drive, token_id, platform_version)
            .expect("a pre-programmed claim on the added token should be recordable");

        let last_paid_time = drive
            .fetch_pre_programmed_distribution_last_paid_time_ms(
                token_id,
                Identifier::from(DISTRIBUTION_RECIPIENT),
                None,
                platform_version,
            )
            .expect("expected to fetch the last paid time");
        assert_eq!(last_paid_time, Some(100));
    }

    /// The frozen side of the gate, through the same dispatcher: protocol
    /// version 13 selects v1, which never creates the distribution storage, so
    /// neither claim can be recorded there.
    #[test]
    fn should_leave_token_added_by_update_without_distribution_storage_on_protocol_version_13() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");

        let (_, token_id) = add_token_with_distributions_by_update(&drive, platform_version);

        record_perpetual_claim(&drive, token_id, platform_version)
            .expect_err("v1 creates no perpetual distribution tree to record the claim under");
        record_pre_programmed_claim(&drive, token_id, platform_version)
            .expect_err("v1 creates no pre-programmed distribution tree to record the claim under");
    }

    /// The distribution storage helpers error when a token's tree already
    /// exists, so an update must leave the tokens it did not add alone, whether
    /// they came from the registration or from an earlier update.
    #[test]
    fn should_not_recreate_distribution_storage_of_tokens_the_contract_already_had() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.config_mut().set_readonly(false);
        contract.set_tokens(BTreeMap::from([(0, token_with_both_distributions())]));

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("insert initial contract with a token");

        // The first update adds a second token next to the registered one, the
        // second update changes nothing about either of them.
        let mut tokens = contract.tokens().clone();
        tokens.insert(1, token_with_both_distributions());
        contract.set_tokens(tokens);

        for _ in 0..2 {
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
                .expect("update keeping existing tokens should succeed");
        }

        for position in [0, 1] {
            let token_id = contract
                .token_id(position)
                .expect("expected both tokens")
                .to_buffer();
            record_perpetual_claim(&drive, token_id, platform_version)
                .expect("a perpetual claim should be recordable on both tokens");
            record_pre_programmed_claim(&drive, token_id, platform_version)
                .expect("a pre-programmed claim should be recordable on both tokens");
        }
    }

    const BASE_SUPPLY: u64 = 1_000_000;
    const BASE_SUPPLY_DESTINATION: [u8; 32] = [9; 32];

    /// A token with a base supply of `BASE_SUPPLY`, credited to `destination`
    /// when there is one and to the contract owner otherwise.
    fn token_with_base_supply(destination: Option<Identifier>) -> TokenConfiguration {
        let mut configuration = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(BASE_SUPPLY),
        );
        configuration
            .distribution_rules_mut()
            .set_new_tokens_destination_identity(destination);
        configuration
    }

    /// Registers a contract without tokens, then adds `configuration` at
    /// position 0 through `update_contract`. Returns the updated contract and
    /// the id of the added token.
    fn add_token_by_update(
        drive: &Drive,
        configuration: TokenConfiguration,
        platform_version: &PlatformVersion,
    ) -> (DataContract, [u8; 32]) {
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

        contract.set_tokens(BTreeMap::from([(0, configuration)]));
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
            .expect("update adding the token should succeed");

        let token_id = contract
            .token_id(0)
            .expect("expected the token added at position 0")
            .to_buffer();

        (contract, token_id)
    }

    fn balance_and_total_supply(
        drive: &Drive,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> (Option<u64>, Option<u64>) {
        let balance = drive
            .fetch_identity_token_balance(token_id, identity_id, None, platform_version)
            .expect("expected to fetch the token balance");
        let total_supply = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("expected to fetch the token total supply");
        (balance, total_supply)
    }

    #[test]
    fn should_mint_base_supply_to_contract_owner_for_token_added_by_update() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let (contract, token_id) =
            add_token_by_update(&drive, token_with_base_supply(None), platform_version);

        assert_eq!(
            balance_and_total_supply(
                &drive,
                token_id,
                contract.owner_id().to_buffer(),
                platform_version
            ),
            (Some(BASE_SUPPLY), Some(BASE_SUPPLY))
        );
        assert_eq!(
            drive
                .fetch_token_total_aggregated_identity_balances(token_id, None, platform_version)
                .expect("expected to fetch the aggregated balances"),
            Some(BASE_SUPPLY),
            "the balances of the token must sum to its total supply"
        );
    }

    #[test]
    fn should_mint_base_supply_to_new_tokens_destination_identity_for_token_added_by_update() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let (contract, token_id) = add_token_by_update(
            &drive,
            token_with_base_supply(Some(Identifier::from(BASE_SUPPLY_DESTINATION))),
            platform_version,
        );

        assert_eq!(
            balance_and_total_supply(&drive, token_id, BASE_SUPPLY_DESTINATION, platform_version),
            (Some(BASE_SUPPLY), Some(BASE_SUPPLY))
        );
        assert_eq!(
            drive
                .fetch_identity_token_balance(
                    token_id,
                    contract.owner_id().to_buffer(),
                    None,
                    platform_version
                )
                .expect("expected to fetch the owner's token balance"),
            None,
            "the owner gets nothing when the token names another destination"
        );
    }

    /// The frozen side of the gate, through the same dispatcher: protocol
    /// version 13 selects v1, which creates the token's trees with a total
    /// supply of zero and credits nobody.
    #[test]
    fn should_not_mint_base_supply_of_token_added_by_update_on_protocol_version_13() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");

        let (contract, token_id) =
            add_token_by_update(&drive, token_with_base_supply(None), platform_version);

        assert_eq!(
            balance_and_total_supply(
                &drive,
                token_id,
                contract.owner_id().to_buffer(),
                platform_version
            ),
            (None, Some(0))
        );
    }

    #[test]
    fn should_leave_token_without_base_supply_added_by_update_at_zero_supply() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let configuration = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(0),
        );
        let (contract, token_id) = add_token_by_update(&drive, configuration, platform_version);

        assert_eq!(
            balance_and_total_supply(
                &drive,
                token_id,
                contract.owner_id().to_buffer(),
                platform_version
            ),
            (None, Some(0))
        );
    }

    #[test]
    fn should_mint_base_supply_of_every_token_added_by_one_update() {
        let drive = setup_drive_with_initial_state_structure(None);
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
            .expect("insert initial contract without tokens");

        let destination = Identifier::from(BASE_SUPPLY_DESTINATION);
        contract.set_tokens(BTreeMap::from([
            (0, token_with_base_supply(None)),
            (1, token_with_base_supply(Some(destination))),
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
            .expect("update adding two tokens should succeed");

        for (position, holder) in [(0, contract.owner_id()), (1, destination)] {
            let token_id = contract
                .token_id(position)
                .expect("expected both tokens")
                .to_buffer();
            assert_eq!(
                balance_and_total_supply(&drive, token_id, holder.to_buffer(), platform_version),
                (Some(BASE_SUPPLY), Some(BASE_SUPPLY)),
                "token at position {position}"
            );
        }
    }

    /// Nothing is minted retroactively. A token added by update before
    /// protocol version 14 got no base supply, and every later update finds it
    /// in the original contract, so it stays that way after the upgrade.
    #[test]
    fn should_not_mint_base_supply_of_token_added_by_update_before_protocol_version_14() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version_13 = PlatformVersion::get(13).expect("expected protocol version 13");
        let platform_version = PlatformVersion::latest();

        let (mut contract, token_id) =
            add_token_by_update(&drive, token_with_base_supply(None), platform_version_13);

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
            .expect("update after the upgrade should succeed");

        assert_eq!(
            balance_and_total_supply(
                &drive,
                token_id,
                contract.owner_id().to_buffer(),
                platform_version
            ),
            (None, Some(0))
        );
    }

    /// Neither refusal is reachable through a state transition: the update's
    /// basic structure validation rejects a base supply over `i64::MAX`, and a
    /// token new to a contract has no supply entry in state.
    #[test]
    fn should_refuse_to_mint_an_unstorable_base_supply_or_over_an_existing_total_supply() {
        let token_id = [3; 32];

        let mut operations = vec![LowLevelDriveOperation::insert_for_known_path_key_element(
            total_tokens_root_supply_path_vec(),
            token_id.to_vec(),
            Element::new_sum_item(0),
        )];
        let result = Drive::mint_base_supply_of_added_token(
            token_id,
            i64::MAX as u64 + 1,
            BASE_SUPPLY_DESTINATION,
            &mut operations,
        );
        assert!(matches!(
            result,
            Err(Error::Protocol(error))
                if matches!(*error, ProtocolError::CriticalCorruptedCreditsCodeExecution(_))
        ));

        // `create_token_trees_operations` queues no total supply insert when
        // state already holds one.
        let result = Drive::mint_base_supply_of_added_token(
            token_id,
            BASE_SUPPLY,
            BASE_SUPPLY_DESTINATION,
            &mut vec![],
        );
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
    }

    /// The base supply is minted once, by the update that adds the token. A
    /// later update sees the token in the original contract and must not mint
    /// again, and neither may it touch a token that got its base supply at
    /// registration.
    #[test]
    fn should_mint_base_supply_only_in_the_update_that_adds_the_token() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.config_mut().set_readonly(false);
        contract.set_tokens(BTreeMap::from([(0, token_with_base_supply(None))]));

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("insert initial contract with a token");

        // The first update adds a second token next to the registered one, the
        // second update changes nothing about either of them.
        let mut tokens = contract.tokens().clone();
        tokens.insert(1, token_with_base_supply(None));
        contract.set_tokens(tokens);

        for _ in 0..2 {
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
                .expect("update keeping existing tokens should succeed");
        }

        for position in [0, 1] {
            let token_id = contract
                .token_id(position)
                .expect("expected both tokens")
                .to_buffer();
            assert_eq!(
                balance_and_total_supply(
                    &drive,
                    token_id,
                    contract.owner_id().to_buffer(),
                    platform_version
                ),
                (Some(BASE_SUPPLY), Some(BASE_SUPPLY)),
                "token at position {position}"
            );
        }
    }
}
