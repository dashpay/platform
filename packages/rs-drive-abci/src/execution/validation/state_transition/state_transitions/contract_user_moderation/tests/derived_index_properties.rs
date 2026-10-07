//! Derived index properties (protocol version 14): a reply indexed by `postId.$ownerId` is
//! filed under the owner of the post it replies to, which the reply never stores. Drive reads
//! the owner from the post whenever it writes or removes the reply's entries, and from the
//! post's removal record once a moderator removed the post. A permanent post's fixed
//! properties can be read the same way (`postId.text`), and so can a moderated post's fixed
//! properties its removal record keeps (`moderatorAbilities.deleteKeepsFields`).

use super::*;
use drive::drive::RootTree;
use drive::grovedb::query_result_type::QueryResultType;
use drive::grovedb::{PathQuery, Query, SizedQuery};

const REPLY: &str = "reply";

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

/// A reply its author may edit but never repoint, indexed by `derived` then `$createdAt`
fn reply_schema(reference_type: &str, derived: &str) -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": true,
        "immutable": ["postId"],
        "indices": [
            {
                "name": "byDerived",
                "properties": [{ derived: "asc" }, { "$createdAt": "asc" }],
            },
        ],
        "properties": {
            "postId": identifier_property(
                0,
                platform_value!({ "type": reference_type, "documentType": POST }),
            ),
            "body": { "type": "string", "minLength": 1, "maxLength": 50, "position": 1 },
        },
        "required": ["postId", "body", "$createdAt"],
        "additionalProperties": false,
    })
}

/// Posts only the moderators take down, on the record, and replies filed under the post's
/// owner
async fn moderated_setup() -> Setup {
    Setup::new_at_with(
        Some(moderation(false, false, THE_MODERATOR)),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({ "canBeDeleted": false })),
            );
            add_document_type(
                contract,
                REPLY,
                reply_schema("moderatedDocument", "postId.$ownerId"),
            );
        },
    )
    .await
}

/// Posts nobody changes or removes, and replies filed under the post's text. The contract
/// keeps a banlist, so its moderation declares something besides the posts it does not touch
async fn permanent_setup() -> Setup {
    Setup::new_at_with(
        Some(moderation(true, false, THE_MODERATOR)),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({
                    "canBeDeleted": false,
                    "documentsMutable": false,
                    "moderatorAbilities": { "delete": false },
                })),
            );
            add_document_type(
                contract,
                REPLY,
                reply_schema("permanentDocument", "postId.text"),
            );
        },
    )
    .await
}

/// Posts nobody changes, only the moderators take down, each removal record keeping the
/// post's text, and replies filed under that text
async fn kept_text_setup() -> Setup {
    Setup::new_at_with(
        Some(moderation(false, false, THE_MODERATOR)),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({
                    "canBeDeleted": false,
                    "documentsMutable": false,
                    "moderatorAbilities": { "delete": true, "deleteKeepsFields": ["text"] },
                })),
            );
            add_document_type(
                contract,
                REPLY,
                reply_schema("moderatedDocument", "postId.text"),
            );
        },
    )
    .await
}

/// `schema` with every top-level entry of `extra` in place of the one with its key
fn with_entries(mut schema: Value, extra: Value) -> Value {
    if let (Value::Map(schema_map), Value::Map(extra_map)) = (&mut schema, extra) {
        for (key, value) in extra_map {
            schema_map.retain(|(existing, _)| existing != &key);
            schema_map.push((key, value));
        }
    }
    schema
}

