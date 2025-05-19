use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Getters;
use dpp::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
use dpp::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
use dpp::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
use dpp::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use dpp::data_contract::config::DataContractConfig;
use dpp::data_contract::group::v0::GroupV0;
use dpp::data_contract::group::Group;
use dpp::data_contract::v1::DataContractV1;
use dpp::data_contract::{TokenConfiguration, INITIAL_DATA_CONTRACT_VERSION};
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::v0::GroupActionV0;
use dpp::group::group_action::GroupAction;
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, KeyID};
use dpp::prelude::*;
use dpp::tokens::calculate_token_id;
use dpp::tokens::status::v0::TokenStatusV0;
use dpp::tokens::status::TokenStatus;
use dpp::tokens::token_event::TokenEvent;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::sync::LazyLock;

const IDENTITY_ID_1: Identifier = Identifier::new([1; 32]);
const IDENTITY_ID_2: Identifier = Identifier::new([2; 32]);
const IDENTITY_ID_3: Identifier = Identifier::new([3; 32]);

const DATA_CONTRACT_ID: Identifier = Identifier::new([3; 32]);

static TOKEN_ID_0: LazyLock<Identifier> =
    LazyLock::new(|| calculate_token_id(&DATA_CONTRACT_ID.to_buffer(), 0).into());

static TOKEN_ID_1: LazyLock<Identifier> =
    LazyLock::new(|| calculate_token_id(&DATA_CONTRACT_ID.to_buffer(), 1).into());

static TOKEN_ID_2: LazyLock<Identifier> =
    LazyLock::new(|| calculate_token_id(&DATA_CONTRACT_ID.to_buffer(), 2).into());

impl<C> Platform<C> {
    /// This data is used for testing token and group queries in RS SDK tests.
    pub(super) fn create_data_for_group_token_queries(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.register_identities(block_info, transaction, platform_version)?;

        self.create_data_contract(block_info, transaction, platform_version)?;

        self.mint_tokens(block_info, transaction, platform_version)?;

        // Freeze tokens for identity 2
        self.drive.token_freeze(
            *TOKEN_ID_0,
            IDENTITY_ID_2,
            block_info,
            true,
            transaction,
            platform_version,
        )?;

        // Pause token 2
        let status = TokenStatus::V0(TokenStatusV0 { paused: true });
        self.drive.token_apply_status(
            TOKEN_ID_1.to_buffer(),
            status,
            block_info,
            true,
            transaction,
            platform_version,
        )?;

        // Add burn token group action
        let action_id = Identifier::new([32; 32]);
        let group_action = GroupAction::V0(GroupActionV0 {
            contract_id: DATA_CONTRACT_ID,
            proposer_id: IDENTITY_ID_1,
            token_contract_position: 2,
            event: GroupActionEvent::TokenEvent(TokenEvent::Burn(
                10,
                IDENTITY_ID_1,
                Some("world on fire".into()),
            )),
        });

        self.drive.add_group_action(
            DATA_CONTRACT_ID,
            2,
            Some(group_action),
            false,
            action_id,
            IDENTITY_ID_1,
            1,
            block_info,
            true,
            transaction,
            platform_version,
        )?;

        Ok(())
    }

    /// Create some test data for token direct prices.
    ///
    /// Define single price pricing for [TOKEN_ID_1], and pricing schedule for [TOKEN_ID_2].
    /// Leave [TOKEN_ID_0] without pricing.
    ///
    /// Tokens must be already created.
    pub(crate) fn create_data_for_token_direct_prices(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive.token_set_direct_purchase_price(
            TOKEN_ID_1.to_buffer(),
            Some(TokenPricingSchedule::SinglePrice(25)),
            block_info,
            true,
            transaction,
            platform_version,
        )?;

        let pricing = TokenPricingSchedule::SetPrices(
            (0..=900)
                .step_by(100)
                .map(|amount| (amount, 1000 - amount))
                .collect(),
        );

        self.drive.token_set_direct_purchase_price(
            TOKEN_ID_2.to_buffer(),
            Some(pricing),
            block_info,
            true,
            transaction,
            platform_version,
        )?;

        Ok(())
    }

