//! Derived index properties (protocol version 14): an index naming a value of the document a
//! reference points at, `"<reference property>.<field>"`. What the parse of the referring type
//! registers and refuses, and what the parse of the whole contract resolves and refuses on the
//! referenced side.

use super::refusal_test_support::assert_refused;
use crate::block::block_info::BlockInfo;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::methods::{
    DocumentTypeBasicMethods, DocumentTypeV0Methods,
};
use crate::data_contract::document_type::{
    DerivedIndexField, DocumentPropertyType, DocumentReferenceKind, DocumentType,
};
use crate::data_contract::methods::validate_update::DataContractUpdateValidationMethodsV0;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::ProtocolError;
use platform_value::{platform_value, Identifier, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

const CONTRACT_ID: [u8; 32] = [7; 32];

fn identifier(position: u64, refers_to: Option<Value>) -> Value {
    let mut property = platform_value!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": position,
    });
    if let (Some(refers_to), Value::Map(map)) = (refers_to, &mut property) {
        map.push((Value::Text("refersTo".to_string()), refers_to));
    }
    property
}

/// Every top-level entry of `extra` in place of the one of `schema` with its key.
fn merged(mut schema: Value, extra: Value) -> Value {
    if let (Value::Map(schema_map), Value::Map(extra_map)) = (&mut schema, extra) {
        for (key, value) in extra_map {
            schema_map.retain(|(existing, _)| existing != &key);
            schema_map.push((key, value));
        }
    }
    schema
}

/// A post its author can not delete and nobody can change, with `extra` merged in.
fn post(extra: Value) -> Value {
    merged(
        platform_value!({
            "type": "object",
            "documentsMutable": false,
            "canBeDeleted": false,
            "properties": {
                "hashtag": { "type": "string", "minLength": 1, "maxLength": 63, "position": 0 },
                "text": { "type": "string", "maxLength": 280, "position": 1 },
            },
            "required": ["hashtag", "$createdAt"],
            "additionalProperties": false,
        }),
        extra,
    )
}

/// A post only the moderators take down, each removal on the record.
fn moderated_post() -> Value {
    post(platform_value!({ "moderatorAbilities": { "delete": true } }))
}

/// A reply whose `postId` refers to a post by `refers_to`, indexed by `index_property` then
/// `$createdAt`, with `extra` merged in. Its author may edit its body, but never repoint it.
fn reply(refers_to: Value, index_property: &str, extra: Value) -> Value {
    merged(
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "canBeDeleted": true,
            "immutable": ["postId"],
            "indices": [
                {
                    "name": "byDerived",
                    "properties": [{ index_property: "asc" }, { "$createdAt": "asc" }],
                },
            ],
            "properties": {
                "postId": identifier(0, Some(refers_to)),
                "body": { "type": "string", "minLength": 1, "maxLength": 280, "position": 1 },
            },
            "required": ["postId", "body", "$createdAt"],
            "additionalProperties": false,
        }),
        extra,
    )
}

fn moderated_reference() -> Value {
    platform_value!({ "type": "moderatedDocument", "documentType": "post" })
}

fn permanent_reference() -> Value {
    platform_value!({ "type": "permanentDocument", "documentType": "post" })
}

fn config() -> DataContractConfig {
    DataContractConfig::default_for_version(PlatformVersion::latest())
        .expect("default config available")
        .with_moderation(Some(ContractModerationConfig {
            banlist: false,
            suspensions: false,
            moderators: ContractModerators::ContractOwner,
            warnings: false,
        }))
}

fn parse(
    post: Value,
    reply: Value,
    full_validation: bool,
) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    let config = config();
    DocumentType::create_document_types_from_document_schemas(
        Identifier::new(CONTRACT_ID),
        1,
        config.version(),
        BTreeMap::from([("post".to_string(), post), ("reply".to_string(), reply)]),
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        false,
        &mut vec![],
        PlatformVersion::latest(),
    )
}

