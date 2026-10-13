//! `propertyConstraints` rules reading the document a `moderatedDocument`
//! reference points at (protocol version 14). Once a moderator removes the
//! post a reply names, the reply's rules read the post's removal record, which
//! keeps its owner, its id and the fields its type lists under
//! `deleteKeepsFields`, the only ones such a rule may read. A moderator's
//! restore checks no reference, and judges no rule reading one. The `where`
//! entries that are not an equality are checked against a removal record as
//! the equalities are.

use super::*;
use dpp::consensus::basic::document::PropertyConstraintViolation;
use dpp::consensus::basic::BasicError;

const REPLY: &str = "reply";
const TOPIC: &str = "topic";
const THREAD_POST: &str = "threadPost";
const LISTING: &str = "listing";
const OFFER: &str = "offer";
const ARTICLE: &str = "article";
const COMMENT: &str = "comment";
const STALL: &str = "stall";

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
        "indices": [
            { "name": "byTopic", "properties": [{ "topicId": "asc" }], "countable": "countable" }
        ],
        "propertyConstraints": {
            "topicOpen": { "equal": [{ "referenced": ["topicId", "open"] }, 1] }
        },
        "additionalProperties": false,
    })
}

/// A listing moderators take down, each removal keeping its price and its
/// end, not its stock
fn listing_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": false,
        "moderatorAbilities": { "delete": true, "deleteKeepsFields": ["price", "endsAt"] },
        "properties": {
            "price": { "type": "integer", "minimum": 0, "maximum": 1000000000, "position": 0 },
            "stock": { "type": "integer", "minimum": 0, "maximum": 1000, "position": 1 },
            "endsAt": { "type": "integer", "minimum": 0, "maximum": 9007199254740991u64, "position": 2 },
        },
        "required": ["price", "stock", "endsAt"],
        "additionalProperties": false,
    })
}

/// An offer on a listing: a bid at least its price, for no more than its
/// stock, while it has not ended
fn offer_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": false,
        "properties": {
            "listingId": identifier_property(
                0,
                platform_value!({
                    "type": "moderatedDocument",
                    "documentType": LISTING,
                    "where": {
                        "price": { "lessThanOrEqual": "bid" },
                        "stock": { "greaterThanOrEqual": "quantity" },
                        "endsAt": { "greaterThan": "$updatedAt" },
                    },
                }),
            ),
            "bid": { "type": "integer", "minimum": 0, "maximum": 1000000000, "position": 1 },
            "quantity": { "type": "integer", "minimum": 1, "maximum": 1000, "position": 2 },
            "note": { "type": "string", "maxLength": 50, "position": 3 },
        },
        "required": ["listingId", "bid", "quantity", "$updatedAt"],
        "additionalProperties": false,
    })
}

/// An article its owner may close, which moderators take down, each removal
/// keeping its text and its status
fn article_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": false,
        "moderatorAbilities": { "delete": true, "deleteKeepsFields": ["text", "status"] },
        "properties": {
            "text": { "type": "string", "minLength": 1, "maxLength": 50, "position": 0 },
            "status": { "type": "string", "enum": ["open", "closed"], "maxLength": 10, "position": 1 },
        },
        "required": ["text", "status"],
        "additionalProperties": false,
    })
}

/// A comment on an open article, which may quote it
fn comment_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": false,
        "properties": {
            "articleId": identifier_property(
                0,
                platform_value!({
                    "type": "moderatedDocument",
                    "documentType": ARTICLE,
                    "where": { "status": { "const": "open" } },
                }),
            ),
            "quote": { "type": "string", "maxLength": 50, "position": 1 },
            "body": { "type": "string", "minLength": 1, "maxLength": 50, "position": 2 },
        },
        "required": ["articleId", "body"],
        "propertyConstraints": {
            "quotesTheArticle": {
                "anyOf": [
                    { "absent": "quote" },
                    { "equal": ["quote", { "referenced": ["articleId", "text"] }] }
                ]
            }
        },
        "additionalProperties": false,
    })
}

/// A stall on a topic, written only while the topic is open, with a note
/// only moderators write. Its rule reads no total: registration refuses a rule
/// reading a referenced document beside one on a type keeping fields for its
/// moderators, whose change is judged against every rule reading a total
fn stall_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": false,
        "moderatorAbilities": { "changeFields": ["note"] },
        "properties": {
            "topicId": identifier_property(
                0,
                platform_value!({ "type": "permanentDocument", "documentType": TOPIC }),
            ),
            "note": { "type": "string", "maxLength": 50, "position": 1 },
        },
        "required": ["topicId"],
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
            add_document_type(contract, LISTING, listing_schema());
            add_document_type(contract, OFFER, offer_schema());
            add_document_type(contract, ARTICLE, article_schema());
            add_document_type(contract, COMMENT, comment_schema());
            add_document_type(contract, STALL, stall_schema());
        },
    )
    .await
}

fn id_value(id: Identifier) -> Value {
    Value::Identifier(id.to_buffer())
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

fn assert_where_refused(execution: &StateTransitionExecutionResult, condition: &str) {
    assert!(
        matches!(
            execution,
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(e)
                ),
                ..
            } if e.referring_property() == condition
        ),
        "expected the where entry {condition} to refuse the write, got {execution:?}"
    );
}

