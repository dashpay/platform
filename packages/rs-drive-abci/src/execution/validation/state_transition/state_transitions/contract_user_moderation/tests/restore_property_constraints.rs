//! A moderator's restore judged by its document type's `propertyConstraints` (protocol version
//! 14): the restore puts the document back into every count and sum its deletion took it out
//! of, so a `countOf` cap or a `sumOf` budget that other documents filled while it was gone
//! refuses it, as it would refuse a create.

use super::*;
use dpp::consensus::basic::document::PropertyConstraintViolation;
use dpp::consensus::basic::BasicError;

const DOCUMENT_PROPERTY_CONSTRAINT_VIOLATED: u32 = 10422;

/// A post each author holds one of at a time: `byOwner` counts each author's posts.
fn one_post_per_author_schema() -> Value {
    post_schema_with(platform_value!({
        "indices": [
            { "name": "byOwner", "properties": [{ "$ownerId": "asc" }], "countable": "countable" },
        ],
        "propertyConstraints": {
            "onePostPerAuthor": {
                "lessThanOrEqual": [{ "countOf": ["post", { "$ownerId": "$ownerId" }] }, 1]
            },
        },
    }))
}

/// A post spending part of a budget of 100 every post shares: the type sums `amount`.
fn budgeted_post_schema() -> Value {
    post_schema_with(platform_value!({
        "properties": {
            "text": { "type": "string", "maxLength": 50, "position": 0 },
            "amount": { "type": "integer", "minimum": 0, "maximum": 1000, "position": 1 },
        },
        "required": ["text", "amount"],
        "documentsSummable": "amount",
        "propertyConstraints": {
            "budget": { "lessThanOrEqual": [{ "sumOf": ["post", "amount"] }, 100] },
        },
    }))
}

/// A contract whose `post` type has `schema`, moderated by a named moderator
async fn setup_with_posts(schema: Value) -> Setup {
    Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| add_document_type(contract, POST, schema),
    )
    .await
}

/// Sets a post's `amount`
fn spending(amount: u64) -> impl FnOnce(&mut Document) {
    move |document: &mut Document| document.set("amount", Value::U64(amount))
}

/// The `post` rule `rule` broke, refusing the transition, paid
fn assert_rule_broken(execution: &StateTransitionExecutionResult, rule: &str) {
    assert_paid_with_code(execution, DOCUMENT_PROPERTY_CONSTRAINT_VIOLATED);
    let StateTransitionExecutionResult::PaidConsensusError {
        error:
            ConsensusError::BasicError(BasicError::DocumentPropertyConstraintViolatedError(error)),
        ..
    } = execution
    else {
        panic!("expected DocumentPropertyConstraintViolatedError, got {execution:?}");
    };
    assert_eq!(error.document_type_name(), POST);
    assert_eq!(error.constraint(), rule);
    assert_eq!(error.violation(), PropertyConstraintViolation::NotMet);
}

/// The post `post_id` is gone and its record is not marked restored
fn assert_not_restored(setup: &Setup, post_id: Identifier, transaction: &Transaction) {
    assert_eq!(
        setup.stored_document(POST, post_id, Some(transaction)),
        None
    );
    assert_eq!(
        setup
            .post_removal(post_id, Some(transaction))
            .map(|removal| removal.restoration),
        Some(None)
    );
}

#[tokio::test]
async fn should_refuse_a_restore_that_would_exceed_a_count_of_cap() {
    let setup = setup_with_posts(one_post_per_author_schema()).await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    let stored = setup
        .stored_document(POST, post.id(), Some(&transaction))
        .expect("expected the post to be stored");
    let bytes = setup.document_bytes(POST, &stored);
    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&delete, &transaction));

    // The deletion left the author no post, and the author wrote another.
    let (other, create_other) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_other, &transaction));
    setup.commit(transaction);

    // Back beside it the author would hold two. The total is the author's, not the
    // moderator's, who holds none: refused by the mempool and by a block, paid.
    let restore = setup
        .moderate(&setup.moderator, restore_action(POST, bytes.clone()))
        .await;
    let mempool_errors = setup.check_tx(&restore);
    assert_eq!(mempool_errors.len(), 1);
    assert_eq!(
        mempool_errors[0].code(),
        DOCUMENT_PROPERTY_CONSTRAINT_VIOLATED
    );
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_rule_broken(
        &setup.process_at(&restore, BLOCK_TIME_MS + 1, &transaction),
        "onePostPerAuthor",
    );
    assert_not_restored(&setup, post.id(), &transaction);

    // Once the author deletes the other post there is room again, and the post comes back as
    // it was, still the author's one post.
    let other = setup
        .stored_document(POST, other.id(), Some(&transaction))
        .expect("expected the other post to be stored");
    let own_delete = own_post_deletion(&setup, &setup.user, other).await;
    assert_success(&setup.process_at(&own_delete, BLOCK_TIME_MS + 2, &transaction));
    let restore = setup
        .moderate(&setup.moderator, restore_action(POST, bytes))
        .await;
    assert_success(&setup.process_at(&restore, BLOCK_TIME_MS + 3, &transaction));
    assert_eq!(
        setup.stored_document(POST, post.id(), Some(&transaction)),
        Some(stored)
    );
    let (_, one_too_many) = setup.create_document_of_type(&setup.user, POST).await;
    assert_rule_broken(
        &setup.process_at(&one_too_many, BLOCK_TIME_MS + 4, &transaction),
        "onePostPerAuthor",
    );
}

#[tokio::test]
async fn should_refuse_a_restore_that_would_exceed_a_sum_of_budget() {
    let setup = setup_with_posts(budgeted_post_schema()).await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup
        .create_document_of_type_with(&setup.user, POST, spending(20))
        .await;
    assert_success(&setup.process(&create, &transaction));
    let stored = setup
        .stored_document(POST, post.id(), Some(&transaction))
        .expect("expected the post to be stored");
    let bytes = setup.document_bytes(POST, &stored);
    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&delete, &transaction));

    // The deletion freed the 20 the post spent, and a stranger spent the whole budget.
    let (other, create_other) = setup
        .create_document_of_type_with(&setup.stranger, POST, spending(100))
        .await;
    assert_success(&setup.process(&create_other, &transaction));

    // 100 + 20 is above 100: refused, paid, nothing restored.
    let restore = setup
        .moderate(&setup.moderator, restore_action(POST, bytes.clone()))
        .await;
    assert_rule_broken(
        &setup.process_at(&restore, BLOCK_TIME_MS + 1, &transaction),
        "budget",
    );
    assert_not_restored(&setup, post.id(), &transaction);

    // The stranger takes back its post and spends 80 instead: 80 + 20 is exactly the budget,
    // and the post comes back.
    let other = setup
        .stored_document(POST, other.id(), Some(&transaction))
        .expect("expected the other post to be stored");
    let own_delete = own_post_deletion(&setup, &setup.stranger, other).await;
    assert_success(&setup.process_at(&own_delete, BLOCK_TIME_MS + 2, &transaction));
    let (_, create_smaller) = setup
        .create_document_of_type_with(&setup.stranger, POST, spending(80))
        .await;
    assert_success(&setup.process_at(&create_smaller, BLOCK_TIME_MS + 3, &transaction));
    let restore = setup
        .moderate(&setup.moderator, restore_action(POST, bytes))
        .await;
    assert_success(&setup.process_at(&restore, BLOCK_TIME_MS + 4, &transaction));
    assert_eq!(
        setup.stored_document(POST, post.id(), Some(&transaction)),
        Some(stored)
    );
}