/// Posts nobody changes, only the moderators take down, each removal record keeping the
/// post's optional topic and its `meta` object whole; replies filed under both, which the
/// moderators may take down too
async fn kept_fields_setup() -> Setup {
    Setup::new_at_with(
        Some(moderation(false, false, THE_MODERATOR)),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({
                    "canBeDeleted": false,
                    "documentsMutable": false,
                    "properties": {
                        "text": { "type": "string", "maxLength": 50, "position": 0 },
                        "topic": { "type": "string", "maxLength": 20, "position": 1 },
                        "meta": {
                            "type": "object",
                            "properties": {
                                "tag": { "type": "string", "maxLength": 20, "position": 0 },
                            },
                            "additionalProperties": false,
                            "position": 2,
                        },
                    },
                    "moderatorAbilities": {
                        "delete": true,
                        "deleteKeepsFields": ["topic", "meta"],
                    },
                })),
            );
            add_document_type(
                contract,
                REPLY,
                with_entries(
                    reply_schema("moderatedDocument", "postId.topic"),
                    platform_value!({
                        "indices": [
                            {
                                "name": "byTopic",
                                "properties": [{ "postId.topic": "asc" }, { "$createdAt": "asc" }],
                            },
                            {
                                "name": "byTag",
                                "properties": [
                                    { "postId.meta.tag": "asc" },
                                    { "$createdAt": "asc" },
                                ],
                            },
                        ],
                        "moderatorAbilities": { "delete": true },
                    }),
                ),
            );
        },
    )
    .await
}

fn id_value(id: Identifier) -> Value {
    Value::Identifier(id.to_buffer())
}

impl Setup {
    /// A reply by `actor` to `post_id`
    async fn reply_to(
        &self,
        actor: &Actor,
        post_id: Identifier,
        body: &str,
    ) -> (Document, StateTransition) {
        self.create_document_of_type_with(actor, REPLY, |document| {
            document.set("postId", id_value(post_id));
            document.set("body", body.into());
        })
        .await
    }

    /// The replacement of `reply` by `actor`, its owner, its revision bumped
    async fn reply_replacement(&self, actor: &Actor, reply: &mut Document) -> StateTransition {
        reply.increment_revision().expect("revision increments");
        BatchTransition::new_document_replacement_transition_from_document(
            reply.clone(),
            self.contract
                .document_type_for_name(REPLY)
                .expect("expected the reply type"),
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

    /// The deletion of `reply` by `actor`, its owner
    async fn reply_deletion(&self, actor: &Actor, reply: Document) -> StateTransition {
        BatchTransition::new_document_deletion_transition_from_document(
            reply,
            self.contract
                .document_type_for_name(REPLY)
                .expect("expected the reply type"),
            &actor.key,
            actor.contract_nonce(),
            0,
            None,
            &actor.signer,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("expected to build the deletion")
    }

    fn reply_query(
        &self,
        where_clause: Value,
        order_by: Value,
        start_after: Option<Identifier>,
    ) -> Result<DriveDocumentQuery<'_>, drive::error::Error> {
        DriveDocumentQuery::from_decomposed_values(
            where_clause,
            Some(order_by),
            Some(100),
            start_after.map(|id| id.to_buffer()),
            false,
            None,
            &self.contract,
            self.contract
                .document_type_for_name(REPLY)
                .expect("expected the reply type"),
            &self.platform.drive.config,
            PlatformVersion::latest(),
        )
    }

    /// The ids of the replies `where_clause` finds, in `$createdAt` order
    fn replies_found(&self, where_clause: Value, transaction: &Transaction) -> Vec<Identifier> {
        let query = self
            .reply_query(where_clause, platform_value!([["$createdAt", "asc"]]), None)
            .expect("expected the query to build");
        self.platform
            .drive
            .query_documents(query, None, false, Some(transaction), None)
            .expect("expected the query to run")
            .documents_owned()
            .into_iter()
            .map(|document| {
                // Never stored: the index holds it, the document does not
                assert!(document.properties().keys().all(|name| !name.contains('.')));
                document.id()
            })
            .collect()
    }

    /// The ids of the replies `where_clause` finds in committed state after `start_after`,
    /// in `$createdAt` order, through a proof the verifier accepts
    fn replies_proved(
        &self,
        where_clause: Value,
        start_after: Option<Identifier>,
    ) -> Vec<Identifier> {
        let platform_version = PlatformVersion::latest();
        let query = self
            .reply_query(
                where_clause,
                platform_value!([["$createdAt", "asc"]]),
                start_after,
            )
            .expect("expected the query to build");
        let (proof, _) = query
            .clone()
            .execute_with_proof(&self.platform.drive, None, None, platform_version)
            .expect("expected the query to prove");
        let (_, proved) = query
            .verify_proof(&proof, platform_version)
            .expect("expected the proof to verify");
        proved.iter().map(|document| document.id()).collect()
    }
}

fn owned_by(owner: Identifier) -> Value {
    platform_value!([["postId.$ownerId", "==", id_value(owner)]])
}

/// A reply is filed under the owner of the post it replies to: a query by that owner finds
/// it, directly and through a proof, and a query by anyone else does not. An edit leaves it
/// where it is, and its deletion takes it out.
#[tokio::test]
async fn should_file_a_reply_under_the_owner_of_its_post() {
    let setup = moderated_setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));

