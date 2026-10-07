//! Transparent document token payments obey the token issuer's movement policy.

use super::*;
use super::gas_sponsorship::gas_sponsorship_tests::Sponsorship;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::batch::action_validation::document::DocumentBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;
use dpp::consensus::codes::ErrorWithCode;
use dpp::data_contract::accessors::v1::DataContractV1Setters;
use dpp::data_contract::accessors::v0::DataContractV0Setters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
use dpp::data_contract::{DataContract, document_type::DocumentType};
use dpp::data_contract::document_type::accessors::DocumentTypeV1Getters;
use dpp::platform_value::platform_value;
use dpp::state_transition::StateTransition;
use dpp::tokens::calculate_token_id;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::status::{v0::TokenStatusV0, TokenStatus};
use dpp::tokens::token_amount_on_contract_token::DocumentActionTokenEffect;
use dpp::tokens::token_payment_info::{TokenPaymentInfo, v0::TokenPaymentInfoV0};
use dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::DefaultForPlatformVersion;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
use drive::grovedb::Transaction;
use drive::util::test_helpers::setup_contract;

fn payment_setup(platform_version: &'static PlatformVersion) -> Sponsorship {
    policy_setup(
        platform_version,
        DocumentActionTokenEffect::TransferTokenToContractOwner,
        Some(false),
        false,
        false,
    )
}

fn policy_setup(
    platform_version: &'static PlatformVersion,
    effect: DocumentActionTokenEffect,
    allow_frozen: Option<bool>,
    external: bool,
    optional: bool,
) -> Sponsorship {
    let issuer_id =
        external.then(|| DataContract::generate_data_contract_id_v0(Identifier::new([9; 32]), 1));
    let setup = Sponsorship::build_customized(
        platform_version,
        GasFeesPaidBy::DocumentOwner,
        optional,
        dash_to_credits!(0.5),
        dash_to_credits!(0.5),
        15,
        None,
        |contract| {
            if external {
                contract
                    .tokens_mut()
                    .expect("tokens")
                    .get_mut(&0)
                    .expect("gold")
                    .allow_transfer_to_frozen_balance(false);
            }
            if !external {
                if let Some(allow) = allow_frozen {
                    contract
                        .tokens_mut()
                        .expect("tokens")
                        .get_mut(&0)
                        .expect("gold")
                        .allow_transfer_to_frozen_balance(allow);
                }
            }
            let mut cost = platform_value!({"tokenPosition": 0u16, "amount": 10u64});
            cost.insert("effect".into(), u8::from(effect).into())
                .expect("effect");
            if let Some(issuer_id) = issuer_id {
                cost.insert("contractId".into(), issuer_id.into())
                    .expect("issuer");
            }
            if optional {
                cost.insert("optional".into(), true.into())
                    .expect("optional");
            }
            let mut costs = platform_value!({});
            for action in [
                "create",
                "replace",
                "delete",
                "transfer",
                "update_price",
                "purchase",
            ] {
                costs
                    .insert(action.into(), cost.clone())
                    .expect("action cost");
            }
            let mut schema = contract
                .document_type_for_name("card")
                .expect("card")
                .schema()
                .clone();
            schema
                .insert("tokenCost".into(), costs)
                .expect("token costs");
            let card = DocumentType::try_from_schema(
                contract.id(),
                1,
                contract.config().version(),
                "card",
                schema,
                None,
                contract.tokens(),
                contract.config(),
                true,
                &mut vec![],
                platform_version,
            )
            .expect("card schema");
            contract.document_types_mut().insert("card".into(), card);
            if platform_version.protocol_version >= 14 {
                let entry_schema = platform_value!({
                    "type": "object", "indexOnly": true, "documentsMutable": false,
                    "canBeDeleted": true,
                    "properties": {"name": {"type": "string", "maxLength": 63, "position": 0u16}},
                    "indices": [{"name": "byName", "properties": [{"name": "asc"}], "terminal": "$ownerId"}],
                    "required": ["name"], "additionalProperties": false,
                    "tokenCost": {"delete": cost}
                });
                let entry = DocumentType::try_from_schema(
                    contract.id(),
                    1,
                    contract.config().version(),
                    "entry",
                    entry_schema,
                    None,
                    contract.tokens(),
                    contract.config(),
                    true,
                    &mut vec![],
                    platform_version,
                )
                .expect("index-only schema");
                contract.document_types_mut().insert("entry".into(), entry);
            }
        },
    );
    if let Some(issuer_id) = issuer_id {
        setup_contract(
            &setup.platform.drive,
            "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency.json",
            Some(issuer_id.to_buffer()), Some([9; 32]),
            Some(|contract: &mut DataContract| {
                contract.set_created_at_epoch(Some(0));
                contract.set_created_at(Some(0));
                contract.set_created_at_block_height(Some(0));
                if let Some(allow) = allow_frozen {
                    contract.tokens_mut().expect("tokens").get_mut(&0).expect("gold")
                        .allow_transfer_to_frozen_balance(allow);
                }
            }), None, Some(platform_version),
        );
        add_tokens_to_identity(&setup.platform, token_id(&setup), setup.user.id(), 15);
    }
    setup
}

