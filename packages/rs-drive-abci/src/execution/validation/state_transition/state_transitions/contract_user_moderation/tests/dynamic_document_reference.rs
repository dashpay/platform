//! A document reference naming its target through the referring document
//! (`contractId` and `documentType` written `"$self.<property>"`, `documentTypeRequirements`,
//! protocol version 15): the document says which contract and which document type its value points into, so
//! everything about the target is checked when the document is written. The shared fixture's
//! target is `post`, whose documents only the contract's moderators remove, each removal leaving
//! a record. A reference may also admit a type of either kind, permanent or moderated, and
//! ask of a property filling a slot what the type says of it (`when`).

use super::*;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::DocumentReferenceKind;
use drive::drive::document::paths::document_history_path;
use drive::grovedb::Element;
use drive::query::document_history_drive_query::{
    DocumentHistoryDriveQuery, DocumentHistoryFilter, DocumentHistoryState,
};

const NOTE: &str = "note";
const BYTES_NOTE: &str = "bytesNote";
const NAMED_NOTE: &str = "namedNote";
const PLAIN: &str = "plain";
const EITHER_NOTE: &str = "eitherNote";
const KEPT_POST: &str = "keptPost";
const ARTICLE: &str = "article";
const PERMANENT_NOTE: &str = "permanentNote";
const TALLY: &str = "tally";
const REFERENCED_ENTITY_NOT_FOUND: u32 = 40120;
const REFERENCED_DOCUMENT_PROPERTY_MISMATCH: u32 = 40127;
const REFERENCED_DOCUMENT_TYPE_NOT_FOUND: u32 = 40121;
const REFERENCED_DOCUMENT_TYPE_NOT_MODERATED: u32 = 40143;
const REFERENCED_DOCUMENT_TYPE_INDEX_ONLY: u32 = 40146;
const REFERENCED_DOCUMENT_TYPE_REQUIREMENT_NOT_MET: u32 = 40153;
const REFERENCED_DOCUMENT_TYPE_DELETABLE: u32 = 40122;

/// Retained revisions never satisfy an existence reference after deletion. A dynamic
/// permanent reference still refuses the deletable type before looking for its document.
#[tokio::test]
async fn should_resolve_history_reference_targets_by_current_existence_and_kind() {
    let version = PlatformVersion::latest();
    let setup = Setup::new_at_with(None, version, |contract| {
        let mut target = plain_schema();
        for keyword in [
            "documentsKeepHistory",
            "documentsMutable",
            "canBeDeleted",
            "canBeErased",
        ] {
            target.insert(keyword.into(), true.into()).unwrap();
        }
        add_document_type(contract, POST, target);
        let mut schema = note_schema(text_or_bytes());
        schema
            .get_mut("properties")
            .unwrap()
            .unwrap()
            .get_mut("descriptionId")
            .unwrap()
            .unwrap()
            .insert(
                "refersTo".into(),
                platform_value!({
                    "type": "deletableDocument",
                    "documentType": POST,
                }),
            )
            .unwrap();
        add_document_type(contract, NOTE, schema);
        add_document_type(contract, PERMANENT_NOTE, permanent_note_schema());
    })
    .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let (mut post, create_post) =
        create_with(&setup, &setup.user, POST, vec![("text", "retained".into())]).await;
    assert_success(&setup.process(&create_post, &transaction));
    let chunk = version
        .system_limits
        .max_document_revisions_erased_per_transition
        .unwrap();
    for _ in 0..chunk {
        let replace = replacement(&setup.user, &setup.contract, POST, &mut post).await;
        assert_success(&setup.process(&replace, &transaction));
    }
    let document_type = setup.contract.document_type_for_name(POST).unwrap();
    for (state, remaining) in [
        (DocumentHistoryState::Active, u64::from(chunk) + 1),
        (DocumentHistoryState::Deleted, u64::from(chunk) + 1),
        (DocumentHistoryState::Erasing, 1),
        (DocumentHistoryState::Absent, 0),
    ] {
        match state {
            DocumentHistoryState::Active => {}
            DocumentHistoryState::Deleted => {
                let deletion = BatchTransition::new_document_deletion_transition_from_document(
                    post.clone(),
                    document_type,
                    &setup.user.key,
                    setup.user.contract_nonce(),
                    0,
                    None,
                    &setup.user.signer,
                    version,
                    None,
                )
                .await
                .unwrap();
                assert_success(&setup.process(&deletion, &transaction));
            }
            DocumentHistoryState::Erasing | DocumentHistoryState::Absent => {
                let erase = BatchTransition::new_document_erase_transition_from_document(
                    post.clone(),
                    document_type,
                    &setup.user.key,
                    setup.user.contract_nonce(),
                    0,
                    &setup.user.signer,
                    version,
                    None,
                )
                .await
                .unwrap();
                assert_success(&setup.process(&erase, &transaction));
            }
        }
        let history = setup
            .platform
            .drive
            .fetch_document_history(
                &DocumentHistoryDriveQuery {
                    contract_id: setup.contract.id().to_buffer(),
                    document_type_name: POST.into(),
                    document_id: post.id().to_buffer(),
                    filter: DocumentHistoryFilter::StartAtTime(0),
                    limit: None,
                },
                document_type,
                Some(&transaction),
                version,
            )
            .unwrap();
        let lifecycle = history.lifecycle.unwrap();
        assert_eq!(lifecycle.state, state);
        assert_eq!(lifecycle.remaining_revisions, remaining);

        let (_, reference) = create_with(
            &setup,
            &setup.stranger,
            NOTE,
            note_naming(setup.contract.id(), POST, post.id()),
        )
        .await;
        let result = setup.process(&reference, &transaction);
        if state == DocumentHistoryState::Active {
            assert_success(&result);
        } else {
            assert_paid_with_code(&result, REFERENCED_ENTITY_NOT_FOUND);
        }
        let (_, dynamic) = create_with(
            &setup,
            &setup.stranger,
            PERMANENT_NOTE,
            note_naming(setup.contract.id(), POST, post.id()),
        )
        .await;
        assert_paid_with_code(
            &setup.process(&dynamic, &transaction),
            REFERENCED_DOCUMENT_TYPE_DELETABLE,
        );
    }
}

