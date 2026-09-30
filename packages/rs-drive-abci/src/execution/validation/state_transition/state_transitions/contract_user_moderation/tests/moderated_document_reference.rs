//! `refersTo: moderatedDocument` (protocol version 14): a reference to a document type whose
//! documents leave state only when the contract's moderators remove them, each removal leaving
//! a record. The document type is `post` with `canBeDeleted: false` beside
//! `moderatorAbilities.delete`, whose records are kept by default.
//!
//! What sets it apart from `deletableDocument` is what a replace does once a moderator removed
//! the referenced post: the reference is kept, resolving to the post's removal record, and is
//! re-validated only as a `permanentDocument` one is. A `where` pair asked about again is
//! checked against the record where the record can answer it (the post's owner and id, and the
//! properties its type keeps under `moderatorAbilities.deleteKeepsFields`) and refused where
//! it can not.

use super::*;

const REPLY: &str = "reply";
const QUOTE: &str = "quote";
const THREAD: &str = "thread";
const REFERENCED_ENTITY_NOT_FOUND: u32 = 40120;
const REFERENCED_DOCUMENT_PROPERTY_MISMATCH: u32 = 40127;
const REFERENCED_DOCUMENT_TYPE_NOT_MODERATED: u32 = 40143;
const REFERENCED_DOCUMENT_REMOVED: u32 = 40145;

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

/// A post only the moderators take down: its author can not delete it, and every removal
/// leaves a record
fn moderated_post_schema() -> Value {
    post_schema_with(platform_value!({ "canBeDeleted": false }))
}

/// A reply to a post, whose body its author may edit
fn reply_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "postId": identifier_property(
                0,
                platform_value!({ "type": "moderatedDocument", "documentType": POST }),
            ),
            "body": { "type": "string", "minLength": 1, "maxLength": 50, "position": 1 },
        },
        "required": ["postId", "body"],
        "additionalProperties": false,
    })
}

/// A quote of one of the writer's own posts, repeating its text: a `where` pair on the post's
/// text, and a writer gate on its owner
fn quote_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "postId": identifier_property(
                0,
                platform_value!({
                    "type": "moderatedDocument",
                    "documentType": POST,
                    "where": { "text": "quotedText", "$ownerId": "$ownerId" },
                }),
            ),
            "quotedText": { "type": "string", "maxLength": 50, "position": 1 },
            "body": { "type": "string", "minLength": 1, "maxLength": 50, "position": 2 },
        },
        "required": ["postId", "quotedText", "body"],
        "additionalProperties": false,
    })
}

/// A thread gathering its writer's own posts: a typed array of them behind a writer gate,
/// and one pinned inside an object beside a note
fn thread_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "title": { "type": "string", "minLength": 1, "maxLength": 50, "position": 0 },
            "postIds": {
                "type": "array",
                "minItems": 0,
                "maxItems": 5,
                "uniqueItems": true,
                "items": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "refersTo": {
                        "type": "moderatedDocument",
                        "documentType": POST,
                        "where": { "$ownerId": "$ownerId" },
                    },
                },
                "position": 1,
            },
            "meta": {
                "type": "object",
                "properties": {
                    "postId": identifier_property(
                        0,
                        platform_value!({ "type": "moderatedDocument", "documentType": POST }),
                    ),
                    "note": { "type": "string", "maxLength": 50, "position": 1 },
                },
                "additionalProperties": false,
                "position": 2,
            },
        },
        "required": ["title", "postIds", "meta"],
        "additionalProperties": false,
    })
}

/// A contract whose named moderator removes posts, with replies, quotes and threads referring
/// to them
async fn setup() -> Setup {
    Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(contract, POST, moderated_post_schema());
            add_document_type(contract, REPLY, reply_schema());
            add_document_type(contract, QUOTE, quote_schema());
            add_document_type(contract, THREAD, thread_schema());
        },
    )
    .await
}

impl Setup {
    /// A document of `document_type_name` by `actor` with `properties` set
    async fn create_with(
        &self,
        actor: &Actor,
        document_type_name: &str,
        properties: Vec<(&str, Value)>,
    ) -> (Document, StateTransition) {
        self.create_document_of_type_with(actor, document_type_name, |document| {
            for (property, value) in properties {
                document.set(property, value);
            }
        })
        .await
    }