fn token_id(setup: &Sponsorship) -> Identifier {
    let cost = setup
        .contract
        .document_type_for_name("card")
        .expect("card")
        .document_creation_token_cost()
        .expect("cost");
    calculate_token_id(
        cost.contract_id.unwrap_or(setup.contract.id()).as_bytes(),
        0,
    )
    .into()
}

fn payment_info(setup: &Sponsorship) -> TokenPaymentInfo {
    TokenPaymentInfo::V0(TokenPaymentInfoV0 {
        payment_token_contract_id: setup
            .contract
            .document_type_for_name("card")
            .expect("card")
            .document_creation_token_cost()
            .expect("cost")
            .contract_id,
        token_contract_position: 0,
        minimum_token_cost: None,
        maximum_token_cost: Some(10),
        gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
    })
}

async fn creation(setup: &Sponsorship, pay_token: bool) -> StateTransition {
    let (document, entropy) = setup.card_of(&setup.user);
    BatchTransition::new_document_creation_transition_from_document(
        document,
        setup.contract.document_type_for_name("card").expect("card"),
        entropy.0,
        &setup.user_key,
        2,
        0,
        pay_token.then(|| payment_info(setup)),
        &setup.user_signer,
        setup.platform_version,
        None,
    )
    .await
    .expect("create transition")
}