fn id_value(id: Identifier) -> Value {
    Value::Identifier(id.to_buffer())
}

fn identifier_property(position: u64) -> Value {
    platform_value!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": position,
    })
}

/// A note describing itself with a moderated document of a type it names, which must meet
/// `requirements`. `textProperty` is there for a slot to read its property name from.
fn note_schema(requirements: Value) -> Value {
    note_schema_of(platform_value!("moderatedDocument"), requirements)
}

/// The same, describing itself with a document of the kind, or of the kinds, `kind` lists.
fn note_schema_of(kind: Value, requirements: Value) -> Value {
    let mut description_id = identifier_property(0);
    description_id
        .insert(
            "refersTo".to_string(),
            platform_value!({
                "type": kind,
                "contractId": "$self.descriptionContractId",
                "documentType": "$self.descriptionDocumentType",
                "documentTypeRequirements": requirements,
            }),
        )
        .expect("expected to declare the reference");
    platform_value!({
        "type": "object",
        "properties": {
            "descriptionId": description_id,
            "descriptionContractId": identifier_property(1),
            "descriptionDocumentType": { "type": "string", "minLength": 1, "maxLength": 64, "position": 2 },
            "textProperty": { "type": "string", "minLength": 1, "maxLength": 64, "position": 3 },
            "body": { "type": "string", "minLength": 1, "maxLength": 50, "position": 4 },
        },
        "required": [
            "descriptionId",
            "descriptionContractId",
            "descriptionDocumentType",
            "textProperty",
            "body"
        ],
        "additionalProperties": false,
    })
}

/// A type holding text, as a string or as bytes: `post` holds a string
fn text_or_bytes() -> Value {
    platform_value!({
        "properties": {
            "body": { "name": "$any", "type": "string" },
            "bodyBytes": { "name": "$any", "type": "array", "byteArray": true },
        },
        "require": { "anyOf": ["body", "bodyBytes"] },
    })
}

/// A document type anyone's documents of which their owner deletes: not a moderated one
fn plain_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "text": { "type": "string", "maxLength": 50, "position": 0 },
        },
        "required": ["text"],
        "additionalProperties": false,
    })
}

/// A document type kept only as index entries, which nobody deletes: its documents never leave
/// state, and none can be fetched by its id
fn tally_schema() -> Value {
    platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "canBeDeleted": false,
        "properties": {
            "label": { "type": "string", "minLength": 1, "maxLength": 63, "position": 0 },
        },
        "indices": [{
            "name": "byLabel",
            "properties": [{ "label": "asc" }],
            "terminal": "$ownerId",
        }],
        "required": ["label"],
        "additionalProperties": false,
    })
}

