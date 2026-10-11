//! `propertyConstraints` rules reading the document a `moderatedDocument`
//! reference points at (protocol version 14). Once a moderator removes the
//! post a reply names, the reply's rules read the post's removal record, which
//! keeps its owner, its id and the fields its type lists under
//! `deleteKeepsFields`, the only ones such a rule may read. A moderator's
//! restore checks no reference, and judges no rule reading one.

use super::*;
use dpp::consensus::basic::document::PropertyConstraintViolation;
use dpp::consensus::basic::BasicError;

const REPLY: &str = "reply";
const TOPIC: &str = "topic";
const THREAD_POST: &str = "threadPost";

fn identifier_property(position: u64, refers_to: Value) -> Value {
    platform_value!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": position,
        "refersTo": refers_to,
    })
}

/// A post only moderators take down, each removal keeping its text
fn moderated_post_schema() -> Value {
    post_schema_with(platform_value!({
        "canBeDeleted": false,
        "moderatorAbilities": { "delete": true, "deleteKeepsFields": ["text"] },
    }))
}

/// A reply to a post, which may quote it word for word, and whose writer is not
/// the post's author
fn reply_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "postId": identifier_property(
                0,
                platform_value!({ "type": "moderatedDocument", "documentType": POST }),
            ),
            "quote": { "type": "string", "maxLength": 50, "position": 1 },
            "body": { "type": "string", "minLength": 1, "maxLength": 50, "position": 2 },
        },
        "required": ["postId", "body"],
        "propertyConstraints": {
            "quotesThePost": {
                "anyOf": [
                    { "absent": "quote" },
                    { "equal": ["quote", { "referenced": ["postId", "text"] }] }
                ]
            },
            "notTheAuthor": {
                "notEqual": [{ "referenced": ["postId", "$ownerId"] }, "$ownerId"]
            }
        },
        "additionalProperties": false,
    })
}

/// A topic its owner may close, never deleted
fn topic_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": false,
        "properties": {
            "open": { "type": "boolean", "position": 0 },
        },
        "required": ["open"],
        "additionalProperties": false,
    })
}

/// A post on a topic, which moderators take down, written only while the topic
/// is open
fn thread_post_schema() -> Value {
    platform_value!({
        "type": "object",
        "canBeDeleted": false,
        "moderatorAbilities": { "delete": true },
        "properties": {
            "topicId": identifier_property(
                0,
                platform_value!({ "type": "permanentDocument", "documentType": TOPIC }),
            ),
            "text": { "type": "string", "minLength": 1, "maxLength": 50, "position": 1 },
        },
        "required": ["topicId", "text"],
        "propertyConstraints": {
            "topicOpen": { "equal": [{ "referenced": ["topicId", "open"] }, 1] }
        },
        "additionalProperties": false,
    })
}

async fn setup() -> Setup {
    Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(contract, POST, moderated_post_schema());
            add_document_type(contract, REPLY, reply_schema());
            add_document_type(contract, TOPIC, topic_schema());
            add_document_type(contract, THREAD_POST, thread_post_schema());
        },
    )
    .await
}

fn id_value(id: Identifier) -> Value {
    Value::Identifier(id.to_buffer())
}

/// A creation by `actor` of a document of `document_type_name` holding exactly
/// `properties`
async fn create_with(
    setup: &Setup,
    actor: &Actor,
    document_type_name: &str,
    properties: Vec<(&str, Value)>,
) -> (Document, StateTransition) {
    setup
        .create_document_of_type_with(actor, document_type_name, |document| {
            document.properties_mut().clear();
            for (property, value) in properties {
                document.set(property, value);
            }
        })
        .await
}

/// The replacement of `document` by `actor`, its owner, its revision bumped
async fn replacement(
    actor: &Actor,
    contract: &DataContract,
    document_type_name: &str,
    document: &mut Document,
) -> StateTransition {
    document.increment_revision().expect("revision increments");
    BatchTransition::new_document_replacement_transition_from_document(
        document.clone(),
        contract
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
    .expect("expected to build the replacement")
}

fn assert_rule_broken(execution: &StateTransitionExecutionResult, rule: &str) {
    assert!(
        matches!(
            execution,
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::BasicError(
                    BasicError::DocumentPropertyConstraintViolatedError(e)
                ),
                ..
            } if e.constraint() == rule && e.violation() == PropertyConstraintViolation::NotMet
        ),
        "expected rule {rule} to refuse the write, got {execution:?}"
    );
}