#[test]
fn should_register_the_owner_of_a_moderated_post_as_a_derived_index_property() {
    let document_types = parse(
        moderated_post(),
        reply(moderated_reference(), "postId.$ownerId", Value::Null),
        true,
    )
    .expect("the contract should parse");
    let reply = document_types.get("reply").expect("the reply type");
    let derived = reply
        .derived_index_properties()
        .get("postId.$ownerId")
        .expect("the derived index property");
    assert_eq!(derived.reference_property, "postId");
    assert_eq!(derived.referenced_document_type_name, "post");
    assert_eq!(derived.kind, DocumentReferenceKind::Moderated);
    assert_eq!(derived.field, DerivedIndexField::OwnerId);
    assert_eq!(
        derived.property_type,
        Some(DocumentPropertyType::Identifier)
    );
    // The one derived name; `$createdAt` is the reply's own
    assert_eq!(reply.derived_index_properties().len(), 1);

    // Its values encode into keys as identifiers do
    let owner = Identifier::new([3; 32]);
    assert_eq!(
        reply
            .as_ref()
            .serialize_value_for_key(
                "postId.$ownerId",
                &Value::Identifier(owner.to_buffer()),
                PlatformVersion::latest(),
            )
            .expect("an identifier encodes"),
        owner.to_vec()
    );
}

#[test]
fn should_resolve_the_type_of_a_permanent_posts_property_on_every_parse() {
    for full_validation in [true, false] {
        let document_types = parse(
            post(Value::Null),
            reply(permanent_reference(), "postId.hashtag", Value::Null),
            full_validation,
        )
        .expect("the contract should parse");
        let reply = document_types.get("reply").expect("the reply type");
        let derived = reply
            .derived_index_properties()
            .get("postId.hashtag")
            .expect("the derived index property");
        assert_eq!(derived.kind, DocumentReferenceKind::Permanent);
        assert_eq!(
            derived.field,
            DerivedIndexField::Property("hashtag".to_string())
        );
        // The post's `hashtag` type, which only the whole contract's parse can see
        let hashtag_type = document_types
            .get("post")
            .and_then(|post| post.flattened_properties().get("hashtag"))
            .map(|property| property.property_type.clone());
        assert!(hashtag_type.is_some());
        assert_eq!(derived.property_type, hashtag_type);
        assert_eq!(
            reply.as_ref().derived_index_property_type("postId.hashtag"),
            hashtag_type.as_ref(),
            "full validation {full_validation}"
        );
    }
}

#[test]
fn should_admit_the_creator_of_a_transferable_permanent_post() {
    let document_types = parse(
        post(platform_value!({ "transferable": 1 })),
        reply(permanent_reference(), "postId.$creatorId", Value::Null),
        true,
    )
    .expect("the contract should parse");
    let derived = document_types
        .get("reply")
        .and_then(|reply| reply.derived_index_properties().get("postId.$creatorId"))
        .cloned()
        .expect("the derived index property");
    assert_eq!(derived.field, DerivedIndexField::CreatorId);
}

#[test]
fn should_refuse_a_reference_whose_document_can_leave_state_without_a_record() {
    assert_refused(
        parse(
            post(platform_value!({ "canBeDeleted": true })),
            reply(
                platform_value!({ "type": "deletableDocument", "documentType": "post" }),
                "postId.$ownerId",
                Value::Null,
            ),
            true,
        ),
        "is a deletableDocument reference",
    );
}

/// A post only the moderators take down, each removal on the record keeping `kept`
/// (`moderatorAbilities.deleteKeepsFields`), with `extra` merged in.
fn moderated_post_keeping(kept: Value, extra: Value) -> Value {
    merged(
        post(platform_value!({
            "moderatorAbilities": { "delete": true, "deleteKeepsFields": kept },
        })),
        extra,
    )
}