/// A note describing itself with a permanent document of a type it names, of which it asks
/// nothing else.
fn permanent_note_schema() -> Value {
    let mut schema = note_schema(text_or_bytes());
    let refers_to = platform_value!({
        "type": "permanentDocument",
        "contractId": "$self.descriptionContractId",
        "documentType": "$self.descriptionDocumentType",
    });
    schema
        .get_mut("properties")
        .expect("expected properties")
        .expect("expected properties")
        .get_mut("descriptionId")
        .expect("expected the reference property")
        .expect("expected the reference property")
        .insert("refersTo".to_string(), refers_to)
        .expect("expected to declare the reference");
    schema
}

/// A contract whose named moderator removes posts, with notes referring to a document of a
/// type they name: `note` asks for text or bytes, `bytesNote` for bytes, and `namedNote` for
/// a string property the note names.
async fn setup() -> Setup {
    Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({ "canBeDeleted": false })),
            );
            add_document_type(contract, PLAIN, plain_schema());
            add_document_type(contract, TALLY, tally_schema());
            add_document_type(contract, NOTE, note_schema(text_or_bytes()));
            add_document_type(contract, PERMANENT_NOTE, permanent_note_schema());
            add_document_type(
                contract,
                BYTES_NOTE,
                note_schema(platform_value!({
                    "properties": {
                        "bodyBytes": { "name": "$any", "type": "array", "byteArray": true },
                    },
                })),
            );
            add_document_type(
                contract,
                NAMED_NOTE,
                note_schema(platform_value!({
                    "properties": {
                        "text": { "name": "$self.textProperty", "type": "string" },
                    },
                })),
            );
        },
    )
    .await
}

/// The properties of a note describing itself with `document_id` of `document_type_name` in
/// `contract_id`
fn note_naming(
    contract_id: Identifier,
    document_type_name: &str,
    document_id: Identifier,
) -> Vec<(&'static str, Value)> {
    vec![
        ("descriptionId", id_value(document_id)),
        ("descriptionContractId", id_value(contract_id)),
        ("descriptionDocumentType", document_type_name.into()),
        ("textProperty", "text".into()),
        ("body", "first".into()),
    ]
}

/// The document names where its reference points: the contract and the type must exist, the
/// type must be a moderated one, and the document must be there.
#[tokio::test]
async fn should_check_a_reference_into_the_document_type_the_document_names() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let contract_id = setup.contract.id();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let (plain, create_plain) = setup.create_document_of_type(&setup.user, PLAIN).await;
    assert_success(&setup.process(&create_plain, &transaction));

    let (_, create_note) = create_with(
        &setup,
        &setup.stranger,
        NOTE,
        note_naming(contract_id, POST, post.id()),
    )
    .await;
    assert_success(&setup.process(&create_note, &transaction));

    // A contract that does not exist, and a type the contract does not have
    let (_, no_contract) = create_with(
        &setup,
        &setup.stranger,
        NOTE,
        note_naming(Identifier::random(), POST, post.id()),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&no_contract, &transaction),
        REFERENCED_DOCUMENT_TYPE_NOT_FOUND,
    );
    let (_, no_type) = create_with(
        &setup,
        &setup.stranger,
        NOTE,
        note_naming(contract_id, "essay", post.id()),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&no_type, &transaction),
        REFERENCED_DOCUMENT_TYPE_NOT_FOUND,
    );

    // A type whose documents their owner deletes is not one a moderated reference names
    let (_, not_moderated) = create_with(
        &setup,
        &setup.stranger,
        NOTE,
        note_naming(contract_id, PLAIN, plain.id()),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&not_moderated, &transaction),
        REFERENCED_DOCUMENT_TYPE_NOT_MODERATED,
    );

    // A document that is not there, though its type qualifies
    let (_, no_document) = create_with(
        &setup,
        &setup.stranger,
        NOTE,
        note_naming(contract_id, POST, Identifier::random()),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&no_document, &transaction),
        REFERENCED_ENTITY_NOT_FOUND,
    );

    // A type kept only as index entries has no document to find by its id. Its documents
    // never leave state, so it is of the kind a permanent reference admits: it is refused,
    // paid, before anything is fetched, whatever the reference asks of the type
    assert!(setup
        .contract
        .document_type_for_name(TALLY)
        .expect("expected the tally type")
        .index_only());
    for note_type in [PERMANENT_NOTE, NOTE] {
        let (_, index_only) = create_with(
            &setup,
            &setup.stranger,
            note_type,
            note_naming(contract_id, TALLY, Identifier::random()),
        )
        .await;
        assert_paid_with_code(
            &setup.process(&index_only, &transaction),
            REFERENCED_DOCUMENT_TYPE_INDEX_ONLY,
        );
    }
}

