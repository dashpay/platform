use super::*;
use dpp::consensus::basic::document::PropertyConstraintViolation;
use dpp::consensus::basic::BasicError;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
use drive::grovedb::TransactionArg;

fn count_schema() -> Value {
    post_schema_with(platform_value!({
        "indices": [{"name": "byOwner", "properties": [{"$ownerId": "asc"}],
            "countable": "countable"}],
        "propertyConstraints": {"onePerOwner": {
            "lessThanOrEqual": [{"countOf": ["post", {"$ownerId": "$ownerId"}]}, 1]
        }}
    }))
}

fn sum_schema() -> Value {
    post_schema_with(platform_value!({
        "properties": {
            "text": {"type": "string", "maxLength": 50, "position": 0},
            "amount": {"type": "integer", "minimum": 0, "maximum": 100, "position": 1}
        },
        "required": ["text", "amount"],
        "documentsSummable": "amount",
        "propertyConstraints": {"budget": {
            "lessThanOrEqual": [{"sumOf": ["post", "amount"]}, 100]
        }}
    }))
}

async fn setup_with_schema(schema: Value) -> Setup {
    Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| add_document_type(contract, POST, schema),
    )
    .await
}

async fn create_post(
    setup: &Setup,
    actor: &Actor,
    text: &str,
    amount: Option<u64>,
    tx: &Transaction<'_>,
) -> Document {
    let (document, transition) = setup
        .create_document_of_type_with(actor, POST, |document| {
            document.properties_mut().clear();
            document.set("text", Value::Text(text.to_string()));
            if let Some(amount) = amount {
                document.set("amount", Value::U64(amount));
            }
        })
        .await;
    assert_success(&setup.process(&transition, tx));
    setup
        .stored_document(POST, document.id(), Some(tx))
        .expect("expected the created post")
}

async fn remove_post(setup: &Setup, document: &Document, tx: &Transaction<'_>) -> Vec<u8> {
    let bytes = setup.document_bytes(POST, document);
    let deletion = setup
        .moderate(&setup.moderator, delete_action(POST, document.id()))
        .await;
    assert_success(&setup.process(&deletion, tx));
    assert!(setup
        .stored_document(POST, document.id(), Some(tx))
        .is_none());
    bytes
}

fn nonce(setup: &Setup, actor: &Actor, tx: TransactionArg) -> IdentityNonce {
    setup
        .platform
        .drive
        .fetch_identity_contract_nonce(
            actor.id().to_buffer(),
            setup.contract.id().to_buffer(),
            true,
            tx,
            PlatformVersion::latest(),
        )
        .expect("expected to read the nonce")
        .unwrap_or_default()
        & IDENTITY_NONCE_VALUE_FILTER
}

/// Reads the stored total, without adding the document that would be restored.
fn total(setup: &Setup, document: &Document, tx: TransactionArg) -> i128 {
    let version = PlatformVersion::latest();
    let document_type = setup.contract.document_type_for_name(POST).unwrap();
    let reads = document_type
        .property_constraints()
        .values()
        .next()
        .unwrap()
        .aggregate_reads();
    let read = reads[0];
    let counted = setup
        .contract
        .document_type_for_name(&read.document_type)
        .unwrap();
    let values = read
        .filter_values(
            counted,
            document.id(),
            &Value::from(document.properties().clone()),
            document.owner_id(),
            version,
        )
        .unwrap()
        .unwrap();
    setup
        .platform
        .drive
        .fetch_property_constraint_aggregate(
            setup.contract.id().to_buffer(),
            counted,
            read,
            &values,
            tx,
            &mut vec![],
            version,
        )
        .unwrap()
}

fn constraint_refusal<'a>(result: &'a StateTransitionExecutionResult, rule: &str) -> &'a FeeResult {
    assert_paid_with_code(result, 10422);
    let StateTransitionExecutionResult::PaidConsensusError {
        error:
            ConsensusError::BasicError(BasicError::DocumentPropertyConstraintViolatedError(error)),
        actual_fees,
        ..
    } = result
    else {
        panic!("expected a paid property constraint refusal, got {result:?}");
    };
    assert_eq!(error.document_type_name(), POST);
    assert_eq!(error.constraint(), rule);
    assert_eq!(error.violation(), PropertyConstraintViolation::NotMet);
    actual_fees
}

