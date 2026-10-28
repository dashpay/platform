//! A token configuration update on a contract registered read-only.

use super::*;
use crate::execution::check_tx::CheckTxLevel;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v0::DataContractV0Setters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::config::v0::DataContractConfigSettersV0;
use dpp::data_contract::DataContract;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::prelude::IdentityNonce;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::tests::json_document::json_document_to_contract_with_ids;
use dpp::tokens::calculate_token_id;
use dpp::version::PlatformVersion;
use rand::Rng;
use rand::SeedableRng;
use simple_signer::signer::SimpleSigner;

/// A platform where an identity registered the basic token contract read-only, through a
/// contract create, with the token's max supply changeable by the contract owner.
struct ReadonlyTokenContract {
    platform: TempPlatform<MockCoreRPCLike>,
    identity: Identity,
    signer: SimpleSigner,
    key: IdentityPublicKey,
    contract_id: Identifier,
    token_id: Identifier,
}

impl ReadonlyTokenContract {
    async fn register(platform_version: &PlatformVersion) -> Self {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();
        let mut rng = StdRng::seed_from_u64(77031);
        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));

        let mut data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/basic-token/basic-token.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");
        data_contract.config_mut().set_readonly(true);
        data_contract
            .token_configuration_mut(0)
            .expect("expected token 0")
            .set_max_supply_change_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                admin_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }));

        let contract_id = DataContract::generate_data_contract_id_v0(identity.id(), 1);
        let token_id = calculate_token_id(contract_id.as_bytes(), 0).into();

        let create = DataContractCreateTransition::new_from_data_contract(
            data_contract,
            1,
            &identity.clone().into_partial_identity_info(),
            key.id(),
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected contract create transition");

        let registered = Self {
            platform,
            identity,
            signer,
            key,
            contract_id,
            token_id,
        };
        let result = registered.process(
            create.serialize_to_bytes().expect("serialize create"),
            platform_version,
        );
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }],
            "registration accepts a read-only contract with a token"
        );
        assert!(registered
            .stored_contract(platform_version)
            .config()
            .readonly());
        registered
    }

    /// A max supply update by the contract owner, signed with the next contract nonce.
    async fn max_supply_update(&self, platform_version: &PlatformVersion) -> Vec<u8> {
        BatchTransition::new_token_config_update_transition(
            self.token_id,
            self.identity.id(),
            self.contract_id,
            0,
            TokenConfigurationChangeItem::MaxSupply(Some(1_000_000)),
            None,
            None,
            &self.key,
            2,
            0,
            &self.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected config update transition")
        .serialize_to_bytes()
        .expect("serialize config update")
    }

    /// Processes `raw` as a validator does a block holding it, and commits the result.
    fn process(
        &self,
        raw: Vec<u8>,
        platform_version: &PlatformVersion,
    ) -> StateTransitionsProcessingResult {
        let platform_state = self.platform.state.load();
        let transaction = self.platform.drive.grove.start_transaction();
        let result = self
            .platform
            .platform
            .process_raw_state_transitions(
                &[raw],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process the state transition");
        self.platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");
        result
    }

    fn check_tx_errors(
        &self,
        raw: &[u8],
        platform_version: &PlatformVersion,
    ) -> Vec<ConsensusError> {
        let platform_state = self.platform.state.load();
        let platform_ref = PlatformRef {
            drive: &self.platform.drive,
            state: &platform_state,
            config: &self.platform.config,
            core_rpc: &self.platform.core_rpc,
        };
        self.platform
            .check_tx(
                raw,
                CheckTxLevel::FirstTimeCheck,
                &platform_ref,
                platform_version,
            )
            .expect("expected check_tx to run")
            .errors
    }

    fn stored_contract(&self, platform_version: &PlatformVersion) -> DataContract {
        self.platform
            .drive
            .fetch_contract(
                self.contract_id.to_buffer(),
                None,
                None,
                None,
                platform_version,
            )
            .unwrap()
            .expect("expected to fetch contract")
            .expect("expected contract")
            .contract
            .clone()
    }

    fn max_supply(&self, platform_version: &PlatformVersion) -> Option<u64> {
        self.stored_contract(platform_version)
            .expected_token_configuration(0)
            .expect("token 0")
            .max_supply()
    }

    fn balance(&self, platform_version: &PlatformVersion) -> Credits {
        self.platform
            .drive
            .fetch_identity_balance(self.identity.id().to_buffer(), None, platform_version)
            .expect("expected to fetch balance")
            .expect("expected a balance")
    }

    fn contract_nonce(&self, platform_version: &PlatformVersion) -> Option<IdentityNonce> {
        self.platform
            .drive
            .fetch_identity_contract_nonce(
                self.identity.id().to_buffer(),
                self.contract_id.to_buffer(),
                true,
                None,
                platform_version,
            )
            .expect("expected to fetch the contract nonce")
    }
}

#[tokio::test]
async fn should_refuse_a_token_config_update_on_a_readonly_contract_charged() {
    let platform_version = PlatformVersion::latest();
    let setup = ReadonlyTokenContract::register(platform_version).await;
    let raw = setup.max_supply_update(platform_version).await;

    assert_matches!(
        setup.check_tx_errors(&raw, platform_version).as_slice(),
        [ConsensusError::StateError(StateError::DataContractIsReadonlyError(error))]
            if *error.data_contract_id() == setup.contract_id
    );

    let balance_before = setup.balance(platform_version);
    let result = setup.process(raw, platform_version);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::DataContractIsReadonlyError(error)),
            ..
        }] if *error.data_contract_id() == setup.contract_id
    );

    assert!(setup.balance(platform_version) < balance_before);
    assert_eq!(setup.contract_nonce(platform_version), Some(2));
    assert_eq!(setup.max_supply(platform_version), None);
}

#[tokio::test]
async fn should_leave_a_token_config_update_on_a_readonly_contract_out_of_the_block_at_protocol_version_14(
) {
    let platform_version = PlatformVersion::get(14).expect("protocol version 14");
    let setup = ReadonlyTokenContract::register(platform_version).await;
    let raw = setup.max_supply_update(platform_version).await;

    let balance_before = setup.balance(platform_version);
    let result = setup.process(raw, platform_version);
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::InternalError(message)]
            if message.contains("contract is readonly")
    );

    assert_eq!(setup.balance(platform_version), balance_before);
    assert_eq!(setup.contract_nonce(platform_version), Some(1));
    assert_eq!(setup.max_supply(platform_version), None);
}
