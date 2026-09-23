//! Joins through an `anyOf` reference (`refersTo: { "anyOf": [...] }`,
//! protocol version 14): a value may be the id of a document of any of its
//! targets' types, or no document at all, so it names no single type the join
//! resolves in, and neither a chained query nor a composite by-id join may take
//! it as a join property, even when one target is the joined type. Both
//! surfaces refuse it while validating the shape, on the server and in the
//! verifier alike.

use crate::error::Error;
use crate::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;

fn identifier(position: u32) -> Value {
    platform_value!({
        "type": "array",
        "byteArray": true,
        "minItems": 32u32,
        "maxItems": 32u32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": position
    })
}

/// A permanent `profile` type, and two types whose `authorId` is the id of a
/// profile or of an identity: the indexOnly `like` a chained query starts from
/// and the plain `note` a composite page starts from.
fn any_of_join_contract() -> DataContract {
    let mut author_id = identifier(0);
    author_id
        .insert(
            "refersTo".to_string(),
            platform_value!({
                "anyOf": [
                    { "type": "permanentDocument", "documentType": "profile" },
                    { "type": "identity" }
                ]
            }),
        )
        .expect("refersTo inserts");
    let referring_type = |index_only: bool| {
        let mut schema = platform_value!({
            "type": "object",
            "properties": { "authorId": author_id.clone() },
            "indices": [{ "name": "byAuthor", "properties": [{ "authorId": "asc" }] }],
            "required": ["authorId"],
            "additionalProperties": false
        });
        if index_only {
            schema
                .insert("indexOnly".to_string(), Value::Bool(true))
                .expect("indexOnly inserts");
            schema
                .insert("documentsMutable".to_string(), Value::Bool(false))
                .expect("documentsMutable inserts");
        }
        schema
    };
    DataContract::from_value(
        platform_value!({
            "$formatVersion": "1",
            "id": Identifier::from([0x5C; 32]),
            "ownerId": Identifier::from([0x5B; 32]),
            "version": 1u32,
            "documentSchemas": {
                "profile": {
                    "type": "object",
                    "canBeDeleted": false,
                    "properties": {
                        "name": { "type": "string", "maxLength": 32u32, "position": 0u32 }
                    },
                    "required": ["name"],
                    "additionalProperties": false
                },
                "like": referring_type(true),
                "note": referring_type(false)
            }
        }),
        true,
        PlatformVersion::latest(),
    )
    .expect("the anyOf join contract parses")
}

fn by_author<'a>(contract: &'a DataContract, type_name: &str) -> DriveDocumentQuery<'a> {
    DriveDocumentQuery {
        contract,
        document_type: contract
            .document_type_for_name(type_name)
            .expect("the document type exists"),
        internal_clauses: InternalClauses::extract_from_clauses(
            vec![WhereClause {
                field: "authorId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier([0x11; 32]),
            }],
            PlatformVersion::latest(),
        )
        .expect("the clauses extract"),
        offset: None,
        limit: Some(10),
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
        resolved_time_ranges: vec![],
        sub_queries: vec![],
    }
    .with_by_id_join(
        "authorId",
        contract
            .document_type_for_name("profile")
            .expect("the profile type exists"),
    )
}

fn assert_refused_as_an_any_of(result: Result<(), Error>) {
    match result {
        Err(Error::Query(error)) => assert!(
            error.to_string().contains("declares a refersTo anyOf"),
            "the refusal should name the anyOf: {error}"
        ),
        other => panic!("expected the join to be refused, got {other:?}"),
    }
}

#[test]
fn should_refuse_a_chained_join_through_an_any_of_reference() {
    let contract = any_of_join_contract();
    assert_refused_as_an_any_of(
        by_author(&contract, "like").validate_chained(PlatformVersion::latest()),
    );
}

#[test]
fn should_refuse_a_composite_by_id_join_through_an_any_of_reference() {
    let contract = any_of_join_contract();
    assert_refused_as_an_any_of(
        by_author(&contract, "note").validate_composite(PlatformVersion::latest()),
    );
}