async fn prepared_action(setup: &mut Sponsorship, action: &str) -> StateTransition {
    if action == "create" {
        return creation(setup, true).await;
    }
    let (other, other_signer, other_key) =
        setup_identity(&mut setup.platform, 450, dash_to_credits!(0.5));
    let creator = if action == "purchase" {
        &other
    } else {
        &setup.user
    };
    let creator_key = if action == "purchase" {
        &other_key
    } else {
        &setup.user_key
    };
    let creator_signer = if action == "purchase" {
        &other_signer
    } else {
        &setup.user_signer
    };
    add_tokens_to_identity(&setup.platform, token_id(setup), creator.id(), 100);
    let (mut document, entropy) = setup.card_of(creator);
    let type_name = if action == "index_only_delete" {
        "entry"
    } else {
        "card"
    };
    let document_type = setup
        .contract
        .document_type_for_name(type_name)
        .expect("document type");
    if action == "index_only_delete" {
        document = document_type
            .random_document_with_identifier_and_entropy(
                &mut StdRng::seed_from_u64(433),
                creator.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                setup.platform_version,
            )
            .expect("entry document");
        document.set("name", "entry".into());
        document
            .set_id_for_creation(document_type, &entropy.0, 2, setup.platform_version)
            .expect("entry id");
    }
    let create = BatchTransition::new_document_creation_transition_from_document(
        document.clone(),
        document_type,
        entropy.0,
        creator_key,
        2,
        0,
        (action != "index_only_delete").then(|| payment_info(setup)),
        creator_signer,
        setup.platform_version,
        None,
    )
    .await
    .expect("setup creation");
    let tx = setup.platform.drive.grove.start_transaction();
    assert_matches!(
        setup.process(&create, &tx),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );
    setup
        .platform
        .drive
        .grove
        .commit_transaction(tx)
        .unwrap()
        .expect("commit setup creation");
    document.set_revision(Some(2));
    let payment = Some(payment_info(setup));
    match action {
        "replace" => {
            document.set("attack", 5.into());
            BatchTransition::new_document_replacement_transition_from_document(
                document,
                document_type,
                &setup.user_key,
                3,
                0,
                payment,
                &setup.user_signer,
                setup.platform_version,
                None,
            )
            .await
        }
        "delete" | "index_only_delete" => {
            BatchTransition::new_document_deletion_transition_from_document(
                document,
                document_type,
                &setup.user_key,
                3,
                0,
                payment,
                &setup.user_signer,
                setup.platform_version,
                None,
            )
            .await
        }
        "transfer" => {
            BatchTransition::new_document_transfer_transition_from_document(
                document,
                document_type,
                other.id(),
                &setup.user_key,
                3,
                0,
                payment,
                &setup.user_signer,
                setup.platform_version,
                None,
            )
            .await
        }
        "update_price" => {
            BatchTransition::new_document_update_price_transition_from_document(
                document,
                document_type,
                1000,
                &setup.user_key,
                3,
                0,
                payment,
                &setup.user_signer,
                setup.platform_version,
                None,
            )
            .await
        }
        "purchase" => {
            let price = BatchTransition::new_document_update_price_transition_from_document(
                document.clone(),
                document_type,
                1000,
                &other_key,
                3,
                0,
                Some(payment_info(setup)),
                &other_signer,
                setup.platform_version,
                None,
            )
            .await
            .expect("setup price");
            let tx = setup.platform.drive.grove.start_transaction();
            assert_matches!(
                setup.process(&price, &tx),
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            );
            setup
                .platform
                .drive
                .grove
                .commit_transaction(tx)
                .unwrap()
                .expect("commit setup price");
            document.set_revision(Some(3));
            BatchTransition::new_document_purchase_transition_from_document(
                document,
                document_type,
                setup.user.id(),
                1000,
                &setup.user_key,
                2,
                0,
                payment,
                &setup.user_signer,
                setup.platform_version,
                None,
            )
            .await
        }
        _ => panic!("unknown document action {action}"),
    }
    .expect("document action")
}

fn balance(setup: &Sponsorship, holder: Identifier, tx: &Transaction) -> u64 {
    setup
        .platform
        .drive
        .fetch_identity_token_balance(
            token_id(setup).to_buffer(),
            holder.to_buffer(),
            Some(tx),
            setup.platform_version,
        )
        .expect("balance")
        .unwrap_or_default()
}

fn pause(setup: &Sponsorship, tx: Option<&Transaction>) {
    setup
        .platform
        .drive
        .token_apply_status(
            token_id(setup).to_buffer(),
            TokenStatus::V0(TokenStatusV0 { paused: true }),
            &BlockInfo::default(),
            true,
            tx,
            setup.platform_version,
        )
        .expect("pause token");
}

fn freeze(setup: &Sponsorship, holder: Identifier) {
    setup
        .platform
        .drive
        .token_freeze(
            token_id(setup),
            holder,
            &BlockInfo::default(),
            true,
            None,
            setup.platform_version,
        )
        .expect("freeze account");
}

