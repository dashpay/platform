#[cfg(feature = "full")]
#[cfg(test)]
mod tests {
    use dpp::data_contract::document_type::{DocumentType, Index, IndexProperty};
    use dpp::platform_value::Identifier;
    use dpp::util::cbor_serializer;
    use serde_json::json;

    use dpp::data_contract::DataContract;
    use crate::drive::config::DriveConfig;
    use crate::error::{query::QuerySyntaxError, Error};
    use crate::query::DriveQuery;

    fn construct_indexed_document_type() -> DocumentType {
        DocumentType::new(
            Identifier::default(),
            "a".to_string(),
            vec![
                Index {
                    name: "a".to_string(),
                    properties: vec![IndexProperty {
                        name: "a".to_string(),
                        ascending: true,
                    }],
                    unique: false,
                },
                Index {
                    name: "b".to_string(),
                    properties: vec![IndexProperty {
                        name: "b".to_string(),
                        ascending: false,
                    }],
                    unique: false,
                },
                Index {
                    name: "c".to_string(),
                    properties: vec![
                        IndexProperty {
                            name: "b".to_string(),
                            ascending: false,
                        },
                        IndexProperty {
                            name: "a".to_string(),
                            ascending: false,
                        },
                    ],
                    unique: false,
                },
                Index {
                    name: "d".to_string(),
                    properties: vec![
                        IndexProperty {
                            name: "b".to_string(),
                            ascending: false,
                        },
                        IndexProperty {
                            name: "a".to_string(),
                            ascending: false,
                        },
                        IndexProperty {
                            name: "d".to_string(),
                            ascending: false,
                        },
                    ],
                    unique: false,
                },
            ],
            Default::default(),
            Default::default(),
            false,
            false,
        )
    }

    #[test]
    fn test_find_best_index() {
        let document_type = construct_indexed_document_type();
        let contract = Contract::default();

        let query_value = json!({
            "where": [
                ["a", "==", "1"],
                ["b", "==", "2"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            &document_type,
            &DriveConfig::default(),
        )
        .expect("query should be valid");
        let index = query.find_best_index().expect("expected to find index");
        assert_eq!(index, document_type.indices.get(2).unwrap());

        let query_value = json!({
            "where": [
                ["a", "==", "1"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            &document_type,
            &DriveConfig::default(),
        )
        .expect("query should be valid");
        let index = query.find_best_index().expect("expected to find index");
        assert_eq!(index, document_type.indices.get(0).unwrap());
    }

    #[test]
    fn test_find_best_index_error() {
        let document_type = construct_indexed_document_type();
        let contract = Contract::default();

        let query_value = json!({
            "where": [
                ["c", "==", "1"]
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            &document_type,
            &DriveConfig::default(),
        )
        .expect("query should be valid");
        let error = query
            .find_best_index()
            .expect_err("expected to not find index");
        assert!(
            matches!(error, Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(message)) if message == "query must be for valid indexes")
        )
    }
}
