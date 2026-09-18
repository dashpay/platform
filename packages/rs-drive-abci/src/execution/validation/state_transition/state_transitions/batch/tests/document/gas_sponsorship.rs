//! End-to-end coverage of a contract owner paying the gas of token-paid document actions
//! (protocol version 14). A document type's token cost offers who pays (`gasFeesPaidBy`), the
//! transition's token payment info asks, and `GasFeesPaidBy::resolve` names the payer: the
//! contract owner is charged instead of the signer, an unfunded signer gets through check tx, an
//! insistence the contract owner cannot honour is refused unpaid, a preference falls back to the
//! signer, a request the document type does not offer is a paid rejection, and a sponsored
//! transition that fails is paid for by its signer.

use super::*;

mod gas_sponsorship_tests {
    use super::*;
    use crate::execution::check_tx::CheckTxLevel::FirstTimeCheck;
    use crate::platform_types::platform::PlatformRef;
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult::{
        SuccessfulExecution, UnpaidConsensusError,
    };
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::balances::credits::TokenAmount;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::accessors::v1::DataContractV1Setters;
    use dpp::data_contract::document_type::accessors::{
        DocumentTypeV0MutGetters, DocumentTypeV1Setters,
    };
    use dpp::data_contract::DataContract;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::prelude::Identifier;
    use dpp::state_transition::StateTransition;
    use dpp::tokens::calculate_token_id;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::tokens::token_amount_on_contract_token::{
        DocumentActionTokenCost, DocumentActionTokenEffect,
    };
    use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
    use dpp::tokens::token_payment_info::TokenPaymentInfo;
    use drive::util::test_helpers::setup_contract;
    use simple_signer::signer::SimpleSigner;

    const GAS_FEES_PAID_BY_NOT_ALLOWED: u32 = 40129;
    const GAS_SPONSOR_INSUFFICIENT_BALANCE: u32 = 40222;
    const IDENTITY_INSUFFICIENT_BALANCE: u32 = 40210;
    const IDENTITY_DOES_NOT_HAVE_ENOUGH_TOKEN_BALANCE: u32 = 40700;

    /// Creating a card costs 10 gold, transferred to the contract owner.
    const CARD_COST: TokenAmount = 10;

    struct Sponsorship {
        platform: TempPlatform<MockCoreRPCLike>,
        contract: DataContract,
        gold_token_id: Identifier,
        contract_owner: Identity,
        user: Identity,
        user_signer: SimpleSigner,
        user_key: IdentityPublicKey,
    }

    impl Sponsorship {
        /// A card game whose `card` creation offers `offered` for the gas, a contract owner
        /// with `owner_credits`, and a user with `user_credits` and `user_gold` gold.
        fn new(
            offered: GasFeesPaidBy,
            owner_credits: Credits,
            user_credits: Credits,
            user_gold: TokenAmount,
        ) -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let (contract_owner, _, _) = setup_identity(&mut platform, 958, owner_credits);
            let (user, user_signer, user_key) = setup_identity(&mut platform, 234, user_credits);

            let data_contract_id =
                DataContract::generate_data_contract_id_v0(contract_owner.id(), 1);
            let contract = setup_contract(
                &platform.drive,
                "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency.json",
                Some(data_contract_id.to_buffer()),
                Some(contract_owner.id().to_buffer()),
                Some(|data_contract: &mut DataContract| {
                    data_contract.set_created_at_epoch(Some(0));
                    data_contract.set_created_at(Some(0));
                    data_contract.set_created_at_block_height(Some(0));
                    let document_type = data_contract
                        .document_types_mut()
                        .get_mut("card")
                        .expect("expected the card document type");
                    document_type.set_document_creation_token_cost(Some(DocumentActionTokenCost {
                        contract_id: None,
                        token_contract_position: 0,
                        token_amount: CARD_COST,
                        effect: DocumentActionTokenEffect::TransferTokenToContractOwner,
                        gas_fees_paid_by: offered,
                    }));
                    let offered_int: u8 = offered.into();
                    let schema = document_type.schema_mut();
                    let token_cost = schema
                        .get_mut("tokenCost")
                        .expect("expected to get the token cost")
                        .expect("expected the token cost to be set");
                    let creation_token_cost = token_cost
                        .get_mut("create")
                        .expect("expected to get the creation token cost")
                        .expect("expected the creation token cost to be set");
                    creation_token_cost
                        .set_value("gasFeesPaidBy", offered_int.into())
                        .expect("expected to set who pays the gas");
                }),
                None,
                Some(platform_version),
            );
            let gold_token_id = calculate_token_id(data_contract_id.as_bytes(), 0).into();