    let (mut reply, create_reply) = setup.reply_to(&setup.stranger, post.id(), "first").await;
    assert_success(&setup.process(&create_reply, &transaction));
    let (other_reply, create_other_reply) =
        setup.reply_to(&setup.moderator, post.id(), "second").await;
    assert_success(&setup.process(&create_other_reply, &transaction));

    let mut both = vec![reply.id(), other_reply.id()];
    both.sort();
    let mut found = setup.replies_found(owned_by(setup.user.id()), &transaction);
    found.sort();
    assert_eq!(found, both);
    assert!(setup
        .replies_found(owned_by(setup.stranger.id()), &transaction)
        .is_empty());

    // The reply stores its own properties only
    let stored = setup
        .stored_document(REPLY, reply.id(), Some(&transaction))
        .expect("expected the reply to be stored");
    assert!(stored.properties().get("postId.$ownerId").is_none());

    reply.set("body", "an edit".into());
    let edit = setup.reply_replacement(&setup.stranger, &mut reply).await;
    assert_success(&setup.process(&edit, &transaction));
    let mut found = setup.replies_found(owned_by(setup.user.id()), &transaction);
    found.sort();
    assert_eq!(found, both);

    let delete = setup.reply_deletion(&setup.stranger, reply).await;
    assert_success(&setup.process(&delete, &transaction));
    assert_eq!(
        setup.replies_found(owned_by(setup.user.id()), &transaction),
        vec![other_reply.id()]
    );

    // A verifier finds the same through a proof
    setup.commit(transaction);
    assert_eq!(
        setup.replies_proved(owned_by(setup.user.id()), None),
        vec![other_reply.id()]
    );
    assert!(setup
        .replies_proved(owned_by(setup.stranger.id()), None)
        .is_empty());
}

/// Once a moderator removes a post, its owner is read from the removal record: a reply to it
/// is still found under that owner, can still be edited, and its deletion still takes its
/// entry out. A restored post is read again.
#[tokio::test]
async fn should_read_the_owner_of_a_removed_post_from_its_removal_record() {
    let setup = moderated_setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let stored_post = setup
        .stored_document(POST, post.id(), Some(&transaction))
        .expect("expected the post to be stored");
    let (mut reply, create_reply) = setup.reply_to(&setup.stranger, post.id(), "first").await;
    assert_success(&setup.process(&create_reply, &transaction));
    let (other_reply, create_other_reply) =
        setup.reply_to(&setup.stranger, post.id(), "second").await;
    assert_success(&setup.process(&create_other_reply, &transaction));

    let remove = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&remove, &transaction));
    assert!(setup
        .stored_document(POST, post.id(), Some(&transaction))
        .is_none());

    // Still filed under the post's owner
    assert_eq!(
        setup
            .replies_found(owned_by(setup.user.id()), &transaction)
            .len(),
        2
    );

    // An edit moves nothing: the owner the record keeps is the one the entry holds
    reply.set("body", "an edit".into());
    let edit = setup.reply_replacement(&setup.stranger, &mut reply).await;
    assert_success(&setup.process(&edit, &transaction));

    // A deletion finds the entry under the owner the record keeps
    let delete = setup.reply_deletion(&setup.stranger, reply).await;
    assert_success(&setup.process(&delete, &transaction));
    assert_eq!(
        setup.replies_found(owned_by(setup.user.id()), &transaction),
        vec![other_reply.id()]
    );

    // Restored, the post is read again
    let restore = setup
        .moderate(
            &setup.moderator,
            restore_action(POST, setup.document_bytes(POST, &stored_post)),
        )
        .await;
    assert_success(&setup.process(&restore, &transaction));
    let delete = setup.reply_deletion(&setup.stranger, other_reply).await;
    assert_success(&setup.process(&delete, &transaction));
    assert!(setup
        .replies_found(owned_by(setup.user.id()), &transaction)
        .is_empty());
}

