//! Preallocated indexes through a `moderatedDocument` reference (protocol version 14): the
//! parse of the whole contract admits one only when the removal record of the referenced
//! document keeps every key of the index path.

use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentType;
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

/// A post its author can not delete, which only the moderators take down, each removal on
/// the record keeping the fields `kept` lists (none when it is empty).
fn moderated_post(kept: Value) -> Value {
    let moderator_abilities = if kept.as_array().is_some_and(|kept| kept.is_empty()) {
        platform_value!({ "delete": true })
    } else {
        platform_value!({ "delete": true, "deleteKeepsFields": kept })
    };
    platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": false,
        "moderatorAbilities": moderator_abilities,
        "properties": {
            "hashtag": { "type": "string", "minLength": 1, "maxLength": 63, "position": 0 },
            "text": { "type": "string", "maxLength": 280, "position": 1 },
        },
        "required": ["hashtag", "$createdAt"],
        "additionalProperties": false,
    })
}

/// An indexOnly like of a post, bound to the post's hashtag and author, with `preallocated`
/// on the index named `preallocated_index`.
fn like(refers_to_type: &str, preallocated_index: &str) -> Value {
    let mut like = platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "canBeDeleted": true,
        "properties": {
            "postId": identifier(0, Some(platform_value!({
                "type": refers_to_type,
                "documentType": "post",
                "where": { "hashtag": "hashtag", "$ownerId": "postAuthor" },
            }))),
            "hashtag": { "type": "string", "minLength": 1, "maxLength": 63, "position": 1 },
            "postAuthor": identifier(2, None),
        },
        "indices": [
            { "name": "byPost", "properties": [{ "postId": "asc" }], "terminal": "$ownerId" },
            {
                "name": "byHashtagPost",
                "properties": [{ "hashtag": "asc" }, { "postId": "asc" }],
                "terminal": "$ownerId",
            },
            {
                "name": "byAuthorPost",
                "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }],
                "terminal": "$ownerId",
            },
        ],
        "required": ["postId", "hashtag", "postAuthor"],
        "additionalProperties": false,
    });
    for index in like
        .get_mut("indices")
        .expect("indices accessible")
        .expect("indices present")
        .as_array_mut()
        .expect("indices is an array")
    {
        if index.get_str("name").expect("index name") == preallocated_index {
            index
                .set_value("preallocated", Value::Bool(true))
                .expect("index key applies");
        }
    }
    like
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
    like: Value,
    full_validation: bool,
) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    let config = config();
    DocumentType::create_document_types_from_document_schemas(
        Identifier::new(CONTRACT_ID),
        1,
        config.version(),
        BTreeMap::from([("post".to_string(), post), ("like".to_string(), like)]),
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        false,
        &mut vec![],
        PlatformVersion::latest(),
    )
}

fn assert_preallocated(document_types: &BTreeMap<String, DocumentType>, index_name: &str) {
    let like = document_types.get("like").expect("the like type");
    assert!(
        like.indexes()
            .get(index_name)
            .expect("the index")
            .preallocated,
        "{index_name} should be preallocated"
    );
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
fn should_preallocate_an_index_keyed_by_a_moderated_posts_id() {
    // The record keeps the id of the removed post, whatever else it drops
    for full_validation in [false, true] {
        let document_types = parse(
            moderated_post(platform_value!([])),
            like("moderatedDocument", "byPost"),
            full_validation,
        )
        .expect("the contract should parse");
        assert_preallocated(&document_types, "byPost");
    }
}

#[test]
fn should_preallocate_an_index_keyed_by_a_moderated_posts_owner() {
    // The record keeps the owner of the removed post
    let document_types = parse(
        moderated_post(platform_value!([])),
        like("moderatedDocument", "byAuthorPost"),
        true,
    )
    .expect("the contract should parse");
    assert_preallocated(&document_types, "byAuthorPost");
}

#[test]
fn should_preallocate_an_index_keyed_by_a_field_a_moderated_posts_record_keeps() {
    let document_types = parse(
        moderated_post(platform_value!(["hashtag"])),
        like("moderatedDocument", "byHashtagPost"),
        true,
    )
    .expect("the contract should parse");
    assert_preallocated(&document_types, "byHashtagPost");
}

#[test]
fn should_refuse_an_index_keyed_by_a_field_a_moderated_posts_record_drops() {
    for kept in [platform_value!([]), platform_value!(["text"])] {
        assert_refused(
            parse(
                moderated_post(kept),
                like("moderatedDocument", "byHashtagPost"),
                true,
            ),
            "index \"byHashtagPost\" of document type \"like\" is preallocated through the \
             moderatedDocument reference \"postId\", but \"hashtag\" of \"post\" is not kept \
             by a moderator's removal",
        );
    }
    // A contract read back from state passed the check when it was registered
    parse(
        moderated_post(platform_value!([])),
        like("moderatedDocument", "byHashtagPost"),
        false,
    )
    .expect("a stored contract parses without the check");
}

#[test]
fn should_preallocate_through_a_permanent_reference_whatever_the_post_keeps() {
    // A permanent post never leaves state: nothing is ever dropped
    let mut post = moderated_post(platform_value!([]));
    post.remove("moderatorAbilities")
        .expect("the post declares moderator abilities");
    let document_types = parse(post, like("permanentDocument", "byHashtagPost"), true)
        .expect("the contract should parse");
    assert_preallocated(&document_types, "byHashtagPost");
}

#[test]
fn should_leave_a_moderated_reference_to_another_kind_of_type_to_the_reference_check() {
    // A post nobody removes admits a permanentDocument reference only: registration refuses
    // the moderatedDocument one for that (`ReferencedDocumentTypeNotModeratedError`), not for
    // what a record it never writes would keep
    let mut post = moderated_post(platform_value!([]));
    post.remove("moderatorAbilities")
        .expect("the post declares moderator abilities");
    parse(post, like("moderatedDocument", "byHashtagPost"), true)
        .expect("the parse leaves the reference kind to the reference check");
}