            if user_gold > 0 {
                add_tokens_to_identity(&mut platform, gold_token_id, user.id(), user_gold);
            }

            Self {
                platform,
                contract,
                gold_token_id,
                contract_owner,
                user,
                user_signer,
                user_key,
            }
        }

        /// The user's card creation, asking `requested` for the gas
        async fn card_creation(&self, requested: GasFeesPaidBy) -> StateTransition {
            let platform_version = PlatformVersion::latest();
            let mut rng = StdRng::seed_from_u64(433);
            let card_document_type = self
                .contract
                .document_type_for_name("card")
                .expect("expected the card document type");
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut document = card_document_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    self.user.id(),
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");
            document.set("attack", 4.into());
            document.set("defense", 7.into());

            BatchTransition::new_document_creation_transition_from_document(
                document,
                card_document_type,
                entropy.0,
                &self.user_key,
                2,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 0,
                    minimum_token_cost: None,
                    maximum_token_cost: Some(CARD_COST),
                    gas_fees_paid_by: requested,
                })),
                &self.user_signer,
                platform_version,
                None,
            )
            .await
            .expect("expected a batch transition")
        }

        fn process(
            &self,
            transition: &StateTransition,
            tx: &drive::grovedb::Transaction,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let state = self.platform.state.load();
            let result = self
                .platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition
                        .serialize_to_bytes()
                        .expect("expected to serialize")],
                    &state,
                    &BlockInfo::default(),
                    tx,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process the state transition");
            result.execution_results()[0].clone()
        }

        fn check_tx(&self, transition: &StateTransition) -> Vec<u32> {
            let platform_version = PlatformVersion::latest();
            let state = self.platform.state.load();
            let platform_ref = PlatformRef {
                drive: &self.platform.drive,
                state: &state,
                config: &self.platform.config,
                core_rpc: &self.platform.core_rpc,
            };
            let raw = transition
                .serialize_to_bytes()
                .expect("expected to serialize");
            self.platform
                .check_tx(&raw, FirstTimeCheck, &platform_ref, platform_version)
                .expect("expected to check the transaction")
                .errors
                .iter()
                .map(|error| error.code())
                .collect()
        }

        fn credits(&self, identity: &Identity, tx: &drive::grovedb::Transaction) -> Credits {
            self.platform
                .drive
                .fetch_identity_balance(
                    identity.id().to_buffer(),
                    Some(tx),
                    PlatformVersion::latest(),
                )
                .expect("expected to fetch the balance")
                .expect("expected a balance")
        }

        fn gold(&self, identity: &Identity, tx: &drive::grovedb::Transaction) -> TokenAmount {
            self.platform
                .drive
                .fetch_identity_token_balance(
                    self.gold_token_id.to_buffer(),
                    identity.id().to_buffer(),
                    Some(tx),
                    PlatformVersion::latest(),
                )
                .expect("expected to fetch the token balance")
                .unwrap_or_default()
        }
    }

    fn total_fee(result: &StateTransitionExecutionResult) -> Credits {
        match result {
            SuccessfulExecution { fee_result, .. } => fee_result.total_base_fee(),
            PaidConsensusError { actual_fees, .. } => actual_fees.total_base_fee(),
            other => panic!("expected a paid result, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn should_charge_the_contract_owner_instead_of_an_unfunded_user() {
        let setup = Sponsorship::new(GasFeesPaidBy::ContractOwner, dash_to_credits!(0.1), 0, 15);
        let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        let tx = setup.platform.drive.grove.start_transaction();

        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        let fee = total_fee(&result);
        assert!(fee > 0);
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1) - fee,
            "the contract owner pays the whole fee"
        );
        assert_eq!(setup.credits(&setup.user, &tx), 0, "the user pays nothing");
        // The token payment is unchanged by who pays the gas
        assert_eq!(setup.gold(&setup.user, &tx), 15 - CARD_COST);
        assert_eq!(setup.gold(&setup.contract_owner, &tx), CARD_COST);
    }

    #[tokio::test]
    async fn should_admit_an_unfunded_users_sponsored_creation_in_check_tx() {
        let setup = Sponsorship::new(GasFeesPaidBy::ContractOwner, dash_to_credits!(0.1), 0, 15);
        let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        assert_eq!(setup.check_tx(&transition), Vec::<u32>::new());

        // Without sponsorship the same user is refused before the contract is even loaded
        let transition = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        assert_eq!(
            setup.check_tx(&transition),
            vec![IDENTITY_INSUFFICIENT_BALANCE]
        );
    }

    #[tokio::test]
    async fn should_refuse_an_insistence_the_contract_owner_cannot_honour_unpaid() {
        let setup = Sponsorship::new(GasFeesPaidBy::ContractOwner, 0, dash_to_credits!(0.1), 15);
        let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;

        assert_eq!(
            setup.check_tx(&transition),
            vec![GAS_SPONSOR_INSUFFICIENT_BALANCE],
            "check tx keeps it out of the mempool"
        );

        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);
        assert_matches!(
            result,
            UnpaidConsensusError(error) if error.code() == GAS_SPONSOR_INSUFFICIENT_BALANCE
        );
        assert_eq!(setup.credits(&setup.user, &tx), dash_to_credits!(0.1));
        assert_eq!(setup.credits(&setup.contract_owner, &tx), 0);
        assert_eq!(setup.gold(&setup.user, &tx), 15, "nothing was created");
    }

    #[tokio::test]
    async fn should_fall_back_to_a_preferring_user_when_the_contract_owner_cannot_pay() {
        let setup = Sponsorship::new(GasFeesPaidBy::ContractOwner, 0, dash_to_credits!(0.1), 15);
        let transition = setup
            .card_creation(GasFeesPaidBy::PreferContractOwner)
            .await;
        assert_eq!(setup.check_tx(&transition), Vec::<u32>::new());

        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        let fee = total_fee(&result);
        assert_eq!(setup.credits(&setup.user, &tx), dash_to_credits!(0.1) - fee);
        assert_eq!(setup.credits(&setup.contract_owner, &tx), 0);
        assert_eq!(setup.gold(&setup.contract_owner, &tx), CARD_COST);
    }

    #[tokio::test]
    async fn should_charge_a_preferred_contract_owner_who_can_pay() {
        let setup = Sponsorship::new(
            GasFeesPaidBy::PreferContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
        );
        let transition = setup
            .card_creation(GasFeesPaidBy::PreferContractOwner)
            .await;
        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        let fee = total_fee(&result);
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1) - fee
        );
        assert_eq!(setup.credits(&setup.user, &tx), dash_to_credits!(0.1));
    }

    #[tokio::test]
    async fn should_reject_an_insistence_the_document_type_does_not_offer_as_a_paid_error() {
        for offered in [
            GasFeesPaidBy::DocumentOwner,
            GasFeesPaidBy::PreferContractOwner,
        ] {
            let setup = Sponsorship::new(offered, dash_to_credits!(0.1), dash_to_credits!(0.1), 15);
            let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
            assert_eq!(
                setup.check_tx(&transition),
                vec![GAS_FEES_PAID_BY_NOT_ALLOWED]
            );

            let tx = setup.platform.drive.grove.start_transaction();
            let result = setup.process(&transition, &tx);
            assert_matches!(
                &result,
                PaidConsensusError { error, .. } if error.code() == GAS_FEES_PAID_BY_NOT_ALLOWED
            );
            let fee = total_fee(&result);
            assert!(fee > 0);
            assert_eq!(
                setup.credits(&setup.user, &tx),
                dash_to_credits!(0.1) - fee,
                "the user pays for the rejection"
            );
            assert_eq!(
                setup.credits(&setup.contract_owner, &tx),
                dash_to_credits!(0.1)
            );
            assert_eq!(setup.gold(&setup.user, &tx), 15, "no token moved");
        }
    }

    #[tokio::test]
    async fn should_let_the_user_opt_out_of_an_offer() {
        let setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
        );
        let transition = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        let fee = total_fee(&result);
        assert_eq!(setup.credits(&setup.user, &tx), dash_to_credits!(0.1) - fee);
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1)
        );
    }

    #[tokio::test]
    async fn should_make_the_signer_pay_for_a_sponsored_creation_that_fails() {
        // The user asks the contract owner to pay, and can, but has too little gold: the state
        // check refuses the creation, and the user pays for that work, not the sponsor.
        let setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            CARD_COST - 1,
        );
        let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);

        assert_matches!(
            &result,
            PaidConsensusError { error, .. } if error.code() == IDENTITY_DOES_NOT_HAVE_ENOUGH_TOKEN_BALANCE
        );
        let fee = total_fee(&result);
        assert!(fee > 0);
        assert_eq!(setup.credits(&setup.user, &tx), dash_to_credits!(0.1) - fee);
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1)
        );
    }
}