/// Once a moderator removes a post whose removal record keeps its text, the text is read from
/// the record: a reply to it is still found under that text, directly and through a proof, can
/// still be edited, and its deletion still takes its entry out.
#[tokio::test]
async fn should_read_a_kept_field_of_a_removed_post_from_its_removal_record() {
    let setup = kept_text_setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup
        .create_document_of_type_with(&setup.user, POST, |document| {
            document.set("text", "hello".into());
        })
        .await;
    assert_success(&setup.process(&create_post, &transaction));
    let (mut reply, create_reply) = setup.reply_to(&setup.stranger, post.id(), "first").await;
    assert_success(&setup.process(&create_reply, &transaction));
    let (other_reply, create_other_reply) =
        setup.reply_to(&setup.stranger, post.id(), "second").await;
    assert_success(&setup.process(&create_other_reply, &transaction));

    let by_text = |text: &str| platform_value!([["postId.text", "==", text]]);
    let mut both = vec![reply.id(), other_reply.id()];
    both.sort();

    let remove = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&remove, &transaction));
    assert!(setup
        .stored_document(POST, post.id(), Some(&transaction))
        .is_none());

    // Still filed under the post's text
    let mut found = setup.replies_found(by_text("hello"), &transaction);
    found.sort();
    assert_eq!(found, both);

    // An edit moves nothing: the text the record keeps is the one the entry holds
    reply.set("body", "an edit".into());
    let edit = setup.reply_replacement(&setup.stranger, &mut reply).await;
    assert_success(&setup.process(&edit, &transaction));
    let mut found = setup.replies_found(by_text("hello"), &transaction);
    found.sort();
    assert_eq!(found, both);

    // A deletion finds the entry under the text the record keeps
    let delete = setup.reply_deletion(&setup.stranger, reply).await;
    assert_success(&setup.process(&delete, &transaction));
    assert_eq!(
        setup.replies_found(by_text("hello"), &transaction),
        vec![other_reply.id()]
    );

    setup.commit(transaction);
    assert_eq!(
        setup.replies_proved(by_text("hello"), None),
        vec![other_reply.id()]
    );
    assert!(setup.replies_proved(by_text("goodbye"), None).is_empty());
}