/// Once a moderator removes the listing an offer names, a replace of the
/// offer checks the `where` entries it asks about against the listing's
/// removal record: one bounded by a property the replace changes, against a
/// field the record keeps (the price) or refused for one it does not (the
/// stock), and one bounded by `$updatedAt` on every replace, at the time of
/// the replace. An entry bounded by a property the replace leaves alone is
/// not checked, though the record no longer holds what it reads.
#[tokio::test]
async fn should_check_where_conditions_against_a_removal_record() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let ends_at = 2 * BLOCK_TIME_MS;
    let (listing, create_listing) = create_with(
        &setup,
        &setup.user,
        LISTING,
        vec![
            ("price", Value::U32(100)),
            ("stock", Value::U16(5)),
            ("endsAt", Value::U64(ends_at)),
        ],
    )
    .await;
    assert_success(&setup.process(&create_listing, &transaction));
    let (mut offer, create_offer) = create_with(
        &setup,
        &setup.stranger,
        OFFER,
        vec![
            ("listingId", id_value(listing.id())),
            ("bid", Value::U32(150)),
            ("quantity", Value::U16(3)),
        ],
    )
    .await;
    assert_success(&setup.process(&create_offer, &transaction));

    let remove = setup
        .moderate(&setup.moderator, delete_action(LISTING, listing.id()))
        .await;
    assert_success(&setup.process(&remove, &transaction));

    // Only the end is asked about, kept and still ahead: the stock, which the
    // record does not keep, is not read
    offer.set("note", "still keen".into());
    let edit = replacement(&setup.stranger, &setup.contract, OFFER, &mut offer).await;
    assert_success(&setup.process(&edit, &transaction));

    // A changed bid asks about the kept price again, the quantity left alone
    let mut low_bid = offer.clone();
    low_bid.set("bid", Value::U32(50));
    let refused = replacement(&setup.stranger, &setup.contract, OFFER, &mut low_bid).await;
    assert_where_refused(
        &setup.process(&refused, &transaction),
        "{ \"lessThanOrEqual\": \"bid\" }",
    );

    // A changed quantity asks about the stock, which no record keeps
    let mut more = offer.clone();
    more.set("quantity", Value::U16(4));
    let refused = replacement(&setup.stranger, &setup.contract, OFFER, &mut more).await;
    let execution = setup.process(&refused, &transaction);
    assert!(
        matches!(
            &execution,
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedDocumentRemovedError(e)),
                ..
            } if e.referenced_property() == "stock"
        ),
        "expected the unkept stock to refuse the replace, got {execution:?}"
    );

    // The end is asked about on every replace, at the replace's own time
    let mut late = offer.clone();
    late.set("note", "too late".into());
    let refused = replacement(&setup.stranger, &setup.contract, OFFER, &mut late).await;
    assert_where_refused(
        &setup.process_at(&refused, ends_at, &transaction),
        "{ \"greaterThan\": \"$updatedAt\" }",
    );
}

/// A rule reads the article a comment names, so every replace of the comment
/// checks the reference, its `where` whole: once the article's owner closes it,
/// no comment on it is edited, and a moderator's removal of the article
/// changes nothing, the record keeping the closed status.
#[tokio::test]
async fn should_check_every_where_entry_of_a_removed_document_a_rule_reads() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (mut article, create_article) = create_with(
        &setup,
        &setup.user,
        ARTICLE,
        vec![("text", "hello".into()), ("status", "open".into())],
    )
    .await;
    assert_success(&setup.process(&create_article, &transaction));
    let (comment, create_comment) = create_with(
        &setup,
        &setup.stranger,
        COMMENT,
        vec![
            ("articleId", id_value(article.id())),
            ("quote", "hello".into()),
            ("body", "nice".into()),
        ],
    )
    .await;
    assert_success(&setup.process(&create_comment, &transaction));

    article.set("status", "closed".into());
    let close = replacement(&setup.user, &setup.contract, ARTICLE, &mut article).await;
    assert_success(&setup.process(&close, &transaction));
    let mut edit = comment.clone();
    edit.set("body", "an edit".into());
    let refused = replacement(&setup.stranger, &setup.contract, COMMENT, &mut edit).await;
    assert_where_refused(
        &setup.process(&refused, &transaction),
        "{ \"const\": \"open\" }",
    );

    let remove = setup
        .moderate(&setup.moderator, delete_action(ARTICLE, article.id()))
        .await;
    assert_success(&setup.process(&remove, &transaction));
    let mut edit = comment.clone();
    edit.set("body", "another edit".into());
    let refused = replacement(&setup.stranger, &setup.contract, COMMENT, &mut edit).await;
    assert_where_refused(
        &setup.process(&refused, &transaction),
        "{ \"const\": \"open\" }",
    );
}

/// A moderator's change of a field judges no rule reading a referenced
/// document: the stall's rule, reading whether its topic is open, is left for
/// the owner's writes, so a moderator changes the note of a stall whose topic
/// has closed.
#[tokio::test]
async fn should_change_a_field_without_judging_a_rule_reading_a_reference() {
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
    let (stall, create_stall) = create_with(
        &setup,
        &setup.stranger,
        STALL,
        vec![("topicId", id_value(topic.id()))],
    )
    .await;
    assert_success(&setup.process(&create_stall, &transaction));

    topic.set("open", Value::Bool(false));
    let close = replacement(&setup.user, &setup.contract, TOPIC, &mut topic).await;
    assert_success(&setup.process(&close, &transaction));

    let change = setup
        .moderate(
            &setup.moderator,
            ContractUserModerationAction::ChangeDocumentFields {
                document_type_name: STALL.to_string(),
                document_id: stall.id(),
                fields: BTreeMap::from([("note".to_string(), Value::Text("tidy".to_string()))]),
                reason: ContractModerationReason::from_text("handled"),
            },
        )
        .await;
    assert_success(&setup.process(&change, &transaction));
}
