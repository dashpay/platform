//! The where clauses that read the elements of a typed array (protocol
//! version 14), keyed for the aggregate and ranked proof verifiers exactly as
//! the server keys them before it routes the request: `contains` as the
//! equality and `containsAny` as the `In` an index level of the array holds.

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;
use drive::query::{answers_per_element, key_array_element_clauses, WhereClause, WhereOperator};

/// `where_clauses`, read against the document type `document_type_name` of
/// `data_contract`, with every clause reading the elements of a typed array
/// keyed ([`key_array_element_clauses`]).
///
/// `several_elements_allowed` is the server's rule for the request:
/// [`aggregate_answers_per_element`] for a count, sum or average, `false` for
/// a ranked or having query. A verifier then refuses exactly the requests the
/// server refuses, so it never accepts a proof answering a total over several
/// elements, which would count a document once per element it holds.
pub fn key_element_clauses(
    data_contract: &DataContract,
    document_type_name: &str,
    where_clauses: Vec<WhereClause>,
    several_elements_allowed: bool,
    platform_version: &PlatformVersion,
) -> Result<Vec<WhereClause>, drive_proof_verifier::Error> {
    let document_type = data_contract
        .document_type_for_name(document_type_name)
        .map_err(|e| drive_proof_verifier::Error::RequestError {
            error: format!("document type {document_type_name} not found in contract: {e}"),
        })?;
    key_array_element_clauses(
        where_clauses,
        document_type,
        several_elements_allowed,
        platform_version,
    )
    .map_err(|e| drive_proof_verifier::Error::RequestError {
        error: format!("where-clause keying failed: {e}"),
    })
}

/// Whether a count, sum or average request grouped by `group_by` answers one
/// total per element when it reads several elements of a typed array
/// ([`answers_per_element`]): grouped by one field an `In` or a `containsAny`
/// constrains, as drive-abci routes `GroupByIn`, with no range clause.
pub fn aggregate_answers_per_element(group_by: &[String], where_clauses: &[WhereClause]) -> bool {
    let grouped_by_in = match group_by {
        [field] => where_clauses.iter().any(|clause| {
            matches!(
                clause.operator,
                WhereOperator::In | WhereOperator::ContainsAny
            ) && clause.field == *field
        }),
        _ => false,
    };
    answers_per_element(grouped_by_in, where_clauses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::DataContractFactory;
    use dpp::platform_value::{platform_value, Identifier, Value};

    fn post_contract() -> DataContract {
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("factory")
            .create_with_value_config(
                Identifier::new([7u8; 32]),
                0,
                platform_value!({"post": {
                    "type": "object",
                    "properties": {
                        "hashtags": {
                            "type": "array",
                            "maxItems": 10,
                            "items": {"type": "string", "minLength": 1, "maxLength": 61},
                            "position": 0
                        }
                    },
                    "required": ["$createdAt"],
                    "indices": [
                        {"name": "byHashtagTime",
                         "properties": [{"hashtags": "asc"}, {"$createdAt": "asc"}],
                         "countable": "countable"}
                    ],
                    "additionalProperties": false
                }}),
                None,
                None,
            )
            .expect("the post contract is well-formed")
            .data_contract_owned()
    }

    fn several_hashtags() -> Vec<WhereClause> {
        vec![WhereClause {
            field: "hashtags".to_string(),
            operator: WhereOperator::ContainsAny,
            value: platform_value!(["dash", "news"]),
        }]
    }

    fn created_after_zero() -> WhereClause {
        WhereClause {
            field: "$createdAt".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::U64(0),
        }
    }

    #[test]
    fn should_answer_several_elements_only_grouped_by_the_array_alone() {
        let by_hashtags = ["hashtags".to_string()];
        let several = several_hashtags();
        assert!(aggregate_answers_per_element(&by_hashtags, &several));
        assert!(!aggregate_answers_per_element(&[], &several));
        let mut in_a_range = several;
        in_a_range.push(created_after_zero());
        assert!(!aggregate_answers_per_element(&by_hashtags, &in_a_range));
        assert!(!aggregate_answers_per_element(
            &["hashtags".to_string(), "$createdAt".to_string()],
            &in_a_range
        ));
    }

    /// A dishonest node could prove the per-element counts of a total the
    /// server refuses; summed, a post holding both elements would count
    /// twice. The verifier refuses the request by the server's own rule.
    #[test]
    fn should_refuse_to_verify_a_total_over_several_elements() {
        let contract = post_contract();
        let platform_version = PlatformVersion::latest();
        let several = several_hashtags();

        let error = key_element_clauses(
            &contract,
            "post",
            several.clone(),
            aggregate_answers_per_element(&[], &several),
            platform_version,
        )
        .expect_err("an ungrouped total over several elements is refused");
        assert!(
            format!("{error:?}").contains("only answered grouped by"),
            "{error:?}"
        );

        let keyed = key_element_clauses(
            &contract,
            "post",
            several.clone(),
            aggregate_answers_per_element(&["hashtags".to_string()], &several),
            platform_version,
        )
        .expect("grouped by the array, one total per element");
        assert_eq!(keyed[0].operator, WhereOperator::In);

        let one = vec![WhereClause {
            field: "hashtags".to_string(),
            operator: WhereOperator::Contains,
            value: Value::Text("dash".to_string()),
        }];
        let keyed = key_element_clauses(&contract, "post", one, false, platform_version)
            .expect("one element is a total of its own");
        assert_eq!(keyed[0].operator, WhereOperator::Equal);
    }
}