    /// The replacement of `document` by `actor`, its owner, its revision bumped
    async fn replacement(
        &self,
        actor: &Actor,
        document_type_name: &str,
        document: &mut Document,
    ) -> StateTransition {
        document.increment_revision().expect("revision increments");
        BatchTransition::new_document_replacement_transition_from_document(
            document.clone(),
            self.contract
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
}

fn id_value(id: Identifier) -> Value {
    Value::Identifier(id.to_buffer())
}

/// A replace keeps a reply to a post a moderator removed, as a permanent reference would be
/// kept: nothing but the reference's own pairs asks about it, and the post is not gone
/// silently, its removal record stands in for it. What the reference can not do is newly name
/// a removed post, on a create or by a repoint; and once a moderator restores the post it is
/// the post again the reference resolves to.
#[tokio::test]
async fn should_keep_a_reference_to_a_post_a_moderator_removed() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let (other_post, create_other_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_other_post, &transaction));
    let stored_post = setup
        .stored_document(POST, post.id(), Some(&transaction))
        .expect("expected the post to be stored");

    let (mut reply, create_reply) = setup
        .create_with(
            &setup.stranger,
            REPLY,
            vec![("postId", id_value(post.id())), ("body", "first".into())],
        )
        .await;
    assert_success(&setup.process(&create_reply, &transaction));

    let (_, reply_to_nothing) = setup
        .create_with(
            &setup.stranger,
            REPLY,
            vec![
                ("postId", id_value(Identifier::random())),
                ("body", "lost".into()),
            ],
        )
        .await;
    assert_paid_with_code(
        &setup.process(&reply_to_nothing, &transaction),
        REFERENCED_ENTITY_NOT_FOUND,
    );

    let remove = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&remove, &transaction));
    assert!(setup
        .stored_document(POST, post.id(), Some(&transaction))
        .is_none());
    assert!(setup.post_removal(post.id(), Some(&transaction)).is_some());

    reply.set("body", "an edit".into());
    let edit = setup.replacement(&setup.stranger, REPLY, &mut reply).await;
    assert_success(&setup.process(&edit, &transaction));

    let (_, reply_to_removed) = setup
        .create_with(
            &setup.stranger,
            REPLY,
            vec![("postId", id_value(post.id())), ("body", "late".into())],
        )
        .await;
    assert_paid_with_code(
        &setup.process(&reply_to_removed, &transaction),
        REFERENCED_ENTITY_NOT_FOUND,
    );

    // A repoint is a new value: it must name a post in state
    let mut repointed = reply.clone();
    repointed.set("postId", id_value(Identifier::random()));
    let repoint = setup
        .replacement(&setup.stranger, REPLY, &mut repointed)
        .await;
    assert_paid_with_code(
        &setup.process(&repoint, &transaction),
        REFERENCED_ENTITY_NOT_FOUND,
    );
    reply.set("postId", id_value(other_post.id()));
    let repoint = setup.replacement(&setup.stranger, REPLY, &mut reply).await;
    assert_success(&setup.process(&repoint, &transaction));

    // Restored, the post is referred to as any other post in state
    let restore = setup
        .moderate(
            &setup.moderator,
            restore_action(POST, setup.document_bytes(POST, &stored_post)),
        )
        .await;
    assert_success(&setup.process(&restore, &transaction));
    let (_, reply_to_restored) = setup
        .create_with(
            &setup.stranger,
            REPLY,
            vec![
                ("postId", id_value(post.id())),
                ("body", "welcome back".into()),
            ],
        )
        .await;
    assert_success(&setup.process(&reply_to_restored, &transaction));
}

/// A removed post's record keeps its id and its owner, and here none of its properties. A
/// replace of a quote whose post a moderator removed re-checks the writer gate on every
/// replace, against the owner the record keeps, and a changed referring property of a pair on
/// the post's text, which the record can not answer and so refuses.
#[tokio::test]
async fn should_check_a_pair_against_the_record_of_a_removed_post() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let text = post.get("text").cloned().expect("expected the post's text");

    let (_, strangers_quote) = setup
        .create_with(
            &setup.stranger,
            QUOTE,
            vec![
                ("postId", id_value(post.id())),
                ("quotedText", text.clone()),
                ("body", "not mine".into()),
            ],
        )
        .await;
    assert_paid_with_code(
        &setup.process(&strangers_quote, &transaction),
        REFERENCED_DOCUMENT_PROPERTY_MISMATCH,
    );
    let (mut quote, create_quote) = setup
        .create_with(
            &setup.user,
            QUOTE,
            vec![
                ("postId", id_value(post.id())),
                ("quotedText", text.clone()),
                ("body", "mine".into()),
            ],
        )
        .await;
    assert_success(&setup.process(&create_quote, &transaction));

    let remove = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&remove, &transaction));

    // The writer gate is asked again, and the record says the post was the writer's
    quote.set("body", "an edit".into());
    let edit = setup.replacement(&setup.user, QUOTE, &mut quote).await;
    assert_success(&setup.process(&edit, &transaction));

    // The pair on the text is asked again once its referring property changes, and the
    // record keeps no text
    let mut requoted = quote.clone();
    requoted.set("quotedText", "something else".into());
    let requote = setup.replacement(&setup.user, QUOTE, &mut requoted).await;
    assert_paid_with_code(
        &setup.process(&requote, &transaction),
        REFERENCED_DOCUMENT_REMOVED,
    );
}