/// A removed post's kept values key a reply wherever the post did: a member of an object the
/// record keeps whole, and an optional field the post never held, under null. Two derived
/// properties read through one reference, and a moderator's removal and restore of a reply
/// take its entries out and insert them again under the values the record keeps.
#[tokio::test]
async fn should_key_a_reply_by_the_kept_values_of_a_removed_post() {
    let setup = kept_fields_setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (tagged, create_tagged) = setup
        .create_document_of_type_with(&setup.user, POST, |document| {
            document.set("text", "hello".into());
            document.set("topic", "dash".into());
            document.set("meta", platform_value!({ "tag": "privacy" }));
        })
        .await;
    assert_success(&setup.process(&create_tagged, &transaction));
    let (bare, create_bare) = setup
        .create_document_of_type_with(&setup.user, POST, |document| {
            document.set("text", "no topic".into());
            // The builder fills every optional field: this post holds neither
            document.remove("topic");
            document.remove("meta");
        })
        .await;
    assert_success(&setup.process(&create_bare, &transaction));

    let (mut reply, create_reply) = setup.reply_to(&setup.stranger, tagged.id(), "first").await;
    assert_success(&setup.process(&create_reply, &transaction));
    let (spam, create_spam) = setup.reply_to(&setup.stranger, tagged.id(), "spam").await;
    assert_success(&setup.process(&create_spam, &transaction));
    let stored_spam = setup
        .stored_document(REPLY, spam.id(), Some(&transaction))
        .expect("expected the reply to be stored");
    let (mut bare_reply, create_bare_reply) = setup
        .reply_to(&setup.stranger, bare.id(), "to nothing")
        .await;
    assert_success(&setup.process(&create_bare_reply, &transaction));

    for post_id in [tagged.id(), bare.id()] {
        let remove = setup
            .moderate(&setup.moderator, delete_action(POST, post_id))
            .await;
        assert_success(&setup.process(&remove, &transaction));
    }

    let by_topic = |topic: &str| platform_value!([["postId.topic", "==", topic]]);
    let by_tag = |tag: &str| platform_value!([["postId.meta.tag", "==", tag]]);
    let found = |where_clause: Value| {
        let mut found = setup.replies_found(where_clause, &transaction);
        found.sort();
        found
    };
    let mut both = vec![reply.id(), spam.id()];
    both.sort();
    assert_eq!(found(by_topic("dash")), both);
    assert_eq!(found(by_tag("privacy")), both);

    // An edit reads both values from the record and moves nothing
    reply.set("body", "an edit".into());
    let edit = setup.reply_replacement(&setup.stranger, &mut reply).await;
    assert_success(&setup.process(&edit, &transaction));
    assert_eq!(found(by_topic("dash")), both);
    assert_eq!(found(by_tag("privacy")), both);

    // A moderator's removal of a reply finds its entries under the kept values, and a restore
    // inserts them there again: a wrong value would file it where no query finds it
    let remove_spam = setup
        .moderate(&setup.moderator, delete_action(REPLY, spam.id()))
        .await;
    assert_success(&setup.process(&remove_spam, &transaction));
    assert_eq!(found(by_topic("dash")), vec![reply.id()]);
    assert_eq!(found(by_tag("privacy")), vec![reply.id()]);
    let restore_spam = setup
        .moderate(
            &setup.moderator,
            restore_action(REPLY, setup.document_bytes(REPLY, &stored_spam)),
        )
        .await;
    assert_success(&setup.process(&restore_spam, &transaction));
    assert_eq!(found(by_topic("dash")), both);
    assert_eq!(found(by_tag("privacy")), both);

    // A reply to a post that held neither is filed under null for both, and the record,
    // keeping nothing for them, finds its entries there: an edit and a deletion succeed
    bare_reply.set("body", "still nothing".into());
    let edit = setup
        .reply_replacement(&setup.stranger, &mut bare_reply)
        .await;
    assert_success(&setup.process(&edit, &transaction));
    let delete = setup.reply_deletion(&setup.stranger, bare_reply).await;
    assert_success(&setup.process(&delete, &transaction));

    // A verifier finds the same through a proof
    setup.commit(transaction);
    let mut proved = setup.replies_proved(by_tag("privacy"), None);
    proved.sort();
    assert_eq!(proved, both);
}

/// A moderator's removal of a reply takes its entry out, and a restore puts it back, both
/// reading the owner of its post again.
#[tokio::test]
async fn should_take_out_and_put_back_the_entry_of_a_reply_a_moderator_removes() {
    let setup = Setup::new_at_with(
        Some(moderation(false, false, THE_MODERATOR)),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({ "canBeDeleted": false })),
            );
            let mut reply = reply_schema("moderatedDocument", "postId.$ownerId");
            if let Value::Map(map) = &mut reply {
                map.push((
                    Value::Text("moderatorAbilities".to_string()),
                    platform_value!({ "delete": true }),
                ));
            }
            add_document_type(contract, REPLY, reply);
        },
    )
    .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let (reply, create_reply) = setup.reply_to(&setup.stranger, post.id(), "spam").await;
    assert_success(&setup.process(&create_reply, &transaction));
    let stored_reply = setup
        .stored_document(REPLY, reply.id(), Some(&transaction))
        .expect("expected the reply to be stored");

    let remove = setup
        .moderate(&setup.moderator, delete_action(REPLY, reply.id()))
        .await;
    assert_success(&setup.process(&remove, &transaction));
    assert!(setup
        .replies_found(owned_by(setup.user.id()), &transaction)
        .is_empty());

    let restore = setup
        .moderate(
            &setup.moderator,
            restore_action(REPLY, setup.document_bytes(REPLY, &stored_reply)),
        )
        .await;
    assert_success(&setup.process(&restore, &transaction));
    assert_eq!(
        setup.replies_found(owned_by(setup.user.id()), &transaction),
        vec![reply.id()]
    );
}