#[test]
fn should_admit_a_field_a_moderated_posts_removal_record_keeps() {
    // Read from the post while it is in state, and from its removal record once a moderator
    // removed it, which keeps the hashtag as the post held it
    for full_validation in [true, false] {
        let document_types = parse(
            moderated_post_keeping(platform_value!(["hashtag"]), Value::Null),
            reply(moderated_reference(), "postId.hashtag", Value::Null),
            full_validation,
        )
        .expect("the contract should parse");
        let reply = document_types.get("reply").expect("the reply type");
        let derived = reply
            .derived_index_properties()
            .get("postId.hashtag")
            .expect("the derived index property");
        assert_eq!(derived.kind, DocumentReferenceKind::Moderated);
        assert_eq!(
            derived.field,
            DerivedIndexField::Property("hashtag".to_string())
        );
        let hashtag_type = document_types
            .get("post")
            .and_then(|post| post.flattened_properties().get("hashtag"))
            .map(|property| property.property_type.clone());
        assert!(hashtag_type.is_some());
        assert_eq!(
            derived.property_type, hashtag_type,
            "full validation {full_validation}"
        );
    }

    // A property inside an object the record keeps whole
    let meta = platform_value!({
        "properties": {
            "hashtag": { "type": "string", "minLength": 1, "maxLength": 63, "position": 0 },
            "meta": {
                "type": "object",
                "properties": {
                    "topic": { "type": "string", "maxLength": 63, "position": 0 },
                },
                "additionalProperties": false,
                "position": 1,
            },
        },
    });
    let document_types = parse(
        moderated_post_keeping(platform_value!(["meta"]), meta),
        reply(moderated_reference(), "postId.meta.topic", Value::Null),
        true,
    )
    .expect("the contract should parse");
    let derived = document_types
        .get("reply")
        .and_then(|reply| reply.derived_index_properties().get("postId.meta.topic"))
        .cloned()
        .expect("the derived index property");
    assert_eq!(
        derived.field,
        DerivedIndexField::Property("meta.topic".to_string())
    );
}

#[test]
fn should_refuse_a_field_a_moderated_posts_removal_record_does_not_keep() {
    // The record keeps the owner of the removed post and the fields its type lists, and
    // nothing else of it
    assert_refused(
        parse(
            moderated_post(),
            reply(moderated_reference(), "postId.hashtag", Value::Null),
            true,
        ),
        "\"hashtag\" of \"post\" is not kept by a moderator's removal",
    );
    assert_refused(
        parse(
            moderated_post_keeping(platform_value!(["text"]), Value::Null),
            reply(moderated_reference(), "postId.hashtag", Value::Null),
            true,
        ),
        "is not kept by a moderator's removal",
    );
    // The refusal gives no advice an update could not follow: the list is fixed with the type
    assert_refused(
        parse(
            moderated_post(),
            reply(moderated_reference(), "postId.hashtag", Value::Null),
            true,
        ),
        "a list fixed when the type is registered, which no update changes",
    );
    // Never its creator, which no record keeps
    assert_refused(
        parse(
            moderated_post_keeping(
                platform_value!(["hashtag"]),
                platform_value!({ "transferable": 1 }),
            ),
            reply(moderated_reference(), "postId.$creatorId", Value::Null),
            true,
        ),
        "never its creator",
    );
}

#[test]
fn should_refuse_a_moderated_posts_field_no_index_can_key_before_asking_it_to_be_kept() {
    // `text` is longer than an index key can hold: keeping it would not make it a key, so
    // that is what the refusal says
    let error = parse(
        moderated_post(),
        reply(moderated_reference(), "postId.text", Value::Null),
        true,
    )
    .expect_err("the contract should be refused");
    assert!(error.to_string().contains("maxLength"), "got {error}");
    assert!(!error.to_string().contains("is not kept"), "got {error}");
}