/// A removed post is kept wherever the stored document held it, not only as a top-level
/// value: as an element of a typed array the replace re-validates for its writer gate without
/// changing it, as an element the stored list held when the replace changes the list, and
/// inside an object whose other property the replace changes. A new element still has to
/// name a post in state.
#[tokio::test]
async fn should_keep_a_removed_post_held_in_a_list_or_an_object() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let mut posts = Vec::new();
    for _ in 0..3 {
        let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
        assert_success(&setup.process(&create_post, &transaction));
        posts.push(post.id());
    }
    let meta = |note: &str| {
        Value::Map(vec![
            (Value::Text("postId".to_string()), id_value(posts[0])),
            (Value::Text("note".to_string()), note.into()),
        ])
    };
    let (mut thread, create_thread) = setup
        .create_with(
            &setup.user,
            THREAD,
            vec![
                ("title", "a thread".into()),
                (
                    "postIds",
                    Value::Array(vec![id_value(posts[0]), id_value(posts[1])]),
                ),
                ("meta", meta("first")),
            ],
        )
        .await;
    assert_success(&setup.process(&create_thread, &transaction));

    let remove = setup
        .moderate(&setup.moderator, delete_action(POST, posts[0]))
        .await;
    assert_success(&setup.process(&remove, &transaction));

    // The list is unchanged, and its writer gate asks about every element: the removed post
    // resolves to its record, which names the writer as its owner
    thread.set("title", "retitled".into());
    let retitle = setup.replacement(&setup.user, THREAD, &mut thread).await;
    assert_success(&setup.process(&retitle, &transaction));

    // The object changed, the reference inside it did not
    thread.set("meta", meta("second"));
    let renote = setup.replacement(&setup.user, THREAD, &mut thread).await;
    assert_success(&setup.process(&renote, &transaction));

    // A changed list keeps the elements the stored list held; a new one must be in state
    let mut grown = thread.clone();
    grown.set(
        "postIds",
        Value::Array(vec![
            id_value(posts[0]),
            id_value(posts[1]),
            id_value(Identifier::random()),
        ]),
    );
    let grow = setup.replacement(&setup.user, THREAD, &mut grown).await;
    assert_paid_with_code(
        &setup.process(&grow, &transaction),
        REFERENCED_ENTITY_NOT_FOUND,
    );
    thread.set(
        "postIds",
        Value::Array(vec![
            id_value(posts[0]),
            id_value(posts[1]),
            id_value(posts[2]),
        ]),
    );
    let grow = setup.replacement(&setup.user, THREAD, &mut thread).await;
    assert_success(&setup.process(&grow, &transaction));
}

/// The three document references are disjoint: a moderated one names a document type whose
/// documents leave state only through a moderator's recorded removal, and no other. A type its
/// owners may delete from, one whose removals leave no record and one whose documents never
/// leave state are each refused.
#[tokio::test]
async fn should_refuse_a_moderated_reference_to_any_other_document_type() {
    let mut setup = Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(contract, POST, moderated_post_schema());
            // Deletable by its owner too
            add_document_type(contract, "draft", post_schema(true));
            // Removed by moderators without a record
            add_document_type(
                contract,
                "flash",
                post_schema_with(platform_value!({
                    "canBeDeleted": false,
                    "moderatorAbilities": { "delete": true, "deleteKeepsRecord": false },
                })),
            );
            // Never leaves state
            add_document_type(
                contract,
                "note",
                post_schema_with(platform_value!({
                    "canBeDeleted": false,
                    "moderatorAbilities": { "delete": false },
                })),
            );
        },
    )
    .await;
    let transaction = setup.platform.drive.grove.start_transaction();

    for target in ["draft", "flash", "note"] {
        let mut referring = setup.contract.clone();
        referring.increment_version();
        add_document_type(
            &mut referring,
            "bookmark",
            platform_value!({
                "type": "object",
                "properties": {
                    "targetId": identifier_property(
                        0,
                        platform_value!({ "type": "moderatedDocument", "documentType": target }),
                    ),
                },
                "required": ["targetId"],
                "additionalProperties": false,
            }),
        );
        let update = setup.contract_update(referring).await;
        assert_paid_with_code(
            &setup.process(&update, &transaction),
            REFERENCED_DOCUMENT_TYPE_NOT_MODERATED,
        );
    }

    let mut referring = setup.contract.clone();
    referring.increment_version();
    add_document_type(
        &mut referring,
        "bookmark",
        platform_value!({
            "type": "object",
            "properties": {
                "targetId": identifier_property(
                    0,
                    platform_value!({ "type": "moderatedDocument", "documentType": POST }),
                ),
            },
            "required": ["targetId"],
            "additionalProperties": false,
        }),
    );
    let update = setup.contract_update(referring.clone()).await;
    assert_success(&setup.process(&update, &transaction));
    setup.contract = referring;
}