/// A permanent post's fixed property can be derived too: a reply is found by the text of the
/// post it replies to.
#[tokio::test]
async fn should_file_a_reply_under_a_fixed_property_of_a_permanent_post() {
    let setup = permanent_setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup
        .create_document_of_type_with(&setup.user, POST, |document| {
            document.set("text", "hello".into());
        })
        .await;
    assert_success(&setup.process(&create_post, &transaction));
    let (reply, create_reply) = setup.reply_to(&setup.stranger, post.id(), "first").await;
    assert_success(&setup.process(&create_reply, &transaction));

    let by_text = |text: &str| platform_value!([["postId.text", "==", text]]);
    assert_eq!(
        setup.replies_found(by_text("hello"), &transaction),
        vec![reply.id()]
    );
    assert!(setup
        .replies_found(by_text("goodbye"), &transaction)
        .is_empty());

    let delete = setup.reply_deletion(&setup.stranger, reply).await;
    assert_success(&setup.process(&delete, &transaction));
    assert!(setup
        .replies_found(by_text("hello"), &transaction)
        .is_empty());
}

/// A cursor places the page by what the document it names stores, which a derived value is
/// not: a query fixing the derived property with `==` pages with one, a range over it can not.
#[tokio::test]
async fn should_page_with_a_cursor_only_where_the_query_fixes_the_derived_property() {
    let setup = moderated_setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let (reply, create_reply) = setup.reply_to(&setup.stranger, post.id(), "first").await;
    assert_success(&setup.process(&create_reply, &transaction));
    let (other_reply, create_other_reply) =
        setup.reply_to(&setup.stranger, post.id(), "second").await;
    assert_success(&setup.process(&create_other_reply, &transaction));

    let mut all = vec![reply.id(), other_reply.id()];
    all.sort();
    let first = all[0];
    let query = setup
        .reply_query(
            owned_by(setup.user.id()),
            platform_value!([["$createdAt", "asc"]]),
            Some(first),
        )
        .expect("expected the query to build");
    let after_first: Vec<Identifier> = setup
        .platform
        .drive
        .query_documents(query, None, false, Some(&transaction), None)
        .expect("a cursor pages a query fixing the derived property")
        .documents_owned()
        .into_iter()
        .map(|document| document.id())
        .collect();
    assert_eq!(after_first, vec![all[1]]);
    // The verifier places the cursor the same way, from the query
    setup.commit(transaction);
    assert_eq!(
        setup.replies_proved(owned_by(setup.user.id()), Some(first)),
        vec![all[1]]
    );
    let transaction = setup.platform.drive.grove.start_transaction();

    let query = setup
        .reply_query(
            platform_value!([["postId.$ownerId", ">", id_value(Identifier::new([0; 32]))]]),
            platform_value!([["postId.$ownerId", "asc"]]),
            Some(first),
        )
        .expect("expected the query to build");
    let error = setup
        .platform
        .drive
        .query_documents(query, None, false, Some(&transaction), None)
        .expect_err("a cursor can not be placed by a derived value");
    assert!(error.to_string().contains("postId.$ownerId"), "got {error}");
}

/// Posts nobody changes, only the moderators take down, each removal record keeping the
/// post's optional topic and its `meta` object whole. A reply may also quote a post, and the
/// moderators may take replies down. `byTopic` files a reply by its post's topic and
/// `byQuotedOwner` by the owner of the post it quotes, each skipping a reply whose value is
/// absent; `byTag` keeps a reply whose post has no tag under null.
async fn skipping_setup() -> Setup {
    Setup::new_at_with(
        Some(moderation(false, false, THE_MODERATOR)),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({
                    "canBeDeleted": false,
                    "documentsMutable": false,
                    "properties": {
                        "text": { "type": "string", "maxLength": 50, "position": 0 },
                        "topic": { "type": "string", "maxLength": 20, "position": 1 },
                        "meta": {
                            "type": "object",
                            "properties": {
                                "tag": { "type": "string", "maxLength": 20, "position": 0 },
                            },
                            "additionalProperties": false,
                            "position": 2,
                        },
                    },
                    "moderatorAbilities": {
                        "delete": true,
                        "deleteKeepsFields": ["topic", "meta"],
                    },
                })),
            );
            add_document_type(
                contract,
                REPLY,
                with_entries(
                    reply_schema("moderatedDocument", "postId.topic"),
                    platform_value!({
                        "immutable": ["postId", "quoteId"],
                        "indices": [
                            {
                                "name": "byTopic",
                                "properties": [{ "postId.topic": "asc" }, { "$createdAt": "asc" }],
                                "skipIfAbsent": ["postId.topic"],
                            },
                            {
                                "name": "byQuotedOwner",
                                "properties": [
                                    { "quoteId.$ownerId": "asc" },
                                    { "$createdAt": "asc" },
                                ],
                                "skipIfAbsent": ["quoteId.$ownerId"],
                            },
                            {
                                "name": "byTag",
                                "properties": [
                                    { "postId.meta.tag": "asc" },
                                    { "$createdAt": "asc" },
                                ],
                            },
                        ],
                        "properties": {
                            "postId": identifier_property(
                                0,
                                platform_value!({ "type": "moderatedDocument", "documentType": POST }),
                            ),
                            "body": { "type": "string", "minLength": 1, "maxLength": 50, "position": 1 },
                            "quoteId": identifier_property(
                                2,
                                platform_value!({ "type": "moderatedDocument", "documentType": POST }),
                            ),
                        },
                        "moderatorAbilities": { "delete": true },
                    }),
                ),
            );
        },
    )
    .await
}