/// Protocol versions 9 to 13 select the whole-contract parse that resolves derived index
/// properties too, but no parser generation before 3, and so no protocol version before 14,
/// declares one: a name reading through an identifier is an unknown property there, refused
/// as it always was, and a contract otherwise parses with none.
#[test]
fn should_declare_no_derived_index_property_before_protocol_version_14() {
    let platform_version = PlatformVersion::get(13).expect("platform version 13 exists");
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    let post = platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": false,
        "properties": {
            "hashtag": { "type": "string", "minLength": 1, "maxLength": 63, "position": 0 },
        },
        "required": ["hashtag"],
        "additionalProperties": false,
    });
    let reply_indexed_by = |index_property: &str| {
        platform_value!({
            "type": "object",
            "documentsMutable": false,
            "indices": [{ "name": "byPost", "properties": [{ index_property: "asc" }] }],
            "properties": { "postId": identifier(0, None) },
            "required": ["postId"],
            "additionalProperties": false,
        })
    };
    let parse_at_13 = |reply: Value| {
        DocumentType::create_document_types_from_document_schemas(
            Identifier::new(CONTRACT_ID),
            1,
            config.version(),
            BTreeMap::from([
                ("post".to_string(), post.clone()),
                ("reply".to_string(), reply),
            ]),
            None,
            &BTreeMap::new(),
            &config,
            true,
            false,
            &mut vec![],
            platform_version,
        )
    };

    let document_types =
        parse_at_13(reply_indexed_by("postId")).expect("the contract should parse at 13");
    assert!(document_types
        .values()
        .all(|document_type| document_type.derived_index_properties().is_empty()));

    parse_at_13(reply_indexed_by("postId.hashtag"))
        .expect_err("a name through an identifier is no property before 14");
}

#[test]
fn should_refuse_the_owner_of_a_post_that_can_change_hands() {
    for extra in [
        platform_value!({ "transferable": 1 }),
        platform_value!({ "tradeMode": 1 }),
    ] {
        assert_refused(
            parse(
                post(extra),
                reply(permanent_reference(), "postId.$ownerId", Value::Null),
                true,
            ),
            "can be transferred or traded",
        );
    }
}

#[test]
fn should_refuse_the_creator_of_a_post_that_records_none() {
    assert_refused(
        parse(
            post(Value::Null),
            reply(permanent_reference(), "postId.$creatorId", Value::Null),
            true,
        ),
        "records no creator",
    );
}

#[test]
fn should_refuse_a_post_property_that_can_change_once_written() {
    // A replace could change it
    assert_refused(
        parse(
            post(platform_value!({ "documentsMutable": true })),
            reply(permanent_reference(), "postId.hashtag", Value::Null),
            true,
        ),
        "can change once written",
    );
    // Frozen only under a condition (here: once the stored post holds it)
    assert_refused(
        parse(
            post(platform_value!({
                "documentsMutable": true,
                "immutable": [
                    "hashtag",
                    { "property": "text", "when": { "present": "$old.text" } }
                ],
            })),
            reply(permanent_reference(), "postId.text", Value::Null),
            true,
        ),
        "can change once written",
    );
    // The moderators write it
    assert_refused(
        parse(
            post(platform_value!({
                "moderatorAbilities": { "changeFields": ["text"] },
            })),
            reply(permanent_reference(), "postId.text", Value::Null),
            true,
        ),
        "can change once written",
    );
    // Frozen by the immutable list, it is admitted
    parse(
        post(platform_value!({
            "documentsMutable": true,
            "immutable": ["hashtag"],
        })),
        reply(permanent_reference(), "postId.hashtag", Value::Null),
        true,
    )
    .expect("an immutable property of a mutable post is fixed once written");
}

#[test]
fn should_refuse_a_post_property_that_is_never_stored_or_missing_or_not_a_key() {
    assert_refused(
        parse(
            post(platform_value!({ "transient": ["text"] })),
            reply(permanent_reference(), "postId.text", Value::Null),
            true,
        ),
        "is transient",
    );
    assert_refused(
        parse(
            post(Value::Null),
            reply(permanent_reference(), "postId.title", Value::Null),
            true,
        ),
        "has no property \"title\"",
    );
    // `text` is longer than an index key can hold
    assert_refused(
        parse(
            post(Value::Null),
            reply(permanent_reference(), "postId.text", Value::Null),
            true,
        ),
        "maxLength",
    );
}