/// A contract `stranger` registers with the identity nonce `identity_nonce`, whose `reply`
/// type refers with a reference of `kind` to the posts of `posts_contract`, its writer's own
/// (a writer gate, which every replace checks again)
fn contract_replying_to(
    setup: &Setup,
    posts_contract: Identifier,
    kind: &str,
    identity_nonce: IdentityNonce,
) -> DataContract {
    let mut contract = get_data_contract_fixture(
        Some(setup.stranger.id()),
        identity_nonce,
        PlatformVersion::latest().protocol_version,
    )
    .data_contract_owned();
    let mut schema = reply_schema();
    schema
        .get_mut("properties")
        .ok()
        .flatten()
        .and_then(|properties| properties.get_mut("postId").ok().flatten())
        .and_then(|post_id| post_id.get_mut("refersTo").ok().flatten())
        .expect("expected the refersTo declaration")
        .insert("contractId".to_string(), id_value(posts_contract))
        .expect("expected to name the posts contract");
    schema
        .get_mut("properties")
        .ok()
        .flatten()
        .and_then(|properties| properties.get_mut("postId").ok().flatten())
        .and_then(|post_id| post_id.get_mut("refersTo").ok().flatten())
        .expect("expected the refersTo declaration")
        .insert("type".to_string(), Value::Text(kind.to_string()))
        .expect("expected to set the reference kind");
    schema
        .get_mut("properties")
        .ok()
        .flatten()
        .and_then(|properties| properties.get_mut("postId").ok().flatten())
        .and_then(|post_id| post_id.get_mut("refersTo").ok().flatten())
        .expect("expected the refersTo declaration")
        .insert(
            "where".to_string(),
            platform_value!({ "$ownerId": "$ownerId" }),
        )
        .expect("expected to set the writer gate");
    add_document_type(&mut contract, REPLY, schema);
    contract
}

/// A moderated reference into another contract reads the removal record the referenced
/// contract keeps: the kind is checked against that contract's type at registration, and a
/// reply there keeps pointing at a post the other contract's moderators removed, its writer
/// gate checked on every edit against the owner the record under the posts contract keeps.
#[tokio::test]
async fn should_keep_a_reference_into_another_contract_to_a_post_its_moderators_removed() {
    let mut setup = setup().await;
    let posts_contract = setup.contract.clone();
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.stranger, POST).await;
    assert_success(&setup.process(&create_post, &transaction));

    // The other contract's type admits a moderated reference alone
    let identity_nonce = setup.stranger.identity_nonce();
    let deletable = contract_replying_to(
        &setup,
        posts_contract.id(),
        "deletableDocument",
        identity_nonce,
    );
    let create = DataContractCreateTransition::new_from_data_contract(
        deletable,
        identity_nonce,
        &setup.stranger.identity.clone().into_partial_identity_info(),
        CRITICAL_KEY_ID,
        &setup.stranger.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("expected to build the contract create");
    assert_paid_with_code(
        &setup.process(&create, &transaction),
        REFERENCED_DOCUMENT_TYPE_MODERATED,
    );
    let identity_nonce = setup.stranger.identity_nonce();
    let replies_contract = contract_replying_to(
        &setup,
        posts_contract.id(),
        "moderatedDocument",
        identity_nonce,
    );
    let create = DataContractCreateTransition::new_from_data_contract(
        replies_contract.clone(),
        identity_nonce,
        &setup.stranger.identity.clone().into_partial_identity_info(),
        CRITICAL_KEY_ID,
        &setup.stranger.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("expected to build the contract create");
    assert_success(&setup.process(&create, &transaction));
    // The create sets the stranger's nonce for the contract to the identity nonce it used
    setup.stranger.next_contract_nonce.set(identity_nonce + 1);

    // Documents of the replies contract are written through it
    setup.contract = replies_contract.clone();
    let (mut reply, create_reply) = setup
        .create_with(
            &setup.stranger,
            REPLY,
            vec![("postId", id_value(post.id())), ("body", "first".into())],
        )
        .await;
    assert_success(&setup.process(&create_reply, &transaction));

    // The posts contract's moderator removes the post
    setup.contract = posts_contract;
    let remove = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&remove, &transaction));

    // The record lives under the posts contract, which is where the reply's reference finds it
    setup.contract = replies_contract;
    reply.set("body", "an edit".into());
    let edit = setup.replacement(&setup.stranger, REPLY, &mut reply).await;
    assert_success(&setup.process(&edit, &transaction));
    let (_, reply_to_removed) = setup
        .create_with(
            &setup.stranger,
            REPLY,
            vec![("postId", id_value(post.id())), ("body", "late".into())],
        )
        .await;
    assert_paid_with_code(
        &setup.process(&reply_to_removed, &transaction),
        REFERENCED_ENTITY_NOT_FOUND,
    );
}

