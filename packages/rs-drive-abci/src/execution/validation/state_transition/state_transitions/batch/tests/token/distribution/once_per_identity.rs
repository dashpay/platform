use super::*;
use crate::execution::validation::state_transition::tests::{
    create_token_contract_with_owner_identity, setup_identity,
};
use crate::test::helpers::setup::TestPlatformBuilder;
use dpp::dash_to_credits;
use dpp::data_contract::TokenConfiguration;
use dpp::state_transition::batch_transition::BatchTransition;
use platform_version::version::PlatformVersion;
use rand::prelude::StdRng;

/// A once-per-identity distribution lets any identity claim a fixed amount exactly once. The
/// claimant is whoever signs the claim; the second claim by the same identity is rejected, and
/// the total paid out is bounded only by the token's max supply.
mod once_per_identity_distribution {
    use super::*;
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::balances::credits::TokenAmount;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::{
        TokenConfigurationV0Getters, TokenConfigurationV0Setters,
    };
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v1::TokenDistributionRulesV1Setters;
    use dpp::data_contract::associated_token::token_once_per_identity_distribution::v0::TokenOncePerIdentityDistributionV0;
    use dpp::data_contract::associated_token::token_once_per_identity_distribution::TokenOncePerIdentityDistribution;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::Identifier;
    use dpp::prelude::DataContract;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use platform_version::version::PlatformVersion;
    use simple_signer::signer::SimpleSigner;

    const CLAIM_AMOUNT: TokenAmount = 1000;

    fn once_per_identity(amount: TokenAmount) -> TokenOncePerIdentityDistribution {
        TokenOncePerIdentityDistribution::V0(TokenOncePerIdentityDistributionV0 { amount })
    }

    fn block_info(time_ms: u64, height: u64) -> BlockInfo {
        BlockInfo {
            time_ms,
            height,
            core_height: 42,
            epoch: Epoch::new(1).unwrap(),
        }
    }

    /// Submits a once-per-identity claim and returns its execution result. The transaction is
    /// committed, so a successful claim is visible to the next one.
    #[allow(clippy::too_many_arguments)]
    async fn claim(
        platform: &TempPlatform<MockCoreRPCLike>,
        token_id: Identifier,
        contract: &DataContract,
        identity: &Identity,
        key: &IdentityPublicKey,
        signer: &SimpleSigner,
        nonce: u64,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> StateTransitionExecutionResult {
        let platform_state = platform.state.load();

        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            TokenDistributionType::OncePerIdentity,
            None,
            key,
            nonce,
            0,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("expect to create claim transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected serialized claim transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition],
                &platform_state,
                block_info,
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        processing_result
            .execution_results()
            .first()
            .expect("expected one execution result")
            .clone()
    }

    fn token_balance(
        platform: &TempPlatform<MockCoreRPCLike>,
        token_id: Identifier,
        identity_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Option<TokenAmount> {
        platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity_id.to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance")
    }

    #[tokio::test]
    async fn test_token_once_per_identity_distribution_any_identity_claims_once() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);

        let (owner, owner_signer, owner_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (identity_2, signer_2, key_2) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_once_per_identity_distribution(Some(once_per_identity(CLAIM_AMOUNT)));
            }),
            None,
            None,
            None,
            platform_version,
        );

        // Nobody has claimed yet.
        assert_eq!(
            platform
                .drive
                .fetch_once_per_identity_distribution_claim(
                    token_id.to_buffer(),
                    identity_2.id(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch the claim"),
            None
        );

        // An identity unrelated to the contract claims the fixed amount.
        let result = claim(
            &platform,
            token_id,
            &contract,
            &identity_2,
            &key_2,
            &signer_2,
            2,
            &block_info(100, 41),
            platform_version,
        )
        .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(
            token_balance(&platform, token_id, identity_2.id(), platform_version),
            Some(CLAIM_AMOUNT)
        );
        assert_eq!(
            platform
                .drive
                .fetch_once_per_identity_distribution_claim(
                    token_id.to_buffer(),
                    identity_2.id(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch the claim"),
            Some(100)
        );

        // The second claim by the same identity is rejected and pays, and the balance stays put.
        let result = claim(
            &platform,
            token_id,
            &contract,
            &identity_2,
            &key_2,
            &signer_2,
            3,
            &block_info(200_000, 42),
            platform_version,
        )
        .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::TokenOncePerIdentityDistributionAlreadyClaimedError(error)
                ),
                ..
            } if error.token_id() == token_id
                && error.identity_id() == identity_2.id()
                && error.claimed_at_ms() == 100
        );
        assert_eq!(
            token_balance(&platform, token_id, identity_2.id(), platform_version),
            Some(CLAIM_AMOUNT)
        );

        // The contract owner is an identity like any other: it claims once too, on top of the
        // base supply it received at registration.
        let owner_balance_before = token_balance(&platform, token_id, owner.id(), platform_version)
            .expect("expected the owner to hold the base supply");
        let result = claim(
            &platform,
            token_id,
            &contract,
            &owner,
            &owner_key,
            &owner_signer,
            2,
            &block_info(300_000, 43),
            platform_version,
        )
        .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(
            token_balance(&platform, token_id, owner.id(), platform_version),
            Some(owner_balance_before + CLAIM_AMOUNT)
        );
    }