#[test]
fn should_refuse_a_reference_a_replace_could_repoint() {
    // A reply whose author may edit it, `postId` included
    assert_refused(
        parse(
            moderated_post(),
            reply(
                moderated_reference(),
                "postId.$ownerId",
                platform_value!({ "immutable": ["body"] }),
            ),
            true,
        ),
        "a replace could point \"postId\" at another document",
    );
    // A reply nobody can edit needs no list
    let reply_schema = merged(
        reply(moderated_reference(), "postId.$ownerId", Value::Null),
        platform_value!({ "documentsMutable": false }),
    );
    let reply_schema = match reply_schema {
        Value::Map(mut map) => {
            map.retain(|(key, _)| key != &Value::Text("immutable".to_string()));
            Value::Map(map)
        }
        other => other,
    };
    parse(moderated_post(), reply_schema, true).expect("an immutable reply keeps its reference");
}

#[test]
fn should_refuse_a_derived_property_in_a_unique_index() {
    assert_refused(
        parse(
            moderated_post(),
            reply(
                moderated_reference(),
                "postId.$ownerId",
                platform_value!({
                    "indices": [
                        {
                            "name": "byDerived",
                            "properties": [{ "postId.$ownerId": "asc" }, { "$createdAt": "asc" }],
                            "unique": true,
                        },
                    ],
                }),
            ),
            true,
        ),
        "a unique index",
    );
}

/// A permanent post that may leave out its `topic`, its `meta` object (whose `tag` it then
/// holds) and its `digest`, a byte array of at least `digest_min_items` bytes, with `required`
/// its required properties.
fn post_with_optional_fields(digest_min_items: u64, required: Value) -> Value {
    post(platform_value!({
        "properties": {
            "hashtag": { "type": "string", "minLength": 1, "maxLength": 63, "position": 0 },
            "topic": { "type": "string", "minLength": 1, "maxLength": 20, "position": 1 },
            "meta": {
                "type": "object",
                "properties": {
                    "tag": { "type": "string", "minLength": 1, "maxLength": 20, "position": 0 },
                },
                "required": ["tag"],
                "additionalProperties": false,
                "position": 2,
            },
            "digest": {
                "type": "array",
                "byteArray": true,
                "minItems": digest_min_items,
                "maxItems": 32,
                "position": 3,
            },
        },
        "required": required,
    }))
}

fn post_required() -> Value {
    platform_value!(["hashtag", "$createdAt"])
}

/// A reply to a permanent post, with an optional `label` of its own and `required` its
/// required properties, and one index `bySkip` on `properties` skipping on `skip`.
fn skipping_reply(properties: Value, skip: Value, required: Value) -> Value {
    merged(
        reply(permanent_reference(), "postId.$ownerId", Value::Null),
        platform_value!({
            "indices": [{ "name": "bySkip", "properties": properties, "skipIfAbsent": skip }],
            "properties": {
                "postId": identifier(0, Some(permanent_reference())),
                "body": { "type": "string", "minLength": 1, "maxLength": 280, "position": 1 },
                "label": { "type": "string", "minLength": 1, "maxLength": 20, "position": 2 },
            },
            "required": required,
        }),
    )
}

/// `derived`, then `$createdAt`.
fn derived_index(derived: &str) -> Value {
    platform_value!([{ derived: "asc" }, { "$createdAt": "asc" }])
}

fn reply_required() -> Value {
    platform_value!(["postId", "body", "$createdAt"])
}

fn skip_set(document_types: &BTreeMap<String, DocumentType>) -> Vec<String> {
    document_types
        .get("reply")
        .and_then(|reply| reply.indexes().get("bySkip"))
        .expect("the index")
        .skip_if_absent_properties
        .clone()
}

#[test]
fn should_skip_on_a_derived_property_the_array_names() {
    for full_validation in [true, false] {
        let document_types = parse(
            post_with_optional_fields(1, post_required()),
            skipping_reply(
                derived_index("postId.topic"),
                platform_value!(["postId.topic"]),
                reply_required(),
            ),
            full_validation,
        )
        .expect("a reply may skip on a topic its post may leave out");
        assert_eq!(
            skip_set(&document_types),
            vec!["postId.topic".to_string()],
            "full validation {full_validation}"
        );
    }
}

