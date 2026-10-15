//! `deleteConstraints` beside moderation (protocol version 14): the rules gate the owner's
//! delete only. A moderator deletes whatever they say, and a banned owner, whose deletions the
//! moderation gate lets through, is held to them like any other.

use super::*;

const DOCUMENT_DELETE_CONSTRAINT_VIOLATED: u32 = 40147;

/// A post its author may delete only while its text is shorter than 3 characters, and its
/// moderators whatever its text
fn gated_post_schema() -> Value {
    post_schema_with(platform_value!({
        "canBeDeleted": true,
        "deleteConstraints": {
            "shortText": { "lessThan": [{ "length": "text" }, 3] },
        },
    }))
}

/// A post by `actor` saying `text`
async fn create_post(setup: &Setup, actor: &Actor, text: &str) -> (Document, StateTransition) {
    setup
        .create_document_of_type_with(actor, POST, |document| {
            let properties = document.properties_mut();
            properties.clear();
            properties.insert("text".to_string(), Value::Text(text.to_string()));
        })
        .await
}

#[tokio::test]
async fn should_let_a_moderator_delete_what_the_owners_rules_refuse() {
    let setup = Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| add_document_type(contract, POST, gated_post_schema()),
    )
    .await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = create_post(&setup, &setup.user, "a long text").await;
    assert_success(&setup.process(&create, &transaction));

    let own_delete = own_post_deletion(&setup, &setup.user, post.clone()).await;
    assert_paid_with_code(
        &setup.process(&own_delete, &transaction),
        DOCUMENT_DELETE_CONSTRAINT_VIOLATED,
    );
    assert!(setup
        .stored_document(POST, post.id(), Some(&transaction))
        .is_some());

    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&delete, &transaction));
    assert!(setup
        .stored_document(POST, post.id(), Some(&transaction))
        .is_none());
    setup.commit(transaction);
}

#[tokio::test]
async fn should_hold_a_banned_owners_delete_to_the_rules() {
    let setup = Setup::new_at_with(
        Some(moderation(true, true, THE_MODERATOR)),
        PlatformVersion::latest(),
        |contract| add_document_type(contract, POST, gated_post_schema()),
    )
    .await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let (long, create) = create_post(&setup, &setup.user, "a long text").await;
    assert_success(&setup.process(&create, &transaction));
    let (short, create) = create_post(&setup, &setup.user, "hi").await;
    assert_success(&setup.process(&create, &transaction));
    let ban = setup
        .moderate(&setup.owner, ban_action(setup.user.id()))
        .await;
    assert_success(&setup.process(&ban, &transaction));

    // The gate lets a banned owner's deletions through; the rules still judge them
    let refused = own_post_deletion(&setup, &setup.user, long).await;
    assert_paid_with_code(
        &setup.process(&refused, &transaction),
        DOCUMENT_DELETE_CONSTRAINT_VIOLATED,
    );
    let accepted = own_post_deletion(&setup, &setup.user, short).await;
    assert_success(&setup.process(&accepted, &transaction));
    setup.commit(transaction);
}
