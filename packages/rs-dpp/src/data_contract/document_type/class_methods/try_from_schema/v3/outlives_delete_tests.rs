//! `outlivesDelete` (protocol version 14): an indexOnly `timeRange` index with
//! a `ttl` whose entries a delete leaves to expire with their window. What the
//! parse admits and refuses, and that the flag is fixed with the index.

use super::immutable_tests::expect_structure_error;
use super::*;
use crate::block::block_info::BlockInfo;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::index_only_row_commits_created_at;
use crate::data_contract::methods::validate_update::DataContractUpdateValidationMethodsV0;
use crate::data_contract::DataContract;
use platform_value::platform_value;

fn parse_with(
    schema: Value,
    platform_version: &PlatformVersion,
    full_validation: bool,
) -> Result<DocumentTypeV2, ProtocolError> {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available on this platform version");
    try_from_schema_generation_3(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "like",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

fn parse_dispatched(
    schema: Value,
    platform_version: &PlatformVersion,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available on this platform version");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "like",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

/// A daily window over `$createdAt`, kept for seven days.
fn daily_window() -> Value {
    platform_value!({ "on": "$createdAt", "range": 86400, "step": 86400, "ttl": 604800 })
}

/// A like of a post with a plain `byPost` index and a daily `byTrendPost`
/// window, which `trend` is merged into.
fn like_schema(trend: Value) -> Value {
    let mut trend_index = platform_value!({
        "name": "byTrendPost",
        "properties": [{ "$createdAt": "asc" }, { "postId": "asc" }],
        "terminal": "$ownerId",
        "countable": "countable",
        "timeRange": daily_window(),
        "outlivesDelete": true,
    });
    if let (Value::Map(index), Value::Map(extra)) = (&mut trend_index, trend) {
        for (key, value) in extra {
            index.retain(|(existing, _)| existing != &key);
            if value != Value::Null {
                index.push((key, value));
            }
        }
    }
    platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "canBeDeleted": true,
        "properties": {
            "postId": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 0,
            },
        },
        "indices": [
            { "name": "byPost", "properties": [{ "postId": "asc" }], "terminal": "$ownerId" },
            trend_index,
        ],
        "required": ["postId", "$createdAt"],
        "additionalProperties": false,
    })
}

#[test]
fn should_admit_an_outliving_window_and_commit_no_timestamp() {
    for full_validation in [false, true] {
        let document_type = parse_with(
            like_schema(Value::Map(vec![])),
            PlatformVersion::latest(),
            full_validation,
        )
        .expect("an outliving daily window parses");
        assert!(
            document_type
                .indices
                .get("byTrendPost")
                .unwrap()
                .outlives_delete
        );
        assert!(!document_type.indices.get("byPost").unwrap().outlives_delete);
        // Only the window involves `$createdAt`, so the row commits to none
        assert!(!index_only_row_commits_created_at(
            &document_type.required_fields,
            &document_type.index_structure,
        ));
        // The flag is stamped on the window's terminating level
        let level = document_type
            .index_structure
            .sub_levels()
            .values()
            .find(|level| level.outlives_delete_at_or_below())
            .expect("the window level");
        assert!(level
            .sub_levels()
            .get("postId")
            .and_then(|level| level.has_index_with_type())
            .is_some_and(|info| info.outlives_delete));
    }
}

#[test]
fn should_refuse_an_outliving_index_without_a_window_that_expires() {
    // No window at all
    expect_structure_error(
        parse_with(
            like_schema(
                platform_value!({ "timeRange": Value::Null, "properties": [{ "postId": "asc" }, { "$createdAt": "asc" }] }),
            ),
            PlatformVersion::latest(),
            false,
        ),
        "without a `timeRange` carrying a `ttl`",
    );
    // A window that never expires
    expect_structure_error(
        parse_with(
            like_schema(platform_value!({
                "timeRange": { "on": "$createdAt", "range": 86400, "step": 86400 },
            })),
            PlatformVersion::latest(),
            false,
        ),
        "without a `timeRange` carrying a `ttl`",
    );
}