async fn replace_post(setup: &Setup, mut document: Document) -> StateTransition {
    document.set("text", Value::Text("still editable".to_string()));
    document.increment_revision().unwrap();
    BatchTransition::new_document_replacement_transition_from_document(
        document,
        setup.contract.document_type_for_name(POST).unwrap(),
        &setup.user.key,
        setup.user.contract_nonce(),
        0,
        None,
        &setup.user.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .unwrap()
}

async fn assert_over_limit_restore_refused(
    schema: Value,
    removed_amount: Option<u64>,
    live_amount: Option<u64>,
    expected_total: i128,
    rule: &str,
) {
    let setup = setup_with_schema(schema).await;
    let tx = setup.platform.drive.grove.start_transaction();
    let removed = create_post(&setup, &setup.user, "removed", removed_amount, &tx).await;
    let bytes = remove_post(&setup, &removed, &tx).await;
    let live = create_post(&setup, &setup.user, "live", live_amount, &tx).await;
    setup.commit(tx);
    let removal = setup.post_removal(removed.id(), None).unwrap();
    assert_eq!(removal.restoration, None);
    assert_eq!(total(&setup, &removed, None), expected_total);

    let expected_nonce = setup.moderator.next_contract_nonce.get();
    let restore = setup
        .moderate(&setup.moderator, restore_action(POST, bytes))
        .await;
    let balance = setup.balance(setup.moderator.id(), None);
    let author_balance = setup.balance(setup.user.id(), None);
    let previous_nonce = nonce(&setup, &setup.moderator, None);
    let admission_errors = setup.check_tx(&restore);
    assert_eq!(setup.balance(setup.moderator.id(), None), balance);
    assert_eq!(nonce(&setup, &setup.moderator, None), previous_nonce);

    let tx = setup.platform.drive.grove.start_transaction();
    let result = setup.process(&restore, &tx);
    let fees = constraint_refusal(&result, rule);
    assert!(fees.total_base_fee() > 0);
    assert_eq!(
        setup.balance(setup.moderator.id(), Some(&tx)),
        balance - fees.total_base_fee()
    );
    assert_eq!(nonce(&setup, &setup.moderator, Some(&tx)), expected_nonce);
    assert_eq!(setup.balance(setup.user.id(), Some(&tx)), author_balance);
    assert!(setup
        .stored_document(POST, removed.id(), Some(&tx))
        .is_none());
    assert_eq!(setup.post_removal(removed.id(), Some(&tx)), Some(removal));
    assert_eq!(
        setup.stored_document(POST, live.id(), Some(&tx)),
        Some(live.clone())
    );
    assert_eq!(total(&setup, &removed, Some(&tx)), expected_total);
    assert_eq!(
        admission_errors
            .iter()
            .map(ErrorWithCode::code)
            .collect::<Vec<_>>(),
        vec![10422]
    );

    // Refusing the restore leaves the surviving post within its owner's budget and editable.
    let replace = replace_post(&setup, live).await;
    assert_success(&setup.process(&replace, &tx));
    assert_eq!(total(&setup, &removed, Some(&tx)), expected_total);
}

#[tokio::test]
async fn should_refuse_restore_exceeding_the_original_owners_count_cap() {
    assert_over_limit_restore_refused(count_schema(), None, None, 1, "onePerOwner").await;
}

#[tokio::test]
async fn should_refuse_restore_exceeding_the_sum_budget_without_sticking_replacements() {
    assert_over_limit_restore_refused(sum_schema(), Some(20), Some(100), 100, "budget").await;
}

#[tokio::test]
async fn should_restore_at_the_exact_sum_boundary_with_retained_document_values() {
    let setup = setup_with_schema(sum_schema()).await;
    let tx = setup.platform.drive.grove.start_transaction();
    let removed = create_post(&setup, &setup.user, "removed", Some(20), &tx).await;
    let bytes = remove_post(&setup, &removed, &tx).await;
    create_post(&setup, &setup.user, "live", Some(80), &tx).await;
    let restore = setup
        .moderate(&setup.moderator, restore_action(POST, bytes))
        .await;
    assert_success(&setup.process(&restore, &tx));
    assert_eq!(total(&setup, &removed, Some(&tx)), 100);
    assert_eq!(
        setup.stored_document(POST, removed.id(), Some(&tx)),
        Some(removed.clone())
    );
    assert!(setup
        .post_removal(removed.id(), Some(&tx))
        .unwrap()
        .restoration
        .is_some());
}

#[tokio::test]
async fn should_restore_for_the_original_owner_when_the_moderator_and_stranger_are_at_cap() {
    let setup = setup_with_schema(count_schema()).await;
    let tx = setup.platform.drive.grove.start_transaction();
    let removed = create_post(&setup, &setup.user, "removed", None, &tx).await;
    let bytes = remove_post(&setup, &removed, &tx).await;
    create_post(&setup, &setup.moderator, "moderator", None, &tx).await;
    create_post(&setup, &setup.stranger, "stranger", None, &tx).await;
    assert_eq!(total(&setup, &removed, Some(&tx)), 0);
    let restore = setup
        .moderate(&setup.moderator, restore_action(POST, bytes))
        .await;
    assert_success(&setup.process(&restore, &tx));
    assert_eq!(total(&setup, &removed, Some(&tx)), 1);
    assert_eq!(
        setup.stored_document(POST, removed.id(), Some(&tx)),
        Some(removed)
    );
}

#[tokio::test]
async fn should_judge_a_later_restore_against_an_earlier_restore_in_the_same_block() {
    let setup = setup_with_schema(count_schema()).await;
    let tx = setup.platform.drive.grove.start_transaction();
    let first = create_post(&setup, &setup.user, "first", None, &tx).await;
    let first_bytes = remove_post(&setup, &first, &tx).await;
    let second = create_post(&setup, &setup.user, "second", None, &tx).await;
    let second_bytes = remove_post(&setup, &second, &tx).await;
    let second_removal = setup.post_removal(second.id(), Some(&tx)).unwrap();
    setup.commit(tx);
    let restore_first = setup
        .moderate(&setup.moderator, restore_action(POST, first_bytes.clone()))
        .await;
    let second_nonce = setup.moderator.next_contract_nonce.get();
    let restore_second = setup
        .moderate(&setup.moderator, restore_action(POST, second_bytes))
        .await;
    assert!(setup.check_tx(&restore_first).is_empty());
    assert!(setup.check_tx(&restore_second).is_empty());
    let tx = setup.platform.drive.grove.start_transaction();
    let state = setup.platform.state.load();
    let raw = vec![
        restore_first.serialize_to_bytes().unwrap(),
        restore_second.serialize_to_bytes().unwrap(),
    ];
    let results = setup
        .platform
        .platform
        .process_raw_state_transitions(
            &raw,
            &state,
            &BlockInfo {
                time_ms: BLOCK_TIME_MS,
                ..Default::default()
            },
            &tx,
            PlatformVersion::latest(),
            false,
            None,
        )
        .unwrap();
    let executions = results.execution_results();
    assert_eq!(executions.len(), 2);
    assert_success(&executions[0]);
    constraint_refusal(&executions[1], "onePerOwner");
    assert_eq!(nonce(&setup, &setup.moderator, Some(&tx)), second_nonce);
    assert_eq!(total(&setup, &first, Some(&tx)), 1);
    assert_eq!(
        setup.stored_document(POST, first.id(), Some(&tx)),
        Some(first.clone())
    );
    assert!(setup
        .stored_document(POST, second.id(), Some(&tx))
        .is_none());
    assert_eq!(
        setup.post_removal(second.id(), Some(&tx)),
        Some(second_removal)
    );

    let repeat = setup
        .moderate(&setup.moderator, restore_action(POST, first_bytes))
        .await;
    assert_paid_with_code(
        &setup.process(&repeat, &tx),
        CONTRACT_DOCUMENT_ALREADY_RESTORED,
    );
    assert_eq!(total(&setup, &first, Some(&tx)), 1);
}

#[tokio::test]
async fn should_refuse_restore_when_its_cross_type_prerequisite_was_deleted() {
    let setup = Setup::new_at_with(Some(moderators_without_lists()), PlatformVersion::latest(), |contract| {
        add_document_type(contract, "profile", platform_value!({
            "type": "object", "properties": {"name": {"type": "string", "maxLength": 32, "position": 0}},
            "required": ["name"], "additionalProperties": false,
            "indices": [{"name": "byOwner", "properties": [{"$ownerId": "asc"}], "countable": "countable"}]
        }));
        add_document_type(contract, POST, post_schema_with(platform_value!({
            "propertyConstraints": {"hasProfile": {"greaterThanOrEqual": [
                {"countOf": ["profile", {"$ownerId": "$ownerId"}]}, 1
            ]}}
        })));
    }).await;
    let tx = setup.platform.drive.grove.start_transaction();
    let (profile, create) = setup.create_document_of_type(&setup.user, "profile").await;
    assert_success(&setup.process(&create, &tx));
    let removed = create_post(&setup, &setup.user, "removed", None, &tx).await;
    let bytes = remove_post(&setup, &removed, &tx).await;
    let removal = setup.post_removal(removed.id(), Some(&tx)).unwrap();
    let delete_profile = BatchTransition::new_document_deletion_transition_from_document(
        profile.clone(),
        setup.contract.document_type_for_name("profile").unwrap(),
        &setup.user.key,
        setup.user.contract_nonce(),
        0,
        None,
        &setup.user.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .unwrap();
    assert_success(&setup.process(&delete_profile, &tx));
    assert_eq!(total(&setup, &removed, Some(&tx)), 0);
    let restore = setup
        .moderate(&setup.moderator, restore_action(POST, bytes))
        .await;
    constraint_refusal(&setup.process(&restore, &tx), "hasProfile");
    assert!(setup
        .stored_document(POST, removed.id(), Some(&tx))
        .is_none());
    assert!(setup
        .stored_document("profile", profile.id(), Some(&tx))
        .is_none());
    assert_eq!(setup.post_removal(removed.id(), Some(&tx)), Some(removal));
}

#[tokio::test]
async fn should_preserve_the_rule_free_restore_fee_and_document() {
    let setup = setup_with_schema(post_schema(true)).await;
    let tx = setup.platform.drive.grove.start_transaction();
    let removed = create_post(&setup, &setup.user, "removed", None, &tx).await;
    let bytes = remove_post(&setup, &removed, &tx).await;
    let restore = setup
        .moderate(&setup.moderator, restore_action(POST, bytes))
        .await;
    let result = setup.process(&restore, &tx);
    assert_success(&result);
    let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } = result else {
        unreachable!();
    };
    assert_eq!(
        fee_result,
        FeeResult {
            storage_fee: 9_612_000,
            processing_fee: 1_345_540,
            ..Default::default()
        }
    );
    assert_eq!(
        setup.stored_document(POST, removed.id(), Some(&tx)),
        Some(removed)
    );
}