#[test]
fn should_skip_on_the_owner_of_a_post_only_through_an_optional_reference() {
    // A reply that may refer to no post has no post owner to be filed under
    let document_types = parse(
        post_with_optional_fields(1, post_required()),
        skipping_reply(
            derived_index("postId.$ownerId"),
            platform_value!(["postId.$ownerId"]),
            platform_value!(["body", "$createdAt"]),
        ),
        true,
    )
    .expect("a reply without a post skips");
    assert_eq!(
        skip_set(&document_types),
        vec!["postId.$ownerId".to_string()]
    );
    // Every post has an owner, so a reply that always refers to one never skips
    assert_refused(
        parse(
            post_with_optional_fields(1, post_required()),
            skipping_reply(
                derived_index("postId.$ownerId"),
                platform_value!(["postId.$ownerId"]),
                reply_required(),
            ),
            true,
        ),
        "never absent: \"postId\" is required and every document of \"post\" has an owner",
    );
}

#[test]
fn should_refuse_a_derived_skip_property_that_is_never_absent() {
    assert_refused(
        parse(
            post_with_optional_fields(1, post_required()),
            skipping_reply(
                derived_index("postId.hashtag"),
                platform_value!(["postId.hashtag"]),
                reply_required(),
            ),
            true,
        ),
        "\"post\" requires \"hashtag\"",
    );
    // `meta.tag` is required inside `meta`, which a post may leave out, taking the tag with it
    parse(
        post_with_optional_fields(1, post_required()),
        skipping_reply(
            derived_index("postId.meta.tag"),
            platform_value!(["postId.meta.tag"]),
            reply_required(),
        ),
        true,
    )
    .expect("a tag inside an optional object can be absent");
    // Once `meta` is required too, every post holds a tag
    assert_refused(
        parse(
            post_with_optional_fields(1, platform_value!(["hashtag", "meta", "$createdAt"])),
            skipping_reply(
                derived_index("postId.meta.tag"),
                platform_value!(["postId.meta.tag"]),
                reply_required(),
            ),
            true,
        ),
        "\"post\" requires \"meta.tag\"",
    );
    // A required field through an optional reference is absent with the reference
    parse(
        post_with_optional_fields(1, post_required()),
        skipping_reply(
            derived_index("postId.hashtag"),
            platform_value!(["postId.hashtag"]),
            platform_value!(["body", "$createdAt"]),
        ),
        true,
    )
    .expect("a reply without a post has no hashtag");
}

#[test]
fn should_refuse_a_derived_skip_byte_array_that_may_be_empty() {
    assert_refused(
        parse(
            post_with_optional_fields(0, post_required()),
            skipping_reply(
                derived_index("postId.digest"),
                platform_value!(["postId.digest"]),
                reply_required(),
            ),
            true,
        ),
        "set its `minItems` to at least 1",
    );
    parse(
        post_with_optional_fields(1, post_required()),
        skipping_reply(
            derived_index("postId.digest"),
            platform_value!(["postId.digest"]),
            reply_required(),
        ),
        true,
    )
    .expect("a digest of at least one byte is never keyed like a missing one");
}

#[test]
fn should_leave_a_derived_property_out_of_skip_if_absent_true() {
    // `true` names the reply's own optional properties: `label`, never the post's topic,
    // whose absence the reply type alone can not tell
    let document_types = parse(
        post_with_optional_fields(1, post_required()),
        skipping_reply(
            platform_value!([{ "postId.topic": "asc" }, { "label": "asc" }]),
            Value::Bool(true),
            reply_required(),
        ),
        true,
    )
    .expect("the index skips on its label");
    assert_eq!(skip_set(&document_types), vec!["label".to_string()]);
    // With no optional property of the reply's own, `true` skips on nothing
    assert_refused(
        parse(
            post_with_optional_fields(1, post_required()),
            skipping_reply(
                derived_index("postId.topic"),
                Value::Bool(true),
                reply_required(),
            ),
            true,
        ),
        "name one in the array form",
    );
}