#[test]
fn should_refuse_an_outliving_index_summing_an_amount() {
    let mut schema = like_schema(platform_value!({ "summable": "amount" }));
    schema
        .get_mut("properties")
        .expect("properties accessible")
        .expect("properties present")
        .set_value(
            "amount",
            platform_value!({ "type": "integer", "minimum": 1, "maximum": 1000, "position": 1 }),
        )
        .expect("amount applies");
    schema
        .set_value(
            "required",
            platform_value!(["postId", "amount", "$createdAt"]),
        )
        .expect("required applies");
    // The amount must sit in an index a delete clears, too
    schema
        .get_mut("indices")
        .expect("indices accessible")
        .expect("indices present")
        .as_array_mut()
        .expect("indices is an array")
        .push(platform_value!({
            "name": "byAmount",
            "properties": [{ "amount": "asc" }],
            "terminal": "$ownerId",
        }));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "together with a sum",
    );
}

#[test]
fn should_refuse_an_outliving_index_on_a_type_with_an_entry_payload() {
    let mut schema = like_schema(Value::Map(vec![]));
    schema
        .get_mut("properties")
        .expect("properties accessible")
        .expect("properties present")
        .set_value(
            "note",
            platform_value!({ "type": "string", "maxLength": 20, "position": 1 }),
        )
        .expect("note applies");
    schema
        .set_value(
            "required",
            platform_value!(["postId", "note", "$createdAt"]),
        )
        .expect("required applies");
    schema
        .set_value("entryPayload", platform_value!(["note"]))
        .expect("entryPayload applies");
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "on a type with `entryPayload`",
    );
}

#[test]
fn should_refuse_a_property_only_an_outliving_index_holds() {
    // `hashtag` sits in the window alone: a delete could not be checked
    // against the entries holding it
    let mut schema = like_schema(platform_value!({
        "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }, { "postId": "asc" }],
    }));
    schema
        .get_mut("properties")
        .expect("properties accessible")
        .expect("properties present")
        .set_value(
            "hashtag",
            platform_value!({ "type": "string", "minLength": 1, "maxLength": 63, "position": 1 }),
        )
        .expect("hashtag applies");
    schema
        .set_value(
            "required",
            platform_value!(["postId", "hashtag", "$createdAt"]),
        )
        .expect("required applies");
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "neither sets skipIfAbsent nor outlivesDelete",
    );
}

#[test]
fn should_refuse_outlives_delete_on_a_stored_type() {
    let mut schema = like_schema(Value::Map(vec![]));
    schema
        .remove_optional_value("indexOnly")
        .expect("indexOnly removes");
    let indices = schema
        .get_mut("indices")
        .expect("indices accessible")
        .expect("indices present")
        .as_array_mut()
        .expect("indices is an array");
    for index in indices.iter_mut() {
        let _ = index.remove_optional_value("terminal");
    }
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "which is only allowed on indexOnly document types",
    );
}

#[test]
fn should_refuse_outlives_delete_below_generation_3() {
    let platform_version_13 = PlatformVersion::get(13).expect("PV13 exists");
    for full_validation in [false, true] {
        assert!(
            parse_dispatched(
                like_schema(Value::Map(vec![])),
                platform_version_13,
                full_validation
            )
            .is_err(),
            "PV13 must reject the outlivesDelete keyword (full_validation: {full_validation})"
        );
    }
}

/// The flag decides what a delete carries and what a row commits to, so an
/// update may not flip it.
#[test]
fn should_refuse_an_update_flipping_outlives_delete() {
    let platform_version = PlatformVersion::latest();
    let contract = |version: u32, outlives: bool| {
        DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": Value::Identifier([7; 32]),
                "ownerId": Value::Identifier([8; 32]),
                "version": version,
                "documentSchemas": {
                    "like": like_schema(platform_value!({ "outlivesDelete": outlives })),
                },
            }),
            true,
            platform_version,
        )
        .expect("the contract parses")
    };
    let result = contract(1, true)
        .validate_update(&contract(2, false), &BlockInfo::default(), platform_version)
        .expect("the update is judged");
    // Any change of an existing index is refused, this one included
    assert!(
        result.errors.iter().any(|error| {
            let error = format!("{error:?}");
            error.contains("DataContractInvalidIndexDefinitionUpdateError")
                && error.contains("byTrendPost")
        }),
        "{:?}",
        result.errors
    );
}

#[test]
fn should_refuse_an_outliving_index_whose_key_holds_no_cleared_index_key() {
    // Keyed by the liker alone in each window: two likes of different posts
    // by one liker would share an entry, and no index a delete clears keys
    // them apart
    expect_structure_error(
        parse_with(
            like_schema(platform_value!({ "properties": [{ "$createdAt": "asc" }] })),
            PlatformVersion::latest(),
            false,
        ),
        "holds the whole key of no index that a delete clears",
    );
}