/// The named type must have the properties the reference's requirements ask for: `post`
/// holds a string, so it fills a string slot and not a byte array one, and a slot reading
/// its property name from the note is filled by the property the note names.
#[tokio::test]
async fn should_hold_the_named_document_type_to_the_requirements() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let contract_id = setup.contract.id();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));

    let (_, needs_bytes) = create_with(
        &setup,
        &setup.stranger,
        BYTES_NOTE,
        note_naming(contract_id, POST, post.id()),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&needs_bytes, &transaction),
        REFERENCED_DOCUMENT_TYPE_REQUIREMENT_NOT_MET,
    );

    // `textProperty` names `text`, the post's string
    let (_, names_the_text) = create_with(
        &setup,
        &setup.stranger,
        NAMED_NOTE,
        note_naming(contract_id, POST, post.id()),
    )
    .await;
    assert_success(&setup.process(&names_the_text, &transaction));
    // and a property the post does not have fills nothing
    let mut names_nothing = note_naming(contract_id, POST, post.id());
    names_nothing[3] = ("textProperty", "summary".into());
    let (_, names_nothing) = create_with(&setup, &setup.stranger, NAMED_NOTE, names_nothing).await;
    assert_paid_with_code(
        &setup.process(&names_nothing, &transaction),
        REFERENCED_DOCUMENT_TYPE_REQUIREMENT_NOT_MET,
    );
}

/// A replace keeps a note describing itself with a post a moderator removed, as it keeps a
/// moderated reference naming its type in the schema. A replace that changes where the note
/// says its reference points checks the reference again, against the new target.
#[tokio::test]
async fn should_keep_a_dynamic_reference_until_a_replace_points_it_elsewhere() {
    let setup = setup().await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let contract_id = setup.contract.id();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let (other_post, create_other_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_other_post, &transaction));

    let (mut note, create_note) = create_with(
        &setup,
        &setup.stranger,
        NOTE,
        note_naming(contract_id, POST, post.id()),
    )
    .await;
    assert_success(&setup.process(&create_note, &transaction));

    let remove = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&remove, &transaction));
    assert!(setup
        .stored_document(POST, post.id(), Some(&transaction))
        .is_none());

    // The reference is kept: nothing it reads changed
    note.set("body", "an edit".into());
    let edit = replacement(&setup.stranger, &setup.contract, NOTE, &mut note).await;
    assert_success(&setup.process(&edit, &transaction));

    // A new note cannot name the removed post
    let (_, names_removed) = create_with(
        &setup,
        &setup.stranger,
        NOTE,
        note_naming(contract_id, POST, post.id()),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&names_removed, &transaction),
        REFERENCED_ENTITY_NOT_FOUND,
    );

    // Naming another type points the same id somewhere else: the reference is checked
    // again, and that type is not a moderated one
    let mut retyped = note.clone();
    retyped.set("descriptionDocumentType", PLAIN.into());
    let retype = replacement(&setup.stranger, &setup.contract, NOTE, &mut retyped).await;
    assert_paid_with_code(
        &setup.process(&retype, &transaction),
        REFERENCED_DOCUMENT_TYPE_NOT_MODERATED,
    );
    // Naming another contract too
    let mut moved = note.clone();
    moved.set("descriptionContractId", id_value(Identifier::random()));
    let move_it = replacement(&setup.stranger, &setup.contract, NOTE, &mut moved).await;
    assert_paid_with_code(
        &setup.process(&move_it, &transaction),
        REFERENCED_DOCUMENT_TYPE_NOT_FOUND,
    );
    // and a new value names a post in state
    note.set("descriptionId", id_value(other_post.id()));
    let repoint = replacement(&setup.stranger, &setup.contract, NOTE, &mut note).await;
    assert_success(&setup.process(&repoint, &transaction));

    // A slot reading its property name from the note is part of where the reference points:
    // naming another property checks the reference again, though its id, its contract and
    // its document type are the ones it was created with
    let (mut named, create_named) = create_with(
        &setup,
        &setup.stranger,
        NAMED_NOTE,
        note_naming(contract_id, POST, other_post.id()),
    )
    .await;
    assert_success(&setup.process(&create_named, &transaction));
    let mut renamed = named.clone();
    renamed.set("textProperty", "summary".into());
    let rename = replacement(&setup.stranger, &setup.contract, NAMED_NOTE, &mut renamed).await;
    assert_paid_with_code(
        &setup.process(&rename, &transaction),
        REFERENCED_DOCUMENT_TYPE_REQUIREMENT_NOT_MET,
    );
    // while an edit of another property keeps it
    named.set("body", "an edit".into());
    let edit = replacement(&setup.stranger, &setup.contract, NAMED_NOTE, &mut named).await;
    assert_success(&setup.process(&edit, &transaction));
}