/// A reply's rules read the post it names: its text and its author. Once a
/// moderator removes the post, a replace of the reply reads them from the
/// post's removal record, which keeps both.
#[tokio::test]
async fn should_judge_a_rule_by_the_record_of_a_removed_post() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) =
        create_with(&setup, &setup.user, POST, vec![("text", "hello".into())]).await;
    assert_success(&setup.process(&create_post, &transaction));

    let (_, misquote) = create_with(
        &setup,
        &setup.stranger,
        REPLY,
        vec![
            ("postId", id_value(post.id())),
            ("quote", "goodbye".into()),
            ("body", "reply".into()),
        ],
    )
    .await;
    assert_rule_broken(&setup.process(&misquote, &transaction), "quotesThePost");

    let (_, own_reply) = create_with(
        &setup,
        &setup.user,
        REPLY,
        vec![("postId", id_value(post.id())), ("body", "me".into())],
    )
    .await;
    assert_rule_broken(&setup.process(&own_reply, &transaction), "notTheAuthor");

    let (mut reply, create_reply) = create_with(
        &setup,
        &setup.stranger,
        REPLY,
        vec![
            ("postId", id_value(post.id())),
            ("quote", "hello".into()),
            ("body", "reply".into()),
        ],
    )
    .await;
    assert_success(&setup.process(&create_reply, &transaction));

    let remove = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&remove, &transaction));
    assert!(setup.post_removal(post.id(), Some(&transaction)).is_some());

    // The record keeps the text and the author, so the rules read them still
    reply.set("body", "an edit".into());
    let edit = replacement(&setup.stranger, &setup.contract, REPLY, &mut reply).await;
    assert_success(&setup.process(&edit, &transaction));

    reply.set("quote", "goodbye".into());
    let misquote = replacement(&setup.stranger, &setup.contract, REPLY, &mut reply).await;
    assert_rule_broken(&setup.process(&misquote, &transaction), "quotesThePost");
}

/// A restore brings a document back as it was, checking no reference, and
/// judges no rule reading one: the topic a removed thread post names has
/// closed since, which a new post can not be written on, and the restore
/// still brings the post back.
#[tokio::test]
async fn should_restore_a_document_without_judging_a_rule_reading_a_reference() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (mut topic, create_topic) = create_with(
        &setup,
        &setup.user,
        TOPIC,
        vec![("open", Value::Bool(true))],
    )
    .await;
    assert_success(&setup.process(&create_topic, &transaction));
    let (thread_post, create_thread_post) = create_with(
        &setup,
        &setup.stranger,
        THREAD_POST,
        vec![("topicId", id_value(topic.id())), ("text", "first".into())],
    )
    .await;
    assert_success(&setup.process(&create_thread_post, &transaction));
    let stored = setup
        .stored_document(THREAD_POST, thread_post.id(), Some(&transaction))
        .expect("expected the thread post to be stored");

    topic.set("open", Value::Bool(false));
    let close = replacement(&setup.user, &setup.contract, TOPIC, &mut topic).await;
    assert_success(&setup.process(&close, &transaction));
    let (_, late) = create_with(
        &setup,
        &setup.stranger,
        THREAD_POST,
        vec![("topicId", id_value(topic.id())), ("text", "late".into())],
    )
    .await;
    assert_rule_broken(&setup.process(&late, &transaction), "topicOpen");

    let remove = setup
        .moderate(
            &setup.moderator,
            delete_action(THREAD_POST, thread_post.id()),
        )
        .await;
    assert_success(&setup.process(&remove, &transaction));
    let restore = setup
        .moderate(
            &setup.moderator,
            restore_action(THREAD_POST, setup.document_bytes(THREAD_POST, &stored)),
        )
        .await;
    assert_success(&setup.process(&restore, &transaction));
    assert!(setup
        .stored_document(THREAD_POST, thread_post.id(), Some(&transaction))
        .is_some());
}
