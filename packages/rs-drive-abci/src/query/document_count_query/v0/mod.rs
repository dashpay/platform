use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_count_request::GetDocumentsCountRequestV0;
use dapi_grpc::platform::v0::get_documents_count_response::{
    get_documents_count_response_v0, GetDocumentsCountResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::{DriveDocumentCountQuery, DriveDocumentQuery, RangeCountOptions, WhereClause};
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_documents_count_v0(
        &self,
        GetDocumentsCountRequestV0 {
            data_contract_id,
            document_type: document_type_name,
            r#where,
            return_distinct_counts_in_range,
            order_by_ascending,
            limit,
            start_after_split_key,
            prove,
        }: GetDocumentsCountRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsCountResponseV0>, Error> {
        let contract_id: Identifier = check_validation_result_with_data!(data_contract_id
            .try_into()
            .map_err(|_| QueryError::InvalidArgument(
                "id must be a valid identifier (32 bytes long)".to_string()
            )));

        let (_, contract) = self.drive.get_contract_with_fetch_info_and_fee(
            contract_id.to_buffer(),
            None,
            true,
            None,
            platform_version,
        )?;

        let contract = check_validation_result_with_data!(contract.ok_or(QueryError::Query(
            QuerySyntaxError::DataContractNotFound(
                "contract not found when querying from value with contract info",
            )
        )));

        let contract_ref = &contract.contract;

        let document_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(document_type_name.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {}",
                document_type_name, contract_id
            ))));

        let where_clause = if r#where.is_empty() {
            Value::Null
        } else {
            check_validation_result_with_data!(ciborium::de::from_reader(r#where.as_slice())
                .map_err(|_| {
                    QueryError::Query(QuerySyntaxError::DeserializationError(
                        "unable to decode 'where' query from cbor".to_string(),
                    ))
                }))
        };

        // Parse where clauses into WhereClause structs so we can match them against
        // index properties for the CountTree path.
        let all_where_clauses: Vec<WhereClause> =
            check_validation_result_with_data!(match &where_clause {
                Value::Null => Ok(vec![]),
                Value::Array(clauses) => clauses
                    .iter()
                    .map(|wc| {
                        if let Value::Array(components) = wc {
                            WhereClause::from_components(components).map_err(|e| match e {
                                drive::error::Error::Query(qe) => QueryError::Query(qe),
                                other => QueryError::InvalidArgument(format!(
                                    "error parsing where clauses: {}",
                                    other
                                )),
                            })
                        } else {
                            Err(QueryError::Query(
                                QuerySyntaxError::InvalidFormatWhereClause(
                                    "where clause must be an array",
                                ),
                            ))
                        }
                    })
                    .collect::<Result<Vec<WhereClause>, QueryError>>(),
                _ => Err(QueryError::Query(
                    QuerySyntaxError::InvalidFormatWhereClause("where clause must be an array"),
                )),
            });

        let response = if prove {
            // Range-count proof short-circuit: if there's a range
            // operator AND a covering `range_countable` index, generate
            // a grovedb `AggregateCountOnRange` proof. The client
            // verifies via `GroveDb::verify_aggregate_count_query`,
            // recovering `(root_hash, count)` without materializing
            // any matching documents — replaces the u16::MAX cap that
            // the materialize-and-count path needed.
            let range_clause_count = all_where_clauses
                .iter()
                .filter(|wc| DriveDocumentCountQuery::is_range_operator(wc.operator))
                .count();
            if range_clause_count > 0 {
                if range_clause_count > 1 {
                    return Ok(QueryValidationResult::new_with_error(
                        QueryError::InvalidArgument(
                            "count query supports at most one range where-clause".to_string(),
                        ),
                    ));
                }
                if return_distinct_counts_in_range {
                    // The proof primitive (`AggregateCountOnRange`)
                    // returns a single aggregate. Per-distinct-value
                    // entries can't be expressed as a single proof
                    // shape, so reject in prove mode and direct the
                    // caller to `prove = false`.
                    return Ok(QueryValidationResult::new_with_error(
                        QueryError::InvalidArgument(
                            "return_distinct_counts_in_range = true is only supported on the \
                             no-prove path; the proof primitive returns a single aggregate"
                                .to_string(),
                        ),
                    ));
                }
                if all_where_clauses
                    .iter()
                    .any(|wc| wc.operator == drive::query::WhereOperator::In)
                {
                    return Ok(QueryValidationResult::new_with_error(
                        QueryError::InvalidArgument(
                            "range count with `prove = true` does not accept `in` on \
                             prefix properties; use `==` for the prefix"
                                .to_string(),
                        ),
                    ));
                }

                let range_index =
                    DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
                        document_type.indexes(),
                        &all_where_clauses,
                    );
                let Some(index) = range_index else {
                    return Ok(QueryValidationResult::new_with_error(
                        QueryError::InvalidArgument(
                            "range count requires a `range_countable: true` index whose last \
                             property matches the range field"
                                .to_string(),
                        ),
                    ));
                };

                let count_query = DriveDocumentCountQuery {
                    document_type,
                    contract_id: contract_id.to_buffer(),
                    document_type_name: document_type_name.clone(),
                    index,
                    where_clauses: all_where_clauses.clone(),
                    split_by_property: None,
                };
                let proof = match count_query.execute_aggregate_count_with_proof(
                    &self.drive,
                    None,
                    platform_version,
                ) {
                    Ok(p) => p,
                    Err(drive::error::Error::Query(qe)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(qe)));
                    }
                    Err(e) => return Err(e.into()),
                };
                let (grovedb_used, proof) =
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;
                return Ok(QueryValidationResult::new_with_data(
                    GetDocumentsCountResponseV0 {
                        result: Some(get_documents_count_response_v0::Result::Proof(proof)),
                        metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
                    },
                ));
            }

            // No range operator → fall back to the materialize-and-
            // count proof path. This still has the u16::MAX cap
            // because grovedb's aggregate primitive doesn't apply to
            // pure point-lookup count queries (each value tree is a
            // CountTree, but the per-CountTree count proof is a
            // separate primitive that's not yet wired through). For
            // larger point-lookup counts, callers should use
            // `prove = false` with a covering countable index.
            let mut drive_query =
                check_validation_result_with_data!(DriveDocumentQuery::from_decomposed_values(
                    where_clause,
                    None,
                    Some(self.config.drive.default_query_limit),
                    None,
                    true,
                    None,
                    contract_ref,
                    document_type,
                    &self.config.drive,
                ));
            drive_query.limit = Some(u16::MAX);

            let proof =
                match drive_query.execute_with_proof(&self.drive, None, None, platform_version) {
                    Ok(result) => result.0,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            // Detect range operators. If any are present we route to the
            // range-countable count path (`execute_range_count_no_proof`)
            // instead of the Equal/In fast path. Range queries require
            // both a `range_countable` index AND that no `In` clause is
            // present (mixing per-value split with range walk produces
            // ambiguous output — caller should split client-side).
            let range_clause_count = all_where_clauses
                .iter()
                .filter(|wc| DriveDocumentCountQuery::is_range_operator(wc.operator))
                .count();
            if range_clause_count > 0 {
                if range_clause_count > 1 {
                    return Ok(QueryValidationResult::new_with_error(
                        QueryError::InvalidArgument(
                            "count query supports at most one range where-clause; combine \
                             two-sided ranges via `between*` instead of separate `>` / `<` \
                             clauses"
                                .to_string(),
                        ),
                    ));
                }
                if all_where_clauses
                    .iter()
                    .any(|wc| wc.operator == drive::query::WhereOperator::In)
                {
                    return Ok(QueryValidationResult::new_with_error(
                        QueryError::InvalidArgument(
                            "range count queries cannot also carry an `in` clause; pick \
                             either per-value split (In) or per-distinct-value range \
                             (return_distinct_counts_in_range)"
                                .to_string(),
                        ),
                    ));
                }

                let range_index =
                    DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
                        document_type.indexes(),
                        &all_where_clauses,
                    );
                let Some(index) = range_index else {
                    return Ok(QueryValidationResult::new_with_error(
                        QueryError::InvalidArgument(
                            "range count requires a `range_countable: true` index whose last \
                             property matches the range field, with all other clauses \
                             covering its prefix as `==` matches"
                                .to_string(),
                        ),
                    ));
                };

                // Server-side limit clamp matches the docs/Documents query
                // behavior: clients may request more than the configured
                // ceiling but the server enforces it.
                let effective_limit =
                    limit.map(|requested| requested.min(self.config.drive.max_query_limit as u32));

                let count_query = DriveDocumentCountQuery {
                    document_type,
                    contract_id: contract_id.to_buffer(),
                    document_type_name: document_type_name.clone(),
                    index,
                    where_clauses: all_where_clauses,
                    split_by_property: None,
                };

                let options = RangeCountOptions {
                    distinct: return_distinct_counts_in_range,
                    limit: effective_limit,
                    start_after_split_key,
                    // Default to ascending — `order_by_ascending` is an
                    // optional bool on the wire, so an unset value means
                    // "use the natural BTreeMap order".
                    order_by_ascending: order_by_ascending.unwrap_or(true),
                };
                let entries: Vec<get_documents_count_response_v0::CountEntry> = count_query
                    .execute_range_count_no_proof(&self.drive, &options, None, platform_version)?
                    .into_iter()
                    .map(|e| get_documents_count_response_v0::CountEntry {
                        key: e.key,
                        count: e.count,
                    })
                    .collect();

                return Ok(QueryValidationResult::new_with_data(
                    GetDocumentsCountResponseV0 {
                        result: Some(get_documents_count_response_v0::Result::Counts(
                            get_documents_count_response_v0::CountResults { entries },
                        )),
                        metadata: Some(
                            self.response_metadata_v0(platform_state, CheckpointUsed::Current),
                        ),
                    },
                ));
            }

            // No range operators → traditional Equal/In path. Reject any
            // other unsupported operator (defense in depth — should be
            // unreachable given the range branch above, but `is_range_operator`
            // and `has_unsupported_operator` are independent checks).
            if DriveDocumentCountQuery::has_unsupported_operator(&all_where_clauses) {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(
                        "count query supports only `==`, `in`, and range operators".to_string(),
                    ),
                ));
            }

            // Reject return_distinct_counts_in_range with no range
            // clause — the flag has no defined meaning without a range.
            if return_distinct_counts_in_range {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(
                        "return_distinct_counts_in_range requires a range where-clause".to_string(),
                    ),
                ));
            }

            // Determine split mode from the where clauses. The unified count
            // endpoint uses an `In` clause as the per-value split signal: at
            // most one `In` is allowed per query, and the In's array becomes
            // the entries in the response (one CountEntry per value, each
            // computed as the count of docs matching that single value).
            // No In clause → total count, single entry with empty key.
            let in_clauses: Vec<&WhereClause> = all_where_clauses
                .iter()
                .filter(|wc| wc.operator == drive::query::WhereOperator::In)
                .collect();
            if in_clauses.len() > 1 {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(
                        "count query supports at most one `in` where-clause; \
                         the In carries the split property and only one split \
                         dimension is supported per request"
                            .to_string(),
                    ),
                ));
            }

            let entries: Vec<get_documents_count_response_v0::CountEntry> = if let Some(in_clause) =
                in_clauses.first().cloned()
            {
                // Per-In-value entries. Replace the In with an Equal on each
                // listed value, ask rs-drive for the count of that single
                // value, and emit a (serialized_value, count) entry. Same
                // value-key encoding as the no-In code path produces (via
                // `serialize_value_for_key`), so wire keys round-trip
                // consistently between modes.
                let in_values =
                    check_validation_result_with_data!(in_clause.value.as_array().ok_or_else(
                        || QueryError::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                            "In where-clause value must be an array",
                        ))
                    ));

                let other_clauses: Vec<WhereClause> = all_where_clauses
                    .iter()
                    .filter(|wc| wc.operator != drive::query::WhereOperator::In)
                    .cloned()
                    .collect();

                let mut entries = Vec::with_capacity(in_values.len());
                let mut seen_keys: std::collections::BTreeSet<Vec<u8>> = Default::default();
                for value in in_values {
                    // Pre-serialize to use as the entry key AND dedupe so a
                    // duplicated In value doesn't produce two entries.
                    let key_bytes = document_type.serialize_value_for_key(
                        in_clause.field.as_str(),
                        value,
                        platform_version,
                    )?;
                    if !seen_keys.insert(key_bytes.clone()) {
                        continue;
                    }

                    let mut clauses_for_value = other_clauses.clone();
                    clauses_for_value.push(WhereClause {
                        field: in_clause.field.clone(),
                        operator: drive::query::WhereOperator::Equal,
                        value: value.clone(),
                    });

                    let countable_index =
                        DriveDocumentCountQuery::find_countable_index_for_where_clauses(
                            document_type.indexes(),
                            &clauses_for_value,
                        );
                    let Some(index) = countable_index else {
                        return Ok(QueryValidationResult::new_with_error(
                            QueryError::InvalidArgument(
                                "count query requires a countable index on the document \
                                 type that matches the where clause properties"
                                    .to_string(),
                            ),
                        ));
                    };

                    let count_query = DriveDocumentCountQuery {
                        document_type,
                        contract_id: contract_id.to_buffer(),
                        document_type_name: document_type_name.clone(),
                        index,
                        where_clauses: clauses_for_value,
                        split_by_property: None,
                    };
                    let results =
                        count_query.execute_no_proof(&self.drive, None, platform_version)?;
                    let count = results.first().map_or(0, |entry| entry.count);

                    entries.push(get_documents_count_response_v0::CountEntry {
                        key: key_bytes,
                        count,
                    });
                }
                entries
            } else {
                // No In clause → total count. Single entry with empty key.
                let countable_index =
                    DriveDocumentCountQuery::find_countable_index_for_where_clauses(
                        document_type.indexes(),
                        &all_where_clauses,
                    );
                let Some(index) = countable_index else {
                    return Ok(QueryValidationResult::new_with_error(
                        QueryError::InvalidArgument(
                            "count query requires a countable index on the document type \
                             that matches the where clause properties"
                                .to_string(),
                        ),
                    ));
                };
                let count_query = DriveDocumentCountQuery {
                    document_type,
                    contract_id: contract_id.to_buffer(),
                    document_type_name: document_type_name.clone(),
                    index,
                    where_clauses: all_where_clauses,
                    split_by_property: None,
                };
                let results = count_query.execute_no_proof(&self.drive, None, platform_version)?;
                vec![get_documents_count_response_v0::CountEntry {
                    key: Vec::new(),
                    count: results.first().map_or(0, |e| e.count),
                }]
            };

            GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Counts(
                    get_documents_count_response_v0::CountResults { entries },
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{setup_platform, store_data_contract, store_document};
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_documents_count_no_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        store_data_contract(&platform, &data_contract, version);

        let data_contract_id = data_contract.id();
        let document_type_name = "person";
        let document_type = data_contract
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(500);
        for _ in 0..5 {
            let random_document = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");
            store_document(
                &platform,
                &data_contract,
                document_type,
                &random_document,
                platform_version,
            );
        }

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            return_distinct_counts_in_range: false,
            order_by_ascending: None,
            limit: None,
            start_after_split_key: None,
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Counts(counts)),
                metadata: Some(_),
            }) => {
                let total: u64 = counts.entries.iter().map(|e| e.count).sum();
                assert_eq!(total, 5, "expected count of 5 documents");
            }
            other => panic!("expected count result, got {:?}", other),
        }
    }

    #[test]
    fn test_documents_count_empty_result() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        store_data_contract(&platform, &data_contract, version);

        let data_contract_id = data_contract.id();
        let document_type_name = "person";

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            return_distinct_counts_in_range: false,
            order_by_ascending: None,
            limit: None,
            start_after_split_key: None,
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Counts(counts)),
                metadata: Some(_),
            }) => {
                let total: u64 = counts.entries.iter().map(|e| e.count).sum();
                assert_eq!(total, 0, "expected count of 0 documents");
            }
            other => panic!("expected count result, got {:?}", other),
        }
    }

    fn serialize_where_clauses_to_cbor(where_clauses: Vec<Value>) -> Vec<u8> {
        use ciborium::value::Value as CborValue;
        let cbor: CborValue = TryInto::<CborValue>::try_into(Value::Array(where_clauses))
            .expect("expected to convert where clauses to cbor value");
        let mut out = Vec::new();
        ciborium::ser::into_writer(&cbor, &mut out).expect("expected to serialize where clauses");
        out
    }

    fn store_person_document(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        data_contract: &dpp::prelude::DataContract,
        id: [u8; 32],
        first_name: &str,
        last_name: &str,
        age: u64,
        platform_version: &PlatformVersion,
    ) {
        use dpp::document::{Document, DocumentV0};
        use std::collections::BTreeMap;

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let mut properties = BTreeMap::new();
        properties.insert("firstName".to_string(), Value::Text(first_name.to_string()));
        properties.insert("lastName".to_string(), Value::Text(last_name.to_string()));
        properties.insert("age".to_string(), Value::U64(age));

        let document: Document = DocumentV0 {
            id: Identifier::from(id),
            owner_id: Identifier::from([0u8; 32]),
            properties,
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();

        store_document(
            platform,
            data_contract,
            document_type,
            &document,
            platform_version,
        );
    }

    #[test]
    fn test_documents_count_with_in_operator() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        store_data_contract(&platform, &data_contract, version);

        // 3 docs with age=30, 2 with age=40, 1 with age=50.
        store_person_document(
            &platform,
            &data_contract,
            [1u8; 32],
            "Alice",
            "Smith",
            30,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [2u8; 32],
            "Bob",
            "Smith",
            30,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [3u8; 32],
            "Carol",
            "Smith",
            30,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [4u8; 32],
            "Dave",
            "Smith",
            40,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [5u8; 32],
            "Eve",
            "Smith",
            40,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [6u8; 32],
            "Frank",
            "Smith",
            50,
            platform_version,
        );

        // [["age", "in", [30, 40]]]
        let where_clauses = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text("in".to_string()),
            Value::Array(vec![Value::U64(30), Value::U64(40)]),
        ])];

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract.id().to_vec(),
            document_type: "person".to_string(),
            r#where: serialize_where_clauses_to_cbor(where_clauses),
            return_distinct_counts_in_range: false,
            order_by_ascending: None,
            limit: None,
            start_after_split_key: None,
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Counts(counts)),
                metadata: Some(_),
            }) => {
                let total: u64 = counts.entries.iter().map(|e| e.count).sum();
                assert_eq!(total, 5, "expected count of 5 (3 age=30 + 2 age=40)");
            }
            other => panic!("expected count result, got {:?}", other),
        }
    }

    #[test]
    fn test_documents_count_range_without_range_countable_index_returns_clear_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        store_data_contract(&platform, &data_contract, version);

        // [["age", ">", 20]] — range operator on a contract whose `age`
        // index is `countable` but NOT `range_countable`. The range
        // path now accepts range operators, but the picker must report
        // "no usable index" so the handler surfaces a clear error.
        let where_clauses = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text(">".to_string()),
            Value::U64(20),
        ])];

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract.id().to_vec(),
            document_type: "person".to_string(),
            r#where: serialize_where_clauses_to_cbor(where_clauses),
            return_distinct_counts_in_range: false,
            order_by_ascending: None,
            limit: None,
            start_after_split_key: None,
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to return validation error");

        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::InvalidArgument(msg)] if msg.contains("range_countable")
            ),
            "expected range_countable-index rejection, got {:?}",
            result.errors
        );
    }

    #[test]
    fn test_documents_count_with_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        store_data_contract(&platform, &data_contract, version);

        let data_contract_id = data_contract.id();
        let document_type_name = "person";
        let document_type = data_contract
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(500);
        for _ in 0..3 {
            let random_document = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");
            store_document(
                &platform,
                &data_contract,
                document_type,
                &random_document,
                platform_version,
            );
        }

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            return_distinct_counts_in_range: false,
            order_by_ascending: None,
            limit: None,
            start_after_split_key: None,
            prove: true,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        assert!(matches!(
            result.data,
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