    fn register_identities(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut rng = StdRng::seed_from_u64(0u64);
        let non_unique_key =
            IdentityPublicKey::random_voting_key_with_rng(11, &mut rng, platform_version)?;

        for id in [IDENTITY_ID_1, IDENTITY_ID_2, IDENTITY_ID_3] {
            // Create identity without keys
            let mut identity = Identity::create_basic_identity(id, platform_version)?;

            // Generate keys
            let seed = id.to_buffer()[0];
            let mut rng = StdRng::seed_from_u64(seed as u64);
            let mut keys = IdentityPublicKey::main_keys_with_random_authentication_keys_with_private_keys_with_rng(3, &mut rng, platform_version)?;
            // every identity has the same non-unique key
            keys.push(non_unique_key.clone());

            for (key, private_key) in keys.iter() {
                let private_key = hex::encode(private_key);

                tracing::info!(
                    key = ?key,
                    private_key,
                    "Generated random {} key {} for test identity {}", key.purpose(), key.id(), id);
            }

            // Print private keys if necessary
            identity.set_public_keys(
                keys.into_iter()
                    .enumerate()
                    .map(|(i, (key, _private_key))| (i as KeyID, key))
                    .collect(),
            );

            self.drive.add_new_identity(
                identity,
                false,
                block_info,
                true,
                transaction,
                platform_version,
            )?;
        }

        Ok(())
    }

    fn create_data_contract(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DataContract, Error> {
        let groups = [
            (
                0,
                Group::V0(GroupV0 {
                    members: [(IDENTITY_ID_1, 1), (IDENTITY_ID_2, 1)].into(),
                    required_power: 1,
                }),
            ),
            (
                1,
                Group::V0(GroupV0 {
                    members: [(IDENTITY_ID_1, 1), (IDENTITY_ID_2, 1), (IDENTITY_ID_3, 1)].into(),
                    required_power: 3,
                }),
            ),
            (
                2,
                Group::V0(GroupV0 {
                    members: [(IDENTITY_ID_1, 1), (IDENTITY_ID_3, 1)].into(),
                    required_power: 2,
                }),
            ),
        ]
        .into();

        let mut token_configuration = TokenConfiguration::V0(TokenConfigurationV0 {
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
            max_supply_change_rules: ChangeControlRulesV0::default().into(),
            distribution_rules: TokenDistributionRulesV0 {
                perpetual_distribution: None,
                perpetual_distribution_rules: ChangeControlRulesV0::default().into(),
                pre_programmed_distribution: None,
                new_tokens_destination_identity: None,
                new_tokens_destination_identity_rules: ChangeControlRulesV0::default().into(),
                minting_allow_choosing_destination: true,
                minting_allow_choosing_destination_rules: ChangeControlRulesV0::default().into(),
                change_direct_purchase_pricing_rules: ChangeControlRulesV0::default().into(),
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
            freeze_rules: ChangeControlRulesV0::default().into(),
            unfreeze_rules: ChangeControlRulesV0::default().into(),
            destroy_frozen_funds_rules: ChangeControlRulesV0::default().into(),
            emergency_action_rules: ChangeControlRulesV0::default().into(),
            main_control_group: None,
            main_control_group_can_be_modified: Default::default(),
            description: Some("Some token description".to_string()),
        });

        token_configuration
            .conventions_mut()
            .localizations_mut()
            .insert(
                "en".to_string(),
                TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                    should_capitalize: false,
                    singular_form: "cat".to_string(),
                    plural_form: "cats".to_string(),
                }),
            );

        let tokens = [
            (0, token_configuration.clone()),
            (1, token_configuration.clone()),
            (2, token_configuration),
        ]
        .into();

        let data_contract = DataContract::V1(DataContractV1 {
            id: DATA_CONTRACT_ID,
            config: DataContractConfig::default_for_version(platform_version)?,
            version: INITIAL_DATA_CONTRACT_VERSION,
            owner_id: IDENTITY_ID_1,
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

        self.drive.apply_contract(
            &data_contract,
            *block_info,
            true,
            None,
            transaction,
            platform_version,
        )?;

        Ok(data_contract)
    }

    fn mint_tokens(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for (token_id, mint_amount) in [(*TOKEN_ID_0, 100), (*TOKEN_ID_1, 200), (*TOKEN_ID_2, 300)]
        {
            self.drive.token_mint(
                token_id.to_buffer(),
                IDENTITY_ID_1.to_buffer(),
                mint_amount,
                false,
                false,
                block_info,
                true,
                transaction,
                platform_version,
            )?;
        }

        for (token_id, mint_amount) in [(*TOKEN_ID_0, 100), (*TOKEN_ID_2, 200)] {
            self.drive.token_mint(
                token_id.to_buffer(),
                IDENTITY_ID_2.to_buffer(),
                mint_amount,
                false,
                false,
                block_info,
                true,
                transaction,
                platform_version,
            )?;
        }

        #[allow(clippy::single_element_loop)]
        for (token_id, mint_amount) in [(*TOKEN_ID_1, 100)] {
            self.drive.token_mint(
                token_id.to_buffer(),
                IDENTITY_ID_3.to_buffer(),
                mint_amount,
                false,
                false,
                block_info,
                true,
                transaction,
                platform_version,
            )?;
        }

        Ok(())
    }
}
