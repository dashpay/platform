pub(crate) mod address_funds;
mod data_contract_based_queries;
mod document_history;
mod document_query;
mod group_queries;
mod identity_based_queries;
mod prefunded_specialized_balances;
mod proofs;
mod response_metadata;
mod service;
mod shielded;
mod system;
mod token_queries;
mod validator_queries;
mod voting;

use crate::error::query::QueryError;

use dpp::validation::ValidationResult;

pub use service::QueryService;

/// A query validation result
pub type QueryValidationResult<TData> = ValidationResult<TData, QueryError>;

#[cfg(test)]
pub(crate) mod tests {
    use crate::error::query::QueryError;
    use crate::platform_types::platform::Platform;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::query::QueryValidationResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::DataContract;

    use crate::config::PlatformConfig;
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::DocumentTypeRef;
    use dpp::document::Document;
    use dpp::prelude::{CoreBlockHeight, TimestampMillis};
    use drive::util::batch::DriveOperation::{DataContractOperation, DocumentOperation};
    use drive::util::batch::{DataContractOperationType, DocumentOperationType};
    use drive::util::object_size_info::{
        DataContractInfo, DocumentInfo, DocumentTypeInfo, OwnedDocumentInfo,
    };
    use drive::util::storage_flags::StorageFlags;
    use platform_version::version::{PlatformVersion, ProtocolVersion};
    use std::borrow::Cow;
    use std::sync::Arc;