fn assert_no_card(setup: &Sponsorship, tx: &Transaction) {
    let query = DriveDocumentQuery::from_sql_expr(
        "select * from card",
        &setup.contract,
        Some(&setup.platform.config.drive),
        setup.platform_version,
    )
    .expect("query");
    let documents = setup
        .platform
        .drive
        .query_documents(query, None, false, Some(tx), None)
        .expect("documents");
    assert!(documents.documents().is_empty());
    assert_eq!(balance(setup, setup.user.id(), tx), 15);
    assert_eq!(balance(setup, setup.contract_owner.id(), tx), 0);
    assert_eq!(
        setup
            .platform
            .drive
            .fetch_token_total_supply(
                token_id(setup).to_buffer(),
                Some(tx),
                setup.platform_version
            )
            .expect("supply"),
        Some(15)
    );
}

fn assert_paid_error(result: &StateTransitionExecutionResult, code: u32) {
    assert_matches!(result, PaidConsensusError { error, actual_fees, .. }
        if error.code() == code && actual_fees.processing_fee > 0);
}

fn validation_operations(
    setup: &Sponsorship,
    payer: Identifier,
    tx: &Transaction,
) -> (SimpleConsensusValidationResult, Vec<ValidationOperation>) {
    let (_, contract) = setup
        .platform
        .drive
        .get_contract_with_fetch_info_and_fee(
            setup.contract.id().to_buffer(),
            Some(&BlockInfo::default().epoch),
            false,
            Some(tx),
            setup.platform_version,
        )
        .expect("document contract");
    let cost = setup
        .contract
        .document_type_for_name("card")
        .expect("card")
        .document_creation_token_cost()
        .expect("cost");
    let base: DocumentBaseTransitionAction = DocumentBaseTransitionActionV0 {
        id: Identifier::new([7; 32]),
        identity_contract_nonce: 2,
        document_type_name: "card".into(),
        data_contract: contract.expect("contract exists"),
        token_cost: Some((token_id(setup), cost.effect, cost.token_amount)),
        gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
        contract_gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
        declared_action_fee: None,
        shielded_token_payment: None,
    }
    .into();
    let state = setup.platform.state.load();
    let platform = PlatformStateRef {
        drive: &setup.platform.drive,
        state: &state,
        config: &setup.platform.config,
    };
    let mut context =
        StateTransitionExecutionContext::default_for_platform_version(setup.platform_version)
            .expect("context");
    let result = base
        .validate_state(
            &platform,
            payer,
            &BlockInfo::default(),
            "create",
            &mut context,
            Some(tx),
            setup.platform_version,
        )
        .expect("validation");
    (result, context.operations_consume())
}

#[test]
fn should_bill_each_document_payment_policy_read_once_in_execution_order() {
    for external in [false, true] {
        for frozen in [false, true] {
            let setup = policy_setup(
                PlatformVersion::latest(),
                Default::default(),
                Some(false),
                external,
                false,
            );
            if frozen {
                freeze(&setup, setup.contract_owner.id());
            }
            let tx = setup.platform.drive.grove.start_transaction();
            let drive = &setup.platform.drive;
            let pv = setup.platform_version;
            let block = BlockInfo::default();
            let token = token_id(&setup).to_buffer();
            let payer = setup.user.id().to_buffer();
            let recipient = setup.contract_owner.id().to_buffer();
            let mut expected = vec![
                ValidationOperation::PrecalculatedOperation(
                    drive
                        .fetch_identity_token_info_with_costs(
                            token,
                            payer,
                            &block,
                            true,
                            Some(&tx),
                            pv,
                        )
                        .expect("payer info")
                        .1,
                ),
                ValidationOperation::PrecalculatedOperation(
                    drive
                        .fetch_identity_token_balance_with_costs(
                            token,
                            payer,
                            &block,
                            true,
                            Some(&tx),
                            pv,
                        )
                        .expect("payer balance")
                        .1,
                ),
                ValidationOperation::PrecalculatedOperation(
                    drive
                        .fetch_token_status_with_costs(token, &block, true, Some(&tx), pv)
                        .expect("pause")
                        .1,
                ),
                ValidationOperation::PrecalculatedOperation(
                    drive
                        .fetch_identity_token_info_with_costs(
                            token,
                            recipient,
                            &block,
                            true,
                            Some(&tx),
                            pv,
                        )
                        .expect("recipient info")
                        .1,
                ),
            ];
            if frozen {
                let (info, fee) = drive
                    .fetch_token_contract_info_with_costs(token, &block, true, Some(&tx), pv)
                    .expect("issuer metadata");
                expected.push(ValidationOperation::PrecalculatedOperation(fee));
                if external {
                    let (fee, _) = drive
                        .get_contract_with_fetch_info_and_fee(
                            info.expect("metadata").contract_id().to_buffer(),
                            Some(&block.epoch),
                            false,
                            Some(&tx),
                            pv,
                        )
                        .expect("external issuer");
                    expected.push(ValidationOperation::PrecalculatedOperation(
                        fee.expect("issuer read fee"),
                    ));
                }
            }
            let (result, operations) = validation_operations(&setup, setup.user.id(), &tx);
            assert_eq!(operations, expected, "external={external}, frozen={frozen}");
            assert_eq!(result.is_valid(), !frozen);
        }
    }
}