impl Setup {
    /// A reply by `actor` to `post_id`, quoting `quote_id` when given
    async fn quoting_reply_to(
        &self,
        actor: &Actor,
        post_id: Identifier,
        quote_id: Option<Identifier>,
        body: &str,
    ) -> (Document, StateTransition) {
        self.create_document_of_type_with(actor, REPLY, |document| {
            document.set("postId", id_value(post_id));
            document.set("body", body.into());
            // The builder fills every optional field
            match quote_id {
                Some(quote_id) => document.set("quoteId", id_value(quote_id)),
                None => {
                    document.remove("quoteId");
                }
            }
        })
        .await
    }

    /// The values the reply index level keyed by `property` holds: every key directly under
    /// it, the empty key standing for null
    fn reply_index_values(&self, property: &str, transaction: &Transaction) -> Vec<Vec<u8>> {
        let path = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract.id().to_vec(),
            vec![1],
            REPLY.as_bytes().to_vec(),
            property.as_bytes().to_vec(),
        ];
        let mut query = Query::new();
        query.insert_all();
        let (elements, _) = self
            .platform
            .drive
            .grove_get_raw_path_query(
                &PathQuery::new(path, SizedQuery::new(query, None, None)),
                Some(transaction),
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &PlatformVersion::latest().drive,
            )
            .expect("expected to read the index level");
        elements.to_keys()
    }
}