    #[tokio::test]
    async fn test_token_once_per_identity_distribution_claim_past_max_supply_is_rejected() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);

        let (owner, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (identity_2, signer_2, key_2) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (identity_3, signer_3, key_3) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        // Room for exactly one claim above the base supply.
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .set_max_supply(Some(token_configuration.base_supply() + CLAIM_AMOUNT));
                token_configuration
                    .distribution_rules_mut()
                    .set_once_per_identity_distribution(Some(once_per_identity(CLAIM_AMOUNT)));
            }),
            None,
            None,
            None,
            platform_version,
        );

        let result = claim(
            &platform,
            token_id,
            &contract,
            &identity_2,
            &key_2,
            &signer_2,
            2,
            &block_info(100, 41),
            platform_version,
        )
        .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let result = claim(
            &platform,
            token_id,
            &contract,
            &identity_3,
            &key_3,
            &signer_3,
            2,
            &block_info(200, 42),
            platform_version,
        )
        .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(_)),
                ..
            }
        );
        assert_eq!(
            token_balance(&platform, token_id, identity_3.id(), platform_version),
            None
        );
        // The failed claim is not recorded, so identity 3 could claim if supply were raised.
        assert_eq!(
            platform
                .drive
                .fetch_once_per_identity_distribution_claim(
                    token_id.to_buffer(),
                    identity_3.id(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch the claim"),
            None
        );
    }

    #[tokio::test]
    async fn test_token_once_per_identity_distribution_claim_on_token_without_it_is_rejected() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);

        let (owner, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (identity_2, signer_2, key_2) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            None::<fn(&mut TokenConfiguration)>,
            None,
            None,
            None,
            platform_version,
        );

        let result = claim(
            &platform,
            token_id,
            &contract,
            &identity_2,
            &key_2,
            &signer_2,
            2,
            &block_info(100, 41),
            platform_version,
        )
        .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::InvalidTokenClaimPropertyMismatch(_)),
                ..
            }
        );
        assert_eq!(
            token_balance(&platform, token_id, identity_2.id(), platform_version),
            None
        );
    }

    /// Registers the basic token contract through the data contract create state transition,
    /// with the once-per-identity distribution set to `amount`, and returns the execution
    /// result together with the contract and token id.
    async fn register_token_with_once_per_identity_amount(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        amount: TokenAmount,
        platform_version: &PlatformVersion,
    ) -> (StateTransitionExecutionResult, DataContract, Identifier) {
        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(platform, 958, dash_to_credits!(1.0));

        let mut data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/basic-token/basic-token.json",
            None,
            None,
            false, //no need to validate the data contracts in tests for drive
            platform_version,
        )
        .expect("expected to get json based contract");

        {
            let token_config = data_contract
                .tokens_mut()
                .expect("expected tokens")
                .get_mut(&0)
                .expect("expected first token");
            token_config
                .distribution_rules_mut()
                .set_once_per_identity_distribution(Some(once_per_identity(amount)));
        }

        // The transition constructor derives the same id and owner from the identity and nonce.
        data_contract.set_id(DataContract::generate_data_contract_id_v0(identity.id(), 1));
        data_contract.set_owner_id(identity.id());

        let contract = data_contract.clone();

        let token_id = contract.token_id(0).expect("expected the token id");

        let data_contract_create_transition = DataContractCreateTransition::new_from_data_contract(
            data_contract,
            1,
            &identity.into_partial_identity_info(),
            key.id(),
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expect to create data contract create transition");

        let data_contract_create_serialized_transition = data_contract_create_transition
            .serialize_to_bytes()
            .expect("expected serialized data contract create transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[data_contract_create_serialized_transition],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let result = processing_result
            .execution_results()
            .first()
            .expect("expected one execution result")
            .clone();

        (result, contract, token_id)
    }

    #[tokio::test]
    async fn test_token_once_per_identity_distribution_registers_and_claims_through_state_transitions(
    ) {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let (result, contract, token_id) = register_token_with_once_per_identity_amount(
            &mut platform,
            CLAIM_AMOUNT,
            platform_version,
        )
        .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let mut rng = StdRng::seed_from_u64(1234);
        let (identity_2, signer_2, key_2) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let result = claim(
            &platform,
            token_id,
            &contract,
            &identity_2,
            &key_2,
            &signer_2,
            1,
            &block_info(100, 41),
            platform_version,
        )
        .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(
            token_balance(&platform, token_id, identity_2.id(), platform_version),
            Some(CLAIM_AMOUNT)
        );
    }

    #[tokio::test]
    async fn test_token_once_per_identity_distribution_zero_amount_is_rejected_at_registration() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let (result, _, _) =
            register_token_with_once_per_identity_amount(&mut platform, 0, platform_version).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::BasicError(
                BasicError::InvalidTokenOncePerIdentityDistributionAmountError(_)
            ))
        );
    }

    #[tokio::test]
    async fn test_token_once_per_identity_distribution_amount_above_i64_max_is_rejected_at_registration(
    ) {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let (result, _, _) = register_token_with_once_per_identity_amount(
            &mut platform,
            i64::MAX as u64 + 1,
            platform_version,
        )
        .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::BasicError(
                BasicError::InvalidTokenOncePerIdentityDistributionAmountError(_)
            ))
        );
    }
}