#[test]
fn should_skip_new_policy_reads_for_elided_owner_self_payment() {
    let setup = payment_setup(PlatformVersion::latest());
    add_tokens_to_identity(
        &setup.platform,
        token_id(&setup),
        setup.contract_owner.id(),
        15,
    );
    pause(&setup, None);
    let tx = setup.platform.drive.grove.start_transaction();
    let (result, operations) = validation_operations(&setup, setup.contract_owner.id(), &tx);
    assert!(result.is_valid());
    assert_eq!(operations.len(), 2, "only payer info and balance are read");
}

#[tokio::test]
async fn should_reject_document_token_payment_while_token_is_paused() {
    for external in [false, true] {
        let setup = policy_setup(
            PlatformVersion::latest(),
            Default::default(),
            Some(false),
            external,
            false,
        );
        pause(&setup, None);
        let transition = creation(&setup, true).await;
        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);
        assert_paid_error(&result, 40711);
        assert_no_card(&setup, &tx);
    }
}

#[tokio::test]
async fn should_reject_document_token_payment_to_frozen_contract_owner_when_issuer_forbids_it() {
    for external in [false, true] {
        let setup = policy_setup(
            PlatformVersion::latest(),
            Default::default(),
            Some(false),
            external,
            false,
        );
        freeze(&setup, setup.contract_owner.id());
        let transition = creation(&setup, true).await;
        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);
        assert_paid_error(&result, 40702);
        assert_matches!(result, PaidConsensusError { error: ConsensusError::StateError(StateError::IdentityTokenAccountFrozenError(ref error)), .. }
            if *error.identity_id() == setup.contract_owner.id() && *error.token_id() == token_id(&setup));
        assert_no_card(&setup, &tx);
    }
}

#[tokio::test]
async fn should_allow_frozen_recipient_when_actual_issuer_uses_default_true() {
    for external in [false, true] {
        let setup = policy_setup(
            PlatformVersion::latest(),
            Default::default(),
            None,
            external,
            false,
        );
        freeze(&setup, setup.contract_owner.id());
        let transition = creation(&setup, true).await;
        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(balance(&setup, setup.user.id(), &tx), 5);
        assert_eq!(balance(&setup, setup.contract_owner.id(), &tx), 10);
    }
}

