#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::config::DriveConfig;
    use crate::error::{query::QuerySyntaxError, Error};
    use crate::query::{DriveDocumentQuery, SkipIfAbsentBinding, WhereClause, WhereOperator};
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::DocumentType;
    use dpp::platform_value::{platform_value, Identifier, Value};
    use dpp::util::cbor_serializer;
    use serde_json::json;
    use std::collections::BTreeMap;

    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::version::PlatformVersion;

    fn construct_indexed_document_type() -> DocumentType {
        let platform_version = PlatformVersion::latest();

        let schema = platform_value!({
            "type": "object",
            "indices": [
                {
                    "name": "a",
                    "properties": [
                        { "a": "asc" }
                    ],
                    "unique": false
                },
                {
                    "name": "b",
                    "properties": [
                        { "b": "asc" }
                    ],
                    "unique": false
                },
                {
                    "name": "c",
                    "properties": [
                        { "b": "asc" },
                        { "a": "asc" }
                    ],
                    "unique": false
                },
                {
                    "name": "d",
                    "properties": [
                        { "b": "asc" },
                        { "a": "asc" },
                        { "d": "asc" }
                    ],
                    "unique": false
                }
            ],
            "properties": {
                "a": {
                    "type": "string",
                    "maxLength": 10,
                    "position": 0,
                },
                "b": {
                    "type": "string",
                    "maxLength": 10,
                    "position": 1,
                },
                "c": {
                    "type": "string",
                    "maxLength": 10,
                    "position": 2,
                },
                "d": {
                    "type": "string",
                    "maxLength": 10,
                    "position": 3,
                }
            },
            "additionalProperties": false,
        });

        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");

        DocumentType::try_from_schema(
            Identifier::random(),
            1,
            config.version(),
            "indexed_type",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            true,
            &mut vec![],
            platform_version,
        )
        .expect("expected to create a document type")
    }

    /// A stored `note`: `hashtag` optional, skipped on by `byHashtagText`.
    fn construct_skip_document_type() -> DocumentType {
        let platform_version = PlatformVersion::latest();
        let schema = platform_value!({
            "type": "object",
            "indices": [
                {
                    "name": "byHashtagText",
                    "properties": [{ "hashtag": "asc" }, { "text": "asc" }],
                    "skipIfAbsent": true
                },
                {
                    "name": "byText",
                    "properties": [{ "text": "asc" }]
                }
            ],
            "properties": {
                "hashtag": { "type": "string", "maxLength": 10, "position": 0 },
                "text": { "type": "string", "maxLength": 10, "position": 1 }
            },
            "required": ["text"],
            "additionalProperties": false,
        });
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");
        DocumentType::try_from_schema(
            Identifier::random(),
            1,
            config.version(),
            "note",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            true,
            &mut vec![],
            platform_version,
        )
        .expect("expected to create a document type")
    }

    fn best_index_name(
        document_type: &DocumentType,
        query_value: serde_json::Value,
    ) -> Result<String, Error> {
        let platform_version = PlatformVersion::latest();
        let contract = get_dpns_data_contract_fixture(None, 0, 1).data_contract_owned();
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type.as_ref(),
            &DriveConfig::default(),
            platform_version,
        )?;
        query
            .find_best_index(platform_version)
            .map(|index| index.name.clone())
    }

    #[test]
    fn should_route_to_a_stored_skip_index_only_when_the_query_excludes_missing_values() {
        let document_type = construct_skip_document_type();
        for (query, routes) in [
            (json!({ "where": [["hashtag", "==", "dash"]] }), true),
            (json!({ "where": [["hashtag", ">", "a"]] }), true),
            (json!({ "where": [["hashtag", "startsWith", "da"]] }), true),
            (json!({ "where": [["hashtag", "in", ["a", "b"]]] }), true),
            // A normal index would return documents without a hashtag first
            // (they sit under the empty key): a skip index cannot serve these.
            (json!({ "where": [["hashtag", "<", "m"]] }), false),
            (
                json!({ "where": [["hashtag", ">", "a"]], "orderBy": [["hashtag", "asc"]] }),
                true,
            ),
            (json!({ "orderBy": [["hashtag", "asc"]] }), false),
        ] {
            let picked = best_index_name(&document_type, query.clone());
            if routes {
                assert_eq!(
                    picked.as_deref().ok(),
                    Some("byHashtagText"),
                    "{query}: expected the skip index, got {picked:?}"
                );
            } else {
                assert!(
                    !matches!(picked.as_deref(), Ok("byHashtagText")),
                    "{query}: must not route to the skip index, got {picked:?}"
                );
            }
        }
        assert_eq!(
            best_index_name(&document_type, json!({ "where": [["text", "==", "x"]] }))
                .expect("byText serves it"),
            "byText"
        );
    }

    #[test]
    fn should_classify_which_where_clauses_exclude_missing_values() {
        let clause = |operator: WhereOperator, value: Value| WhereClause {
            field: "hashtag".to_string(),
            operator,
            value,
        };
        let text = |value: &str| Value::Text(value.to_string());
        for (where_clause, excludes_missing) in [
            (clause(WhereOperator::Equal, text("a")), true),
            (clause(WhereOperator::Equal, Value::Null), false),
            (
                clause(WhereOperator::In, Value::Array(vec![text("a")])),
                true,
            ),
            (
                clause(
                    WhereOperator::In,
                    Value::Array(vec![text("a"), Value::Null]),
                ),
                false,
            ),
            (clause(WhereOperator::GreaterThan, Value::Null), true),
            (clause(WhereOperator::GreaterThanOrEquals, text("a")), true),
            (
                clause(WhereOperator::GreaterThanOrEquals, Value::Null),
                false,
            ),
            (clause(WhereOperator::LessThan, text("m")), false),
            (clause(WhereOperator::LessThanOrEquals, text("m")), false),
            (
                clause(
                    WhereOperator::Between,
                    Value::Array(vec![text("a"), text("m")]),
                ),
                true,
            ),
            (
                clause(
                    WhereOperator::Between,
                    Value::Array(vec![Value::Null, text("m")]),
                ),
                false,
            ),
            (
                clause(
                    WhereOperator::BetweenExcludeLeft,
                    Value::Array(vec![Value::Null, text("m")]),
                ),
                true,
            ),
            (clause(WhereOperator::StartsWith, text("da")), true),
            (clause(WhereOperator::StartsWith, text("")), false),
            // An empty byte array, in any spelling a byteArray property
            // accepts, encodes to the empty key a missing value takes.
            (clause(WhereOperator::Equal, Value::Bytes(vec![])), false),
            (clause(WhereOperator::Equal, Value::Array(vec![])), false),
            (clause(WhereOperator::Equal, text("")), false),
            (clause(WhereOperator::Equal, Value::Bytes(vec![1])), true),
            (
                clause(WhereOperator::GreaterThanOrEquals, Value::Bytes(vec![])),
                false,
            ),
            (
                clause(
                    WhereOperator::In,
                    Value::Array(vec![Value::Bytes(vec![1]), Value::Bytes(vec![])]),
                ),
                false,
            ),
            (
                clause(
                    WhereOperator::Between,
                    Value::Array(vec![Value::Bytes(vec![]), Value::Bytes(vec![9])]),
                ),
                false,
            ),
            // Bytes spell the values of an `in` on a U8 property.
            (clause(WhereOperator::In, Value::Bytes(vec![1, 2])), true),
        ] {
            assert_eq!(
                SkipIfAbsentBinding::for_where_clause(&where_clause).excludes_missing,
                excludes_missing,
                "{where_clause:?}"
            );
        }
    }

    #[test]
    fn test_find_best_index() {
        let document_type = construct_indexed_document_type();
        let contract = get_dpns_data_contract_fixture(None, 0, 1).data_contract_owned();

        let platform_version = PlatformVersion::latest();

        let query_value = json!({
            "where": [
                ["a", "==", "1"],
                ["b", "==", "2"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type.as_ref(),
            &DriveConfig::default(),
            platform_version,
        )
        .expect("query should be valid");
        let index = query
            .find_best_index(platform_version)
            .expect("expected to find index");
        let mut iter = document_type.indexes().iter();
        iter.next();
        iter.next();
        assert_eq!(index, iter.next().unwrap().1); //position 2

        let query_value = json!({
            "where": [
                ["a", "==", "1"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type.as_ref(),
            &DriveConfig::default(),
            platform_version,
        )
        .expect("query should be valid");
        let index = query
            .find_best_index(platform_version)
            .expect("expected to find index");
        assert_eq!(index, document_type.indexes().iter().next().unwrap().1);
    }

    #[test]
    fn test_find_best_index_gapped_equality() {
        // `d == "2"` binds only the LAST property of the [b, a, d] index.
        // The v0 matcher (protocol versions <= 13, frozen on chain) scores
        // it as a set-membership match with difference 2, even though the
        // positional lowering cannot represent the two-property gap. From
        // protocol version 14 the bound fields must cover a contiguous
        // index prefix, so no index matches.
        let document_type = construct_indexed_document_type();
        let contract = get_dpns_data_contract_fixture(None, 0, 1).data_contract_owned();

        let query_value = json!({
            "where": [
                ["d", "==", "2"]
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let protocol_v13 = PlatformVersion::get(13).expect("protocol version 13 exists");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type.as_ref(),
            &DriveConfig::default(),
            protocol_v13,
        )
        .expect("query should be valid");
        let index = query
            .find_best_index(protocol_v13)
            .expect("v13 must keep matching the gapped candidate");
        assert_eq!(
            index,
            document_type.indexes().iter().nth(3).unwrap().1,
            "v13 selects the [b, a, d] index on its last property alone"
        );

        let platform_version = PlatformVersion::latest();
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type.as_ref(),
            &DriveConfig::default(),
            platform_version,
        )
        .expect("query should be valid");
        let error = query
            .find_best_index(platform_version)
            .expect_err("a gapped binding must not match any index");
        assert!(
            matches!(
                &error,
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(_))
            ),
            "expected WhereClauseOnNonIndexedProperty, got {error:?}"
        );
    }

    #[test]
    fn test_find_best_index_error() {
        let document_type = construct_indexed_document_type();
        let contract = get_dpns_data_contract_fixture(None, 0, 1).data_contract_owned();

        let platform_version = PlatformVersion::latest();

        let query_value = json!({
            "where": [
                ["c", "==", "1"]
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type.as_ref(),
            &DriveConfig::default(),
            platform_version,
        )
        .expect("query should be valid");
        let error = query
            .find_best_index(platform_version)
            .expect_err("expected to not find index");
        assert!(
            matches!(error, Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(message)) if message.contains("query must be for valid indexes"))
        )
    }
}
