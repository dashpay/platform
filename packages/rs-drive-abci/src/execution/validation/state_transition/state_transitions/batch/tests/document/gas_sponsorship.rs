//! End-to-end coverage of a contract owner paying the gas of token-paid document actions
//! (protocol version 14). A document type's token cost offers who pays (`gasFeesPaidBy`), the
//! transition's token payment info asks, and `GasFeesPaidBy::resolve` names the payer: the
//! contract owner is charged instead of the signer, an unfunded signer gets through check tx, an
//! insistence the contract owner cannot honour is refused unpaid, a preference falls back to the
//! signer, a request the document type does not offer is a paid rejection, and a sponsored
//! transition that fails is paid for by its signer.

use super::*;

pub(crate) mod gas_sponsorship_tests {
    use super::*;
    use crate::execution::check_tx::CheckTxLevel;
    use crate::execution::check_tx::CheckTxLevel::{FirstTimeCheck, Recheck};
    use crate::execution::validation::state_transition::tests::setup_identity_without_adding_it;
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
        DocumentTypeV0MutGetters, DocumentTypeV1Getters, DocumentTypeV1Setters,
    };
    use dpp::data_contract::document_type::action_fees::agreement::{
        AgreedFeeMultiplier, DocumentActionFeeAgreement,
    };
    use dpp::data_contract::DataContract;
    use dpp::document::Document;
    use dpp::identity::accessors::IdentitySettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::prelude::{Identifier, Revision};
    use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;
    use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
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
    pub(crate) const GAS_SPONSOR_INSUFFICIENT_BALANCE: u32 = 40222;
    pub(crate) const IDENTITY_INSUFFICIENT_BALANCE: u32 = 40210;
    const IDENTITY_DOES_NOT_HAVE_ENOUGH_TOKEN_BALANCE: u32 = 40700;
    const REQUIRED_TOKEN_PAYMENT_INFO_NOT_SET: u32 = 40115;
    const INVALID_DOCUMENT_REVISION: u32 = 40106;

    /// Creating a card costs 10 gold, transferred to the contract owner.
    const CARD_COST: TokenAmount = 10;

    pub(crate) struct Sponsorship {
        pub(crate) platform: TempPlatform<MockCoreRPCLike>,
        pub(crate) platform_version: &'static PlatformVersion,
        pub(crate) contract: DataContract,
        gold_token_id: Identifier,
        pub(crate) contract_owner: Identity,
        contract_owner_signer: SimpleSigner,
        contract_owner_key: IdentityPublicKey,
        pub(crate) user: Identity,
        pub(crate) user_signer: SimpleSigner,
        pub(crate) user_key: IdentityPublicKey,
    }

    impl Sponsorship {
        /// A card game whose `card` creation and replacement offer `offered` for the gas, a contract owner
        /// with `owner_credits`, and a user with `user_credits` and `user_gold` gold.
        fn new(
            offered: GasFeesPaidBy,
            owner_credits: Credits,
            user_credits: Credits,
            user_gold: TokenAmount,
        ) -> Self {
            Self::build(
                PlatformVersion::latest(),
                offered,
                false,
                owner_credits,
                user_credits,
                user_gold,
                None,
            )
        }

        /// The same game with optional creation and deletion costs: a transition may leave the
        /// token payment out and pay credits instead
        fn with_optional_cost(
            offered: GasFeesPaidBy,
            owner_credits: Credits,
            user_credits: Credits,
            user_gold: TokenAmount,
        ) -> Self {
            Self::build(
                PlatformVersion::latest(),
                offered,
                true,
                owner_credits,
                user_credits,
                user_gold,
                None,
            )
        }

        /// Like `new`, on a chain running `platform_version`, with creation and deletion costs
        /// that are `optional`, and with `user_key_budget` as the budget of the user's signing
        /// key.
        fn build(
            platform_version: &'static PlatformVersion,
            offered: GasFeesPaidBy,
            optional: bool,
            owner_credits: Credits,
            user_credits: Credits,
            user_gold: TokenAmount,
            user_key_budget: Option<Credits>,
        ) -> Self {
            Self::build_customized(
                platform_version,
                offered,
                optional,
                owner_credits,
                user_credits,
                user_gold,
                user_key_budget,
                |_| {},
            )
        }

        /// Like `build`, with `customize` changing the contract after everything else was set
        /// on it and before it is stored.
        #[allow(clippy::too_many_arguments)]
        pub(crate) fn build_customized(
            platform_version: &'static PlatformVersion,
            offered: GasFeesPaidBy,
            optional: bool,
            owner_credits: Credits,
            user_credits: Credits,
            user_gold: TokenAmount,
            user_key_budget: Option<Credits>,
            customize: impl FnOnce(&mut DataContract),
        ) -> Self {
            let mut platform = TestPlatformBuilder::new()
                .with_initial_protocol_version(platform_version.protocol_version)
                .build_with_mock_rpc()
                .set_genesis_state();

            let (contract_owner, contract_owner_signer, contract_owner_key) =
                setup_identity(&mut platform, 958, owner_credits);
            // The transitions are signed with the key as the signer knows it, without limits:
            // validators enforce the ones of the key in Drive.
            let (mut user, user_signer, user_key) =
                setup_identity_without_adding_it(234, user_credits);
            if user_key_budget.is_some() {
                user.add_public_key(user_key.clone().with_limits(user_key_budget, None));
            }
            platform
                .drive
                .add_new_identity(
                    user.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add the user");

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
                        optional,
                    }));
                    let deletion_token_cost = document_type
                        .document_deletion_token_cost()
                        .expect("expected the card game to charge for a deletion");
                    document_type.set_document_deletion_token_cost(Some(
                        DocumentActionTokenCost {
                            optional,
                            ..deletion_token_cost
                        },
                    ));
                    let replacement_token_cost = document_type
                        .document_replacement_token_cost()
                        .expect("expected the card game to charge for a replacement");
                    document_type.set_document_replacement_token_cost(Some(
                        DocumentActionTokenCost {
                            gas_fees_paid_by: offered,
                            ..replacement_token_cost
                        },
                    ));
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
                    if optional {
                        creation_token_cost
                            .set_value("optional", true.into())
                            .expect("expected to mark the creation cost optional");
                        token_cost
                            .get_mut("delete")
                            .expect("expected to get the deletion token cost")
                            .expect("expected the deletion token cost to be set")
                            .set_value("optional", true.into())
                            .expect("expected to mark the deletion cost optional");
                    }
                    token_cost
                        .get_mut("replace")
                        .expect("expected to get the replacement token cost")
                        .expect("expected the replacement token cost to be set")
                        .set_value("gasFeesPaidBy", offered_int.into())
                        .expect("expected to set who pays the gas of a replacement");
                    customize(data_contract);
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
                platform_version,
                contract,
                gold_token_id,
                contract_owner,
                contract_owner_signer,
                contract_owner_key,
                user,
                user_signer,
                user_key,
            }
        }

        /// The user's card creation, asking `requested` for the gas
        pub(crate) async fn card_creation(&self, requested: GasFeesPaidBy) -> StateTransition {
            self.card_creation_by(
                &self.user,
                &self.user_key,
                &self.user_signer,
                Some(requested),
            )
            .await
        }

        /// The user's card creation without any token payment info
        async fn card_creation_without_token_payment(&self) -> StateTransition {
            self.card_creation_by(&self.user, &self.user_key, &self.user_signer, None)
                .await
        }

        /// The user's deletion of the card they created, without any token payment info
        async fn card_deletion_without_token_payment(&self) -> StateTransition {
            let (document, _) = self.card_of(&self.user);
            BatchTransition::new_document_deletion_transition_from_document(
                document,
                self.contract
                    .document_type_for_name("card")
                    .expect("expected the card document type"),
                &self.user_key,
                3,
                0,
                None,
                &self.user_signer,
                self.platform_version,
                None,
            )
            .await
            .expect("expected a batch transition")
        }

        /// The contract owner's own card creation, asking `requested` for the gas
        pub(crate) async fn card_creation_by_the_contract_owner(
            &self,
            requested: GasFeesPaidBy,
        ) -> StateTransition {
            self.card_creation_by(
                &self.contract_owner,
                &self.contract_owner_key,
                &self.contract_owner_signer,
                Some(requested),
            )
            .await
        }

        /// The user's replacement of the card they created, at `revision`, preferring that the
        /// contract owner pays the gas
        async fn card_replacement(&self, revision: Revision) -> StateTransition {
            let (mut document, _) = self.card_of(&self.user);
            document.set("attack", 5.into());
            document.set_revision(Some(revision));

            BatchTransition::new_document_replacement_transition_from_document(
                document,
                self.contract
                    .document_type_for_name("card")
                    .expect("expected the card document type"),
                &self.user_key,
                3,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 1,
                    minimum_token_cost: None,
                    maximum_token_cost: Some(2),
                    gas_fees_paid_by: GasFeesPaidBy::PreferContractOwner,
                })),
                &self.user_signer,
                self.platform_version,
                None,
            )
            .await
            .expect("expected a batch transition")
        }

        /// The card `creator` creates, always the same one, with the entropy of its id
        pub(crate) fn card_of(&self, creator: &Identity) -> (Document, Bytes32) {
            let mut rng = StdRng::seed_from_u64(433);
            let card_document_type = self
                .contract
                .document_type_for_name("card")
                .expect("expected the card document type");
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut document = card_document_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    creator.id(),
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    self.platform_version,
                )
                .expect("expected a random document");
            document.set("attack", 4.into());
            document.set("defense", 7.into());
            // the id commits to the nonce of the creation, which `card_creation_by` sends
            // with the first identity contract nonce
            document
                .set_id_for_creation(card_document_type, &entropy.0, 2, self.platform_version)
                .expect("expected to set the document id");
            (document, entropy)
        }

        pub(crate) async fn card_creation_by(
            &self,
            creator: &Identity,
            key: &IdentityPublicKey,
            signer: &SimpleSigner,
            requested: Option<GasFeesPaidBy>,
        ) -> StateTransition {
            let agreement = self.card_action_fee_agreement(
                DocumentTransitionActionType::Create,
                self.fee_schedule_multiplier(),
            );
            self.card_creation_agreeing_by(creator, key, signer, requested, agreement)
                .await
        }

        /// The fee multiplier of the fee schedule, with no tolerance: what a signer knows on a
        /// chain whose epochs all carry the schedule's multiplier
        pub(crate) fn fee_schedule_multiplier(&self) -> AgreedFeeMultiplier {
            AgreedFeeMultiplier {
                known_permille: self
                    .platform_version
                    .fee_version
                    .uses_version_fee_multiplier_permille
                    .expect("expected the fee schedule to set a multiplier"),
                increase_tolerance_percent: 0,
            }
        }

        /// What `action` on a card must agree to pay in action fees, read off the contract the
        /// harness holds; `None` when the card type charges nothing for it
        pub(crate) fn card_action_fee_agreement(
            &self,
            action: DocumentTransitionActionType,
            fee_multiplier: AgreedFeeMultiplier,
        ) -> Option<DocumentActionFeeAgreement> {
            DocumentActionFeeAgreement::for_document_type_action(
                self.contract
                    .document_type_for_name("card")
                    .expect("expected the card document type"),
                action,
                fee_multiplier,
            )
        }

        /// The user's card creation asking `requested` for the gas and carrying `agreement`
        pub(crate) async fn card_creation_agreeing(
            &self,
            requested: GasFeesPaidBy,
            agreement: Option<DocumentActionFeeAgreement>,
        ) -> StateTransition {
            self.card_creation_agreeing_by(
                &self.user,
                &self.user_key,
                &self.user_signer,
                Some(requested),
                agreement,
            )
            .await
        }

        /// `creator`'s card creation carrying `agreement` as its action fee agreement
        pub(crate) async fn card_creation_agreeing_by(
            &self,
            creator: &Identity,
            key: &IdentityPublicKey,
            signer: &SimpleSigner,
            requested: Option<GasFeesPaidBy>,
            agreement: Option<DocumentActionFeeAgreement>,
        ) -> StateTransition {
            let (document, entropy) = self.card_of(creator);

            BatchTransition::new_document_creation_transition_from_document(
                document,
                self.contract
                    .document_type_for_name("card")
                    .expect("expected the card document type"),
                entropy.0,
                key,
                2,
                0,
                requested.map(|gas_fees_paid_by| {
                    TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                        payment_token_contract_id: None,
                        token_contract_position: 0,
                        minimum_token_cost: None,
                        maximum_token_cost: Some(CARD_COST),
                        gas_fees_paid_by,
                    })
                }),
                signer,
                self.platform_version,
                Some(StateTransitionCreationOptions {
                    action_fee_agreement: agreement,
                    ..Default::default()
                }),
            )
            .await
            .expect("expected a batch transition")
        }

        pub(crate) fn process(
            &self,
            transition: &StateTransition,
            tx: &drive::grovedb::Transaction,
        ) -> StateTransitionExecutionResult {
            self.process_in(transition, &BlockInfo::default(), tx)
        }

        /// `process` in the block `block_info` describes
        pub(crate) fn process_in(
            &self,
            transition: &StateTransition,
            block_info: &BlockInfo,
            tx: &drive::grovedb::Transaction,
        ) -> StateTransitionExecutionResult {
            let platform_version = self.platform_version;
            let state = self.platform.state.load();
            let result = self
                .platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition
                        .serialize_to_bytes()
                        .expect("expected to serialize")],
                    &state,
                    block_info,
                    tx,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process the state transition");
            result.execution_results()[0].clone()
        }

        pub(crate) fn check_tx(&self, transition: &StateTransition) -> Vec<u32> {
            self.check_tx_at(transition, FirstTimeCheck)
        }

        pub(crate) fn check_tx_at(
            &self,
            transition: &StateTransition,
            level: CheckTxLevel,
        ) -> Vec<u32> {
            let platform_version = self.platform_version;
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
                .check_tx(&raw, level, &platform_ref, platform_version)
                .expect("expected to check the transaction")
                .errors
                .iter()
                .map(|error| error.code())
                .collect()
        }

        pub(crate) fn credits(
            &self,
            identity: &Identity,
            tx: &drive::grovedb::Transaction,
        ) -> Credits {
            self.platform
                .drive
                .fetch_identity_balance(identity.id().to_buffer(), Some(tx), self.platform_version)
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
                    self.platform_version,
                )
                .expect("expected to fetch the token balance")
                .unwrap_or_default()
        }
    }

    pub(crate) fn total_fee(result: &StateTransitionExecutionResult) -> Credits {
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

    #[tokio::test]
    async fn should_keep_an_unfunded_users_failing_sponsored_creation_out_of_the_mempool() {
        // The user has no credits and too little gold. A failed batch is never sponsored, so
        // nobody could be charged for this one: check tx validates it against the state in
        // full, as it does a masternode vote, instead of leaving the failure to a proposer.
        let setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            0,
            CARD_COST - 1,
        );
        let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        assert_eq!(
            setup.check_tx(&transition),
            vec![IDENTITY_DOES_NOT_HAVE_ENOUGH_TOKEN_BALANCE]
        );

        // A user who can pay for their own failure is admitted as before, and pays for it in
        // the block (`should_make_the_signer_pay_for_a_sponsored_creation_that_fails`).
        let setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            CARD_COST - 1,
        );
        let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        assert_eq!(setup.check_tx(&transition), Vec::<u32>::new());
    }

    #[tokio::test]
    async fn should_keep_an_unfunded_users_sponsored_creation_on_recheck() {
        let setup = Sponsorship::new(GasFeesPaidBy::ContractOwner, dash_to_credits!(0.1), 0, 15);
        let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        assert_eq!(setup.check_tx_at(&transition, Recheck), Vec::<u32>::new());
    }

    #[tokio::test]
    async fn should_drop_an_unfunded_users_sponsored_creation_on_recheck_once_its_gold_is_spent() {
        // Admitted while the user held the gold, which another of their transitions then spent.
        // Nobody could be charged for the failure in a block, so the recheck validates the state
        // in full as the first time check did, and the batch leaves the mempool.
        let setup = Sponsorship::new(GasFeesPaidBy::ContractOwner, dash_to_credits!(0.1), 0, 15);
        let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        assert_eq!(setup.check_tx(&transition), Vec::<u32>::new());

        setup
            .platform
            .drive
            .remove_from_identity_token_balance(
                setup.gold_token_id.to_buffer(),
                setup.user.id().to_buffer(),
                15,
                &BlockInfo::default(),
                true,
                None,
                setup.platform_version,
                None,
            )
            .expect("expected to spend the gold");

        assert_eq!(
            setup.check_tx_at(&transition, Recheck),
            vec![IDENTITY_DOES_NOT_HAVE_ENOUGH_TOKEN_BALANCE]
        );

        // A user who can pay for their own failure is rechecked as before: the batch stays and
        // they pay for it in the block.
        let setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            CARD_COST - 1,
        );
        let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        assert_eq!(setup.check_tx_at(&transition, Recheck), Vec::<u32>::new());
    }

    #[tokio::test]
    async fn should_keep_an_unfunded_users_replacement_with_a_wrong_revision_out_of_the_mempool() {
        // The revision of a replacement is judged by the transformer, which check tx leaves to
        // the block. A signer who relies on a gas sponsor could not pay for that failure, so
        // check tx asks the transformer to validate against the state too, on the first check
        // and on a recheck, while its validation mode stays what it is.
        let setup = Sponsorship::new(GasFeesPaidBy::ContractOwner, dash_to_credits!(0.1), 0, 15);
        // Replacing a card costs 2 of the game's second token
        let gas_token_id: Identifier = calculate_token_id(setup.contract.id().as_bytes(), 1).into();
        add_tokens_to_identity(&setup.platform, gas_token_id, setup.user.id(), 5);

        let creation = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        let tx = setup.platform.drive.grove.start_transaction();
        assert_matches!(setup.process(&creation, &tx), SuccessfulExecution { .. });
        setup
            .platform
            .drive
            .grove
            .commit_transaction(tx)
            .unwrap()
            .expect("expected to commit the creation");

        for (revision, expected) in [(2, Vec::<u32>::new()), (3, vec![INVALID_DOCUMENT_REVISION])] {
            let replacement = setup.card_replacement(revision).await;
            assert_eq!(setup.check_tx(&replacement), expected, "first check");
            assert_eq!(
                setup.check_tx_at(&replacement, Recheck),
                expected,
                "recheck"
            );
        }
    }

    #[tokio::test]
    async fn should_let_a_contract_owner_who_insists_pay_for_their_own_creation() {
        // The signer is never their own sponsor: the contract owner simply pays as the signer.
        let setup = Sponsorship::new(GasFeesPaidBy::ContractOwner, dash_to_credits!(0.1), 0, 0);
        add_tokens_to_identity(
            &setup.platform,
            setup.gold_token_id,
            setup.contract_owner.id(),
            15,
        );
        let transition = setup
            .card_creation_by_the_contract_owner(GasFeesPaidBy::ContractOwner)
            .await;
        assert_eq!(setup.check_tx(&transition), Vec::<u32>::new());

        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        let fee = total_fee(&result);
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1) - fee
        );
    }

    #[tokio::test]
    async fn should_not_spend_a_key_budget_on_a_fee_the_contract_owner_paid() {
        let budget = dash_to_credits!(0.05);
        let setup = Sponsorship::build(
            PlatformVersion::latest(),
            GasFeesPaidBy::ContractOwner,
            false,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
            Some(budget),
        );
        let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        let fee = total_fee(&result);
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1) - fee
        );
        assert_eq!(setup.credits(&setup.user, &tx), dash_to_credits!(0.1));
        let remaining_budget = setup
            .platform
            .drive
            .fetch_identity_key_remaining_budget(
                setup.user.id().to_buffer(),
                setup.user_key.id(),
                Some(&tx),
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the remaining budget");
        assert_eq!(
            remaining_budget,
            Some(budget),
            "the creation moved nothing out of the identity, and the sponsor paid the fee"
        );
    }

    #[tokio::test]
    async fn should_ignore_who_is_asked_to_pay_the_gas_at_protocol_version_13() {
        // Up to protocol version 13 both sides of the field were carried and never read: the
        // signer pays whatever the transition asks for and whatever the document type offers.
        let platform_version =
            PlatformVersion::get(13).expect("expected protocol version 13 to exist");
        for offered in [GasFeesPaidBy::ContractOwner, GasFeesPaidBy::DocumentOwner] {
            let setup = Sponsorship::build(
                platform_version,
                offered,
                false,
                dash_to_credits!(0.1),
                dash_to_credits!(0.1),
                15,
                None,
            );
            let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
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
    }

    // ---------- Optional token costs ----------

    #[tokio::test]
    async fn should_let_a_user_skip_an_optional_token_payment_and_pay_credits() {
        // No gold at all: the user leaves the token payment out and pays the gas in credits, as
        // on an action without a token cost. The offer of sponsorship does not apply.
        let setup = Sponsorship::with_optional_cost(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            0,
        );
        let transition = setup.card_creation_without_token_payment().await;
        assert_eq!(setup.check_tx(&transition), Vec::<u32>::new());

        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        let fee = total_fee(&result);
        assert!(fee > 0);
        assert_eq!(setup.credits(&setup.user, &tx), dash_to_credits!(0.1) - fee);
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1)
        );
        assert_eq!(
            setup.gold(&setup.contract_owner, &tx),
            0,
            "no token was paid"
        );
    }

    #[tokio::test]
    async fn should_charge_the_token_and_sponsor_the_gas_when_an_optional_payment_is_supplied() {
        let setup = Sponsorship::with_optional_cost(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            0,
            15,
        );
        let transition = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        let fee = total_fee(&result);
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1) - fee
        );
        assert_eq!(setup.credits(&setup.user, &tx), 0);
        assert_eq!(setup.gold(&setup.user, &tx), 15 - CARD_COST);
        assert_eq!(setup.gold(&setup.contract_owner, &tx), CARD_COST);
    }

    #[tokio::test]
    async fn should_still_require_the_token_payment_on_a_required_cost() {
        let setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
        );
        let transition = setup.card_creation_without_token_payment().await;
        assert_eq!(
            setup.check_tx(&transition),
            vec![REQUIRED_TOKEN_PAYMENT_INFO_NOT_SET]
        );

        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);
        assert_matches!(
            &result,
            PaidConsensusError { error, .. } if error.code() == REQUIRED_TOKEN_PAYMENT_INFO_NOT_SET
        );
        assert_eq!(setup.gold(&setup.user, &tx), 15);
    }

    #[tokio::test]
    async fn should_not_fall_back_to_credits_when_a_supplied_optional_payment_cannot_be_covered() {
        // The client decides between token and credits before signing: a payment it supplied
        // but cannot cover is a rejection, never a silent switch to credits.
        let setup = Sponsorship::with_optional_cost(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            CARD_COST - 1,
        );
        let transition = setup
            .card_creation(GasFeesPaidBy::PreferContractOwner)
            .await;
        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);

        assert_matches!(
            &result,
            PaidConsensusError { error, .. } if error.code() == IDENTITY_DOES_NOT_HAVE_ENOUGH_TOKEN_BALANCE
        );
        assert_eq!(setup.gold(&setup.user, &tx), CARD_COST - 1);
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1)
        );
    }

    #[tokio::test]
    async fn should_let_a_user_skip_an_optional_token_payment_on_a_deletion_too() {
        // The waiver is the same for every action: the user creates a card and deletes it
        // without paying a token for either, on their own credits.
        let setup = Sponsorship::with_optional_cost(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            0,
        );
        let creation = setup.card_creation_without_token_payment().await;
        let tx = setup.platform.drive.grove.start_transaction();
        assert_matches!(setup.process(&creation, &tx), SuccessfulExecution { .. });
        setup
            .platform
            .drive
            .grove
            .commit_transaction(tx)
            .unwrap()
            .expect("expected to commit the creation");

        let deletion = setup.card_deletion_without_token_payment().await;
        assert_eq!(setup.check_tx(&deletion), Vec::<u32>::new());
        let tx = setup.platform.drive.grove.start_transaction();
        assert_matches!(setup.process(&deletion, &tx), SuccessfulExecution { .. });
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1),
            "the contract owner pays for neither"
        );

        // A required deletion cost still asks for the payment
        let setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
        );
        let creation = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        let tx = setup.platform.drive.grove.start_transaction();
        assert_matches!(setup.process(&creation, &tx), SuccessfulExecution { .. });
        setup
            .platform
            .drive
            .grove
            .commit_transaction(tx)
            .unwrap()
            .expect("expected to commit the creation");
        let deletion = setup.card_deletion_without_token_payment().await;
        assert_eq!(
            setup.check_tx(&deletion),
            vec![REQUIRED_TOKEN_PAYMENT_INFO_NOT_SET]
        );
    }
}