/// The contract a note names need not be its own: a note of another contract describes
/// itself with a post of the posts contract, whose moderators' removal record it keeps
/// resolving to.
#[tokio::test]
async fn should_follow_a_dynamic_reference_into_another_contract() {
    let mut setup = setup().await;
    let posts_contract = setup.contract.clone();
    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));

    // Nothing about the posts contract is in the notes contract's schema
    let identity_nonce = setup.stranger.identity_nonce();
    let mut notes_contract = get_data_contract_fixture(
        Some(setup.stranger.id()),
        identity_nonce,
        PlatformVersion::latest().protocol_version,
    )
    .data_contract_owned();
    add_document_type(&mut notes_contract, NOTE, note_schema(text_or_bytes()));
    let create = DataContractCreateTransition::new_from_data_contract(
        notes_contract.clone(),
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

    // Documents of the notes contract are written through it
    setup.contract = notes_contract.clone();
    let (mut note, create_note) = create_with(
        &setup,
        &setup.stranger,
        NOTE,
        note_naming(posts_contract.id(), POST, post.id()),
    )
    .await;
    assert_success(&setup.process(&create_note, &transaction));
    // An index-only type of the posts contract is refused there as it is at home
    let (_, index_only) = create_with(
        &setup,
        &setup.stranger,
        NOTE,
        note_naming(posts_contract.id(), TALLY, Identifier::random()),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&index_only, &transaction),
        REFERENCED_DOCUMENT_TYPE_INDEX_ONLY,
    );
    // The notes contract has no `post` type of its own to find the post in
    let (_, names_itself) = create_with(
        &setup,
        &setup.stranger,
        NOTE,
        note_naming(notes_contract.id(), POST, post.id()),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&names_itself, &transaction),
        REFERENCED_DOCUMENT_TYPE_NOT_FOUND,
    );

    // The posts contract's moderator removes the post
    setup.contract = posts_contract.clone();
    let remove = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&remove, &transaction));

    // The note keeps its reference
    setup.contract = notes_contract;
    note.set("body", "an edit".into());
    let edit = replacement(&setup.stranger, &setup.contract, NOTE, &mut note).await;
    assert_success(&setup.process(&edit, &transaction));
}

