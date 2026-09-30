//! Derived index properties (protocol version 14): an index naming a value of the document a
//! reference points at, `"<reference property>.<field>"`. What the parse of the referring type
//! registers and refuses, and what the parse of the whole contract resolves and refuses on the
//! referenced side.

use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::methods::{
    DocumentTypeBasicMethods, DocumentTypeV0Methods,
};
use crate::data_contract::document_type::{
    DerivedIndexField, DocumentPropertyType, DocumentReferenceKind, DocumentType,
};
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

fn assert_refused(result: Result<BTreeMap<String, DocumentType>, ProtocolError>, fragment: &str) {
    let error = result.expect_err("the contract should be refused");
    // A paid refusal needs the consensus variant: a bare data contract error would surface as
    // an internal error in a block
    assert!(
        matches!(error, ProtocolError::ConsensusError(_)),
        "expected a consensus error, got {error:?}"
    );
    assert!(
        error.to_string().contains(fragment),
        "expected {fragment:?} in: {error}"
    );
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

#[test]
fn should_refuse_any_field_but_the_owner_through_a_moderated_reference() {
    // The removal record keeps the owner of the removed post, and nothing else of it
    assert_refused(
        parse(
            moderated_post(),
            reply(moderated_reference(), "postId.hashtag", Value::Null),
            true,
        ),
        "reads $ownerId only",
    );
    assert_refused(
        parse(
            merged(moderated_post(), platform_value!({ "transferable": 1 })),
            reply(moderated_reference(), "postId.$creatorId", Value::Null),
            true,
        ),
        "reads $ownerId only",
    );
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
    // Frozen, but settable while absent
    assert_refused(
        parse(
            post(platform_value!({
                "documentsMutable": true,
                "immutable": ["hashtag", "text"],
                "immutableAllowSetting": ["text"],
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
fn should_refuse_a_derived_property_in_a_unique_or_skipping_index() {
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
                            "skipIfAbsent": true,
                        },
                    ],
                }),
            ),
            true,
        ),
        "skip",
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