/// An index skipping on a derived property leaves out a reply whose value is absent: one
/// whose post has no topic, and one quoting no post, while an index without the flag files
/// them under null. The skipped reply is still edited, deleted, removed and restored, its
/// value read from the removal record once the moderators remove its post.
#[tokio::test]
async fn should_skip_a_reply_whose_derived_value_is_absent() {
    let setup = skipping_setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (topical, create_topical) = setup
        .create_document_of_type_with(&setup.user, POST, |document| {
            document.set("text", "hello".into());
            document.set("topic", "dash".into());
            document.set("meta", platform_value!({ "tag": "privacy" }));
        })
        .await;
    assert_success(&setup.process(&create_topical, &transaction));
    let (bare, create_bare) = setup
        .create_document_of_type_with(&setup.user, POST, |document| {
            document.set("text", "no topic".into());
            // The builder fills every optional field: this post holds neither
            document.remove("topic");
            document.remove("meta");
        })
        .await;
    assert_success(&setup.process(&create_bare, &transaction));

    // On topic, quoting nothing
    let (mut on_topic, create_on_topic) = setup
        .quoting_reply_to(&setup.stranger, topical.id(), None, "on topic")
        .await;
    assert_success(&setup.process(&create_on_topic, &transaction));
    // Off topic, quoting the topical post
    let (quoting, create_quoting) = setup
        .quoting_reply_to(&setup.stranger, bare.id(), Some(topical.id()), "quoting")
        .await;
    assert_success(&setup.process(&create_quoting, &transaction));
    let stored_quoting = setup
        .stored_document(REPLY, quoting.id(), Some(&transaction))
        .expect("expected the reply to be stored");
    // Off topic, quoting nothing: both skip indexes leave it out
    let (mut neither, create_neither) = setup
        .quoting_reply_to(&setup.stranger, bare.id(), None, "neither")
        .await;
    assert_success(&setup.process(&create_neither, &transaction));

    let user = setup.user.id().to_vec();
    let null = Vec::<u8>::new();
    assert_eq!(
        setup.reply_index_values("postId.topic", &transaction),
        vec![b"dash".to_vec()],
        "byTopic writes no null value for the replies to the post without a topic"
    );
    assert_eq!(
        setup.reply_index_values("quoteId.$ownerId", &transaction),
        vec![user.clone()],
        "byQuotedOwner writes no null value for the replies quoting nothing"
    );
    assert!(
        setup
            .reply_index_values("postId.meta.tag", &transaction)
            .contains(&null),
        "byTag does not skip: the replies to the post without a tag are under null"
    );

    let by_topic = |topic: &str| platform_value!([["postId.topic", "==", topic]]);
    let by_quoted_owner =
        |owner: Identifier| platform_value!([["quoteId.$ownerId", "==", id_value(owner)]]);
    assert_eq!(
        setup.replies_found(by_topic("dash"), &transaction),
        vec![on_topic.id()]
    );
    assert_eq!(
        setup.replies_found(by_quoted_owner(setup.user.id()), &transaction),
        vec![quoting.id()]
    );

    // An edit moves nothing, into an index or out of one
    on_topic.set("body", "an edit".into());
    let edit = setup
        .reply_replacement(&setup.stranger, &mut on_topic)
        .await;
    assert_success(&setup.process(&edit, &transaction));
    neither.set("body", "still neither".into());
    let edit = setup.reply_replacement(&setup.stranger, &mut neither).await;
    assert_success(&setup.process(&edit, &transaction));
    assert_eq!(
        setup.reply_index_values("postId.topic", &transaction),
        vec![b"dash".to_vec()]
    );
    assert_eq!(
        setup.reply_index_values("quoteId.$ownerId", &transaction),
        vec![user.clone()]
    );

    // Once the moderators remove both posts, the records keep the topic and the owner, and
    // keep no topic for the post that had none
    for post_id in [topical.id(), bare.id()] {
        let remove = setup
            .moderate(&setup.moderator, delete_action(POST, post_id))
            .await;
        assert_success(&setup.process(&remove, &transaction));
    }
    on_topic.set("body", "a second edit".into());
    let edit = setup
        .reply_replacement(&setup.stranger, &mut on_topic)
        .await;
    assert_success(&setup.process(&edit, &transaction));
    assert_eq!(
        setup.replies_found(by_topic("dash"), &transaction),
        vec![on_topic.id()]
    );
    // The skipped reply's deletion looks for nothing in either skip index
    let delete = setup.reply_deletion(&setup.stranger, neither).await;
    assert_success(&setup.process(&delete, &transaction));

    // A moderator's removal of the quoting reply takes it out of the index it is in, and a
    // restore puts it back, reading the quoted post's owner from its record
    let remove_quoting = setup
        .moderate(&setup.moderator, delete_action(REPLY, quoting.id()))
        .await;
    assert_success(&setup.process(&remove_quoting, &transaction));
    assert!(setup
        .reply_index_values("quoteId.$ownerId", &transaction)
        .is_empty());
    let restore_quoting = setup
        .moderate(
            &setup.moderator,
            restore_action(REPLY, setup.document_bytes(REPLY, &stored_quoting)),
        )
        .await;
    assert_success(&setup.process(&restore_quoting, &transaction));
    assert_eq!(
        setup.replies_found(by_quoted_owner(setup.user.id()), &transaction),
        vec![quoting.id()]
    );
    assert_eq!(
        setup.reply_index_values("postId.topic", &transaction),
        vec![b"dash".to_vec()]
    );

    // A verifier finds the same through a proof
    setup.commit(transaction);
    assert_eq!(
        setup.replies_proved(by_topic("dash"), None),
        vec![on_topic.id()]
    );
    assert_eq!(
        setup.replies_proved(by_quoted_owner(setup.user.id()), None),
        vec![quoting.id()]
    );
}
