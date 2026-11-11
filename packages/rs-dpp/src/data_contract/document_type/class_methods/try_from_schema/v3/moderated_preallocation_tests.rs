//! Preallocated indexes through a `moderatedDocument` reference (protocol version 14): the
//! parse of the whole contract admits one only when the removal record of the referenced
//! document keeps every key of the index path.

use super::refusal_test_support::assert_refused;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentType;
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

/// The bindings a post's insert preallocates through, read against the parsed post type: only
/// those whose every key the post's removal record keeps
#[test]
fn should_preallocate_through_the_bindings_a_moderated_posts_record_keeps() {
    let bindings_for = |kept: Value, index_name: &str| {
        // Without full validation, as a stored contract is read, so the refused shape parses
        let document_types = parse(
            moderated_post(kept),
            like("moderatedDocument", index_name),
            false,
        )
        .expect("the contract should parse");
        let like = document_types.get("like").expect("the like type").as_ref();
        let post = document_types.get("post").expect("the post type").as_ref();
        like.indexes()
            .get(index_name)
            .expect("the index")
            .preallocation_bindings_for_target(
                like.flattened_properties(),
                like.data_contract_id(),
                post,
            )
            .len()
    };
    assert_eq!(bindings_for(platform_value!([]), "byPost"), 1);
    assert_eq!(bindings_for(platform_value!([]), "byAuthorPost"), 1);
    assert_eq!(bindings_for(platform_value!([]), "byHashtagPost"), 0);
    assert_eq!(bindings_for(platform_value!(["text"]), "byHashtagPost"), 0);
    assert_eq!(
        bindings_for(platform_value!(["hashtag"]), "byHashtagPost"),
        1
    );
}

/// A type added to a contract one at a time under full validation is judged as the parse of
/// the whole contract judges it
#[test]
fn should_refuse_a_like_type_set_on_a_contract_whose_post_record_drops_its_key() {
    let platform_version = PlatformVersion::latest();
    let contract_with_post = || {
        DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": Value::Identifier(CONTRACT_ID),
                "ownerId": Value::Identifier([8; 32]),
                "version": 1,
                "config": platform_value::to_value(config()).expect("the config converts"),
                "documentSchemas": { "post": moderated_post(platform_value!([])) },
            }),
            true,
            platform_version,
        )
        .expect("the contract parses")
    };
    let mut contract = contract_with_post();
    assert_refused(
        contract
            .set_document_schema(
                "like",
                like("moderatedDocument", "byHashtagPost"),
                true,
                &mut vec![],
                platform_version,
            )
            .map(|()| BTreeMap::new()),
        "\"hashtag\" of \"post\" is not kept by a moderator's removal",
    );
    let mut contract = contract_with_post();
    contract
        .set_document_schema(
            "like",
            like("moderatedDocument", "byHashtagPost"),
            false,
            &mut vec![],
            platform_version,
        )
        .expect("without full validation the type is added as a stored contract's is read");
    assert!(contract.document_type_for_name("like").is_ok());
}