#[test]
fn should_refuse_other_system_fields_and_other_forms_of_reference() {
    assert_refused(
        parse(
            moderated_post(),
            reply(moderated_reference(), "postId.$id", Value::Null),
            true,
        ),
        "index \"postId\"",
    );
    assert_refused(
        parse(
            post(Value::Null),
            reply(permanent_reference(), "postId.$createdAt", Value::Null),
            true,
        ),
        "not $createdAt",
    );
    assert_refused(
        parse(
            post(Value::Null),
            reply(
                platform_value!({
                    "type": "permanentDocument",
                    "documentType": "post",
                    "contractId": Value::Identifier([9; 32]),
                }),
                "postId.$ownerId",
                Value::Null,
            ),
            true,
        ),
        "refers into another contract",
    );
    assert_refused(
        parse(
            post(Value::Null),
            reply(
                platform_value!({ "type": "identity" }),
                "postId.$ownerId",
                Value::Null,
            ),
            true,
        ),
        "a reference to something other than a document",
    );
}

#[test]
fn should_leave_every_other_name_to_the_type_s_own_properties() {
    // `body` is a string, not a reference: the name is an undefined index property
    let error = parse(
        moderated_post(),
        reply(moderated_reference(), "body.$ownerId", Value::Null),
        true,
    )
    .expect_err("the contract should be refused");
    assert!(
        format!("{error:?}").contains("UndefinedIndexPropertyError"),
        "got {error:?}"
    );
}

fn contract_value(version: u32, document_schemas: Value) -> Value {
    platform_value!({
        "$formatVersion": "1",
        "id": Value::Identifier(CONTRACT_ID),
        "ownerId": Value::Identifier([8; 32]),
        "version": version,
        "documentSchemas": document_schemas,
    })
}

/// A contract declaring a derived index property can be updated: the update re-parses the
/// whole contract, resolves the derived property again, and passes the update rules.
#[test]
fn should_let_a_contract_declaring_a_derived_index_property_be_updated() {
    let platform_version = PlatformVersion::latest();
    let reply_schema = reply(permanent_reference(), "postId.hashtag", Value::Null);
    let old = DataContract::from_value(
        contract_value(
            1,
            platform_value!({ "post": post(Value::Null), "reply": reply_schema.clone() }),
        ),
        true,
        platform_version,
    )
    .expect("the contract parses");
    let new = DataContract::from_value(
        contract_value(
            2,
            platform_value!({
                "post": post(Value::Null),
                "reply": reply_schema,
                "note": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "maxLength": 63, "position": 0 },
                    },
                    "additionalProperties": false,
                },
            }),
        ),
        true,
        platform_version,
    )
    .expect("the updated contract parses");
    let result = old
        .validate_update(&new, &BlockInfo::default(), platform_version)
        .expect("the update is judged");
    assert!(result.is_valid(), "{:?}", result.errors);
    assert!(new
        .document_type_for_name("reply")
        .expect("the reply type")
        .derived_index_property_type("postId.hashtag")
        .is_some());
}

/// A contract built one document type at a time resolves a derived property's type as the
/// parse of the whole contract does, whichever type is added first.
#[test]
fn should_resolve_a_derived_type_when_document_types_are_set_one_at_a_time() {
    let platform_version = PlatformVersion::latest();
    let mut contract = DataContract::from_value(
        contract_value(1, platform_value!({ "post": post(Value::Null) })),
        true,
        platform_version,
    )
    .expect("the contract parses");
    contract
        .set_document_schema(
            "reply",
            reply(permanent_reference(), "postId.hashtag", Value::Null),
            true,
            &mut vec![],
            platform_version,
        )
        .expect("the reply type is added");
    assert!(contract
        .document_type_for_name("reply")
        .expect("the reply type")
        .derived_index_property_type("postId.hashtag")
        .is_some());
}