/// A quote whose repeated text sits inside an object beside a note: an edit of the note makes
/// the replace ask about the pair on the text again without changing the text
fn boxed_quote_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "postId": identifier_property(
                0,
                platform_value!({
                    "type": "moderatedDocument",
                    "documentType": POST,
                    "where": { "text": "quoted.text" },
                }),
            ),
            "quoted": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                    "note": { "type": "string", "maxLength": 50, "position": 1 },
                },
                "additionalProperties": false,
                "position": 1,
            },
        },
        "required": ["postId", "quoted"],
        "additionalProperties": false,
    })
}

/// A contract whose posts' removal records keep their text or not, with boxed quotes of them
async fn setup_with_boxed_quotes(keeps_text: bool) -> Setup {
    let abilities = if keeps_text {
        platform_value!({ "delete": true, "deleteKeepsFields": ["text"] })
    } else {
        platform_value!({ "delete": true })
    };
    Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({
                    "canBeDeleted": false,
                    "moderatorAbilities": abilities,
                })),
            );
            add_document_type(contract, QUOTE, boxed_quote_schema());
        },
    )
    .await
}

/// A pair on a property the removed post's record keeps (`deleteKeepsFields`) is checked
/// against the kept value, as against the post itself: the same text holds, another one is a
/// mismatch. On a type whose records keep no text the same edit is refused, the record having
/// nothing to answer with.
#[tokio::test]
async fn should_check_a_pair_against_a_value_the_record_keeps() {
    for keeps_text in [true, false] {
        let setup = setup_with_boxed_quotes(keeps_text).await;
        let transaction = setup.platform.drive.grove.start_transaction();
        let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
        assert_success(&setup.process(&create_post, &transaction));
        let text = post.get("text").cloned().expect("expected the post's text");
        let quoted = |text: Value, note: &str| {
            Value::Map(vec![
                (Value::Text("text".to_string()), text),
                (Value::Text("note".to_string()), note.into()),
            ])
        };
        let (mut quote, create_quote) = setup
            .create_with(
                &setup.user,
                QUOTE,
                vec![
                    ("postId", id_value(post.id())),
                    ("quoted", quoted(text.clone(), "first")),
                ],
            )
            .await;
        assert_success(&setup.process(&create_quote, &transaction));

        let remove = setup
            .moderate(&setup.moderator, delete_action(POST, post.id()))
            .await;
        assert_success(&setup.process(&remove, &transaction));

        // The object changed, so the pair on the text inside it is asked about again
        quote.set("quoted", quoted(text.clone(), "second"));
        let renote = setup.replacement(&setup.user, QUOTE, &mut quote).await;
        if keeps_text {
            assert_success(&setup.process(&renote, &transaction));
        } else {
            assert_paid_with_code(
                &setup.process(&renote, &transaction),
                REFERENCED_DOCUMENT_REMOVED,
            );
            continue;
        }

        // A text the kept one is not is a mismatch
        let mut requoted = quote.clone();
        requoted.set("quoted", quoted("something else".into(), "second"));
        let requote = setup.replacement(&setup.user, QUOTE, &mut requoted).await;
        assert_paid_with_code(
            &setup.process(&requote, &transaction),
            REFERENCED_DOCUMENT_PROPERTY_MISMATCH,
        );
    }
}