/// Dynamic references check existence through the history pointer; a static reference's
/// agreement must read the current revision even while the old value remains in history.
#[tokio::test]
async fn should_check_dynamic_and_static_references_to_a_keep_history_target() {
    let platform_version = PlatformVersion::latest();
    let setup = Setup::new_at_with(None, platform_version, |contract| {
        add_document_type(
            contract,
            POST,
            platform_value!({
                "type": "object",
                "documentsKeepHistory": true,
                "documentsMutable": true,
                "canBeDeleted": false,
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                },
                "required": ["text"],
                "additionalProperties": false,
            }),
        );
        add_document_type(contract, PERMANENT_NOTE, permanent_note_schema());
        let mut post_id = identifier_property(0);
        post_id
            .insert(
                "refersTo".into(),
                platform_value!({
                    "type": "permanentDocument",
                    "documentType": POST,
                    "where": { "text": "quotedText" },
                }),
            )
            .expect("expected the reference declaration");
        add_document_type(
            contract,
            "quote",
            platform_value!({
                "type": "object",
                "properties": {
                    "postId": post_id,
                    "quotedText": { "type": "string", "maxLength": 50, "position": 1 },
                },
                "required": ["postId", "quotedText"],
                "additionalProperties": false,
            }),
        );
    })
    .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let contract_id = setup.contract.id();
    let (mut post, create_post) =
        create_with(&setup, &setup.user, POST, vec![("text", "first".into())]).await;
    assert_success(&setup.process(&create_post, &transaction));

    let (_, dynamic) = create_with(
        &setup,
        &setup.stranger,
        PERMANENT_NOTE,
        note_naming(contract_id, POST, post.id()),
    )
    .await;
    assert_success(&setup.process(&dynamic, &transaction));

    post.set("text", "current".into());
    let replace = replacement(&setup.user, &setup.contract, POST, &mut post).await;
    assert_success(&setup.process(&replace, &transaction));

    let mut primary_path =
        document_history_path(contract_id.as_bytes(), POST, post.id().as_bytes());
    primary_path.truncate(4);
    primary_path.push(vec![0]);
    let primary_entry = setup
        .platform
        .drive
        .grove
        .get_raw(
            primary_path.as_slice().into(),
            post.id().as_bytes(),
            Some(&transaction),
            &platform_version.drive.grove_version,
        )
        .value
        .expect("expected the primary entry");
    assert!(matches!(primary_entry, Element::Reference(..)));
    let history = setup
        .platform
        .drive
        .fetch_document_history(
            &DocumentHistoryDriveQuery {
                contract_id: contract_id.to_buffer(),
                document_type_name: POST.into(),
                document_id: post.id().to_buffer(),
                filter: DocumentHistoryFilter::StartAtTime(0),
                limit: None,
            },
            setup
                .contract
                .document_type_for_name(POST)
                .expect("post type"),
            Some(&transaction),
            platform_version,
        )
        .expect("expected both retained revisions");
    assert_eq!(
        history
            .entries
            .iter()
            .map(|entry| (entry.revision, entry.document.get("text").cloned()))
            .collect::<Vec<_>>(),
        [(1, Some("first".into())), (2, Some("current".into()))]
    );

    for (id, expected_code) in [
        (post.id(), None),
        (
            Identifier::new([255; 32]),
            Some(REFERENCED_ENTITY_NOT_FOUND),
        ),
    ] {
        let (_, dynamic) = create_with(
            &setup,
            &setup.stranger,
            PERMANENT_NOTE,
            note_naming(contract_id, POST, id),
        )
        .await;
        let result = setup.process(&dynamic, &transaction);
        if let Some(code) = expected_code {
            assert_paid_with_code(&result, code);
        } else {
            assert_success(&result);
        }
    }

    for (text, expected_code) in [
        ("first", Some(REFERENCED_DOCUMENT_PROPERTY_MISMATCH)),
        ("current", None),
    ] {
        let (_, quote) = create_with(
            &setup,
            &setup.stranger,
            "quote",
            vec![("postId", id_value(post.id())), ("quotedText", text.into())],
        )
        .await;
        let result = setup.process(&quote, &transaction);
        if let Some(code) = expected_code {
            assert_paid_with_code(&result, code);
        } else {
            assert_success(&result);
        }
    }
}

/// A string a removal takes with the document and its author writes, when the type is a
/// moderated one; any string of a permanent type.
fn text_removed_with_the_document() -> Value {
    platform_value!({
        "properties": {
            "body": {
                "name": "$any",
                "type": "string",
                "when": {
                    "ifThen": [
                        { "equal": ["$kind", { "const": "moderatedDocument" }] },
                        { "allOf": [
                            { "not": { "contains": ["$moderatorAbilities.deleteKeepsFields", "$property"] } },
                            { "not": { "contains": ["$moderatorAbilities.changeFields", "$property"] } },
                        ] },
                    ],
                },
            },
        },
    })
}

