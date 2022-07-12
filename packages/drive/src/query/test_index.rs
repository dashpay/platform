#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::common;
    use crate::contract::{Contract, DocumentType, Index, IndexProperty};
    use crate::error::{query::QueryError, Error};
    use crate::query::DriveQuery;

    fn construct_indexed_document_type() -> DocumentType {
        DocumentType {
            name: "a".to_string(),
            indices: vec![
                Index {
                    properties: vec![IndexProperty {
                        name: "a".to_string(),
                        ascending: true,
                    }],
                    unique: false,
                },
                Index {
                    properties: vec![IndexProperty {
                        name: "b".to_string(),
                        ascending: false,
                    }],
                    unique: false,
                },
                Index {
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
            properties: Default::default(),
            required_fields: Default::default(),
            documents_keep_history: false,
            documents_mutable: false,
        }
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
        let where_cbor = common::value_to_cbor(query_value, None);
        let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect("query should be valid");
        let index = query.find_best_index().expect("expected to find index");
        assert_eq!(index, document_type.indices.get(2).unwrap());

        let query_value = json!({
            "where": [
                ["a", "==", "1"],
            ]
        });
        let where_cbor = common::value_to_cbor(query_value, None);
        let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
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
        let where_cbor = common::value_to_cbor(query_value, None);
        let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect("query should be valid");
        let error = query
            .find_best_index()
            .expect_err("expected to not find index");
        assert!(
            matches!(error, Error::Query(QueryError::WhereClauseOnNonIndexedProperty(message)) if message == "query must be for valid indexes")
        )
    }
}