    pub fn setup_platform<'a>(
        with_genesis_state: Option<(TimestampMillis, CoreBlockHeight)>,
        network: Network,
        initial_protocol_version: Option<ProtocolVersion>,
    ) -> (
        TempPlatform<MockCoreRPCLike>,
        Arc<PlatformState>,
        &'a PlatformVersion,
    ) {
        let platform = if let Some((timestamp, activation_core_block_height)) = with_genesis_state {
            let mut platform_builder = TestPlatformBuilder::new()
                .with_config(PlatformConfig::default_for_network(network));

            if let Some(initial_protocol_version) = initial_protocol_version {
                platform_builder =
                    platform_builder.with_initial_protocol_version(initial_protocol_version);
            }

            platform_builder
                .build_with_mock_rpc()
                .set_genesis_state_with_activation_info(timestamp, activation_core_block_height)
        } else {
            let mut platform_builder = TestPlatformBuilder::new()
                .with_config(PlatformConfig::default_for_network(network));

            if let Some(initial_protocol_version) = initial_protocol_version {
                platform_builder =
                    platform_builder.with_initial_protocol_version(initial_protocol_version);
            }

            platform_builder
                .build_with_mock_rpc()
                .set_initial_state_structure()
        };

        // We can't return a reference to Arc (`load` method) so we clone Arc (`load_full`).
        // This is a bit slower, but we don't care since we are in test environment
        let platform_state = platform.platform.state.load_full();

        let platform_version = platform_state.current_platform_version().unwrap();

        (platform, platform_state, platform_version)
    }

    pub fn store_data_contract(
        platform: &Platform<MockCoreRPCLike>,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) {
        let operation = DataContractOperation(DataContractOperationType::ApplyContract {
            contract: Cow::Owned(data_contract.to_owned()),
            storage_flags: None,
        });

        let block_info = BlockInfo::genesis();

        platform
            .drive
            .apply_drive_operations(
                vec![operation],
                true,
                &block_info,
                None,
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");
    }

    pub fn store_document(
        platform: &Platform<MockCoreRPCLike>,
        data_contract: &DataContract,
        document_type: DocumentTypeRef,
        document: &Document,
        platform_version: &PlatformVersion,
    ) {
        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        let operation = DocumentOperation(DocumentOperationType::AddDocument {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentInfo::DocumentRefInfo((document, storage_flags)),
                owner_id: None,
            },
            contract_info: DataContractInfo::BorrowedDataContract(data_contract),
            document_type_info: DocumentTypeInfo::DocumentTypeRef(document_type),
            override_document: false,
        });

        let block_info = BlockInfo::genesis();

        platform
            .drive
            .apply_drive_operations(
                vec![operation],
                true,
                &block_info,
                None,
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");
    }

    pub fn assert_invalid_identifier<TData: Clone>(
        validation_result: QueryValidationResult<TData>,
    ) {
        assert!(matches!(
            validation_result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("id must be a valid identifier (32 bytes long)")
        ));
    }

    /// Set up a platform with token state for testing token queries.
    ///
    /// Creates 3 identities and a data contract with 3 tokens.
    /// Token 0: minted 100 to identity 1, 100 to identity 2. Identity 2 frozen on token 0.
    /// Token 1: minted 200 to identity 1, 100 to identity 3. Token 1 is paused.
    /// Token 2: minted 300 to identity 1, 200 to identity 2. Has pre-programmed distributions.
    ///
    /// Returns (platform, state, version, contract_id, [token_id_0, token_id_1, token_id_2],
    ///          [identity_id_1, identity_id_2, identity_id_3])
    #[allow(clippy::type_complexity)]
    pub fn setup_platform_with_token_state<'a>() -> (
        TempPlatform<MockCoreRPCLike>,
        Arc<PlatformState>,
        &'a PlatformVersion,
        [u8; 32],
        [[u8; 32]; 3],
        [[u8; 32]; 3],
    ) {
        use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
        use dpp::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
        use dpp::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
        use dpp::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
        use dpp::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
        use dpp::data_contract::associated_token::token_marketplace_rules::v0::TokenMarketplaceRulesV0;
        use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
        use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
        use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
        use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
        use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
        use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
        use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
        use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
        use dpp::data_contract::change_control_rules::v0::ChangeControlRulesV0;
        use dpp::data_contract::config::DataContractConfig;
        use dpp::data_contract::group::v0::GroupV0;
        use dpp::data_contract::group::Group;
        use dpp::data_contract::v1::DataContractV1;
        use dpp::data_contract::{TokenConfiguration, INITIAL_DATA_CONTRACT_VERSION};
        use dpp::identifier::Identifier;
        use dpp::identity::accessors::IdentitySettersV0;
        use dpp::identity::Identity;
        use dpp::prelude::IdentityPublicKey;
        use dpp::tokens::calculate_token_id;
        use dpp::tokens::status::v0::TokenStatusV0;
        use dpp::tokens::status::TokenStatus;
        use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        use std::collections::BTreeMap;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;

        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let identity_id_1 = [1u8; 32];
        let identity_id_2 = [2u8; 32];
        let identity_id_3 = [3u8; 32];
        let contract_id = [3u8; 32];

        let token_id_0 = calculate_token_id(&contract_id, 0);
        let token_id_1 = calculate_token_id(&contract_id, 1);
        let token_id_2 = calculate_token_id(&contract_id, 2);

        // Register identities
        let credits_per_identity: u64 = 10_000_000_000_000;
        let total_credits = credits_per_identity * 3;
        platform
            .platform
            .drive
            .add_to_system_credits(total_credits, None, version)
            .expect("expected to add system credits");

        let mut rng = StdRng::seed_from_u64(0u64);
        let non_unique_key = IdentityPublicKey::random_voting_key_with_rng(11, &mut rng, version)
            .expect("expected to create voting key");

        for id_bytes in [identity_id_1, identity_id_2, identity_id_3] {
            let id = Identifier::new(id_bytes);
            let mut identity =
                Identity::create_basic_identity(id, version).expect("expected to create identity");
            identity.set_balance(credits_per_identity);

            let seed = id_bytes[0];
            let mut rng = StdRng::seed_from_u64(seed as u64);
            let mut keys =
                IdentityPublicKey::main_keys_with_random_authentication_keys_with_private_keys_with_rng(
                    3, &mut rng, version,
                )
                .expect("expected to create keys");

            let transfer_key =
                IdentityPublicKey::random_masternode_transfer_key_with_rng(3, &mut rng, version)
                    .expect("expected to create transfer key");
            keys.push(transfer_key);
            keys.push(non_unique_key.clone());

            identity.set_public_keys(keys.into_iter().map(|(key, _)| (key.id(), key)).collect());

            platform
                .platform
                .drive
                .add_new_identity(identity, false, &BlockInfo::genesis(), true, None, version)
                .expect("expected to add identity");
        }

        // Create data contract with tokens
        let groups = [
            (
                0,
                Group::V0(GroupV0 {
                    members: [
                        (Identifier::new(identity_id_1), 1),
                        (Identifier::new(identity_id_2), 1),
                    ]
                    .into(),
                    required_power: 1,
                }),
            ),
            (
                1,
                Group::V0(GroupV0 {
                    members: [
                        (Identifier::new(identity_id_1), 1),
                        (Identifier::new(identity_id_2), 1),
                        (Identifier::new(identity_id_3), 1),
                    ]
                    .into(),
                    required_power: 3,
                }),
            ),
            (
                2,
                Group::V0(GroupV0 {
                    members: [
                        (Identifier::new(identity_id_1), 1),
                        (Identifier::new(identity_id_3), 1),
                    ]
                    .into(),
                    required_power: 1,
                }),
            ),
        ]
        .into();

        let token_configuration = TokenConfiguration::V0(TokenConfigurationV0 {
            conventions: TokenConfigurationConventionV0 {
                localizations: Default::default(),
                decimals: 8,
            }
            .into(),
            conventions_change_rules: ChangeControlRulesV0::default().into(),
            base_supply: 100000,
            max_supply: None,
            keeps_history: TokenKeepsHistoryRulesV0::default().into(),
            start_as_paused: false,
            allow_transfer_to_frozen_balance: true,
            max_supply_change_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                admin_action_takers: Default::default(),
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            distribution_rules: TokenDistributionRulesV0 {
                perpetual_distribution: Some(TokenPerpetualDistribution::V0(
                    TokenPerpetualDistributionV0 {
                        distribution_type: RewardDistributionType::BlockBasedDistribution {
                            interval: 10,
                            function: DistributionFunction::FixedAmount { amount: 100 },
                        },
                        distribution_recipient: TokenDistributionRecipient::ContractOwner,
                    },
                )),
                perpetual_distribution_rules: ChangeControlRulesV0::default().into(),
                pre_programmed_distribution: None,
                new_tokens_destination_identity: None,
                new_tokens_destination_identity_rules: ChangeControlRulesV0::default().into(),
                minting_allow_choosing_destination: true,
                minting_allow_choosing_destination_rules: ChangeControlRulesV0::default().into(),
                change_direct_purchase_pricing_rules: ChangeControlRulesV0 {
                    authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                    admin_action_takers: Default::default(),
                    changing_authorized_action_takers_to_no_one_allowed: false,
                    changing_admin_action_takers_to_no_one_allowed: false,
                    self_changing_admin_action_takers_allowed: false,
                }
                .into(),
            }
            .into(),
            marketplace_rules: TokenMarketplaceRulesV0 {
                trade_mode: Default::default(),
                trade_mode_change_rules: ChangeControlRulesV0::default().into(),
            }
            .into(),
            manual_minting_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::Group(0),
                admin_action_takers: Default::default(),
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            manual_burning_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::Group(2),
                admin_action_takers: Default::default(),
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            freeze_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                admin_action_takers: Default::default(),
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            unfreeze_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                admin_action_takers: Default::default(),
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            destroy_frozen_funds_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                admin_action_takers: Default::default(),
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            emergency_action_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                admin_action_takers: Default::default(),
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            main_control_group: None,
            main_control_group_can_be_modified: Default::default(),
            description: Some("Some token description".to_string()),
        });

        // Token 2 gets pre-programmed distributions
        let mut token_configuration_2 = token_configuration.clone();
        if let TokenConfiguration::V0(ref mut cfg) = token_configuration_2 {
            if let TokenDistributionRules::V0(ref mut rules) = cfg.distribution_rules {
                rules.pre_programmed_distribution = Some(TokenPreProgrammedDistribution::V0(
                    TokenPreProgrammedDistributionV0 {
                        distributions: BTreeMap::from([
                            (
                                1000,
                                BTreeMap::from([
                                    (Identifier::new(identity_id_1), 500),
                                    (Identifier::new(identity_id_2), 300),
                                ]),
                            ),
                            (
                                5000,
                                BTreeMap::from([(Identifier::new(identity_id_1), 1000)]),
                            ),
                            (
                                10000,
                                BTreeMap::from([
                                    (Identifier::new(identity_id_2), 750),
                                    (Identifier::new(identity_id_3), 250),
                                ]),
                            ),
                        ]),
                    },
                ));
            }
        }

        let tokens = [
            (0, token_configuration.clone()),
            (1, token_configuration),
            (2, token_configuration_2),
        ]
        .into();

        let data_contract = DataContract::V1(DataContractV1 {
            id: Identifier::new(contract_id),
            config: DataContractConfig::default_for_version(version)
                .expect("expected contract config"),
            version: INITIAL_DATA_CONTRACT_VERSION,
            owner_id: Identifier::new(identity_id_1),
            document_types: Default::default(),
            schema_defs: Default::default(),
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups,
            tokens,
            keywords: vec!["cat".into(), "white".into()],
            description: Some("Some contract description".to_string()),
        });

        let block_info = BlockInfo::genesis();

        platform
            .platform
            .drive
            .apply_contract(&data_contract, block_info, true, None, None, version)
            .expect("expected to apply contract");

        // Mint tokens
        // Token 0: 100 to identity_1, 100 to identity_2
        // Token 1: 200 to identity_1, 100 to identity_3
        // Token 2: 300 to identity_1, 200 to identity_2
        for (token_id, id, amount) in [
            (token_id_0, identity_id_1, 100u64),
            (token_id_0, identity_id_2, 100),
            (token_id_1, identity_id_1, 200),
            (token_id_1, identity_id_3, 100),
            (token_id_2, identity_id_1, 300),
            (token_id_2, identity_id_2, 200),
        ] {
            platform
                .platform
                .drive
                .token_mint(
                    token_id,
                    id,
                    amount,
                    false,
                    false,
                    &block_info,
                    true,
                    None,
                    version,
                )
                .expect("expected to mint tokens");
        }

        // Freeze identity 2 on token 0
        platform
            .platform
            .drive
            .token_freeze(
                Identifier::new(token_id_0),
                Identifier::new(identity_id_2),
                &block_info,
                true,
                None,
                version,
            )
            .expect("expected to freeze");

        // Pause token 1
        let status = TokenStatus::V0(TokenStatusV0 { paused: true });
        platform
            .platform
            .drive
            .token_apply_status(token_id_1, status, &block_info, true, None, version)
            .expect("expected to apply status");

        // Set direct purchase prices
        platform
            .platform
            .drive
            .token_set_direct_purchase_price(
                token_id_1,
                Some(TokenPricingSchedule::SinglePrice(25)),
                &block_info,
                true,
                None,
                version,
            )
            .expect("expected to set direct purchase price");

        let pricing = TokenPricingSchedule::SetPrices(
            (0..=900)
                .step_by(100)
                .map(|amount| (amount, 1000 - amount))
                .collect(),
        );
        platform
            .platform
            .drive
            .token_set_direct_purchase_price(
                token_id_2,
                Some(pricing),
                &block_info,
                true,
                None,
                version,
            )
            .expect("expected to set direct purchase price");

        (
            platform,
            state,
            version,
            contract_id,
            [token_id_0, token_id_1, token_id_2],
            [identity_id_1, identity_id_2, identity_id_3],
        )
    }
}