/// A reference listing both kinds admits a permanent type and a moderated one, and is
/// checked as the kind the named type is: a moderated type is held to the slot's `when`,
/// which asks that a removal take the text, a permanent one is not, and a type whose
/// documents their owner deletes is refused as it is for a permanent reference. Into a
/// moderated type it is kept as a moderated reference is, once the post is removed.
#[tokio::test]
async fn should_check_a_reference_of_either_kind_as_the_kind_the_named_type_is() {
    let setup = Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({ "canBeDeleted": false })),
            );
            // A moderated type whose removal record keeps the text
            add_document_type(
                contract,
                KEPT_POST,
                post_schema_with(platform_value!({
                    "canBeDeleted": false,
                    "moderatorAbilities": { "delete": true, "deleteKeepsFields": ["text"] },
                })),
            );
            // A permanent type: nobody deletes or changes an article
            add_document_type(
                contract,
                ARTICLE,
                platform_value!({
                    "type": "object",
                    "canBeDeleted": false,
                    "documentsMutable": false,
                    "properties": {
                        "text": { "type": "string", "maxLength": 50, "position": 0 },
                    },
                    "required": ["text"],
                    "additionalProperties": false,
                }),
            );
            add_document_type(contract, PLAIN, plain_schema());
            add_document_type(
                contract,
                EITHER_NOTE,
                note_schema_of(
                    platform_value!(["permanentDocument", "moderatedDocument"]),
                    text_removed_with_the_document(),
                ),
            );
            add_document_type(contract, NOTE, note_schema(text_or_bytes()));
        },
    )
    .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let contract_id = setup.contract.id();
    for (document_type_name, kind) in [
        (POST, DocumentReferenceKind::Moderated),
        (KEPT_POST, DocumentReferenceKind::Moderated),
        (ARTICLE, DocumentReferenceKind::Permanent),
        (PLAIN, DocumentReferenceKind::Deletable),
    ] {
        assert_eq!(
            setup
                .contract
                .document_type_for_name(document_type_name)
                .expect("expected the type")
                .document_reference_kind(),
            kind,
            "{document_type_name}"
        );
    }
    let (post, create_post) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create_post, &transaction));
    let (kept_post, create_kept_post) = setup.create_document_of_type(&setup.user, KEPT_POST).await;
    assert_success(&setup.process(&create_kept_post, &transaction));
    let (article, create_article) = setup.create_document_of_type(&setup.user, ARTICLE).await;
    assert_success(&setup.process(&create_article, &transaction));
    let (plain, create_plain) = setup.create_document_of_type(&setup.user, PLAIN).await;
    assert_success(&setup.process(&create_plain, &transaction));

    // A moderated type whose text a removal takes
    let (mut note, names_a_post) = create_with(
        &setup,
        &setup.stranger,
        EITHER_NOTE,
        note_naming(contract_id, POST, post.id()),
    )
    .await;
    assert_success(&setup.process(&names_a_post, &transaction));
    // A moderated type whose removal record keeps its one string: nothing fills the slot
    let (_, names_kept_text) = create_with(
        &setup,
        &setup.stranger,
        EITHER_NOTE,
        note_naming(contract_id, KEPT_POST, kept_post.id()),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&names_kept_text, &transaction),
        REFERENCED_DOCUMENT_TYPE_REQUIREMENT_NOT_MET,
    );
    // which a reference that asks only for text accepts
    let (_, asks_for_text) = create_with(
        &setup,
        &setup.stranger,
        NOTE,
        note_naming(contract_id, KEPT_POST, kept_post.id()),
    )
    .await;
    assert_success(&setup.process(&asks_for_text, &transaction));

    // A permanent type, which a moderated reference refuses and the condition asks
    // nothing of
    let (_, names_an_article) = create_with(
        &setup,
        &setup.stranger,
        EITHER_NOTE,
        note_naming(contract_id, ARTICLE, article.id()),
    )
    .await;
    assert_success(&setup.process(&names_an_article, &transaction));
    let (_, moderated_only) = create_with(
        &setup,
        &setup.stranger,
        NOTE,
        note_naming(contract_id, ARTICLE, article.id()),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&moderated_only, &transaction),
        REFERENCED_DOCUMENT_TYPE_NOT_MODERATED,
    );

    // A type whose documents their owner deletes is neither
    let (_, names_a_deletable) = create_with(
        &setup,
        &setup.stranger,
        EITHER_NOTE,
        note_naming(contract_id, PLAIN, plain.id()),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&names_a_deletable, &transaction),
        REFERENCED_DOCUMENT_TYPE_DELETABLE,
    );
    // and a document that is not there is missing, of either kind
    for document_type_name in [POST, ARTICLE] {
        let (_, no_document) = create_with(
            &setup,
            &setup.stranger,
            EITHER_NOTE,
            note_naming(contract_id, document_type_name, Identifier::random()),
        )
        .await;
        assert_paid_with_code(
            &setup.process(&no_document, &transaction),
            REFERENCED_ENTITY_NOT_FOUND,
        );
    }

    // Into a moderated type the reference is a moderated one: the note outlives the
    // post's removal, and no new note names the removed post
    let remove = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&remove, &transaction));
    note.set("body", "an edit".into());
    let edit = replacement(&setup.stranger, &setup.contract, EITHER_NOTE, &mut note).await;
    assert_success(&setup.process(&edit, &transaction));
    let (_, names_removed) = create_with(
        &setup,
        &setup.stranger,
        EITHER_NOTE,
        note_naming(contract_id, POST, post.id()),
    )
    .await;
    assert_paid_with_code(
        &setup.process(&names_removed, &transaction),
        REFERENCED_ENTITY_NOT_FOUND,
    );
    // A replace pointing the note at the article checks it again, as a permanent reference
    note.set("descriptionDocumentType", ARTICLE.into());
    note.set("descriptionId", id_value(article.id()));
    let repoint = replacement(&setup.stranger, &setup.contract, EITHER_NOTE, &mut note).await;
    assert_success(&setup.process(&repoint, &transaction));
}