#[tokio::test]
async fn should_allow_optional_credit_payment_while_explicit_token_payment_is_restricted() {
    let setup = policy_setup(
        PlatformVersion::latest(),
        Default::default(),
        Some(false),
        true,
        true,
    );
    pause(&setup, None);
    freeze(&setup, setup.contract_owner.id());
    let explicit = creation(&setup, true).await;
    let tx = setup.platform.drive.grove.start_transaction();
    assert_paid_error(&setup.process(&explicit, &tx), 40711);
    // Each transaction starts at the same contract nonce so these are independent choices.
    drop(tx);
    let credits = creation(&setup, false).await;
    let tx = setup.platform.drive.grove.start_transaction();
    assert_matches!(
        setup.process(&credits, &tx),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );
    assert_eq!(balance(&setup, setup.user.id(), &tx), 15);
    assert_eq!(balance(&setup, setup.contract_owner.id(), &tx), 0);
}

#[tokio::test]
async fn should_pause_document_burn_payments_without_changing_native_burn_policy() {
    let setup = policy_setup(
        PlatformVersion::latest(),
        DocumentActionTokenEffect::BurnToken,
        Some(false),
        false,
        false,
    );
    let transition = creation(&setup, true).await;
    let tx = setup.platform.drive.grove.start_transaction();
    assert_matches!(
        setup.process(&transition, &tx),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );
    assert_eq!(balance(&setup, setup.user.id(), &tx), 5);
    assert_eq!(
        setup
            .platform
            .drive
            .fetch_token_total_supply(
                token_id(&setup).to_buffer(),
                Some(&tx),
                setup.platform_version
            )
            .expect("supply"),
        Some(5)
    );
    drop(tx);
    pause(&setup, None);
    let tx = setup.platform.drive.grove.start_transaction();
    assert_paid_error(&setup.process(&transition, &tx), 40711);
    assert_no_card(&setup, &tx);
}

#[tokio::test]
async fn should_preserve_elided_owner_self_payment_while_token_is_paused() {
    let setup = payment_setup(PlatformVersion::latest());
    add_tokens_to_identity(
        &setup.platform,
        token_id(&setup),
        setup.contract_owner.id(),
        15,
    );
    pause(&setup, None);
    let transition = setup
        .card_creation_by_the_contract_owner(GasFeesPaidBy::DocumentOwner)
        .await;
    let tx = setup.platform.drive.grove.start_transaction();
    assert_matches!(
        setup.process(&transition, &tx),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );
    assert_eq!(balance(&setup, setup.contract_owner.id(), &tx), 15);
    assert_eq!(
        setup
            .platform
            .drive
            .fetch_token_total_supply(
                token_id(&setup).to_buffer(),
                Some(&tx),
                setup.platform_version
            )
            .expect("supply"),
        Some(30)
    );
}

#[tokio::test]
async fn should_preserve_paused_and_frozen_recipient_payments_at_previous_protocol() {
    let setup = payment_setup(PlatformVersion::get(13).expect("previous protocol"));
    pause(&setup, None);
    freeze(&setup, setup.contract_owner.id());
    let transition = creation(&setup, true).await;
    let tx = setup.platform.drive.grove.start_transaction();
    assert_matches!(
        setup.process(&transition, &tx),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );
    assert_eq!(balance(&setup, setup.user.id(), &tx), 5);
    assert_eq!(balance(&setup, setup.contract_owner.id(), &tx), 10);
}

#[tokio::test]
async fn should_reject_paused_payments_on_all_seven_document_action_paths() {
    let mut failures = vec![];
    for action in [
        "create",
        "replace",
        "delete",
        "transfer",
        "update_price",
        "purchase",
        "index_only_delete",
    ] {
        let mut setup = payment_setup(PlatformVersion::latest());
        let transition = prepared_action(&mut setup, action).await;
        pause(&setup, None);
        let tx = setup.platform.drive.grove.start_transaction();
        let before_payer = balance(&setup, setup.user.id(), &tx);
        let before_recipient = balance(&setup, setup.contract_owner.id(), &tx);
        let result = setup.process(&transition, &tx);
        if !matches!(&result, PaidConsensusError { error, .. } if error.code() == 40711) {
            failures.push((action, result));
            continue;
        }
        assert_eq!(
            balance(&setup, setup.user.id(), &tx),
            before_payer,
            "{action}"
        );
        assert_eq!(
            balance(&setup, setup.contract_owner.id(), &tx),
            before_recipient,
            "{action}"
        );
    }
    assert!(
        failures.is_empty(),
        "paused token payments accepted or misclassified: {failures:?}"
    );
}

