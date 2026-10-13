//! The where clauses that read the elements of a typed array (protocol
//! version 14), keyed for the aggregate and ranked proof verifiers exactly as
//! the server keys them before it routes the request: `contains` as the
//! equality and `containsAny` as the `In` an index level of the array holds.

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;
use drive::query::{
    key_array_element_clauses, validate_and_canonicalize_where_clauses, WhereClause, WhereOperator,
};

/// `where_clauses`, read against the document type `document_type_name` of
/// `data_contract`, with every clause reading the elements of a typed array
/// keyed ([`key_array_element_clauses`]), for a request that reads one
/// element at most: a ranked or having query (`grouped_by_in` false), or a
/// test of the keying itself.
///
/// `grouped_by_in` is whether the request is grouped by its `In` field
/// ([`grouped_by_in_field`]); several elements are read only then, and
/// without a range clause, as the server reads them, so a verifier refuses
/// exactly the requests the server refuses and never accepts a proof
/// answering a total over several elements, which would count a document
/// once per element it holds.
pub fn key_element_clauses(
    data_contract: &DataContract,
    document_type_name: &str,
    where_clauses: Vec<WhereClause>,
    grouped_by_in: bool,
    platform_version: &PlatformVersion,
) -> Result<Vec<WhereClause>, drive_proof_verifier::Error> {
    let document_type = data_contract
        .document_type_for_name(document_type_name)
        .map_err(|e| drive_proof_verifier::Error::RequestError {
            error: format!("document type {document_type_name} not found in contract: {e}"),
        })?;
    key_clauses_of(
        document_type,
        where_clauses,
        grouped_by_in,
        platform_version,
    )
}

/// Whether a request grouped by `group_by` is grouped by one field that an
/// `In` or a `containsAny` constrains: the shape drive-abci routes as
/// `GroupByIn` (its `is_in_field`), which the SDK cannot call.
pub fn grouped_by_in_field(group_by: &[String], where_clauses: &[WhereClause]) -> bool {
    match group_by {
        [field] => where_clauses.iter().any(|clause| {
            matches!(
                clause.operator,
                WhereOperator::In | WhereOperator::ContainsAny
            ) && clause.field == *field
        }),
        _ => false,
    }
}

/// The where clauses of a count, sum or average proof request on
/// `document_type`, as the server's dispatcher reads them before its mode
/// detection: the elements of a typed array keyed, then canonicalized (the
/// shared step in `drive::query::canonicalize`), since the prover merged
/// `[f > A, f < B]` pairs into one `between*` clause before its mode
/// detection, and the verifier's must run over the same shape or a valid
/// proof is refused.
///
/// `folds_entries` is whether the caller adds the verified entries up into
/// one total (`DocumentCount`, `DocumentSum`, `DocumentAverage`): several
/// elements are then refused even grouped by the array, since the total
/// would count a document once per element it holds.
pub(super) fn aggregate_where_clauses(
    document_type: DocumentTypeRef,
    group_by: &[String],
    where_clauses: Vec<WhereClause>,
    folds_entries: bool,
    platform_version: &PlatformVersion,
) -> Result<Vec<WhereClause>, drive_proof_verifier::Error> {
    let grouped_by_in = !folds_entries && grouped_by_in_field(group_by, &where_clauses);
    let keyed = key_clauses_of(
        document_type,
        where_clauses,
        grouped_by_in,
        platform_version,
    )?;
    validate_and_canonicalize_where_clauses(keyed, platform_version).map_err(|e| {
        drive_proof_verifier::Error::RequestError {
            error: format!("where-clause canonicalization failed: {e}"),
        }
    })
}

fn key_clauses_of(
    document_type: DocumentTypeRef,
    where_clauses: Vec<WhereClause>,
    grouped_by_in: bool,
    platform_version: &PlatformVersion,
) -> Result<Vec<WhereClause>, drive_proof_verifier::Error> {
    key_array_element_clauses(
        where_clauses,
        document_type,
        grouped_by_in,
        platform_version,
    )
    .map_err(|e| drive_proof_verifier::Error::RequestError {
        error: format!("where-clause keying failed: {e}"),
    })
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
        let contract = post_contract();
        let platform_version = PlatformVersion::latest();
        let by_hashtags = ["hashtags".to_string()];
        let several = several_hashtags();
        assert!(grouped_by_in_field(&by_hashtags, &several));
        assert!(!grouped_by_in_field(&[], &several));
        assert!(!grouped_by_in_field(
            &["hashtags".to_string(), "$createdAt".to_string()],
            &several
        ));
        // Grouped by the array, a range clause still folds a group's totals.
        let mut in_a_range = several;
        in_a_range.push(created_after_zero());
        key_element_clauses(&contract, "post", in_a_range, true, platform_version)
            .expect_err("several elements under a range are refused");
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
            grouped_by_in_field(&[], &several),
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
            grouped_by_in_field(&["hashtags".to_string()], &several),
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
