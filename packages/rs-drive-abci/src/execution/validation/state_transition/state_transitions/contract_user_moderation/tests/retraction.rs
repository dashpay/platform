//! A banned or suspended author's retraction (`retractedWhen`, protocol version 14): on a type
//! whose documents can not be deleted, the one replace a barred owner may still make.

use super::*;
use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;

const MESSAGE: &str = "message";
const NOTE: &str = "note";
const TALLY: &str = "tally";
const DOCUMENT_PROPERTY_CONSTRAINT_VIOLATED: u32 = 10422;
const DOCUMENT_IMMUTABLE_PROPERTY_CHANGED: u32 = 40128;

/// A message its author can not delete, retracted by setting `deleted`: a retracted message
/// carries no text, and stays retracted.
fn message_schema() -> Value {
    platform_value!({
        "type": "object",
        "canBeDeleted": false,
        "properties": {
            "text": { "type": "string", "maxLength": 50, "position": 0 },
            "deleted": { "type": "boolean", "position": 1 },
        },
        "additionalProperties": false,
        "retractedWhen": { "present": "deleted" },
        "propertyConstraints": {
            "retractedIsBlank": { "anyOf": [{ "absent": "deleted" }, { "absent": "text" }] },
        },
        "immutable": [{ "property": "deleted", "when": { "present": "$old.deleted" } }],
    })
}

/// A note its author can not delete either, with no way to retract it
fn note_schema() -> Value {
    platform_value!({
        "type": "object",
        "canBeDeleted": false,
        "properties": {
            "text": { "type": "string", "maxLength": 50, "position": 0 },
            "deleted": { "type": "boolean", "position": 1 },
        },
        "additionalProperties": false,
    })
}

/// A tally whose retraction condition can fault: 10 / score above 1 holds for a score of 1 to
/// 9, not from 10, and divides by zero for a score of 0
fn tally_schema() -> Value {
    platform_value!({
        "type": "object",
        "canBeDeleted": false,
        "properties": {
            "score": { "type": "integer", "minimum": 0, "maximum": 100, "position": 0 },
        },
        "additionalProperties": false,
        "retractedWhen": { "greaterThan": [{ "divide": [10, "score"] }, 1] },
    })
}

/// A contract keeping both lists, with messages, notes and tallies
async fn setup() -> Setup {
    Setup::new_at_with(
        Some(moderation(true, true, THE_MODERATOR)),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(contract, MESSAGE, message_schema());
            add_document_type(contract, NOTE, note_schema());
            add_document_type(contract, TALLY, tally_schema());
        },
    )
    .await
}

/// A document of `document_type_name` by `actor` saying `text`
async fn create(
    setup: &Setup,
    actor: &Actor,
    document_type_name: &str,
    text: &str,
) -> (Document, StateTransition) {
    setup
        .create_document_of_type_with(actor, document_type_name, |document| {
            let properties = document.properties_mut();
            properties.clear();
            properties.insert("text".to_string(), Value::Text(text.to_string()));
        })
        .await
}

/// A replacement by `actor` of `document`, of `document_type_name`, with its properties set to
/// `properties`
async fn replace(
    setup: &Setup,
    actor: &Actor,
    document_type_name: &str,
    document: &Document,
    properties: Value,
) -> StateTransition {
    let mut replaced = document.clone();
    *replaced.properties_mut() = properties
        .into_btree_string_map()
        .expect("expected a map of properties");
    replaced.increment_revision().expect("expected a revision");
    BatchTransition::new_document_replacement_transition_from_document(
        replaced,
        setup
            .contract
            .document_type_for_name(document_type_name)
            .expect("expected the document type"),
        &actor.key,
        actor.contract_nonce(),
        0,
        None,
        &actor.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("expected to build the document replacement")
}

fn retracted() -> Value {
    platform_value!({ "deleted": true })
}

#[tokio::test]
async fn should_let_a_barred_author_retract_a_document_it_can_not_delete_and_nothing_else() {
    let setup = setup().await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let (suspended_with, create_one) = create(&setup, &setup.user, MESSAGE, "first").await;
    assert_success(&setup.process(&create_one, &transaction));
    let (banned_with, create_two) = create(&setup, &setup.user, MESSAGE, "second").await;
    assert_success(&setup.process(&create_two, &transaction));

    // Suspended: an edit is refused, the retraction passes.
    let suspend = setup
        .moderate(
            &setup.owner,
            suspend_action(user_id, BLOCK_TIME_MS + 10_000),
        )
        .await;
    assert_success(&setup.process(&suspend, &transaction));
    let edit = replace(
        &setup,
        &setup.user,
        MESSAGE,
        &suspended_with,
        platform_value!({ "text": "edited" }),
    )
    .await;
    assert_paid_with_code(&setup.process(&edit, &transaction), CONTRACT_USER_SUSPENDED);
    let retract = replace(&setup, &setup.user, MESSAGE, &suspended_with, retracted()).await;
    assert_success(&setup.process(&retract, &transaction));
    let stored = setup
        .stored_document(MESSAGE, suspended_with.id(), Some(&transaction))
        .expect("expected the retracted message");
    assert_eq!(
        stored.properties(),
        &retracted()
            .into_btree_string_map()
            .expect("expected a map of properties")
    );
    assert_eq!(stored.revision(), Some(2));

    // Banned: the same. A retraction is still judged by every other rule of the type: one
    // keeping its text breaks the type's own rule, not the bar.
    let ban = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_success(&setup.process(&ban, &transaction));
    let edit = replace(
        &setup,
        &setup.user,
        MESSAGE,
        &banned_with,
        platform_value!({ "text": "edited" }),
    )
    .await;
    assert_paid_with_code(&setup.process(&edit, &transaction), CONTRACT_USER_BANNED);
    let keeps_text = replace(
        &setup,
        &setup.user,
        MESSAGE,
        &banned_with,
        platform_value!({ "text": "second", "deleted": true }),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&keeps_text, &transaction),
        DOCUMENT_PROPERTY_CONSTRAINT_VIOLATED,
    );
    let retract = replace(&setup, &setup.user, MESSAGE, &banned_with, retracted()).await;
    assert_success(&setup.process(&retract, &transaction));

    // Taking the retraction back is no retraction: the bar refuses it before the type's own
    // `immutable` entry would.
    let mut retracted_message = suspended_with.clone();
    retracted_message
        .increment_revision()
        .expect("expected a revision");
    let restore = replace(
        &setup,
        &setup.user,
        MESSAGE,
        &retracted_message,
        platform_value!({ "text": "first" }),
    )
    .await;
    assert_paid_with_code(&setup.process(&restore, &transaction), CONTRACT_USER_BANNED);
    // Nor can a barred author write anything new.
    let (_, refused) = create(&setup, &setup.user, MESSAGE, "third").await;
    assert_paid_with_code(&setup.process(&refused, &transaction), CONTRACT_USER_BANNED);
    setup.commit(transaction);

    // Unbarred, the type's own rule keeps a retracted message retracted.
    let transaction = setup.platform.drive.grove.start_transaction();
    let unban = setup.moderate(&setup.owner, unban_action(user_id)).await;
    assert_success(&setup.process(&unban, &transaction));
    let restore = replace(
        &setup,
        &setup.user,
        MESSAGE,
        &retracted_message,
        platform_value!({ "text": "first" }),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&restore, &transaction),
        DOCUMENT_IMMUTABLE_PROPERTY_CHANGED,
    );
    setup.commit(transaction);
}

