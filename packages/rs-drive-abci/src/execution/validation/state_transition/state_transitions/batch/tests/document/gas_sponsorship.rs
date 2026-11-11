//! End-to-end coverage of a contract owner paying the gas of token-paid document actions
//! (protocol version 14). A document type's token cost offers who pays (`gasFeesPaidBy`), the
//! transition's token payment info asks, and `GasFeesPaidBy::resolve` names the payer: the
//! contract owner is charged instead of the signer, an unfunded signer gets through check tx, an
//! insistence the contract owner cannot honour is refused unpaid, a preference falls back to the
//! signer, a request the document type does not offer is a paid rejection, a sponsored
//! transition that fails is paid for by its signer, and the fee increase a signer chooses is
//! always the signer's to pay.

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
    use dpp::data_contract::config::moderation::{
        ContractModerationConfig, ContractModerationReason, ContractModerators,
    };
    use dpp::data_contract::document_type::accessors::{
        DocumentTypeV0MutGetters, DocumentTypeV1Getters, DocumentTypeV1Setters,
    };
    use dpp::data_contract::document_type::action_fees::agreement::{
        AgreedFeeMultiplier, DocumentActionFeeAgreement,
    };
    use dpp::data_contract::DataContract;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    use dpp::document::Document;
    use dpp::identity::accessors::IdentitySettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::platform_value;
    use dpp::prelude::{Identifier, IdentityNonce, Revision, UserFeeIncrease};
    use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;
    use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
    use dpp::state_transition::contract_user_moderation_transition::methods::ContractUserModerationTransitionMethodsV0;
    use dpp::state_transition::contract_user_moderation_transition::{
        ContractUserModerationAction, ContractUserModerationTransition,
    };
    use dpp::state_transition::StateTransition;
    use dpp::tokens::calculate_token_id;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::tokens::token_amount_on_contract_token::{
        DocumentActionTokenCost, DocumentActionTokenEffect,
    };
    use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
    use dpp::tokens::token_payment_info::TokenPaymentInfo;
    use dpp::document::DocumentV0Setters;
    use dpp::fee::epoch::{DEFAULT_EPOCHS_PER_ERA, PERPETUAL_STORAGE_ERAS};
    use dpp::fee::fee_result::FeeResult;
    use drive::drive::RootTree;
    use drive::query::DriveDocumentQuery;
    use drive::util::batch::drive_op_batch::{
        DocumentOperation, DocumentOperationsForContractDocumentType, UpdateOperationInfo,
    };
    use drive::util::batch::{DocumentOperationType, DriveOperation};
    use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use drive::util::object_size_info::{DataContractInfo, DocumentTypeInfo, OwnedDocumentInfo};
    use drive::util::storage_flags::StorageFlags;
    use drive::util::test_helpers::setup_contract;
    use simple_signer::signer::SimpleSigner;
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    const GAS_FEES_PAID_BY_NOT_ALLOWED: u32 = 40129;
    pub(crate) const GAS_SPONSOR_INSUFFICIENT_BALANCE: u32 = 40222;
    pub(crate) const IDENTITY_INSUFFICIENT_BALANCE: u32 = 40210;
    const IDENTITY_PUBLIC_KEY_BUDGET_EXCEEDED: u32 = 40218;
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
        pub(crate) contract_owner_signer: SimpleSigner,
        pub(crate) contract_owner_key: IdentityPublicKey,
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
            self.card_deletion_paying(3, None).await
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

        /// A payment of at most `maximum` of the game's second token, which pays for replacing,
        /// deleting and transferring a card, asking `requested` for the gas
        fn game_token_payment(maximum: TokenAmount, requested: GasFeesPaidBy) -> TokenPaymentInfo {
            TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                payment_token_contract_id: None,
                token_contract_position: 1,
                minimum_token_cost: None,
                maximum_token_cost: Some(maximum),
                gas_fees_paid_by: requested,
            })
        }

        /// The user's replacement of the card they created, at `revision`, preferring that the
        /// contract owner pays the gas
        async fn card_replacement(&self, revision: Revision) -> StateTransition {
            let (mut document, _) = self.card_of(&self.user);
            document.set("attack", 5.into());
            document.set_revision(Some(revision));
            self.card_replacement_of(document, 3, GasFeesPaidBy::PreferContractOwner)
                .await
        }

        /// The user's replacement of the card they created, at `revision` and identity contract
        /// `nonce`, giving it a description of `description_length` characters and asking
        /// `requested` for the gas
        async fn card_replacement_describing(
            &self,
            revision: Revision,
            nonce: IdentityNonce,
            description_length: usize,
            requested: GasFeesPaidBy,
        ) -> StateTransition {
            let (mut document, _) = self.card_of(&self.user);
            document.set("description", "d".repeat(description_length).into());
            document.set_revision(Some(revision));
            self.card_replacement_of(document, nonce, requested).await
        }

        /// The user's replacement of their card by `document`, at identity contract `nonce`,
        /// paying its token and asking `requested` for the gas
        async fn card_replacement_of(
            &self,
            document: Document,
            nonce: IdentityNonce,
            requested: GasFeesPaidBy,
        ) -> StateTransition {
            BatchTransition::new_document_replacement_transition_from_document(
                document,
                self.contract
                    .document_type_for_name("card")
                    .expect("expected the card document type"),
                &self.user_key,
                nonce,
                0,
                Some(Self::game_token_payment(2, requested)),
                &self.user_signer,
                self.platform_version,
                None,
            )
            .await
            .expect("expected a batch transition")
        }

        /// The user's deletion of the card they created, at identity contract `nonce`, paying
        /// its token and the gas themselves: the card game offers no sponsorship of a deletion
        async fn card_deletion(&self, nonce: IdentityNonce) -> StateTransition {
            self.card_deletion_paying(
                nonce,
                Some(Self::game_token_payment(1, GasFeesPaidBy::DocumentOwner)),
            )
            .await
        }

        /// The user's deletion of the card they created, at identity contract `nonce`, with
        /// `token_payment_info`
        async fn card_deletion_paying(
            &self,
            nonce: IdentityNonce,
            token_payment_info: Option<TokenPaymentInfo>,
        ) -> StateTransition {
            let (document, _) = self.card_of(&self.user);
            BatchTransition::new_document_deletion_transition_from_document(
                document,
                self.contract
                    .document_type_for_name("card")
                    .expect("expected the card document type"),
                &self.user_key,
                nonce,
                0,
                token_payment_info,
                &self.user_signer,
                self.platform_version,
                None,
            )
            .await
            .expect("expected a batch transition")
        }

        /// Gives the user `amount` of the game's second token, which pays for replacing and
        /// deleting a card
        fn give_the_user_game_tokens(&self, amount: TokenAmount) {
            let token_id: Identifier = calculate_token_id(self.contract.id().as_bytes(), 1).into();
            add_tokens_to_identity(&self.platform, token_id, self.user.id(), amount);
        }

        /// Processes `transition` in a block of its own and commits it
        fn process_and_commit(
            &self,
            transition: &StateTransition,
        ) -> StateTransitionExecutionResult {
            self.process_in_and_commit(transition, &BlockInfo::default())
        }

        /// `process_and_commit` in the block `block_info` describes
        fn process_in_and_commit(
            &self,
            transition: &StateTransition,
            block_info: &BlockInfo,
        ) -> StateTransitionExecutionResult {
            let tx = self.platform.drive.grove.start_transaction();
            let result = self.process_in(transition, block_info, &tx);
            self.platform
                .drive
                .grove
                .commit_transaction(tx)
                .unwrap()
                .expect("expected to commit the transition");
            result
        }

        /// The identity the committed storage flags of the user's card name as its storage owner
        fn stored_card_storage_owner(&self) -> Option<Identifier> {
            let (_, storage_flags) = self.stored_card();
            storage_flags
                .and_then(|storage_flags| storage_flags.owner_id().copied())
                .map(Identifier::from)
        }

        /// The user's card as it is stored, with its storage flags
        fn stored_card(&self) -> (Document, Option<StorageFlags>) {
            let (card, _) = self.card_of(&self.user);
            let query = DriveDocumentQuery::new_primary_key_single_item_query(
                &self.contract,
                self.contract
                    .document_type_for_name("card")
                    .expect("expected the card document type"),
                card.id(),
            );
            let mut documents = self
                .platform
                .drive
                .query_documents_with_flags(
                    query,
                    None,
                    false,
                    None,
                    Some(self.platform_version.protocol_version),
                )
                .expect("expected to query the card")
                .documents_owned();
            documents.pop().expect("expected the card to be stored")
        }

        /// The user's transfer of the card they created to `recipient`, at identity contract
        /// nonce 3, paying its token and the gas themselves
        async fn card_transfer(&self, recipient: Identifier) -> StateTransition {
            let (mut card, _) = self.card_of(&self.user);
            card.increment_revision()
                .expect("expected the revision to increment");
            BatchTransition::new_document_transfer_transition_from_document(
                card,
                self.contract
                    .document_type_for_name("card")
                    .expect("expected the card document type"),
                recipient,
                &self.user_key,
                3,
                0,
                Some(Self::game_token_payment(1, GasFeesPaidBy::DocumentOwner)),
                &self.user_signer,
                self.platform_version,
                None,
            )
            .await
            .expect("expected a batch transition")
        }

        /// The owner the card's entry in its `attack` index names
        fn stored_attack_index_reference_owner(&self) -> Option<Identifier> {
            let (card, _) = self.card_of(&self.user);
            let card_type = self
                .contract
                .document_type_for_name("card")
                .expect("expected the card document type");
            let attack = card_type
                .serialize_value_for_key(
                    "attack",
                    card.get("attack").expect("expected an attack"),
                    self.platform_version,
                )
                .expect("expected the attack to serialize");
            let path: Vec<Vec<u8>> = vec![
                vec![RootTree::DataContractDocuments as u8],
                self.contract.id().to_vec(),
                vec![1],
                b"card".to_vec(),
                b"attack".to_vec(),
                attack,
                vec![0],
            ];
            // Read raw: `get` follows the reference to the card, and would answer with the
            // card's flags
            let element = self
                .platform
                .drive
                .grove
                .get_raw(
                    path.iter()
                        .map(Vec::as_slice)
                        .collect::<Vec<&[u8]>>()
                        .as_slice()
                        .into(),
                    card.id().as_bytes(),
                    None,
                    &self.platform_version.drive.grove_version,
                )
                .unwrap()
                .expect("expected the card's index entry");
            assert!(element.is_reference(), "expected the index entry itself");
            StorageFlags::from_element_flags_ref(element.get_flags().as_ref()?)
                .expect("expected flags that decode")
                .and_then(|flags| flags.owner_id().copied())
                .map(Identifier::from)
        }

        /// The committed credits held in the trees
        fn committed_credits_in_trees(&self) -> Credits {
            let tx = self.platform.drive.grove.start_transaction();
            credits_in_trees_of(&self.platform, &tx, self.platform_version)
        }

        /// The committed balances of the user and of the contract owner
        fn committed_credits(&self) -> (Credits, Credits) {
            let tx = self.platform.drive.grove.start_transaction();
            (
                self.credits(&self.user, &tx),
                self.credits(&self.contract_owner, &tx),
            )
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
            self.card_creation_agreeing_by(creator, key, signer, requested, agreement, 0)
                .await
        }

        /// The user's card creation asking `requested` for the gas and raising its processing
        /// fee by `user_fee_increase` percent
        async fn card_creation_with_fee_increase(
            &self,
            requested: GasFeesPaidBy,
            user_fee_increase: UserFeeIncrease,
        ) -> StateTransition {
            let agreement = self.card_action_fee_agreement(
                DocumentTransitionActionType::Create,
                self.fee_schedule_multiplier(),
            );
            self.card_creation_agreeing_by(
                &self.user,
                &self.user_key,
                &self.user_signer,
                Some(requested),
                agreement,
                user_fee_increase,
            )
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
                0,
            )
            .await
        }

        /// `creator`'s card creation carrying `agreement` as its action fee agreement and
        /// raising its processing fee by `user_fee_increase` percent
        pub(crate) async fn card_creation_agreeing_by(
            &self,
            creator: &Identity,
            key: &IdentityPublicKey,
            signer: &SimpleSigner,
            requested: Option<GasFeesPaidBy>,
            agreement: Option<DocumentActionFeeAgreement>,
            user_fee_increase: UserFeeIncrease,
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
                user_fee_increase,
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
            process_alone(
                &self.platform,
                transition,
                block_info,
                tx,
                self.platform_version,
            )
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
            balance_of(&self.platform, identity, tx, self.platform_version)
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

    /// Processes `transition` as the only state transition of the block `block_info` describes,
    /// in `tx`
    pub(crate) fn process_alone(
        platform: &TempPlatform<MockCoreRPCLike>,
        transition: &StateTransition,
        block_info: &BlockInfo,
        tx: &drive::grovedb::Transaction,
        platform_version: &PlatformVersion,
    ) -> StateTransitionExecutionResult {
        let result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transition
                    .serialize_to_bytes()
                    .expect("expected to serialize")],
                &platform.state.load(),
                block_info,
                tx,
                platform_version,
                false,
                None,
            )
            .expect("expected to process the state transition");
        result.execution_results()[0].clone()
    }

    /// The balance of `identity` in `tx`
    pub(crate) fn balance_of(
        platform: &TempPlatform<MockCoreRPCLike>,
        identity: &Identity,
        tx: &drive::grovedb::Transaction,
        platform_version: &PlatformVersion,
    ) -> Credits {
        platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), Some(tx), platform_version)
            .expect("expected to fetch the balance")
            .expect("expected a balance")
    }

    /// The credits held in the trees in `tx`
    pub(crate) fn credits_in_trees_of(
        platform: &TempPlatform<MockCoreRPCLike>,
        tx: &drive::grovedb::Transaction,
        platform_version: &PlatformVersion,
    ) -> Credits {
        platform
            .drive
            .calculate_total_credits_balance(Some(tx), &platform_version.drive)
            .expect("expected to sum the credits")
            .total_in_trees()
            .expect("expected the credits to add up")
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
    async fn should_make_the_signer_pay_for_duplicate_keys_in_a_sponsored_creation() {
        let version = PlatformVersion::latest();
        let setup = Sponsorship::build_customized(
            version,
            GasFeesPaidBy::ContractOwner,
            false,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
            None,
            |contract| {
                let mut schema = contract
                    .document_type_for_name("card")
                    .unwrap()
                    .schema()
                    .clone();
                let properties = schema.get_mut("properties").unwrap().unwrap();
                let position = properties.as_map().unwrap().len() as u32;
                properties.insert("meta".into(), platform_value!({
                    "type": "object", "position": position,
                    "properties": {"name": {"type": "string", "maxLength": 32, "position": 0}},
                    "required": ["name"], "additionalProperties": false
                })).unwrap();
                contract
                    .set_document_schema("card", schema, true, &mut Vec::new(), version)
                    .unwrap();
            },
        );
        let (mut document, entropy) = setup.card_of(&setup.user);
        document.set(
            "meta",
            Value::Map(vec![
                ("name".into(), "first".into()),
                ("name".into(), "last".into()),
            ]),
        );
        let transition = BatchTransition::new_document_creation_transition_from_document(
            document,
            setup.contract.document_type_for_name("card").unwrap(),
            entropy.0,
            &setup.user_key,
            2,
            0,
            Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                payment_token_contract_id: None,
                token_contract_position: 0,
                minimum_token_cost: None,
                maximum_token_cost: Some(CARD_COST),
                gas_fees_paid_by: GasFeesPaidBy::ContractOwner,
            })),
            &setup.user_signer,
            version,
            None,
        )
        .await
        .unwrap();
        assert_eq!(setup.check_tx(&transition), vec![10103]);
        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);
        assert_matches!(&result, PaidConsensusError {error, ..} if error.code() == 10103);
        let fee = total_fee(&result);
        assert!(fee > 0);
        assert_eq!(setup.credits(&setup.user, &tx), dash_to_credits!(0.1) - fee);
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1)
        );
        assert_eq!(setup.gold(&setup.user, &tx), 15);
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
        setup.give_the_user_game_tokens(5);

        let creation = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        assert_matches!(
            setup.process_and_commit(&creation),
            SuccessfulExecution { .. }
        );

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

    /// The user's card creation insisting on the contract owner for the gas and raising its
    /// processing fee by `user_fee_increase` percent, processed on a fresh chain where the
    /// contract owner holds 0.1 Dash and the user 1 Dash, enough for the largest increase: the
    /// result, and what the contract owner and the user paid
    async fn sponsored_creation_with_fee_increase(
        user_fee_increase: UserFeeIncrease,
    ) -> (StateTransitionExecutionResult, Credits, Credits) {
        let setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(1),
            15,
        );
        let transition = setup
            .card_creation_with_fee_increase(GasFeesPaidBy::ContractOwner, user_fee_increase)
            .await;
        assert_eq!(setup.check_tx(&transition), Vec::<u32>::new());
        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);
        assert_matches!(result, SuccessfulExecution { .. });
        let owner_paid = dash_to_credits!(0.1) - setup.credits(&setup.contract_owner, &tx);
        let user_paid = dash_to_credits!(1) - setup.credits(&setup.user, &tx);
        (result, owner_paid, user_paid)
    }

    #[tokio::test]
    async fn should_charge_the_fee_increase_to_the_signer_who_chose_it_and_not_the_contract_owner()
    {
        let (unraised, owner_paid_unraised, user_paid_unraised) =
            sponsored_creation_with_fee_increase(0).await;
        assert_eq!(user_paid_unraised, 0);
        assert_eq!(owner_paid_unraised, total_fee(&unraised));
        let SuccessfulExecution {
            fee_result: unraised_fee,
            ..
        } = unraised
        else {
            panic!("expected a successful execution, got {unraised:?}");
        };

        let (raised, owner_paid, user_paid) =
            sponsored_creation_with_fee_increase(UserFeeIncrease::MAX).await;
        assert_eq!(
            owner_paid, owner_paid_unraised,
            "the contract owner pays the fee as if no increase was chosen"
        );
        assert_eq!(
            user_paid,
            unraised_fee.processing_fee * UserFeeIncrease::MAX as Credits / 100,
            "the user pays the processing fee its increase adds"
        );
        assert_eq!(
            total_fee(&raised),
            owner_paid + user_paid,
            "the result reports the fee both of them paid"
        );
    }

    #[tokio::test]
    async fn should_let_a_contract_owner_who_covers_only_the_unincreased_gas_sponsor_a_fee_increase(
    ) {
        for gas_fees_paid_by in [
            GasFeesPaidBy::ContractOwner,
            GasFeesPaidBy::PreferContractOwner,
        ] {
            // The gas estimated with the contract owner paying and no increase, read off the
            // same creation on an identical chain: exactly what their balance has to hold
            let probe = Sponsorship::new(
                gas_fees_paid_by,
                dash_to_credits!(0.1),
                dash_to_credits!(0.1),
                15,
            );
            let transition = probe
                .card_creation_with_fee_increase(gas_fees_paid_by, 0)
                .await;
            let tx = probe.platform.drive.grove.start_transaction();
            let SuccessfulExecution {
                estimated_fees: Some(estimated_fees),
                ..
            } = probe.process(&transition, &tx)
            else {
                panic!("expected a successful execution with an estimate");
            };
            let unincreased_gas = estimated_fees.total_base_fee();
            // A 100% increase doubles the processing fee, so the gas with it no longer fits
            assert!(estimated_fees.processing_fee > 0);

            let setup =
                Sponsorship::new(gas_fees_paid_by, unincreased_gas, dash_to_credits!(1), 15);
            let transition = setup
                .card_creation_with_fee_increase(gas_fees_paid_by, 100)
                .await;
            assert_eq!(setup.check_tx(&transition), Vec::<u32>::new());

            let tx = setup.platform.drive.grove.start_transaction();
            let result = setup.process(&transition, &tx);
            assert_matches!(result, SuccessfulExecution { .. });
            let owner_paid = unincreased_gas - setup.credits(&setup.contract_owner, &tx);
            let user_paid = dash_to_credits!(1) - setup.credits(&setup.user, &tx);
            assert!(owner_paid > 0, "the contract owner pays the gas");
            assert!(user_paid > 0, "the user pays its fee increase");
            assert_eq!(total_fee(&result), owner_paid + user_paid);
        }
    }

    #[tokio::test]
    async fn should_refuse_a_sponsored_creation_whose_signer_cannot_pay_its_fee_increase_unpaid() {
        for gas_fees_paid_by in [
            GasFeesPaidBy::ContractOwner,
            GasFeesPaidBy::PreferContractOwner,
        ] {
            let setup = Sponsorship::new(gas_fees_paid_by, dash_to_credits!(0.1), 0, 15);
            let transition = setup
                .card_creation_with_fee_increase(gas_fees_paid_by, 100)
                .await;

            assert_eq!(
                setup.check_tx(&transition),
                vec![IDENTITY_INSUFFICIENT_BALANCE],
                "check tx keeps it out of the mempool"
            );

            let tx = setup.platform.drive.grove.start_transaction();
            let result = setup.process(&transition, &tx);
            assert_matches!(
                result,
                UnpaidConsensusError(error) if error.code() == IDENTITY_INSUFFICIENT_BALANCE
            );
            assert_eq!(
                setup.credits(&setup.contract_owner, &tx),
                dash_to_credits!(0.1),
                "the contract owner pays nothing"
            );
            assert_eq!(setup.gold(&setup.user, &tx), 15, "nothing was created");
        }
    }

    #[tokio::test]
    async fn should_spend_the_fee_increase_from_the_key_budget_when_the_contract_owner_pays() {
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
        let transition = setup
            .card_creation_with_fee_increase(GasFeesPaidBy::ContractOwner, 100)
            .await;
        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);

        assert_matches!(result, SuccessfulExecution { .. });
        let user_paid = dash_to_credits!(0.1) - setup.credits(&setup.user, &tx);
        assert!(user_paid > 0, "the user pays its fee increase");
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
        assert_eq!(remaining_budget, Some(budget - user_paid));
    }

    #[tokio::test]
    async fn should_refuse_a_fee_increase_the_key_budget_cannot_cover_when_the_contract_owner_pays()
    {
        let setup = Sponsorship::build(
            PlatformVersion::latest(),
            GasFeesPaidBy::ContractOwner,
            false,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
            Some(1),
        );
        let transition = setup
            .card_creation_with_fee_increase(GasFeesPaidBy::ContractOwner, 100)
            .await;
        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);

        assert_matches!(
            result,
            UnpaidConsensusError(error) if error.code() == IDENTITY_PUBLIC_KEY_BUDGET_EXCEEDED
        );
        assert_eq!(
            setup.credits(&setup.contract_owner, &tx),
            dash_to_credits!(0.1)
        );
        assert_eq!(setup.credits(&setup.user, &tx), dash_to_credits!(0.1));
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

    // ---------- Storage refunds ----------

    /// What processing `transition` in the block `block_info` describes and committing it
    /// refunded the user and the contract owner. Checks that whoever paid was charged the fee
    /// less their own refund, that the other was credited theirs, and that only the fee less
    /// the refunds left the credits held in the trees.
    fn settle(
        setup: &Sponsorship,
        transition: &StateTransition,
        block_info: &BlockInfo,
        paid_by_the_contract_owner: bool,
    ) -> (Option<Credits>, Option<Credits>) {
        let (user_credits, contract_owner_credits) = setup.committed_credits();
        let credits_in_trees = setup.committed_credits_in_trees();

        let result = setup.process_in_and_commit(transition, block_info);

        let SuccessfulExecution { fee_result, .. } = &result else {
            panic!("expected the transition to execute, got {result:?}");
        };
        let fee = fee_result.total_base_fee();
        let refunded_to_user = fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(setup.user.id());
        let refunded_to_contract_owner = fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(setup.contract_owner.id());
        let (user_fee, contract_owner_fee) = if paid_by_the_contract_owner {
            (0, fee)
        } else {
            (fee, 0)
        };
        assert_eq!(
            setup.committed_credits(),
            (
                user_credits - user_fee + refunded_to_user.unwrap_or_default(),
                contract_owner_credits - contract_owner_fee
                    + refunded_to_contract_owner.unwrap_or_default(),
            ),
            "the payer is charged the fee, and each is credited their refund"
        );
        assert_eq!(
            setup.committed_credits_in_trees(),
            credits_in_trees - fee
                + refunded_to_user.unwrap_or_default()
                + refunded_to_contract_owner.unwrap_or_default(),
            "only the fee less the refunds leaves the trees"
        );
        (refunded_to_user, refunded_to_contract_owner)
    }

    #[tokio::test]
    async fn should_refund_a_sponsored_cards_storage_to_the_contract_owner() {
        // The contract owner paid for the card's storage, so its refund is theirs: a user who
        // deletes the card pays for the deletion and gets nothing back.
        let setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
        );
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let creation = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        settle(&setup, &creation, &block, true);

        let deletion = setup.card_deletion(3).await;
        let (refunded_to_user, refunded_to_contract_owner) =
            settle(&setup, &deletion, &block, false);

        assert_eq!(refunded_to_user, None, "the user is refunded nothing");
        assert!(
            refunded_to_contract_owner.is_some_and(|refund| refund > 0),
            "the contract owner is refunded the storage they paid for"
        );
    }

    #[tokio::test]
    async fn should_refund_the_contract_owner_in_full_when_the_user_replaces_a_sponsored_card() {
        // Writing the card at the user's own expense names the user, so the handover is full:
        // the contract owner gets back what they paid for and the user pays for the whole card.
        let setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
        );
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let creation = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        settle(&setup, &creation, &block, true);

        let replacement = setup
            .card_replacement_describing(2, 3, 200, GasFeesPaidBy::DocumentOwner)
            .await;
        let (refunded_to_user, refunded_to_contract_owner) =
            settle(&setup, &replacement, &block, false);
        assert_eq!(refunded_to_user, None);
        assert!(refunded_to_contract_owner.is_some_and(|refund| refund > 0));
        assert_eq!(setup.stored_card_storage_owner(), Some(setup.user.id()));
        assert_eq!(
            setup.stored_attack_index_reference_owner(),
            Some(setup.user.id()),
            "the index entries the replacement leaves unchanged are handed over too"
        );

        let deletion = setup.card_deletion(4).await;
        let (refunded_to_user, _) = settle(&setup, &deletion, &block, false);
        assert!(
            refunded_to_user.is_some_and(|refund| refund > 0),
            "the user is refunded the card they paid for"
        );
    }

    #[tokio::test]
    async fn should_refund_the_user_more_than_the_contract_owner_after_replacing_a_sponsored_card()
    {
        // After a full handover the card and its index entries are the user's. All that stays
        // the contract owner's are the index trees their sponsored creation made.
        let setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
        );
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let creation = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        settle(&setup, &creation, &block, true);

        let (mut document, _) = setup.card_of(&setup.user);
        document.set("attack", 5.into());
        document.set_revision(Some(2));
        let replacement = setup
            .card_replacement_of(document, 3, GasFeesPaidBy::DocumentOwner)
            .await;
        let (refunded_to_user, refunded_to_contract_owner) =
            settle(&setup, &replacement, &block, false);
        assert_eq!(refunded_to_user, None);
        assert!(refunded_to_contract_owner.is_some_and(|refund| refund > 0));

        let deletion = setup.card_deletion(4).await;
        let (refunded_to_user, refunded_to_contract_owner) =
            settle(&setup, &deletion, &block, false);
        assert!(
            refunded_to_user.unwrap_or_default() > refunded_to_contract_owner.unwrap_or_default(),
            "the user holds the card, got {refunded_to_user:?} and {refunded_to_contract_owner:?}"
        );
    }

    #[tokio::test]
    async fn should_refund_the_contract_owner_when_a_sponsored_card_is_transferred() {
        // The contract owner paid for this card, so a transfer is a full handover and not the
        // keep-the-bytes move of an ordinary transfer: otherwise the user could transfer it to
        // a second identity, delete it there and keep the contract owner's refund.
        let mut setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
        );
        let (recipient, _, _) = setup_identity(&mut setup.platform, 571, dash_to_credits!(0.1));
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let creation = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        settle(&setup, &creation, &block, true);
        assert_eq!(
            setup.stored_card_storage_owner(),
            Some(setup.contract_owner.id())
        );

        let transfer = setup.card_transfer(recipient.id()).await;
        let (refunded_to_user, refunded_to_contract_owner) =
            settle(&setup, &transfer, &block, false);

        assert_eq!(refunded_to_user, None, "the user is refunded nothing");
        assert!(
            refunded_to_contract_owner.is_some_and(|refund| refund > 0),
            "the contract owner gets back what they paid for"
        );
        assert_eq!(setup.stored_card_storage_owner(), Some(recipient.id()));
    }

    /// The card game with a card type that keeps history, its creation and replacement
    /// offering the contract owner's gas
    fn history_card_game() -> Sponsorship {
        let setup = Sponsorship::build_customized(
            PlatformVersion::latest(),
            GasFeesPaidBy::ContractOwner,
            false,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
            None,
            |contract| {
                let mut schema = contract
                    .document_type_for_name("card")
                    .expect("expected the card document type")
                    .schema()
                    .clone();
                schema
                    .set_value("documentsKeepHistory", true.into())
                    .expect("expected to keep history");
                schema
                    .set_value("canBeDeleted", false.into())
                    .expect("expected to refuse deletion");
                contract
                    .set_document_schema(
                        "card",
                        schema,
                        false,
                        &mut vec![],
                        PlatformVersion::latest(),
                    )
                    .expect("expected the card type to keep history");
            },
        );
        assert!(setup
            .contract
            .document_type_for_name("card")
            .expect("expected the card document type")
            .documents_keep_history());
        setup
    }

    #[tokio::test]
    async fn should_hand_a_sponsored_history_version_to_the_user_who_rewrites_it_in_its_block() {
        // A type keeping history stores each version under the block's time, so a second
        // update in the same block rewrites the version the first wrote. The user's growth
        // refunds the contract owner, and the version is the user's from then on.
        let setup = history_card_game();
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let creation = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        settle(&setup, &creation, &block, true);

        let growth = setup
            .card_replacement_describing(2, 3, 200, GasFeesPaidBy::DocumentOwner)
            .await;
        let (refunded_to_user, refunded_to_contract_owner) = settle(&setup, &growth, &block, false);
        assert_eq!(refunded_to_user, None);
        assert!(refunded_to_contract_owner.is_some_and(|refund| refund > 0));

        let shrink = setup
            .card_replacement_describing(3, 4, 10, GasFeesPaidBy::DocumentOwner)
            .await;
        let (refunded_to_user, refunded_to_contract_owner) = settle(&setup, &shrink, &block, false);
        assert!(
            refunded_to_user.is_some_and(|refund| refund > 0),
            "the user is refunded what the shrink frees of the version they now hold"
        );
        assert_eq!(refunded_to_contract_owner, None);
    }

    #[tokio::test]
    async fn should_estimate_a_handed_over_history_rewrite_at_no_less_than_it_costs() {
        // The estimate reads nothing stored, so it prices the update as a whole insertion,
        // which covers a full handover.
        let setup = history_card_game();
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let creation = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        settle(&setup, &creation, &block, true);

        let growth = setup
            .card_replacement_describing(2, 3, 200, GasFeesPaidBy::DocumentOwner)
            .await;
        let result = setup.process_and_commit(&growth);

        let SuccessfulExecution {
            estimated_fees: Some(estimated_fees),
            fee_result,
            ..
        } = &result
        else {
            panic!("expected an execution with an estimated fee, got {result:?}");
        };
        assert!(
            estimated_fees.processing_fee >= fee_result.processing_fee,
            "estimated {} below the {} the rewrite cost",
            estimated_fees.processing_fee,
            fee_result.processing_fee
        );
        assert!(estimated_fees.total_base_fee() >= fee_result.total_base_fee());
    }

    #[tokio::test]
    async fn should_leave_a_history_version_the_user_paid_for_with_the_user_in_a_later_block() {
        // A version written in a later block than the sponsored one is a new element, paid for
        // by the user: it names the user, so the user's shrink of it in that block refunds the
        // user. The contract owner's version stays stored, and theirs.
        let setup = history_card_game();
        setup.give_the_user_game_tokens(5);
        let creation_block = BlockInfo::default();
        let later_block = BlockInfo {
            time_ms: 1_000,
            height: 1,
            ..Default::default()
        };
        let creation = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        settle(&setup, &creation, &creation_block, true);

        let growth = setup
            .card_replacement_describing(2, 3, 200, GasFeesPaidBy::DocumentOwner)
            .await;
        let (refunded_to_user, _) = settle(&setup, &growth, &later_block, false);
        assert_eq!(refunded_to_user, None);

        let shrink = setup
            .card_replacement_describing(3, 4, 10, GasFeesPaidBy::DocumentOwner)
            .await;
        let (refunded_to_user, refunded_to_contract_owner) =
            settle(&setup, &shrink, &later_block, false);

        assert!(
            refunded_to_user.is_some_and(|refund| refund > 0),
            "the user is refunded what the shrink frees of the version they paid for"
        );
        assert_eq!(refunded_to_contract_owner, None);
    }

    #[tokio::test]
    async fn should_refund_the_user_in_full_when_the_contract_owner_sponsors_a_replacement() {
        // The sponsor pays for the whole card and holds it afterwards, and the user gets back
        // what they had paid for.
        let setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
        );
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let creation = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        settle(&setup, &creation, &block, false);

        let replacement = setup
            .card_replacement_describing(2, 3, 200, GasFeesPaidBy::ContractOwner)
            .await;
        let (refunded_to_user, refunded_to_contract_owner) =
            settle(&setup, &replacement, &block, true);
        assert!(refunded_to_user.is_some_and(|refund| refund > 0));
        assert_eq!(refunded_to_contract_owner, None);
        assert_eq!(
            setup.stored_card_storage_owner(),
            Some(setup.contract_owner.id())
        );
        assert_eq!(
            setup.stored_attack_index_reference_owner(),
            Some(setup.contract_owner.id()),
            "the index entries the replacement leaves unchanged are handed over too"
        );

        let deletion = setup.card_deletion(4).await;
        let (_, refunded_to_contract_owner) = settle(&setup, &deletion, &block, false);
        assert!(
            refunded_to_contract_owner.is_some_and(|refund| refund > 0),
            "the contract owner is refunded the card"
        );
    }

    /// The storage fee estimated for the user's own growing replacement of a card they paid
    /// for, on a game whose card type offers `offered`
    async fn estimated_storage_fee_of_a_replacement(offered: GasFeesPaidBy) -> Credits {
        let setup = Sponsorship::new(offered, dash_to_credits!(0.1), dash_to_credits!(0.1), 15);
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let creation = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        settle(&setup, &creation, &block, false);

        let replacement = setup
            .card_replacement_describing(2, 3, 200, GasFeesPaidBy::DocumentOwner)
            .await;
        let result = setup.process_and_commit(&replacement);
        let SuccessfulExecution {
            estimated_fees: Some(estimated_fees),
            ..
        } = &result
        else {
            panic!("expected an execution with an estimated fee, got {result:?}");
        };
        estimated_fees.storage_fee
    }

    #[tokio::test]
    async fn should_estimate_a_possible_full_handover_as_any_other_update() {
        // A document update is estimated as the insertion of everything it writes, which covers
        // a full handover, so offering sponsorship must not change the estimate.
        let offering = estimated_storage_fee_of_a_replacement(GasFeesPaidBy::ContractOwner).await;
        let not_offering =
            estimated_storage_fee_of_a_replacement(GasFeesPaidBy::DocumentOwner).await;
        assert_eq!(offering, not_offering);
    }

    #[tokio::test]
    async fn should_move_a_transferred_cards_stored_bytes_to_the_recipient() {
        // The user paid for this card, so nobody involved is a sponsor: the recipient takes the
        // stored bytes as they are, with nobody refunded, and the index entries the transfer
        // leaves unchanged move with them.
        let mut setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
        );
        let (recipient, _, _) = setup_identity(&mut setup.platform, 572, dash_to_credits!(0.1));
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let creation = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        settle(&setup, &creation, &block, false);
        assert_eq!(setup.stored_card_storage_owner(), Some(setup.user.id()));
        assert_eq!(
            setup.stored_attack_index_reference_owner(),
            Some(setup.user.id())
        );

        let transfer = setup.card_transfer(recipient.id()).await;
        let (_, refunded_to_contract_owner) = settle(&setup, &transfer, &block, false);

        assert_eq!(refunded_to_contract_owner, None);
        assert_eq!(setup.stored_card_storage_owner(), Some(recipient.id()));
        assert_eq!(
            setup.stored_attack_index_reference_owner(),
            Some(recipient.id()),
            "the index entries the transfer leaves unchanged move too"
        );
    }

    /// What the user and the contract owner are each refunded, in total, by a card's creation,
    /// its replacement giving it a description, and the user's deletion of it. `creation` and
    /// `replacement` say whom each asks for the gas; without a `replacement` the card is
    /// deleted as it was created.
    async fn refunds_over_a_cards_life(
        creation: GasFeesPaidBy,
        replacement: Option<GasFeesPaidBy>,
    ) -> (Credits, Credits) {
        let setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
        );
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let mut refunds = vec![settle(
            &setup,
            &setup.card_creation(creation).await,
            &block,
            creation == GasFeesPaidBy::ContractOwner,
        )];
        let mut nonce = 3;
        if let Some(replacement) = replacement {
            let transition = setup
                .card_replacement_describing(2, nonce, 200, replacement)
                .await;
            refunds.push(settle(
                &setup,
                &transition,
                &block,
                replacement == GasFeesPaidBy::ContractOwner,
            ));
            nonce += 1;
        }
        refunds.push(settle(
            &setup,
            &setup.card_deletion(nonce).await,
            &block,
            false,
        ));
        refunds
            .into_iter()
            .fold((0, 0), |(user, contract_owner), refunded| {
                (
                    user + refunded.0.unwrap_or_default(),
                    contract_owner + refunded.1.unwrap_or_default(),
                )
            })
    }

    /// Checks that a refund paid out `in_two_parts` is the refund paid out `at_once`. A refund
    /// rounds each epoch's share of the storage window down, so two parts can be up to a
    /// credit per epoch short of the whole, which is far less than one stored byte is worth.
    fn assert_same_refund_in_two_parts(in_two_parts: Credits, at_once: Credits) {
        let rounding = Credits::from(PERPETUAL_STORAGE_ERAS * DEFAULT_EPOCHS_PER_ERA);
        assert!(
            in_two_parts <= at_once && at_once - in_two_parts <= rounding,
            "{in_two_parts} refunded in two parts, {at_once} at once"
        );
    }

    #[tokio::test]
    async fn should_refund_a_first_payer_once_for_all_of_a_card_that_was_handed_over() {
        // Whoever paid for a card's creation is refunded all of it exactly once, whether the
        // card is deleted as it was created or handed over in full first: the handover refunds
        // the card and its index entries, the deletion the rest.
        let (_, contract_owner_alone) =
            refunds_over_a_cards_life(GasFeesPaidBy::ContractOwner, None).await;
        let (user_alone, _) = refunds_over_a_cards_life(GasFeesPaidBy::DocumentOwner, None).await;
        assert!(contract_owner_alone > 0);
        assert_eq!(
            user_alone, contract_owner_alone,
            "the same card is stored whoever pays for it"
        );

        // The user takes a sponsored card over, and is refunded only what they then paid for:
        // the card with its description
        let (user_after_taking_over, contract_owner_handing_over) = refunds_over_a_cards_life(
            GasFeesPaidBy::ContractOwner,
            Some(GasFeesPaidBy::DocumentOwner),
        )
        .await;
        assert_same_refund_in_two_parts(contract_owner_handing_over, contract_owner_alone);
        assert!(user_after_taking_over > 0);

        // The reverse: the contract owner takes the user's card over
        let (user_handing_over, contract_owner_after_taking_over) = refunds_over_a_cards_life(
            GasFeesPaidBy::DocumentOwner,
            Some(GasFeesPaidBy::ContractOwner),
        )
        .await;
        assert_same_refund_in_two_parts(user_handing_over, user_alone);
        assert_eq!(
            contract_owner_after_taking_over, user_after_taking_over,
            "either taker holds the same bytes afterwards"
        );
    }

    // ---------- Protocol version 14, which names the document's owner whoever paid ----------

    fn protocol_version_14() -> &'static PlatformVersion {
        PlatformVersion::get(14).expect("expected protocol version 14 to exist")
    }

    #[tokio::test]
    async fn should_name_the_user_as_holder_of_a_sponsored_card_at_protocol_version_14() {
        // The contract owner pays the gas, and the storage flags name the document's owner as
        // they always did: a paying sponsor is named from protocol version 15.
        let setup = Sponsorship::build(
            protocol_version_14(),
            GasFeesPaidBy::ContractOwner,
            false,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
            None,
        );
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let creation = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        settle(&setup, &creation, &block, true);

        assert_eq!(setup.stored_card_storage_owner(), Some(setup.user.id()));
        assert_eq!(
            setup.stored_attack_index_reference_owner(),
            Some(setup.user.id())
        );

        let deletion = setup.card_deletion(3).await;
        let (refunded_to_user, refunded_to_contract_owner) =
            settle(&setup, &deletion, &block, false);
        assert!(refunded_to_user.is_some_and(|refund| refund > 0));
        assert_eq!(refunded_to_contract_owner, None);
    }

    #[tokio::test]
    async fn should_leave_a_transferred_card_with_its_first_holder_at_protocol_version_14() {
        // A transfer rewrites the card at the size it had, which up to protocol version 14
        // keeps the stored owner: the card and the index entries the transfer leaves unchanged
        // stay with the user. Version 15 moves them to the recipient.
        let mut setup = Sponsorship::build(
            protocol_version_14(),
            GasFeesPaidBy::ContractOwner,
            false,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
            None,
        );
        let (recipient, _, _) = setup_identity(&mut setup.platform, 573, dash_to_credits!(0.1));
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let creation = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        settle(&setup, &creation, &block, false);

        let transfer = setup.card_transfer(recipient.id()).await;
        settle(&setup, &transfer, &block, false);

        assert_eq!(setup.stored_card().0.owner_id(), recipient.id());
        assert_eq!(setup.stored_card_storage_owner(), Some(setup.user.id()));
        assert_eq!(
            setup.stored_attack_index_reference_owner(),
            Some(setup.user.id())
        );
    }

    // ---------- Updates Drive applies outside a state transition's batch ----------

    /// The card game with the card a sponsored creation left the contract owner holding
    async fn game_with_a_sponsored_card() -> Sponsorship {
        let setup = Sponsorship::new(
            GasFeesPaidBy::ContractOwner,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
        );
        let creation = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        settle(&setup, &creation, &BlockInfo::default(), true);
        assert_eq!(
            setup.stored_card_storage_owner(),
            Some(setup.contract_owner.id())
        );
        setup
    }

    /// Storage flags naming `holder`, as an update in epoch 0 writes them
    fn flags_naming<'a>(holder: Identifier) -> Option<Cow<'a, StorageFlags>> {
        Some(Cow::Owned(StorageFlags::new_single_epoch(
            0,
            Some(holder.to_buffer()),
        )))
    }

    /// Applies `update`, the only operation of its batch
    fn apply_alone(
        setup: &Sponsorship,
        update: DocumentOperationType,
        platform_version: &PlatformVersion,
    ) -> FeeResult {
        setup
            .platform
            .drive
            .apply_drive_operations(
                vec![DriveOperation::DocumentOperation(update)],
                true,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected the update to apply")
    }

    /// Checks that an update written in the user's name handed the sponsored card over in
    /// full: the contract owner is refunded, and the card and the index entry the update left
    /// unchanged are the user's.
    fn assert_handed_over_in_full_to_the_user(setup: &Sponsorship, fee_result: &FeeResult) {
        assert!(
            fee_result
                .fee_refunds
                .calculate_refunds_amount_for_identity(setup.contract_owner.id())
                .is_some_and(|refund| refund > 0),
            "the contract owner gets back what they paid for"
        );
        assert_eq!(
            fee_result
                .fee_refunds
                .calculate_refunds_amount_for_identity(setup.user.id()),
            None
        );
        assert_eq!(setup.stored_card_storage_owner(), Some(setup.user.id()));
        assert_eq!(
            setup.stored_attack_index_reference_owner(),
            Some(setup.user.id())
        );
    }

    #[tokio::test]
    async fn should_hand_a_sponsored_card_over_in_full_when_drive_updates_it_on_its_own() {
        // `update_document_for_contract` applies a batch of its own, which follows the rule of
        // a state transition's batch.
        let setup = game_with_a_sponsored_card().await;
        let (card, _) = setup.stored_card();

        let fee_result = setup
            .platform
            .drive
            .update_document_for_contract(
                &card,
                &setup.contract,
                setup
                    .contract
                    .document_type_for_name("card")
                    .expect("expected the card document type"),
                Some(setup.user.id().to_buffer()),
                BlockInfo::default(),
                true,
                flags_naming(setup.user.id()),
                None,
                setup.platform_version,
            )
            .expect("expected the card to update");

        assert_handed_over_in_full_to_the_user(&setup, &fee_result);
    }

    #[tokio::test]
    async fn should_hand_a_sponsored_card_over_in_full_when_its_update_names_only_the_contract_id()
    {
        // The batch reads the contract to learn that its owner may hold the card.
        let setup = game_with_a_sponsored_card().await;
        let (card, _) = setup.stored_card();

        let fee_result = apply_alone(
            &setup,
            DocumentOperationType::UpdateDocument {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&card, flags_naming(setup.user.id()))),
                    owner_id: Some(setup.user.id().to_buffer()),
                },
                contract_info: DataContractInfo::DataContractId(setup.contract.id()),
                document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr("card"),
            },
            setup.platform_version,
        );

        assert_handed_over_in_full_to_the_user(&setup, &fee_result);
    }

    #[tokio::test]
    async fn should_hand_a_sponsored_card_over_in_full_when_its_update_is_one_of_a_document_type() {
        let setup = game_with_a_sponsored_card().await;
        let (card, _) = setup.stored_card();

        let fee_result = apply_alone(
            &setup,
            DocumentOperationType::MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations: vec![DocumentOperation::UpdateOperation(UpdateOperationInfo {
                        document: &card,
                        serialized_document: None,
                        owner_id: Some(setup.user.id().to_buffer()),
                        storage_flags: flags_naming(setup.user.id()),
                    })],
                    contract: &setup.contract,
                    document_type: setup
                        .contract
                        .document_type_for_name("card")
                        .expect("expected the card document type"),
                },
            },
            setup.platform_version,
        );

        assert_handed_over_in_full_to_the_user(&setup, &fee_result);
    }

    /// Rewrites the user's card as it is stored, changing only what `change` does, with flags
    /// naming `holder`, as Drive running `platform_version` does on its own
    fn rewrite_card_for(
        setup: &Sponsorship,
        holder: Identifier,
        change: impl FnOnce(&mut Document),
        platform_version: &PlatformVersion,
    ) -> FeeResult {
        let (mut card, _) = setup.stored_card();
        change(&mut card);
        setup
            .platform
            .drive
            .update_document_for_contract(
                &card,
                &setup.contract,
                setup
                    .contract
                    .document_type_for_name("card")
                    .expect("expected the card document type"),
                Some(setup.user.id().to_buffer()),
                BlockInfo::default(),
                true,
                flags_naming(holder),
                None,
                platform_version,
            )
            .expect("expected the card to update")
    }

    /// A card game without sponsorship on a chain running `platform_version`, with the card
    /// the user created and paid for
    async fn game_with_the_users_own_card(
        platform_version: &'static PlatformVersion,
    ) -> Sponsorship {
        let setup = Sponsorship::build(
            platform_version,
            GasFeesPaidBy::DocumentOwner,
            false,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
            None,
        );
        let creation = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        settle(&setup, &creation, &BlockInfo::default(), false);
        setup
    }

    #[tokio::test]
    async fn should_move_a_cards_stored_bytes_to_the_holder_a_drive_update_names() {
        // No sponsor is involved, so the named holder takes the stored bytes of the card and
        // of its unchanged index entries as they are, and nobody is refunded.
        let setup = game_with_the_users_own_card(PlatformVersion::latest()).await;
        let holder = Identifier::from([9; 32]);

        let fee_result = rewrite_card_for(&setup, holder, |_| {}, setup.platform_version);

        assert_eq!(
            fee_result
                .fee_refunds
                .calculate_refunds_amount_for_identity(setup.user.id()),
            None
        );
        assert_eq!(setup.stored_card_storage_owner(), Some(holder));
        assert_eq!(setup.stored_attack_index_reference_owner(), Some(holder));
    }

    #[tokio::test]
    async fn should_leave_a_card_with_its_first_holder_when_drive_updates_it_at_protocol_version_14(
    ) {
        let setup = game_with_the_users_own_card(protocol_version_14()).await;

        rewrite_card_for(
            &setup,
            Identifier::from([9; 32]),
            |_| {},
            setup.platform_version,
        );

        assert_eq!(setup.stored_card_storage_owner(), Some(setup.user.id()));
        assert_eq!(
            setup.stored_attack_index_reference_owner(),
            Some(setup.user.id())
        );
    }

    #[tokio::test]
    async fn should_leave_index_entries_an_earlier_version_split_from_their_card_with_their_payer()
    {
        // Up to protocol version 14 a card that grows under a new holder goes to that holder
        // while its unchanged index entries stay with whoever paid for them. Version 15 moves
        // index entries when the card changes hands, so an update keeping the card's holder
        // leaves such entries where they are, and the next change of holder moves them.
        let setup = game_with_the_users_own_card(protocol_version_14()).await;
        let second_holder = Identifier::from([9; 32]);
        let third_holder = Identifier::from([8; 32]);
        rewrite_card_for(
            &setup,
            second_holder,
            |card| card.set("description", "d".repeat(200).into()),
            protocol_version_14(),
        );
        assert_eq!(setup.stored_card_storage_owner(), Some(second_holder));
        assert_eq!(
            setup.stored_attack_index_reference_owner(),
            Some(setup.user.id())
        );

        rewrite_card_for(&setup, second_holder, |_| {}, PlatformVersion::latest());
        assert_eq!(setup.stored_card_storage_owner(), Some(second_holder));
        assert_eq!(
            setup.stored_attack_index_reference_owner(),
            Some(setup.user.id()),
            "the user paid for the entry, and the card did not change hands"
        );

        rewrite_card_for(&setup, third_holder, |_| {}, PlatformVersion::latest());
        assert_eq!(setup.stored_card_storage_owner(), Some(third_holder));
        assert_eq!(
            setup.stored_attack_index_reference_owner(),
            Some(third_holder)
        );
    }

    /// The card game with a `grade` on its cards that only moderators write, and the moderator
    /// its contract appoints: an identity that is neither the contract owner nor the user
    fn moderated_card_game() -> (Sponsorship, Identity, SimpleSigner) {
        let (moderator, moderator_signer, _) =
            setup_identity_without_adding_it(771, dash_to_credits!(0.1));
        let moderator_id = moderator.id();
        let setup = Sponsorship::build_customized(
            PlatformVersion::latest(),
            GasFeesPaidBy::ContractOwner,
            false,
            dash_to_credits!(0.1),
            dash_to_credits!(0.1),
            15,
            None,
            |contract| {
                contract.set_config(contract.config().clone().with_moderation(Some(
                    ContractModerationConfig {
                        banlist: false,
                        suspensions: false,
                        warnings: false,
                        moderators: ContractModerators::AppointedModerators(
                            [moderator_id].into_iter().collect(),
                        ),
                    },
                )));
                let mut schema = contract
                    .document_type_for_name("card")
                    .expect("expected the card document type")
                    .schema()
                    .clone();
                schema
                    .set_value(
                        "moderatorAbilities",
                        platform_value!({ "changeFields": ["grade"] }),
                    )
                    .expect("expected to let moderators change the grade");
                schema
                    .get_mut("properties")
                    .expect("expected to get the properties")
                    .expect("expected the card type to have properties")
                    .set_value(
                        "grade",
                        platform_value!({
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 10,
                            "position": 7,
                        }),
                    )
                    .expect("expected to add the grade");
                contract
                    .set_document_schema(
                        "card",
                        schema,
                        false,
                        &mut vec![],
                        PlatformVersion::latest(),
                    )
                    .expect("expected the card type to take a grade");
            },
        );
        setup
            .platform
            .drive
            .add_new_identity(
                moderator.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                setup.platform_version,
            )
            .expect("expected to add the moderator");
        (setup, moderator, moderator_signer)
    }

    /// Processes the moderator's grading of the user's card and commits it. Returns what it
    /// refunded the user and the contract owner, after checking that the moderator paid the
    /// fee and that nobody else was charged.
    async fn grade_the_card(
        setup: &Sponsorship,
        moderator: &Identity,
        moderator_signer: &SimpleSigner,
    ) -> (Option<Credits>, Option<Credits>) {
        let (card, _) = setup.card_of(&setup.user);
        let grading = ContractUserModerationTransition::try_from_identity_with_signer(
            moderator,
            &1,
            setup.contract.id(),
            ContractUserModerationAction::ChangeDocumentFields {
                document_type_name: "card".to_string(),
                document_id: card.id(),
                fields: BTreeMap::from([("grade".to_string(), Value::U64(9))]),
                reason: ContractModerationReason::from_text("graded"),
            },
            1,
            0,
            moderator_signer,
            setup.platform_version,
            None,
        )
        .await
        .expect("expected to build the moderation transition");

        let credits = setup.committed_credits();
        let moderator_credits = {
            let tx = setup.platform.drive.grove.start_transaction();
            setup.credits(moderator, &tx)
        };
        let result = setup.process_and_commit(&grading);
        let SuccessfulExecution { fee_result, .. } = &result else {
            panic!("expected the moderator's change to execute, got {result:?}");
        };
        let refunded_to_user = fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(setup.user.id());
        let refunded_to_contract_owner = fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(setup.contract_owner.id());
        assert_eq!(
            setup.committed_credits(),
            (
                credits.0 + refunded_to_user.unwrap_or_default(),
                credits.1 + refunded_to_contract_owner.unwrap_or_default(),
            ),
            "neither the user nor the contract owner is charged for a moderator's change"
        );
        let tx = setup.platform.drive.grove.start_transaction();
        assert_eq!(
            setup.credits(moderator, &tx),
            moderator_credits - fee_result.total_base_fee(),
            "the moderator pays for their change"
        );
        (refunded_to_user, refunded_to_contract_owner)
    }

    #[tokio::test]
    async fn should_leave_a_sponsored_card_with_the_contract_owner_when_a_moderator_changes_it() {
        // A moderator pays for what their change adds and owns nothing: the card stays with the
        // contract owner who paid for it. Naming the card's owner instead would refund the
        // contract owner, bill the moderator the whole card, and leave it for the user to
        // delete and collect.
        let (setup, moderator, moderator_signer) = moderated_card_game();
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let creation = setup.card_creation(GasFeesPaidBy::ContractOwner).await;
        settle(&setup, &creation, &block, true);
        assert_eq!(
            setup.stored_card_storage_owner(),
            Some(setup.contract_owner.id())
        );

        let (refunded_to_user, refunded_to_contract_owner) =
            grade_the_card(&setup, &moderator, &moderator_signer).await;
        assert_eq!(refunded_to_user, None);
        assert_eq!(
            refunded_to_contract_owner, None,
            "the contract owner keeps the card, so nothing of it is refunded"
        );
        assert_eq!(
            setup.stored_card_storage_owner(),
            Some(setup.contract_owner.id())
        );
        assert_eq!(
            setup.stored_attack_index_reference_owner(),
            Some(setup.contract_owner.id())
        );

        let deletion = setup.card_deletion(3).await;
        let (refunded_to_user, refunded_to_contract_owner) =
            settle(&setup, &deletion, &block, false);
        assert_eq!(refunded_to_user, None, "the user is refunded nothing");
        assert!(
            refunded_to_contract_owner.is_some_and(|refund| refund > 0),
            "the contract owner is refunded the card they paid for"
        );
    }

    #[tokio::test]
    async fn should_leave_a_card_its_user_paid_for_with_the_user_when_a_moderator_changes_it() {
        let (setup, moderator, moderator_signer) = moderated_card_game();
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let creation = setup.card_creation(GasFeesPaidBy::DocumentOwner).await;
        settle(&setup, &creation, &block, false);
        assert_eq!(setup.stored_card_storage_owner(), Some(setup.user.id()));

        let (refunded_to_user, refunded_to_contract_owner) =
            grade_the_card(&setup, &moderator, &moderator_signer).await;
        assert_eq!(refunded_to_user, None);
        assert_eq!(refunded_to_contract_owner, None);
        assert_eq!(setup.stored_card_storage_owner(), Some(setup.user.id()));
        assert_eq!(
            setup.stored_attack_index_reference_owner(),
            Some(setup.user.id())
        );
    }

    #[tokio::test]
    async fn should_refund_the_user_who_paid_when_the_contract_owner_could_not() {
        // A preference the contract owner cannot honour falls back to the user, who then paid
        // for the storage and is refunded it.
        let setup = Sponsorship::new(GasFeesPaidBy::ContractOwner, 0, dash_to_credits!(0.1), 15);
        setup.give_the_user_game_tokens(5);
        let block = BlockInfo::default();
        let creation = setup
            .card_creation(GasFeesPaidBy::PreferContractOwner)
            .await;
        settle(&setup, &creation, &block, false);

        let deletion = setup.card_deletion(3).await;
        let (refunded_to_user, refunded_to_contract_owner) =
            settle(&setup, &deletion, &block, false);

        assert!(
            refunded_to_user.is_some_and(|refund| refund > 0),
            "the user is refunded the storage they paid for"
        );
        assert_eq!(refunded_to_contract_owner, None);
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
        assert_matches!(
            setup.process_and_commit(&creation),
            SuccessfulExecution { .. }
        );

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
        assert_matches!(
            setup.process_and_commit(&creation),
            SuccessfulExecution { .. }
        );
        let deletion = setup.card_deletion_without_token_payment().await;
        assert_eq!(
            setup.check_tx(&deletion),
            vec![REQUIRED_TOKEN_PAYMENT_INFO_NOT_SET]
        );
    }
}
