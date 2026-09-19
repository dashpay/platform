use crate::drive::tokens::paths::{
    token_ms_timed_at_time_distributions_path_vec, token_ms_timed_distributions_path_vec,
    token_pre_programmed_at_time_distribution_path_vec, token_pre_programmed_distributions_path,
    token_root_pre_programmed_distributions_path,
    TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY,
    TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::{DriveKeyInfo, PathInfo, PathKeyElementInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::associated_token::token_distribution_key::{
    TokenDistributionKey, TokenDistributionType,
};
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::element::reference_path::ReferencePathType;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Version 1 of `add_pre_programmed_distributions` (protocol version 14).
    ///
    /// Version 0 except that the release-time tree under the millisecond timed distributions,
    /// which all tokens share, is also looked for among `batch_operations`. A contract whose
    /// tokens release at the same time therefore queues that tree once instead of once per
    /// token.
    ///
    /// Queued twice, the batch is refused by a Drive with `batching_consistency_verification`
    /// on, as an internal error. The shipped default is off, and there GroveDB folds the two
    /// identical inserts into one, so such a contract was stored, and is stored the same here.
    /// The later tokens no longer pay for the existence read of the tree, so the processing
    /// fee is lower: that, and not the stored state, is what makes this a new version. For a
    /// single token the operations are the ones version 0 produces. The layout is documented
    /// on [`Drive::add_pre_programmed_distributions`].
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_pre_programmed_distributions_v1(
        &self,
        token_id: [u8; 32],
        owner_id: [u8; 32],
        distribution: &TokenPreProgrammedDistribution,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_token_pre_programmed_distribution(
                token_id,
                Some(distribution.distributions().keys()),
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;

            Drive::add_estimation_costs_for_root_token_ms_interval_distribution(
                distribution.distributions().keys(),
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }
        let storage_flags = StorageFlags::new_single_epoch(block_info.epoch.index, Some(owner_id));

        let pre_programmed_distributions_path = token_root_pre_programmed_distributions_path();

        // Insert the tree for this token's perpetual distribution
        let apply_tree_type_no_storage_flags = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: 0,
            }
        };

        let apply_tree_type_with_storage_flags = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: storage_flags.serialized_size(),
            }
        };

        let token_tree_key_info = DriveKeyInfo::Key(token_id.to_vec());
        let pre_programmed_distributions_path_key_info = token_tree_key_info.add_path_info::<3>(
            PathInfo::PathFixedSizeArray(pre_programmed_distributions_path),
        );

        let inserted = self.batch_insert_empty_tree_if_not_exists(
            pre_programmed_distributions_path_key_info,
            TreeType::NormalTree,
            None, // we will never clean this part up
            apply_tree_type_no_storage_flags,
            transaction,
            &mut None,
            batch_operations,
            &platform_version.drive,
        )?;

        if !inserted {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution("we can not insert the pre programmed distribution as it already existed, this should have been validated before insertion")));
        }
        let pre_programmed_distributions_path = token_pre_programmed_distributions_path(&token_id);

        self.batch_insert_empty_tree(
            pre_programmed_distributions_path,
            DriveKeyInfo::Key(vec![
                TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY,
            ]),
            None, // we will never clean this part up
            batch_operations,
            &platform_version.drive,
        )?;

        for (time, distribution) in distribution.distributions() {
            self.batch_insert_empty_sum_tree(
                pre_programmed_distributions_path,
                DriveKeyInfo::Key(time.to_be_bytes().to_vec()),
                None, // we will never clean this part up
                batch_operations,
                &platform_version.drive,
            )?;

            let ms_time_distribution_path = token_ms_timed_distributions_path_vec();

            let time_tree_key_info = DriveKeyInfo::Key(time.to_be_bytes().to_vec());
            let time_tree_reference_path_key_info = time_tree_key_info
                .add_path_info::<0>(PathInfo::PathAsVec(ms_time_distribution_path));

            // Every token releasing at this time keeps its references under this one tree, so
            // an earlier token of the same contract may already have queued it. Version 0
            // looked for the tree in state only and queued it a second time, which a Drive
            // verifying the consistency of its batches refuses ("insertion order error").
            self.batch_insert_empty_tree_if_not_exists_check_existing_operations(
                time_tree_reference_path_key_info,
                false,
                Some(&storage_flags),
                apply_tree_type_with_storage_flags,
                transaction,
                batch_operations,
                &platform_version.drive,
            )?;

            let pre_programmed_at_time_distribution_path =
                token_pre_programmed_at_time_distribution_path_vec(token_id, *time);
            let ms_time_at_time_distribution_path =
                token_ms_timed_at_time_distributions_path_vec(*time);

            for (recipient, amount) in distribution {
                if *amount > i64::MAX as u64 {
                    return Err(Error::Protocol(Box::new(ProtocolError::Overflow(
                        "distribution amount over i64::Max",
                    ))));
                }
                // We use a sum tree to be able to ask "at this time how much was distributed"
                self.batch_insert(
                    PathKeyElementInfo::<0>::PathKeyElement((
                        pre_programmed_at_time_distribution_path.clone(),
                        recipient.to_vec(),
                        Element::new_sum_item(*amount as i64),
                    )),
                    batch_operations,
                    &platform_version.drive,
                )?;

                let distribution_key = TokenDistributionKey {
                    token_id: token_id.into(),
                    recipient: TokenDistributionRecipient::Identity(*recipient),
                    distribution_type: TokenDistributionType::PreProgrammed,
                };

                let serialized_key = distribution_key.serialize_consume_to_bytes()?;

                let remaining_reference = vec![
                    vec![TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY],
                    token_id.to_vec(),
                    time.to_be_bytes().to_vec(),
                    recipient.to_vec(),
                ];

                let reference =
                    ReferencePathType::UpstreamRootHeightReference(2, remaining_reference);

                // Now we create the reference
                self.batch_insert(
                    PathKeyElementInfo::<0>::PathKeyElement((
                        ms_time_at_time_distribution_path.clone(),
                        serialized_key,
                        Element::new_reference_with_flags(
                            reference,
                            storage_flags.to_some_element_flags(),
                        ),
                    )),
                    batch_operations,
                    &platform_version.drive,
                )?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::config::DriveConfig;
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};
    use dpp::balances::credits::TokenAmount;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::TokenContractPosition;
    use dpp::prelude::{DataContract, Identifier, TimestampMillis};
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    const RECIPIENT: [u8; 32] = [7; 32];
    const RELEASE_TIME: TimestampMillis = 100;
    const RELEASE_AMOUNT: TokenAmount = 445;

    /// A token releasing `RELEASE_AMOUNT` to `RECIPIENT` at `RELEASE_TIME`.
    fn token_releasing_at_the_shared_time() -> TokenConfiguration {
        let mut configuration = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(0),
        );
        configuration
            .distribution_rules_mut()
            .set_pre_programmed_distribution(Some(TokenPreProgrammedDistribution::V0(
                TokenPreProgrammedDistributionV0 {
                    distributions: BTreeMap::from([(
                        RELEASE_TIME,
                        BTreeMap::from([(Identifier::from(RECIPIENT), RELEASE_AMOUNT)]),
                    )]),
                },
            )));
        configuration
    }

    fn contract_without_tokens(id_seed: u64, platform_version: &PlatformVersion) -> DataContract {
        let mut contract =
            get_dashpay_contract_fixture(None, id_seed, platform_version.protocol_version)
                .data_contract_owned();
        contract.config_mut().set_readonly(false);
        contract
    }

    /// A contract with two tokens, both releasing at `RELEASE_TIME`.
    fn contract_with_two_tokens_sharing_a_release_time(
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let mut contract = contract_without_tokens(0, platform_version);
        contract.set_tokens(BTreeMap::from([
            (0, token_releasing_at_the_shared_time()),
            (1, token_releasing_at_the_shared_time()),
        ]));
        contract
    }

    fn insert_contract(
        drive: &Drive,
        contract: &DataContract,
        apply: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        drive
            .apply_contract(
                contract,
                BlockInfo::default(),
                apply,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .map(|_| ())
    }

    fn token_id(contract: &DataContract, position: TokenContractPosition) -> [u8; 32] {
        contract
            .token_id(position)
            .expect("expected a token at the position")
            .to_buffer()
    }

    /// Asserts the release of the token is stored and that a claim of it, which consumes the
    /// reference kept under the release-time tree all tokens share, can be recorded.
    fn assert_release_is_stored_and_claimable(
        drive: &Drive,
        token_id: [u8; 32],
        platform_version: &PlatformVersion,
    ) {
        let stored = drive
            .fetch_token_pre_programmed_distributions(token_id, None, None, None, platform_version)
            .expect("expected to fetch the pre-programmed distributions");
        assert_eq!(
            stored,
            BTreeMap::from([(
                RELEASE_TIME,
                BTreeMap::from([(Identifier::from(RECIPIENT), RELEASE_AMOUNT)])
            )])
        );

        let operations = drive
            .mark_pre_programmed_release_as_distributed_operations(
                token_id,
                RECIPIENT,
                RELEASE_TIME,
                &BlockInfo::default(),
                &mut None,
                None,
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
            .expect("expected the claim to be recorded");
    }

    #[test]
    fn should_insert_contract_whose_tokens_share_a_pre_programmed_release_time() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let contract = contract_with_two_tokens_sharing_a_release_time(platform_version);

        insert_contract(&drive, &contract, true, platform_version)
            .expect("expected the contract to be inserted");

        assert_release_is_stored_and_claimable(&drive, token_id(&contract, 0), platform_version);
        assert_release_is_stored_and_claimable(&drive, token_id(&contract, 1), platform_version);
    }

    #[test]
    fn should_estimate_contract_whose_tokens_share_a_pre_programmed_release_time() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let contract = contract_with_two_tokens_sharing_a_release_time(platform_version);

        insert_contract(&drive, &contract, false, platform_version)
            .expect("expected the contract insert to be estimated");
    }

    #[test]
    fn should_update_contract_adding_tokens_that_share_a_pre_programmed_release_time() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let mut contract = contract_without_tokens(0, platform_version);

        insert_contract(&drive, &contract, true, platform_version)
            .expect("expected the contract without tokens to be inserted");

        contract.set_tokens(BTreeMap::from([
            (0, token_releasing_at_the_shared_time()),
            (1, token_releasing_at_the_shared_time()),
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
            .expect("expected the update adding both tokens to succeed");

        assert_release_is_stored_and_claimable(&drive, token_id(&contract, 0), platform_version);
        assert_release_is_stored_and_claimable(&drive, token_id(&contract, 1), platform_version);
    }

    #[test]
    fn should_share_the_release_time_tree_with_a_contract_inserted_earlier() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);

        let mut first_contract = contract_without_tokens(0, platform_version);
        first_contract.set_tokens(BTreeMap::from([(0, token_releasing_at_the_shared_time())]));
        let mut second_contract = contract_without_tokens(1, platform_version);
        second_contract.set_tokens(BTreeMap::from([(0, token_releasing_at_the_shared_time())]));
        assert_ne!(token_id(&first_contract, 0), token_id(&second_contract, 0));

        insert_contract(&drive, &first_contract, true, platform_version)
            .expect("expected the first contract to be inserted");
        insert_contract(&drive, &second_contract, true, platform_version)
            .expect("expected the second contract to be inserted");

        assert_release_is_stored_and_claimable(
            &drive,
            token_id(&first_contract, 0),
            platform_version,
        );
        assert_release_is_stored_and_claimable(
            &drive,
            token_id(&second_contract, 0),
            platform_version,
        );
    }

    #[test]
    fn should_still_queue_the_shared_release_time_tree_twice_on_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = contract_with_two_tokens_sharing_a_release_time(platform_version);

        // This Drive verifies the consistency of its batches, which a shipped node does not
        let error = insert_contract(&drive, &contract, true, platform_version)
            .expect_err("version 0 queues the shared release-time tree once per token");

        assert!(
            matches!(&error, Error::Drive(DriveError::GroveDBInsertion(message)) if message.contains("insertion order error")),
            "unexpected error: {error:?}"
        );
    }

    /// A Drive configured the way a shipped node is. `setup_drive_with_initial_state_structure`
    /// turns `batching_consistency_verification` on, the shipped default is off, and only with
    /// it on does Drive refuse a batch that queues one tree twice.
    fn setup_drive_with_shipped_batching_config(platform_version: &PlatformVersion) -> Drive {
        let drive = setup_drive(Some(DriveConfig::default()));
        assert!(
            !drive.config.batching_consistency_verification,
            "this fixture exists to exercise the shipped default"
        );
        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create the initial state structure");
        drive
    }

    /// With the shipped batching config version 0 stores such a contract: GroveDB folds the two
    /// identical inserts of the shared release-time tree into one. Version 1 has to leave that
    /// state untouched. What it does change is the processing fee, by the existence read the
    /// second token no longer makes, which is why it is a new version and not an edit.
    #[test]
    fn should_store_the_state_version_0_stores_with_the_shipped_batching_config() {
        let latest_platform_version = PlatformVersion::latest();
        let mut version_0_platform_version = latest_platform_version.clone();
        version_0_platform_version
            .drive
            .methods
            .token
            .distribution
            .add_pre_programmed_distributions = 0;

        let contract = contract_with_two_tokens_sharing_a_release_time(latest_platform_version);

        let [(version_0_fee, version_0_root_hash), (version_1_fee, version_1_root_hash)] =
            [&version_0_platform_version, latest_platform_version].map(|platform_version| {
                let drive = setup_drive_with_shipped_batching_config(platform_version);
                let fee = drive
                    .apply_contract(
                        &contract,
                        BlockInfo::default(),
                        true,
                        StorageFlags::optional_default_as_cow(),
                        None,
                        platform_version,
                    )
                    .expect("expected the contract to be inserted");
                let root_hash = drive
                    .grove
                    .root_hash(None, &platform_version.drive.grove_version)
                    .unwrap()
                    .expect("expected the root hash");

                for position in [0, 1] {
                    assert_release_is_stored_and_claimable(
                        &drive,
                        token_id(&contract, position),
                        platform_version,
                    );
                }

                (fee, root_hash)
            });

        assert_eq!(version_0_root_hash, version_1_root_hash);
        assert_eq!(version_0_fee.storage_fee, version_1_fee.storage_fee);
        assert!(version_1_fee.processing_fee < version_0_fee.processing_fee);
    }

    /// Why validation has to bound the amounts (`TokenPreProgrammedDistribution::
    /// validate_amounts`): a release is a sum tree, so GroveDB refuses one whose amounts each fit
    /// an `i64` but total more. The cost estimation does not notice, only the write does.
    #[test]
    fn should_fail_to_store_a_release_whose_amounts_total_over_i64_max() {
        let platform_version = PlatformVersion::latest();

        let mut configuration = token_releasing_at_the_shared_time();
        configuration
            .distribution_rules_mut()
            .set_pre_programmed_distribution(Some(TokenPreProgrammedDistribution::V0(
                TokenPreProgrammedDistributionV0 {
                    distributions: BTreeMap::from([(
                        RELEASE_TIME,
                        BTreeMap::from([
                            (Identifier::from([1; 32]), i64::MAX as TokenAmount),
                            (Identifier::from([2; 32]), 1),
                        ]),
                    )]),
                },
            )));
        let mut contract = contract_without_tokens(0, platform_version);
        contract.set_tokens(BTreeMap::from([(0, configuration)]));

        let drive = setup_drive_with_initial_state_structure(None);

        insert_contract(&drive, &contract, false, platform_version)
            .expect("expected the estimation to succeed");

        let error = insert_contract(&drive, &contract, true, platform_version)
            .expect_err("expected the sum tree to overflow");

        assert!(
            matches!(&error, Error::GroveDB(grove_error) if grove_error.to_string().contains("sum is overflowing")),
            "unexpected error: {error:?}"
        );
    }
}