#[tokio::test]
async fn should_reject_external_issuer_frozen_recipient_on_all_seven_document_action_paths() {
    let mut failures = vec![];
    for action in [
        "create",
        "replace",
        "delete",
        "transfer",
        "update_price",
        "purchase",
        "index_only_delete",
    ] {
        let mut setup = policy_setup(
            PlatformVersion::latest(),
            Default::default(),
            Some(false),
            true,
            false,
        );
        let transition = prepared_action(&mut setup, action).await;
        freeze(&setup, setup.contract_owner.id());
        let tx = setup.platform.drive.grove.start_transaction();
        let before_payer = balance(&setup, setup.user.id(), &tx);
        let before_recipient = balance(&setup, setup.contract_owner.id(), &tx);
        let result = setup.process(&transition, &tx);
        if !matches!(&result, PaidConsensusError { error, .. } if error.code() == 40702) {
            failures.push((action, result));
            continue;
        }
        assert_eq!(
            balance(&setup, setup.user.id(), &tx),
            before_payer,
            "{action}"
        );
        assert_eq!(
            balance(&setup, setup.contract_owner.id(), &tx),
            before_recipient,
            "{action}"
        );
    }
    assert!(
        failures.is_empty(),
        "frozen recipient token payments accepted or misclassified: {failures:?}"
    );
}

#[tokio::test]
async fn should_preserve_payer_freeze_and_balance_precedence_before_pause_and_recipient() {
    for payer_frozen in [false, true] {
        let setup = payment_setup(PlatformVersion::latest());
        setup
            .platform
            .drive
            .token_burn(
                token_id(&setup).to_buffer(),
                setup.user.id().to_buffer(),
                15,
                &BlockInfo::default(),
                true,
                None,
                setup.platform_version,
            )
            .expect("empty payer balance");
        pause(&setup, None);
        freeze(&setup, setup.contract_owner.id());
        if payer_frozen {
            freeze(&setup, setup.user.id());
        }
        let transition = creation(&setup, true).await;
        let tx = setup.platform.drive.grove.start_transaction();
        let result = setup.process(&transition, &tx);
        assert_paid_error(&result, if payer_frozen { 40702 } else { 40700 });
        if payer_frozen {
            assert_matches!(result, PaidConsensusError { error: ConsensusError::StateError(StateError::IdentityTokenAccountFrozenError(ref error)), .. }
                if *error.identity_id() == setup.user.id());
        }
        let (_, operations) = validation_operations(&setup, setup.user.id(), &tx);
        assert_eq!(operations.len(), if payer_frozen { 1 } else { 2 });
    }
    let setup = payment_setup(PlatformVersion::latest());
    pause(&setup, None);
    freeze(&setup, setup.contract_owner.id());
    let transition = creation(&setup, true).await;
    let tx = setup.platform.drive.grove.start_transaction();
    assert_paid_error(&setup.process(&transition, &tx), 40711);
    let (_, operations) = validation_operations(&setup, setup.user.id(), &tx);
    assert_eq!(
        operations.len(),
        3,
        "pause refusal precedes recipient lookup"
    );
}

#[tokio::test]
async fn should_read_pause_written_in_the_same_transaction_as_document_payment() {
    let setup = payment_setup(PlatformVersion::latest());
    let transition = creation(&setup, true).await;
    let tx = setup.platform.drive.grove.start_transaction();
    pause(&setup, Some(&tx));
    assert_paid_error(&setup.process(&transition, &tx), 40711);
    assert_no_card(&setup, &tx);
}