/// The mempool judges a barred author's replaces as a block does: the retraction is let in, the
/// edit is refused with the bar.
#[tokio::test]
async fn should_judge_a_barred_authors_retraction_in_the_mempool() {
    let setup = setup().await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let (message, create_message) = create(&setup, &setup.user, MESSAGE, "first").await;
    assert_success(&setup.process(&create_message, &transaction));
    let ban = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_success(&setup.process(&ban, &transaction));
    setup.commit(transaction);

    let edit = replace(
        &setup,
        &setup.user,
        MESSAGE,
        &message,
        platform_value!({ "text": "edited" }),
    )
    .await;
    let errors = setup.check_tx(&edit);
    assert!(
        errors
            .iter()
            .any(|error| error.code() == CONTRACT_USER_BANNED),
        "expected the bar, got {errors:?}"
    );
    let retract = replace(&setup, &setup.user, MESSAGE, &message, retracted()).await;
    assert!(setup.check_tx(&retract).is_empty());
}

/// A type that declares no retraction keeps every replace of a barred author out, one shaped like
/// a retraction too.
#[tokio::test]
async fn should_refuse_every_replace_of_a_barred_author_on_a_type_without_retracted_when() {
    let setup = setup().await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let (note, create_note) = create(&setup, &setup.user, NOTE, "first").await;
    assert_success(&setup.process(&create_note, &transaction));
    let ban = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_success(&setup.process(&ban, &transaction));
    let retract = replace(&setup, &setup.user, NOTE, &note, retracted()).await;
    assert_paid_with_code(&setup.process(&retract, &transaction), CONTRACT_USER_BANNED);
    setup.commit(transaction);
}

/// A condition that faults (a division by zero) counts as not met: a fault never lifts a bar. The
/// replace is refused with the bar in the mempool and in a block, paid with the nonce bumped, and
/// the stored document is left as it was; the same condition met lets the replace through.
#[tokio::test]
async fn should_keep_the_bar_when_the_retraction_condition_faults() {
    let setup = setup().await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let (tally, create_tally) = setup
        .create_document_of_type_with(&setup.user, TALLY, |document| {
            let properties = document.properties_mut();
            properties.clear();
            properties.insert("score".to_string(), Value::U64(50));
        })
        .await;
    assert_success(&setup.process(&create_tally, &transaction));
    let ban = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_success(&setup.process(&ban, &transaction));
    setup.commit(transaction);

    // 10 / 0 faults
    let faults = replace(
        &setup,
        &setup.user,
        TALLY,
        &tally,
        platform_value!({ "score": 0u64 }),
    )
    .await;
    let refused_nonce = setup.user.next_contract_nonce.get() - 1;
    let errors = setup.check_tx(&faults);
    assert!(
        errors
            .iter()
            .any(|error| error.code() == CONTRACT_USER_BANNED),
        "expected the bar, got {errors:?}"
    );
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_paid_with_code(&setup.process(&faults, &transaction), CONTRACT_USER_BANNED);
    let stored = setup
        .stored_document(TALLY, tally.id(), Some(&transaction))
        .expect("expected the tally");
    assert_eq!(stored.revision(), Some(1));
    // Stored at the width the score's bounds give it
    assert!(stored
        .properties()
        .get("score")
        .is_some_and(|score| score.equal_underlying_data(&Value::U64(50))));
    let nonce = setup
        .platform
        .drive
        .fetch_identity_contract_nonce(
            user_id.to_buffer(),
            setup.contract.id().to_buffer(),
            true,
            Some(&transaction),
            PlatformVersion::latest(),
        )
        .expect("expected to fetch the nonce")
        .expect("expected a nonce");
    assert_eq!(nonce & IDENTITY_NONCE_VALUE_FILTER, refused_nonce);

    // 10 / 2 is above 1
    let met = replace(
        &setup,
        &setup.user,
        TALLY,
        &tally,
        platform_value!({ "score": 2u64 }),
    )
    .await;
    assert_success(&setup.process(&met, &transaction));
    setup.commit(transaction);
}