/// The processing fee a write was charged, accepted or refused.
fn processing_fee(execution: &StateTransitionExecutionResult) -> u64 {
    match execution {
        StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } => {
            fee_result.processing_fee
        }
        StateTransitionExecutionResult::PaidConsensusError { actual_fees, .. } => {
            actual_fees.processing_fee
        }
        other => panic!("expected a paid execution, got {other:?}"),
    }
}

/// Judging the named type against the requirements is billed by the comparisons it may
/// take, before it is judged: a slot's `when` adds its nodes for each property the slot
/// tries, and a write refused because the type does not meet the requirements pays for
/// the judging like one that is accepted.
#[tokio::test]
async fn should_bill_the_comparisons_judging_the_named_type_takes() {
    const MEASURED: &str = "measuredNote";
    let any_text = |when: Option<Value>| {
        let mut slot = platform_value!({ "name": "$any", "type": "string" });
        if let Some(when) = when {
            slot.insert("when".to_string(), when)
                .expect("expected to set the condition");
        }
        platform_value!({ "properties": { "body": slot } })
    };
    // The seven nodes of "a removal takes the text", and the same with two more
    let seven = || {
        text_removed_with_the_document()
            .get("properties")
            .expect("expected properties")
            .expect("expected properties")
            .get("body")
            .expect("expected the slot")
            .expect("expected the slot")
            .get("when")
            .expect("expected the condition")
            .expect("expected the condition")
            .clone()
    };
    let nine = || {
        platform_value!({ "allOf": [
            seven(),
            { "notEqual": ["$property", { "const": "never" }] },
        ] })
    };
    let per_comparison = PlatformVersion::latest()
        .fee_version
        .document_reference
        .requirement_credits_per_comparison;
    assert_eq!(per_comparison, 100);

    // Each write is measured on a platform of its own, built the same way every time but
    // for the `when` of the one slot of the note it writes: the note type has the same
    // name and the same place in the contract, so two writes differ by the judging alone
    let mut fee_of = Vec::new();
    for (when, target_type) in [
        (None, POST),
        (Some(seven()), POST),
        (Some(nine()), POST),
        (Some(seven()), KEPT_POST),
        (Some(nine()), KEPT_POST),
    ] {
        let setup = Setup::new_at_with(
            Some(moderators_without_lists()),
            PlatformVersion::latest(),
            |contract| {
                add_document_type(
                    contract,
                    POST,
                    post_schema_with(platform_value!({ "canBeDeleted": false })),
                );
                add_document_type(
                    contract,
                    KEPT_POST,
                    post_schema_with(platform_value!({
                        "canBeDeleted": false,
                        "moderatorAbilities": { "delete": true, "deleteKeepsFields": ["text"] },
                    })),
                );
                add_document_type(
                    contract,
                    MEASURED,
                    note_schema_of(platform_value!("moderatedDocument"), any_text(when)),
                );
            },
        )
        .await;
        // A post has one property, so a slot taking any property tries one
        assert_eq!(
            setup
                .contract
                .document_type_for_name(target_type)
                .expect("expected the type")
                .properties()
                .len(),
            1
        );
        let transaction = setup.platform.drive.grove.start_transaction();
        let (target, create_target) = setup
            .create_document_of_type(&setup.user, target_type)
            .await;
        assert_success(&setup.process(&create_target, &transaction));
        let (_, create) = create_with(
            &setup,
            &setup.stranger,
            MEASURED,
            note_naming(setup.contract.id(), target_type, target.id()),
        )
        .await;
        let execution = setup.process(&create, &transaction);
        if target_type == POST {
            assert_success(&execution);
        } else {
            assert_paid_with_code(&execution, REFERENCED_DOCUMENT_TYPE_REQUIREMENT_NOT_MET);
        }
        fee_of.push(processing_fee(&execution));
    }
    let [none, seven_accepted, nine_accepted, seven_refused, nine_refused] = fee_of[..] else {
        panic!("expected five fees");
    };

    // One property tried: 1 comparison, 1 + 7 and 1 + 9
    assert_eq!(seven_accepted - none, 7 * per_comparison);
    assert_eq!(nine_accepted - seven_accepted, 2 * per_comparison);
    // and a refused write pays for the judging that refused it
    assert_eq!(nine_refused - seven_refused, 2 * per_comparison);
}
